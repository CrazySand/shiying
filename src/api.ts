import { invoke, isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { Settings, SettingsView, Video, DownloadTask } from './types'
export const desktop = isTauri()
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!desktop) return Promise.reject(new Error('请使用 npm run tauri dev 启动桌面应用。浏览器模式仅用于界面预览。'))
  return invoke<T>(command, args)
}
export const api = {
  settings: () => call<SettingsView>('get_settings'),
  saveSettings: (settings: Settings, cookie: string | null, authorization: string | null) => call<SettingsView>('save_settings', { settings, cookie, authorization }),
  parse: (input: string) => call<Video[]>('parse_video', { input }),
  download: (video: Video, variantIndex: number) => call<string>('start_download', { video, variantIndex }),
  tasks: () => call<DownloadTask[]>('list_tasks'),
  cancel: (id: string) => call<void>('cancel_download', { id }),
  remove: (id: string) => call<void>('remove_download', { id }),
  reveal: (id: string) => call<void>('reveal_download', { id }),
  pickDirectory: () => open({ directory: true, multiple: false, title: '选择视频保存目录' }),
}
