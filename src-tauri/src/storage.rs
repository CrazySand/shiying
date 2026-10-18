use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle, filename: &str) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|root| root.join(filename))
        .map_err(|e| e.to_string())
}

pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let local = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let previous = app.path().app_config_dir().map_err(|e| e.to_string())?;
    migrate(&previous, &local)
}

fn migrate(previous: &Path, local: &Path) -> Result<(), String> {
    std::fs::create_dir_all(local).map_err(|e| format!("创建应用数据目录失败：{e}"))?;
    if previous == local {
        return Ok(());
    }
    for filename in ["settings.json", "downloads.json"] {
        let source = previous.join(filename);
        let destination = local.join(filename);
        if !destination.exists() && source.is_file() {
            let temporary = local.join(format!(".migration-{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| {
                std::fs::copy(&source, &temporary)?;
                std::fs::rename(&temporary, &destination)
            })();
            if let Err(error) = result {
                let _ = std::fs::remove_file(&temporary);
                return Err(format!("迁移 {filename} 失败：{error}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_old_data_once_and_preserves_existing_local_files() {
        let root = std::env::temp_dir().join(format!("shiying-migration-{}", uuid::Uuid::new_v4()));
        let old = root.join("Roaming");
        let local = root.join("Local");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(
            old.join("settings.json"),
            r#"{"cookie":"example","authorization":"Bearer example"}"#,
        )
        .unwrap();
        std::fs::write(old.join("downloads.json"), "[]").unwrap();
        migrate(&old, &local).unwrap();
        assert_eq!(
            std::fs::read(old.join("settings.json")).unwrap(),
            std::fs::read(local.join("settings.json")).unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(local.join("downloads.json")).unwrap(),
            "[]"
        );
        std::fs::write(local.join("settings.json"), "updated local settings").unwrap();
        migrate(&old, &local).unwrap();
        assert_eq!(
            std::fs::read_to_string(local.join("settings.json")).unwrap(),
            "updated local settings"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
