use crate::{
    history,
    models::{DownloadTask, Video},
    parser::valid_media_url,
};
use md5::{Digest, Md5};
use reqwest::Client;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Semaphore,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct TaskEntry {
    task: DownloadTask,
    cancel: CancellationToken,
}
pub struct DownloadManager {
    tasks: Mutex<HashMap<String, TaskEntry>>,
    slots: Arc<Semaphore>,
    history_path: PathBuf,
}
impl DownloadManager {
    pub fn load(history_path: PathBuf) -> Result<Self, String> {
        let tasks = history::load(&history_path)?
            .into_iter()
            .map(|task| {
                (
                    task.id.clone(),
                    TaskEntry {
                        task,
                        cancel: CancellationToken::new(),
                    },
                )
            })
            .collect();
        Ok(Self {
            tasks: Mutex::new(tasks),
            slots: Arc::new(Semaphore::new(2)),
            history_path,
        })
    }
    fn snapshot(tasks: &HashMap<String, TaskEntry>) -> Vec<DownloadTask> {
        let mut result: Vec<_> = tasks.values().map(|entry| entry.task.clone()).collect();
        result.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        result
    }
    pub fn list(&self) -> Result<Vec<DownloadTask>, String> {
        let tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        Ok(Self::snapshot(&tasks))
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        let entry = tasks.get(id).ok_or("下载任务不存在")?;
        if entry.task.status == "queued" || entry.task.status == "running" {
            entry.cancel.cancel();
        }
        Ok(())
    }
    pub fn remove(&self, id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        let entry = tasks.get(id).ok_or("下载记录不存在")?;
        if entry.task.status == "queued" || entry.task.status == "running" {
            return Err("请先取消下载，再删除记录".into());
        }
        let remaining: Vec<_> = Self::snapshot(&tasks)
            .into_iter()
            .filter(|task| task.id != id)
            .collect();
        history::save(&self.history_path, &remaining)?;
        tasks.remove(id);
        Ok(())
    }

    pub fn completed_path(&self, id: &str) -> Result<String, String> {
        let tasks = self.tasks.lock().map_err(|e| e.to_string())?;
        let entry = tasks.get(id).ok_or("下载任务不存在")?;
        if entry.task.status != "completed" {
            return Err("文件尚未下载完成".into());
        }
        if !std::path::Path::new(&entry.task.path).is_file() {
            return Err("文件已被移动或删除".into());
        }
        Ok(entry.task.path.clone())
    }
    fn update(&self, app: &AppHandle, id: &str, f: impl FnOnce(&mut DownloadTask)) {
        if let Ok(mut tasks) = self.tasks.lock() {
            if let Some(entry) = tasks.get_mut(id) {
                let previous_status = entry.task.status.clone();
                f(&mut entry.task);
                let status_changed = entry.task.status != previous_status;
                let _ = app.emit("download-progress", &entry.task);
                // Save state transitions, without writing each progress tick to disk.
                if status_changed {
                    if let Err(error) = history::save(&self.history_path, &Self::snapshot(&tasks)) {
                        let _ = app.emit("download-history-error", error);
                    }
                }
            }
        }
    }
}
pub fn start(
    app: AppHandle,
    manager: Arc<DownloadManager>,
    video: Video,
    variant_index: usize,
    directory: String,
) -> Result<String, String> {
    let variant = video
        .variants
        .get(variant_index)
        .ok_or("请选择有效的视频画质")?;
    if !valid_media_url(&variant.url) {
        return Err("不支持此视频地址".into());
    }
    if video.id.len() > 80
        || video.id.is_empty()
        || !video.id.bytes().all(|c| c.is_ascii_digit() || c == b'_')
    {
        return Err("视频 ID 不合法".into());
    }
    let dir = PathBuf::from(directory);
    if !dir.is_absolute() {
        return Err("下载目录必须是绝对路径".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建下载目录：{e}"))?;
    let id = Uuid::new_v4().to_string();
    let filename = format!("{}_{}.mp4", video.id, &id[..8]);
    let path = dir.join(&filename);
    let cancel = CancellationToken::new();
    let task = DownloadTask {
        id: id.clone(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        title: video.title,
        thumbnail: video.thumbnail,
        filename,
        path: path.to_string_lossy().into_owned(),
        status: "queued".into(),
        downloaded: 0,
        total: None,
        speed: 0,
        error: None,
    };
    {
        let mut tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
        tasks.insert(
            id.clone(),
            TaskEntry {
                task: task.clone(),
                cancel: cancel.clone(),
            },
        );
        if let Err(error) = history::save(&manager.history_path, &DownloadManager::snapshot(&tasks))
        {
            tasks.remove(&id);
            return Err(error);
        }
    }
    let _ = app.emit("download-progress", &task);
    let url = variant.url.clone();
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let result = run(&app, &manager, &task_id, &url, &path, &cancel).await;
        match result {
            Ok(final_path) => manager.update(&app, &task_id, |task| {
                task.filename = final_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                task.path = final_path.to_string_lossy().into_owned();
                task.status = "completed".into();
                task.speed = 0;
            }),
            Err(message) => {
                let cancelled = message == "下载已取消";
                manager.update(&app, &task_id, |task| {
                    task.status = if cancelled { "cancelled" } else { "failed" }.into();
                    task.speed = 0;
                    task.error = if cancelled { None } else { Some(message) };
                });
            }
        }
    });
    Ok(id)
}
async fn run(
    app: &AppHandle,
    manager: &DownloadManager,
    id: &str,
    url: &str,
    path: &std::path::Path,
    cancel: &CancellationToken,
) -> Result<PathBuf, String> {
    let _permit = tokio::select! {
        _ = cancel.cancelled() => return Err("下载已取消".into()),
        permit = manager.slots.clone().acquire_owned() => permit.map_err(|_| "下载队列已关闭")?,
    };
    manager.update(app, id, |task| task.status = "running".into());
    let partial = path.with_extension("mp4.part");
    // create_new avoids overwriting any existing file, including an unfinished task.
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .await
        .map_err(|e| format!("无法创建下载文件：{e}"))?;
    let result = transfer(app, manager, id, url, &mut file, cancel).await;
    drop(file);
    let hash = match result {
        Ok(hash) => hash,
        Err(error) => {
            return match tokio::fs::remove_file(&partial).await {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}；临时文件清理失败：{cleanup}")),
            };
        }
    };
    if cancel.is_cancelled() {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err("下载已取消".into());
    }
    let directory = path.parent().ok_or("下载目录无效")?;
    let result = publish_hashed_file(&partial, directory, &hash).await;
    let _ = tokio::fs::remove_file(&partial).await;
    result
}
async fn publish_hashed_file(
    partial: &std::path::Path,
    directory: &std::path::Path,
    hash: &str,
) -> Result<PathBuf, String> {
    let destination = directory.join(format!("{hash}.mp4"));
    match tokio::fs::hard_link(partial, &destination).await {
        Ok(()) => Ok(destination),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if same_contents(partial, &destination).await? {
                Ok(destination)
            } else {
                Err("同名 MD5 文件已存在但内容不同，未覆盖原文件".into())
            }
        }
        Err(error) => Err(format!("保存最终文件失败：{error}")),
    }
}

async fn same_contents(left: &std::path::Path, right: &std::path::Path) -> Result<bool, String> {
    let mut left = tokio::fs::File::open(left)
        .await
        .map_err(|e| e.to_string())?;
    let mut right = tokio::fs::File::open(right)
        .await
        .map_err(|e| e.to_string())?;
    if left.metadata().await.map_err(|e| e.to_string())?.len()
        != right.metadata().await.map_err(|e| e.to_string())?.len()
    {
        return Ok(false);
    }
    let mut left_buffer = vec![0; 64 * 1024];
    let mut right_buffer = vec![0; 64 * 1024];
    loop {
        let read = left
            .read(&mut left_buffer)
            .await
            .map_err(|e| e.to_string())?;
        if read == 0 {
            return Ok(true);
        }
        right
            .read_exact(&mut right_buffer[..read])
            .await
            .map_err(|e| e.to_string())?;
        if left_buffer[..read] != right_buffer[..read] {
            return Ok(false);
        }
    }
}

async fn transfer(
    app: &AppHandle,
    manager: &DownloadManager,
    id: &str,
    url: &str,
    file: &mut tokio::fs::File,
    cancel: &CancellationToken,
) -> Result<String, String> {
    // A separate client never sends X session headers to media hosts.
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法创建下载连接")?;
    let mut response = tokio::select! {
        _ = cancel.cancelled() => return Err("下载已取消".into()),
        result = client.get(url).send() => result.map_err(|_| "连接视频服务器失败，请重试")?,
    };
    if !response.status().is_success() {
        return Err(format!(
            "视频服务器返回 HTTP {}，请重新解析后重试",
            response.status()
        ));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !(content_type.starts_with("video/") || content_type.starts_with("application/octet-stream"))
    {
        return Err("视频服务器返回了非视频内容".into());
    }
    let total = response.content_length();
    manager.update(app, id, |task| task.total = total);
    let mut hasher = Md5::new();
    let mut downloaded = 0u64;
    let mut last_bytes = 0u64;
    let mut last_update = Instant::now();
    loop {
        let chunk = tokio::select! {
            _ = cancel.cancelled() => return Err("下载已取消".into()),
            result = response.chunk() => result.map_err(|_| "下载中断或超时，请重新添加下载")?,
        };
        let Some(chunk) = chunk else { break };
        tokio::select! {
            _ = cancel.cancelled() => return Err("下载已取消".into()),
            result = file.write_all(&chunk) => result.map_err(|e| format!("写入文件失败：{e}"))?,
        }
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        let elapsed = last_update.elapsed().as_secs_f64();
        if elapsed >= 0.2 {
            let speed = ((downloaded - last_bytes) as f64 / elapsed) as u64;
            manager.update(app, id, |task| {
                task.downloaded = downloaded;
                task.speed = speed;
            });
            last_bytes = downloaded;
            last_update = Instant::now();
        }
    }
    if downloaded == 0 || total.is_some_and(|size| downloaded != size) {
        return Err("视频数据不完整，请重新添加下载".into());
    }
    file.flush()
        .await
        .map_err(|e| format!("写入文件失败：{e}"))?;
    manager.update(app, id, |task| task.downloaded = downloaded);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_history_keeps_video_and_survives_restart() {
        let root = std::env::temp_dir().join(format!("shiying-remove-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let video = root.join("video.mp4");
        std::fs::write(&video, b"example video").unwrap();
        let history_path = root.join("downloads.json");
        let task = DownloadTask {
            id: "completed".into(),
            created_at: 1,
            title: "Example".into(),
            thumbnail: String::new(),
            filename: "video.mp4".into(),
            path: video.to_string_lossy().into_owned(),
            status: "completed".into(),
            downloaded: 13,
            total: Some(13),
            speed: 0,
            error: None,
        };
        history::save(&history_path, &[task]).unwrap();
        let manager = DownloadManager::load(history_path.clone()).unwrap();
        manager
            .tasks
            .lock()
            .unwrap()
            .get_mut("completed")
            .unwrap()
            .task
            .status = "running".into();
        assert!(manager.remove("completed").is_err());
        assert_eq!(manager.list().unwrap().len(), 1);
        manager
            .tasks
            .lock()
            .unwrap()
            .get_mut("completed")
            .unwrap()
            .task
            .status = "completed".into();
        // A write failure must keep the record in memory.
        std::fs::remove_file(&history_path).unwrap();
        std::fs::create_dir(&history_path).unwrap();
        assert!(manager.remove("completed").is_err());
        assert_eq!(manager.list().unwrap().len(), 1);
        std::fs::remove_dir(&history_path).unwrap();
        manager.remove("completed").unwrap();
        assert!(manager.list().unwrap().is_empty());
        assert!(DownloadManager::load(history_path)
            .unwrap()
            .list()
            .unwrap()
            .is_empty());
        assert_eq!(std::fs::read(&video).unwrap(), b"example video");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn md5_filename_reuses_identical_file_without_overwriting_other_content() {
        let root = std::env::temp_dir().join(format!("shiying-md5-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root).await.unwrap();
        let partial = root.join("download.part");
        tokio::fs::write(&partial, b"abc").await.unwrap();
        let mut hasher = Md5::new();
        hasher.update(b"a");
        hasher.update(b"bc");
        let hash = format!("{:x}", hasher.finalize());
        let destination = publish_hashed_file(&partial, &root, &hash).await.unwrap();
        assert_eq!(
            destination.file_name().unwrap(),
            "900150983cd24fb0d6963f7d28e17f72.mp4"
        );
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), b"abc");
        assert_eq!(
            publish_hashed_file(&partial, &root, &hash).await.unwrap(),
            destination
        );
        tokio::fs::remove_file(&partial).await.unwrap();
        tokio::fs::write(&partial, b"xyz").await.unwrap();
        assert!(publish_hashed_file(&partial, &root, &hash).await.is_err());
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), b"abc");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
