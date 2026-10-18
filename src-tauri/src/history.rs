use crate::models::DownloadTask;
use std::path::Path;

pub fn load(path: &Path) -> Result<Vec<DownloadTask>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("读取下载记录失败：{error}")),
    };
    let mut tasks: Vec<DownloadTask> =
        serde_json::from_str(&text).map_err(|e| format!("下载记录文件损坏：{e}"))?;
    for task in &mut tasks {
        task.speed = 0;
        if task.status == "queued" || task.status == "running" {
            task.status = "failed".into();
            task.error = Some("应用关闭，下载已中断，请重新解析下载".into());
        }
    }
    tasks.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(tasks)
}

pub fn save(path: &Path, tasks: &[DownloadTask]) -> Result<(), String> {
    let parent = path.parent().ok_or("下载记录路径无效")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("创建下载记录目录失败：{e}"))?;
    let temporary = parent.join(format!(".downloads-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(tasks).map_err(|e| e.to_string())?;
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("保存下载记录失败：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, status: &str, created_at: u64) -> DownloadTask {
        DownloadTask {
            id: id.into(),
            title: "视频标题".into(),
            thumbnail: "https://pbs.twimg.com/example.jpg".into(),
            filename: format!("{id}.mp4"),
            path: format!("C:/Downloads/{id}.mp4"),
            status: status.into(),
            downloaded: 1024,
            total: Some(2048),
            speed: 512,
            error: None,
            created_at,
        }
    }

    #[test]
    fn restart_retains_records_and_marks_unfinished_tasks_interrupted() {
        let dir = std::env::temp_dir().join(format!("shiying-history-{}", uuid::Uuid::new_v4()));
        let path = dir.join("downloads.json");
        assert!(load(&path).unwrap().is_empty());
        save(
            &path,
            &[
                task("done", "completed", 1),
                task("active", "running", 3),
                task("waiting", "queued", 2),
            ],
        )
        .unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(
            loaded.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            vec!["active", "waiting", "done"]
        );
        assert_eq!(loaded[2].status, "completed");
        assert_eq!(loaded[2].thumbnail, "https://pbs.twimg.com/example.jpg");
        assert_eq!(loaded[2].path, "C:/Downloads/done.mp4");
        for task in &loaded[..2] {
            assert_eq!(task.status, "failed");
            assert_eq!(task.speed, 0);
            assert!(task.error.as_ref().unwrap().contains("已中断"));
        }
        save(&path, &[task("done", "completed", 1)]).unwrap();
        assert_eq!(load(&path).unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn older_records_without_thumbnail_or_timestamp_still_load() {
        let mut json = serde_json::to_value(task("old", "cancelled", 0)).unwrap();
        json.as_object_mut().unwrap().remove("thumbnail");
        json.as_object_mut().unwrap().remove("createdAt");
        let restored: DownloadTask = serde_json::from_value(json).unwrap();
        assert!(restored.thumbnail.is_empty());
        assert_eq!(restored.created_at, 0);
    }
}
