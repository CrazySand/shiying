export interface Settings { downloadDir: string; queryId: string }
export interface SettingsView extends Settings { dataDir: string; cookie: string; authorization: string; hasCookie: boolean; hasAuthorization: boolean }
export interface Variant { url: string; bitrate: number; label: string }
export interface Video { id: string; tweetId: string; title: string; thumbnail: string; variants: Variant[] }
export type TaskStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled'
export interface DownloadTask {
  id: string; title: string; thumbnail: string; filename: string; path: string; status: TaskStatus;
  downloaded: number; total: number | null; speed: number; error: string | null;
}
