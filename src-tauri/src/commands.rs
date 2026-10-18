use crate::{
    downloader::{self, DownloadManager},
    models::{DownloadTask, Settings, SettingsView, Video},
    parser,
    settings::{self, AppSettings},
};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_settings(state: State<'_, AppSettings>) -> Result<SettingsView, String> {
    settings::view(&state)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    manager: State<'_, Arc<DownloadManager>>,
    state: State<'_, AppSettings>,
    settings: Settings,
    cookie: Option<String>,
    authorization: Option<String>,
) -> Result<SettingsView, String> {
    let view = settings::save(&app, &state, settings, cookie, authorization)?;
    manager.set_concurrency(view.settings.max_concurrent_downloads)?;
    Ok(view)
}

#[tauri::command]
pub async fn parse_video(
    state: State<'_, AppSettings>,
    input: String,
) -> Result<Vec<Video>, String> {
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    let authorization = state
        .authorization
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let query_id = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .query_id
        .clone();
    parser::parse(input, cookie, authorization, query_id).await
}

#[tauri::command]
pub fn start_download(
    app: AppHandle,
    settings: State<'_, AppSettings>,
    manager: State<'_, Arc<DownloadManager>>,
    video: Video,
    variant_index: usize,
) -> Result<String, String> {
    let directory = settings
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .download_dir
        .clone();
    downloader::start(
        app,
        manager.inner().clone(),
        video,
        variant_index,
        directory,
    )
}

#[tauri::command]
pub fn list_tasks(manager: State<'_, Arc<DownloadManager>>) -> Result<Vec<DownloadTask>, String> {
    manager.list()
}

#[tauri::command]
pub fn cancel_download(manager: State<'_, Arc<DownloadManager>>, id: String) -> Result<(), String> {
    manager.cancel(&id)
}

#[tauri::command]
pub fn reveal_download(
    app: AppHandle,
    manager: State<'_, Arc<DownloadManager>>,
    id: String,
) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(manager.completed_path(&id)?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_download(manager: State<'_, Arc<DownloadManager>>, id: String) -> Result<(), String> {
    manager.remove(&id)
}
