# GoShootFilm

<p align="center">
  <img src="./src/assets/app-icon.png" width="88" alt="GoShootFilm 应用图标" />
</p>

<p align="center">
  <strong>面向胶片与数码摄影的本地桌面影像归档工具</strong>
</p>

GoShootFilm 把相机机身、镜头、胶卷、拍摄卷、数码相册和照片整理在同一个本地资料库中。它不需要注册账号，也不会把照片上传到云端：记录保存在本机 SQLite 数据库，原始影像保存在你指定的图库目录。

> 当前版本 **0.2.6**，官方构建面向 **Windows x64**。安装后的日常操作见 [GoShootFilm 使用说明](./GoShootFilm-使用说明.md)。

## 功能

### 胶片工作流

- **器材档案（Equipment）**：管理胶片机身、数码机身、镜头和其他器材；记录品牌、型号、画幅/CMOS 规格、购入日期、状态和备注，镜头可记录卡口，其他器材可记录具体类型；从机身名片可以进入关联的拍摄卷或数码相册。
- **胶卷资料（Films）**：内置 110+ 款常见胶卷，支持按品牌、类型（B&W、Color Negative、Slide）和拍摄状态筛选，也可以自行新增、编辑或删除胶卷资料。
- **拍摄卷（Rolls）**：以全局连续卷号管理拍摄卷，记录使用机身、胶卷、拍摄月份、城市和备注；支持按品牌、型号、类型、机身、年份和月份组合筛选。
- **双版本照片**：同一 Frame 可以分别保存原始扫描和调色图，在拍摄卷详情中切换查看；收藏照片会参与拍摄卷封面选择。
- **Frame 批量导入**：从文件名末尾识别 Frame 01–99，导入前集中检查冲突；可选择跳过、替换引用或取消整批，任一文件失败时整批不写入。
- **TIFF 预览**：原始 TIFF 保留在正式图库中，界面使用独立、可再生的预览资源；预览生成失败不会修改原件。

### 数码工作流

- **数码相册（Albums）**：按拍摄批次创建相册，记录标题、拍摄月份、数码机身、地点和备注；支持按数码相机筛选。
- **原图与调色图**：同一配对名称可以分别保存原图和调色图，并在详情页切换查看；原图支持 CR2、TIF、TIFF、JPG、JPEG，调色图支持 PNG、JPG、JPEG、WebP。
- **按文件名配对**：数码照片按完整文件名配对，导入前可以修改配对名称；重复配对、同版本冲突会在导入确认阶段提示，支持跳过、替换记录版本或取消整批。
- **CR2 / TIFF 预览**：CR2 使用内嵌 JPEG 生成预览，TIFF 生成独立预览；原图文件始终保留在正式图库中。
- **收藏与封面**：收藏照片会参与相册封面选择；删除相册或照片记录时会清理可再生预览，但保留正式原图和调色图。

### 资料库与更新

- **首页概览**：汇总器材、已拍摄卷数、数码相册和已归档照片，并展示胶卷拍摄进度与机身概览。
- **本地优先**：数据库、设置和日志位于应用数据目录，照片位于你选择的图库目录，不依赖在线账户。
- **图库迁移**：在设置中可以更换图库位置；迁移会先复制并校验文件，成功后切换设置，旧图库不会自动删除。
- **应用更新**：启动后自动检查更新，也可以在设置中手动检查；更新包使用 Tauri Updater 签名校验。

### 支持的图片格式

| 场景 | 原始文件 | 调色文件 |
| --- | --- | --- |
| 胶片拍摄卷 | PNG、JPG、JPEG、TIF、TIFF、WebP、BMP、GIF | PNG、JPG、JPEG、WebP |
| 数码相册 | CR2、TIF、TIFF、JPG、JPEG | PNG、JPG、JPEG、WebP |

## 下载与安装

前往 [Releases](https://github.com/K1TAproject/GoShootFilm/releases/latest) 下载最新的 Windows 安装包：

```text
GoShootFilm_<版本号>_x64-setup.exe
```

首次启动后：

1. 打开左侧底部的 **设置**。
2. 在“图库”区域点击 **选择图库目录**。
3. 选择一个新的、可写的空目录，例如 `D:\GoShootFilmLibrary`。

图库不能位于应用安装目录中。设置完成后即可在 Rolls 或 Albums 中导入照片。

## 数据与文件

应用数据位于：

```text
%APPDATA%\com.tauri-app.goshootfilm\
  goshootfilm.db      SQLite 数据库
  settings.json       图库路径等本地设置
  logs\startup.log    启动与崩溃日志
```

照片保存在你选择的图库目录中：

```text
<图库目录>\
  media\
    rolls\<rollId>\lab\        胶片原始扫描
    rolls\<rollId>\edit\       胶片调色图
    digital\<albumId>\raw\     数码原图
    digital\<albumId>\edit\    数码调色图
  previews\
    rolls\...                  可再生的胶片预览
    digital\...                可再生的数码预览
```

`media/` 保存正式文件，`previews/` 只是可再生的预览缓存。不要在应用外随意移动或重命名 `media/` 中的目录，否则数据库记录会找不到对应文件。

删除照片会移除照片记录；删除拍摄卷、胶卷或数码相册会删除对应记录及其关联的照片记录；删除相机会删除机身和胶片拍摄卷，数码相册会保留但解除与机身的关联。以上操作都会清理可再生预览，但保留正式图库中的原始扫描、原图和调色图。请不要把“删除记录”当作文件清理功能。需要释放磁盘空间时，请先确认备份，再自行整理图库目录。

## 技术栈

- Vue 3、TypeScript、Vite
- Tauri 2、Rust
- SQLite、SQLx
- `image` crate（TIFF / CR2 预览）
- GitHub Actions（Windows 构建、测试与发布）

## 本地开发

环境要求：

- Node.js 22
- Rust stable 与 MSVC C++ 构建工具
- WebView2 Runtime（Windows 10/11 通常已自带）

安装依赖并启动完整桌面应用：

```powershell
npm ci
npm run tauri dev
```

只启动前端开发服务器：

```powershell
npm run dev
```

> `npm run dev` 只运行 Vite，Tauri API 不可用；需要调试完整功能时请使用 `npm run tauri dev`。开发模式默认也会读写同一个应用数据目录，调试前建议先备份本地数据库和设置。

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
.github/workflows/release.yml   Windows 发布、测试与 Updater 构建
design/                         应用图标概念稿
public/film-stocks/             胶卷封面图与占位图
scripts/sync-film-images.ps1    同步胶卷封面资源
src/
  components/                   通用界面组件（标题栏、照片墙、导入对话框等）
  data/film-catalog.csv         内置胶卷目录
  types/index.ts                前端类型定义
  utils/                        胶卷、照片与错误处理工具
  views/
    Home.vue                    首页概览
    Cameras.vue                 器材页面（路由 /cameras，界面标题 Equipment）
    Films.vue                   胶卷资料
    Rolls.vue                   胶片拍摄卷
    Albums.vue                  数码相册
  App.vue / main.ts / router.ts / style.css
src-tauri/
  migrations/                   SQLite 迁移（0001–0006）
  src/db.rs                     数据库初始化与胶卷目录同步
  src/digital.rs                数码相册、原图/调色图导入与预览
  src/equipment.rs              镜头与其他器材
  src/lib.rs                    Tauri 命令与胶片业务逻辑
  src/library.rs                图库路径、迁移与校验
  src/models.rs                 后端数据模型
  src/startup.rs                启动日志与错误提示
  src/validation.rs             输入校验
  capabilities/default.json     权限配置
  icons/                        应用图标
  windows/hooks.nsh             NSIS 安装钩子
  Cargo.toml / tauri.conf.json
tools/                          维护辅助脚本（胶卷封面背景、文档内容提取）
allfims_official.txt            官方胶卷目录原始清单
```

## 发布说明

推送 `v*` 标签后，[release.yml](./.github/workflows/release.yml) 会执行前端构建、Rust 格式检查、Clippy、测试和 SQL 迁移换行符检查，然后生成 Windows 安装包、Updater 签名及 `latest.json`。工作流默认创建草稿 Release，确认资产完整后再公开发布。

Updater 依赖 `tauri.conf.json` 中的公钥，以及 GitHub Actions 中的以下密钥：

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

私钥、密码、发布令牌和本地 `.env` 文件不得提交到仓库。

## 仓库约定

- 文本文件统一使用 LF；`.gitattributes` 会在提交和检出时执行规范化。
- 在 Windows 上建议设置 `core.autocrlf=false`；当前工作区已设置，减少仅换行符不同的改动。
- `node_modules/`、`dist/`、`src-tauri/target/`、`*.db`、`.env`、本地图库和 `work/` 不进入版本库。
- 提交前先运行 `git status`，确认没有把本地数据库、照片、构建产物或个人草稿加入暂存区。

## 文档

- [GoShootFilm 使用说明](./GoShootFilm-使用说明.md)
- [Releases](https://github.com/K1TAproject/GoShootFilm/releases)
