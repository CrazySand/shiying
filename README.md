<p align="center">
  <img src="src-tauri/icons/icon.svg" width="80" height="80" alt="拾影 Logo">
</p>

<h1 align="center">拾影 · Shiying</h1>

<p align="center">轻量的 Windows X / Twitter 视频下载工具</p>
<p align="center">Tauri 2 · Vue 3 · TypeScript · Rust · Element Plus</p>

粘贴推文链接，选择画质，将视频保存到自己的电脑。拾影使用 WebView2 显示界面，由 Rust 处理解析、下载和本地数据，采用 Element Plus 蓝白色界面。

## 功能

- **链接解析**：支持 X / Twitter 推文链接或推文 ID，展示视频封面和可用画质。
- **下载队列**：最多同时下载 2 个视频，显示进度、速度和文件大小，支持取消。
- **内容命名**：下载完成后，以视频文件内容的 32 位小写 MD5 命名。
- **历史记录**：本地保存下载记录，重启后恢复标题、封面、文件路径和状态。
- **文件定位**：下载完成后，可在资源管理器中定位视频文件。
- **记录管理**：删除前二次确认，只删除记录，保留视频文件。
- **本地设置**：保存下载目录、Cookie 和 Authorization，设置中可查看数据目录。

目前主要面向 Windows，尚未验证 macOS 和 Linux。

## 使用

1. 打开右上角的设置，选择视频保存位置。
2. 填写自己的 X 登录 **Cookie** 和 **Authorization**，点击「保存设置」。
3. 粘贴推文链接或推文 ID，点击输入框右侧的箭头，或按回车解析。
4. 在视频卡片中选择画质，点击下载。
5. 在下载列表查看进度；完成后点击文件夹图标定位文件。

支持的输入示例：

```text
https://x.com/username/status/1234567890123456789
https://twitter.com/username/status/1234567890123456789
1234567890123456789
```

以上仅用于展示输入格式，请使用实际存在且有权访问的推文。

### Cookie 与 Authorization

程序不内置账号凭据，需要用户手动填写：

| 字段 | 填写内容 |
| --- | --- |
| Cookie | 已登录 X 的 Cookie，必须包含非空的 `auth_token` 和 `ct0` |
| Authorization | 完整的 `Bearer …` 请求头值，或单独的 Token |

保存后，重新打开设置会显示已有内容，可以修改；清空某个输入框并保存，会删除对应凭据。凭据失效后需要重新填写。

**Cookie 和 Authorization 目前以明文保存在本机的 `settings.json` 中。不要公开配置文件，也不要在截图、Issue 或日志中附带真实凭据。**

### 文件与任务

下载完成后的文件名格式：

```text
<文件内容的 MD5>.mp4
```

同一保存目录下，内容完全相同的视频会复用已有文件；内容不同的同名文件不会被覆盖。视频标题仍保留在下载列表中，方便辨认。

任务包含「排队中」「下载中」「已完成」「失败」「已取消」五种状态。关闭应用后，尚未完成的任务会在下次启动时标记为失败，并注明下载已中断，需要重新解析下载，暂不支持断点续传。

失败或取消时会尝试清理 `.part` 临时文件；强制退出或进程崩溃可能留下临时文件，可在确认应用已退出后手动清理。

## 数据存储

应用数据统一保存在：

```text
%LOCALAPPDATA%\site.crazysand.shiying\
├─ settings.json     下载目录、Cookie、Authorization 等设置
├─ downloads.json    下载历史记录
└─ EBWebView/        WebView2 浏览器数据和缓存
```

设置中的「数据目录」显示实际完整路径，只读，可选中复制。下载的视频保存在用户选择的保存位置。

兼容旧版本：启动时，如果本地目录缺少对应文件，会从 `%APPDATA%\site.crazysand.shiying\` 复制旧设置和下载记录。已有本地文件不会被覆盖，旧目录中的文件会保留。

## 本地开发

### 环境

- Windows。
- Node.js 22.12 或更高版本，以及 npm。
- Rust stable，使用 MSVC 工具链。
- Visual Studio Build Tools，包含 C++ 桌面开发工具和 Windows SDK。
- Microsoft Edge WebView2 Runtime。

### 启动桌面应用

在项目根目录执行：

```powershell
npm ci
npm run tauri dev
```

该命令会启动 Vite 开发服务和 Tauri 桌面窗口。前端支持热更新，Rust 或图标资源变更需要开发编译后生效。

仅预览前端：

```powershell
npm run dev
```

浏览器预览地址为 `http://127.0.0.1:1420`。浏览器模式只用于界面预览，解析、下载和系统目录选择需要桌面应用。

### 检查

```powershell
# 前端类型检查
npx vue-tsc --noEmit

# 界面交互测试
npm run test:ui

# Rust 格式、测试和静态检查
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

界面测试使用 Playwright，当前配置依赖本机安装的 Google Chrome，会自动启动或复用 Vite 开发服务。

`npm run build` 用于检查并构建前端，输出到 `dist/`，不会生成桌面安装包。

## Windows 打包

在开发环境准备完成后执行：

```powershell
npm run build:exe
```

该命令会构建前端、编译 Rust，并生成 NSIS 安装程序。

| 产物 | 位置 |
| --- | --- |
| 应用可执行文件 | `src-tauri/target/release/shiying.exe` |
| NSIS 安装程序 | `src-tauri/target/release/bundle/nsis/` |
| 前端静态资源 | `dist/` |

安装后的用户无需安装 Node.js、npm、Rust、Python 或 uv。Windows 上需要 WebView2 Runtime，当前安装包配置会在缺少时联网下载并安装。

当前尚未配置代码签名。WinGet 发布清单及发布自动化也尚未包含在仓库中。

## 项目结构

```text
src/
├─ components/       链接输入、视频卡片、下载列表、设置
├─ composables/      下载任务状态和事件处理
├─ assets/           全局样式与 Element Plus 样式
├─ api.ts            Tauri 命令封装
├─ types.ts          前端类型
└─ App.vue           主界面

src-tauri/
├─ src/
│  ├─ commands.rs    前后端命令入口
│  ├─ parser.rs      X 视频解析
│  ├─ downloader.rs  下载队列、MD5 命名及文件保存
│  ├─ history.rs     下载记录读写和重启恢复
│  ├─ settings.rs    设置及凭据管理
│  ├─ storage.rs     数据目录及旧数据迁移
│  └─ models.rs      数据模型
├─ icons/            应用图标及 SVG 源文件
└─ tauri.conf.json   桌面窗口与打包配置

tests/               Playwright 界面测试
```

## 已知限制与反馈

- 当前使用 X 的非公开 `TweetDetail` 接口，并非 X 官方客户端。平台变更可能导致解析失效。
- 查询 ID 和请求参数由程序内部维护；接口变更时可能需要更新 `settings.rs`、`parser.rs` 或已保存配置中的查询 ID。
- 登录状态、账号权限、网络及平台限制可能影响解析和下载。
- 本地测试不依赖真实账号，不代表在线接口始终可用。
- 下载记录中保存的是封面地址，封面显示需要网络，地址失效时会显示默认图标。

提交问题时，请提供应用版本、Windows 版本、复现步骤和错误提示，并隐藏账号凭据及其他隐私信息。

请仅保存你有权下载和使用的内容，遵守平台规则及适用的版权要求。
