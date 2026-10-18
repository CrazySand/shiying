mod commands;
mod downloader;
mod history;
mod models;
mod parser;
mod settings;
mod storage;

use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    let command_handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
        commands::get_settings,
        commands::save_settings,
        commands::parse_video,
        commands::start_download,
        commands::list_tasks,
        commands::cancel_download,
        commands::remove_download,
        commands::reveal_download,
    ];
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            storage::initialize(app.handle()).map_err(std::io::Error::other)?;
            app.manage(settings::load(app.handle()).map_err(std::io::Error::other)?);
            app.manage(Arc::new(
                downloader::DownloadManager::load(
                    storage::path(app.handle(), "downloads.json").map_err(std::io::Error::other)?,
                )
                .map_err(std::io::Error::other)?,
            ));
            Ok(())
        })
        .invoke_handler(move |invoke| {
            if invoke.message.webview_ref().label() != "main" {
                invoke.resolver.reject("此窗口无权调用本地应用功能");
                return true;
            }
            command_handler(invoke)
        })
        .run(tauri::generate_context!())
        .expect("拾影启动失败");
}
