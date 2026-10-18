use crate::models::{Settings, SettingsView};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager};

pub struct AppSettings {
    pub data_dir: String,
    pub settings: Mutex<Settings>,
    pub cookie: Mutex<String>,
    pub authorization: Mutex<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    #[serde(flatten)]
    settings: Settings,
    #[serde(default)]
    cookie: String,
    #[serde(default)]
    authorization: String,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    crate::storage::path(app, "settings.json")
}

pub fn load(app: &AppHandle) -> Result<AppSettings, String> {
    let path = config_path(app)?;
    let data_dir = path
        .parent()
        .ok_or("应用数据目录无效")?
        .to_string_lossy()
        .into_owned();
    let stored = if path.exists() {
        let text = std::fs::read_to_string(path).map_err(|e| format!("读取设置失败：{e}"))?;
        serde_json::from_str::<StoredSettings>(&text).map_err(|e| format!("设置文件损坏：{e}"))?
    } else {
        StoredSettings {
            cookie: String::new(),
            authorization: String::new(),
            settings: Settings {
                download_dir: app
                    .path()
                    .download_dir()
                    .or_else(|_| app.path().home_dir())
                    .map_err(|e| e.to_string())?
                    .join("拾影")
                    .to_string_lossy()
                    .into_owned(),
                query_id: "6uCvnic3m5reVuehkvHa3w".into(),
                max_concurrent_downloads: 2,
            },
        }
    };
    Ok(AppSettings {
        data_dir,
        settings: Mutex::new(stored.settings),
        cookie: Mutex::new(stored.cookie),
        authorization: Mutex::new(stored.authorization),
    })
}

pub fn view(state: &AppSettings) -> Result<SettingsView, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    let authorization = state
        .authorization
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    Ok(SettingsView {
        data_dir: state.data_dir.clone(),
        settings,
        has_cookie: !cookie.is_empty(),
        has_authorization: !authorization.is_empty(),
        cookie,
        authorization,
    })
}

pub fn save(
    app: &AppHandle,
    state: &AppSettings,
    settings: Settings,
    cookie: Option<String>,
    authorization: Option<String>,
) -> Result<SettingsView, String> {
    if !(1..=8).contains(&settings.max_concurrent_downloads) {
        return Err("同时下载数量必须在 1–8 之间".into());
    }
    if settings.query_id.is_empty()
        || settings.query_id.len() > 200
        || !settings
            .query_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("查询 ID 格式不正确".into());
    }
    let dir = PathBuf::from(&settings.download_dir);
    if !dir.is_absolute() {
        return Err("请选择完整的下载目录路径".into());
    }
    if let Some(value) = cookie.as_ref().filter(|value| !value.is_empty()) {
        if value.contains(['\r', '\n']) || value.len() > 32768 {
            return Err("Cookie 格式不正确".into());
        }
        for name in ["ct0", "auth_token"] {
            if !value.split(';').any(|part| {
                part.trim()
                    .split_once('=')
                    .is_some_and(|(key, v)| key == name && !v.is_empty())
            }) {
                return Err(format!("Cookie 缺少 {name}"));
            }
        }
    }
    let authorization = authorization
        .as_deref()
        .map(normalize_authorization)
        .transpose()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("下载目录不可用：{e}"))?;
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut current_settings = state.settings.lock().map_err(|e| e.to_string())?;
    let mut current_cookie = state.cookie.lock().map_err(|e| e.to_string())?;
    let mut current_authorization = state.authorization.lock().map_err(|e| e.to_string())?;
    let stored = StoredSettings {
        settings,
        cookie: cookie.unwrap_or_else(|| current_cookie.clone()),
        authorization: authorization.unwrap_or_else(|| current_authorization.clone()),
    };
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&stored).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("保存设置失败：{e}"))?;
    *current_settings = stored.settings;
    *current_cookie = stored.cookie;
    *current_authorization = stored.authorization;
    drop(current_settings);
    drop(current_cookie);
    drop(current_authorization);
    view(state)
}

pub fn normalize_authorization(value: &str) -> Result<String, String> {
    if value.contains(['\r', '\n']) || value.len() > 8192 || !value.is_ascii() {
        return Err("Authorization 格式不正确".into());
    }
    let parts: Vec<_> = value.split_whitespace().collect();
    let token = match parts.as_slice() {
        [] => return Ok(String::new()),
        [token] if !token.eq_ignore_ascii_case("Bearer") => *token,
        [scheme, token] if scheme.eq_ignore_ascii_case("Bearer") => *token,
        _ => return Err("请填写 Bearer Token 或完整的 Bearer 请求头值".into()),
    };
    let normalized = format!("Bearer {token}");
    reqwest::header::HeaderValue::from_str(&normalized).map_err(|_| "Authorization 格式不正确")?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{normalize_authorization, StoredSettings};

    #[test]
    fn credentials_survive_config_round_trip() {
        let original: StoredSettings = serde_json::from_str(
            r#"{"downloadDir":"C:/Downloads","queryId":"query","cookie":"auth_token=example; ct0=csrf","authorization":"Bearer example"}"#,
        ).unwrap();
        let path =
            std::env::temp_dir().join(format!("shiying-settings-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, serde_json::to_vec_pretty(&original).unwrap()).unwrap();
        let restored: StoredSettings =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(restored.cookie, original.cookie);
        assert_eq!(restored.authorization, original.authorization);
        assert_eq!(
            restored.settings.download_dir,
            original.settings.download_dir
        );
    }

    #[test]
    fn old_settings_without_credentials_still_load() {
        let stored: StoredSettings =
            serde_json::from_str(r#"{"downloadDir":"C:/Downloads","queryId":"query"}"#).unwrap();
        assert_eq!(stored.settings.max_concurrent_downloads, 2);
        assert!(stored.cookie.is_empty());
        assert!(stored.authorization.is_empty());
    }

    #[test]
    fn accepts_bare_token_and_bearer_header_value() {
        assert_eq!(
            normalize_authorization("example%3Dtoken").unwrap(),
            "Bearer example%3Dtoken"
        );
        assert_eq!(
            normalize_authorization(" Bearer example-token ").unwrap(),
            "Bearer example-token"
        );
        assert_eq!(
            normalize_authorization("bearer example-token").unwrap(),
            "Bearer example-token"
        );
        assert_eq!(normalize_authorization(" ").unwrap(), "");
    }

    #[test]
    fn rejects_missing_token_and_header_injection() {
        for input in [
            "Bearer",
            "Bearer one two",
            "Basic token",
            "token\r\nX-Injected: value",
            "token\u{7f}",
        ] {
            assert!(normalize_authorization(input).is_err());
        }
    }
}
