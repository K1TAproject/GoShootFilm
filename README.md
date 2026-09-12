# GoShootFilm

GoShootFilm 是一款面向胶片摄影爱好者的本地桌面归档工具，用于管理相机、胶卷资料、拍摄卷和数字化照片。应用采用深色界面，数据保存在本机，不依赖在线账户。

> 当前官方构建面向 Windows x64。使用方法见 [GoShootFilm 使用说明](./GoShootFilm-使用说明.md)。

## 主要功能

- **首页概览**：查看相机、胶卷、拍摄卷、照片和收藏数量。
- **Cameras**：新增、编辑和归档相机资料，查看每台相机关联的拍摄卷。
- **Films**：内置常见胶卷目录，支持按品牌、类型和拍摄状态筛选，也可自行维护胶卷资料。
- **Rolls**：以全局卷号管理拍摄卷，记录使用设备、胶卷、拍摄日期、城市和备注。
- **双版本照片**：同一 Frame 可分别保存原始扫描和调色图，并在详情页切换查看。
- **TIFF 预览**：原始 TIFF 保留在正式图库中，界面使用独立、可再生的预览资源。
- **批量导入**：从文件名末尾识别 Frame 01–99，导入前集中处理冲突，失败时整批回滚。
- **图库迁移**：可在设置中选择图库位置；迁移时先复制并校验，旧图库不会自动删除。
- **应用更新**：支持启动检查和设置页手动检查更新。

## 安装

前往 [Releases](https://github.com/K1TAproject/GoShootFilm/releases/latest) 下载最新的 Windows 安装包：

```text
GoShootFilm_<版本号>_x64-setup.exe
```

安装后首次启动，请在左侧 **设置 → 图库 → 选择图库目录** 中选择一个新的空目录。图库不能位于应用安装目录中。

## 数据与文件

数据库和应用设置位于：

```text
%APPDATA%\com.tauri-app.goshootfilm\
```

照片位于用户选择的图库目录中：

```text
media/rolls/{rollId}/lab/    原始扫描
media/rolls/{rollId}/edit/   调色图
previews/rolls/              可再生的 TIFF 预览
```

删除照片、拍摄卷、相机或胶卷资料时，应用会删除相应数据库记录，但保留正式图库中的原始扫描和调色文件。请不要把“删除记录”当作文件清理功能。

## 技术栈

- Vue 3、TypeScript、Vite
- Tauri 2、Rust
- SQLite、SQLx

## 本地开发

环境要求：Node.js 22、Rust stable，以及 Tauri 在 Windows 上需要的 WebView2 与 MSVC 构建工具。

```powershell
npm ci
npm run tauri dev
```

生产构建：

```powershell
npm run tauri build
```

提交前检查：

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

## 项目结构

```text
src/                           Vue 前端
  components/                  通用界面组件
  views/                       Home、Cameras、Films、Rolls 页面
  types/                       集中的 TypeScript 类型
  utils/                       胶卷、照片和错误处理工具
src-tauri/                     Rust / Tauri 后端
  migrations/                 SQLite 迁移
  src/db.rs                    数据库初始化与胶卷目录同步
  src/library.rs               图库路径、迁移与校验
  src/lib.rs                   Tauri 命令与主要业务逻辑
  src/startup.rs               启动阶段日志与错误提示
.github/workflows/release.yml  Windows 发布工作流
```

## 发布说明

推送 `v*` 标签后，GitHub Actions 会执行前端构建、Rust 格式检查、Clippy 和测试，并生成 Windows 安装包、Updater 签名及 `latest.json`。工作流默认创建草稿 Release，确认资产完整后再公开发布。

Updater 发布依赖仓库中的既有公钥和 GitHub Actions 中的签名密钥。私钥、密码和发布令牌不得提交到仓库。
