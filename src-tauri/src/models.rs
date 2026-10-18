use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub download_dir: String,
    pub query_id: String,
    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: usize,
}

pub fn default_max_concurrent_downloads() -> usize {
    2
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub data_dir: String,
    #[serde(flatten)]
    pub settings: Settings,
    pub cookie: String,
    pub authorization: String,
    pub has_cookie: bool,
    pub has_authorization: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Variant {
    pub url: String,
    pub bitrate: u64,
    pub label: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub tweet_id: String,
    pub title: String,
    pub thumbnail: String,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: String,
    #[serde(default)]
    pub created_at: u64,
    pub title: String,
    #[serde(default)]
    pub thumbnail: String,
    pub filename: String,
    pub path: String,
    pub status: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub error: Option<String>,
}
