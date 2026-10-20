# GoShootFilm 移动端同步（第一版）

本文记录手机端只读同步的实现范围、协议、安全边界和当前未完成项。

## 1. 当前状态

- 桌面端局域网同步服务、二维码配对、设备撤销、增量元数据同步和移动端派生图片：已实现。
- PWA：`src-mobile/` 保留为设计/调试参考，可在 Android Chrome 中打开。
- Android 原生客户端：`android/` 下的 Kotlin + Compose 工程已实现并可构建 debug/release APK；包名 `com.k1taproject.goshootfilm`，应用名 `GSFMobile`。
- 原生客户端使用 CameraX + ZXing 扫码、Android Keystore 保存凭证、Room 保存元数据，并可从 `/edit-originals/{assetId}` 下载正式调色原图。
- 同步显示分为“检查更新”和“下载数据”两个阶段：检查阶段使用不确定进度条，下载阶段显示进度条和百分比；没有新变更或缺失文件时显示“已是最新”。
- Films 的已拍摄名片可进入该胶卷的关联 Rolls；Equipment 的相机名片显示拍摄卷数，详情中可进入具体 Rolls。
- 原生客户端仍需真实 Android 设备验证；模拟器没有摄像头，无法验证扫码和弱网/存储边界。
- PWA 仍受局域网 HTTP 限制：摄像头、Service Worker 冷启动离线可能不可用。原生 APK 不受这些限制。

## 2. 代码组织

```text
src/
  shared/
    sync.ts              同步协议类型（桌面和手机共用）
    entities.ts          手机端实体数据类型
  components/
    MobileSyncSettings.vue   桌面设置中的移动端同步面板
src-mobile/               移动优先 PWA（保留）
  src/lib/db.ts           IndexedDB 本地数据库与事务
  src/lib/store.ts        同步状态、实体查询、按需下载、缓存 LRU
  src/lib/api.ts          配对、增量同步、资源下载 HTTP 客户端
  src/lib/secure.ts       凭证存储桥接（原生 Keystore / 浏览器回退）
  src/lib/hash.ts         SHA-256 校验（WebCrypto + 纯 JS 回退）
  src/views/              首页、器材、Films、Rolls、Albums、详情、设置、配对
android/                  Kotlin + Compose 原生客户端
  app/src/main/java/...   Room、同步、Keystore、CameraX/ZXing、Compose UI
  app/src/main/assets/    胶卷目录与优化后的 WebP 封面
  tools/                  胶卷资源生成、签名 release 构建脚本
src-tauri/src/sync/
  mod.rs                  服务生命周期、配对状态、设备凭证、Tauri 命令
  server.rs               axum 路由、鉴权、同步与资源接口
  data.rs                 增量 payload、统计、删除墓碑
  assets.rs               移动端缩略图/查看图生成与哈希
  edit_originals.rs       正式调色原图描述、SHA-256 缓存和流式下载
src-tauri/migrations/
  0007_mobile_sync.sql
  0008_mobile_sync_parent_updates.sql
  0009_mobile_edit_originals.sql
```

## 3. 数据库迁移

### 0007：同步基础

新增表：

- `sync_entities`：`entity_type + local_id` 到稳定 `sync_id` 的映射，保存 `revision`、`updated_at`、`deleted`。
- `sync_changes`：单调递增的 `seq` 变更日志，删除也会写入墓碑。
- `mobile_devices`：设备名称、平台、凭证 SHA-256、配对时间、最近同步时间、撤销时间。
- `mobile_pair_requests`：待确认/已批准/已拒绝/已过期的一次性配对请求。
- `mobile_assets`：缩略图和查看图的派生资源记录、源文件元数据、SHA-256、尺寸。

迁移会为现有相机、器材、胶卷、拍摄卷、胶片照片、数码相册和数码照片回填稳定 ID，并为每行写入初始变更。现有自增 ID 只用于桌面本地查询，不进入手机协议。

### 0008：父级摘要刷新

当照片收藏、路径、Frame 变化，或照片新增/删除时，会同步提升所属拍摄卷/数码相册的修订号。相机或胶卷改名时也会刷新关联拍摄卷/相册的摘要。这样手机拿到的封面、照片数量和相机/胶卷显示名不会滞后。

### 0009：正式调色原图

新增 `mobile_edit_originals`，只记录 `edit` 目录中的正式调色文件（PNG/JPEG/WebP）：

- `entity_type + entity_sync_id` 唯一；
- 保存源相对路径、MIME、字节数、SHA-256、源修改时间；
- 不修改 `mobile_assets` 的 CHECK 约束，PWA 的 `edit_display` 行为不变；
- 原生客户端通过 `/edit-originals/{assetId}` 按需下载；服务端只允许 `rolls/<id>/edit` 和 `digital/<id>/edit` 路径。

## 4. 同步协议

所有接口均在 `http://<局域网地址>:<端口>/api/v1/` 下，协议版本为 `1`。

| 方法 | 路径 | 鉴权 | 说明 |
| --- | --- | --- | --- |
| GET | `/health` | 无 | 协议版本、服务指纹、应用版本、配对可用状态 |
| POST | `/pair/request` | 配对令牌 | 提交手机名称/平台，创建待确认请求 |
| GET | `/pair/status/{requestId}` | 请求 ID | 轮询 pending/approved/denied/expired，批准后一次性返回设备凭证 |
| GET | `/sync/changes?cursor=&limit=` | Bearer 设备凭证 | 返回增量变更、统计、下一游标、是否有更多 |
| GET | `/resources/{assetId}` | Bearer 设备凭证 | 下载派生 JPEG，响应带 `x-goshootfilm-sha256` |
| GET | `/edit-originals/{assetId}` | Bearer 设备凭证 | 下载正式调色原文件（PNG/JPEG/WebP），流式返回并带 `x-goshootfilm-sha256` |

同步实体类型：

- `camera`、`equipment`、`film`、`roll`、`film_photo`、`digital_album`、`digital_photo`

删除以 `{ deleted: true, syncId, revision }` 墓碑返回；原生客户端在 Room 事务中删除旧实体和资源记录，PWA 在 IndexedDB 事务中执行相同语义。

`GET /sync/changes` 的关键字段：

```json
{
  "protocolVersion": 1,
  "cursor": 128,
  "resetRequired": false,
  "hasMore": true,
  "serverTime": "2026-10-18T02:30:00.000Z",
  "stats": { "cameraCount": 6, "rollCount": 31, "...": 0 },
  "changes": [
    {
      "entityType": "film_photo",
      "syncId": "…",
      "revision": 3,
      "deleted": false,
      "data": { "rollSyncId": "…", "frameNumber": 8, "...": null },
      "assets": [
        { "id": "…", "assetType": "thumbnail", "version": "edit", "sha256": null, "available": false }
      ],
      "editOriginal": {
        "id": "…",
        "entityType": "film_photo",
        "entitySyncId": "…",
        "mimeType": "image/png",
        "byteSize": 12345678,
        "sha256": null,
        "available": true
      }
    }
  ]
}
```

服务端只返回资源 ID 和描述符，不返回桌面图库绝对路径。`editOriginal.sha256` 可能暂为空；首次通过 `/edit-originals/{id}` 下载时服务端计算并缓存哈希，手机仍以响应头为准校验。

## 5. 二维码与设备凭证

二维码 JSON：

```json
{
  "v": 1,
  "url": "http://192.168.1.20:47821",
  "token": "<一次性配对令牌>",
  "fingerprint": "<服务身份指纹>",
  "expiresAt": 1792290000,
  "serverName": "GoShootFilm"
}
```

流程：

1. 桌面设置中启动同步服务，再点击“启动配对”生成二维码；令牌 2 分钟有效。
2. 手机提交配对请求，电脑端必须点击“批准”。
3. 电脑为设备生成独立凭证，数据库只保存 SHA-256；明文凭证只在内存中等待手机领取一次。
4. 手机保存凭证；之后每个同步/资源请求使用 `Authorization: Bearer <credential>`。
5. 撤销设备会立即写入 `revoked_at`，下一次请求即失败。

服务身份指纹由持久化的随机服务密钥计算 SHA-256 前 32 位；它用于二维码展示和手机记录，但在第一版 HTTP 传输中不能替代 TLS 证书校验。

第一版服务只绑定当前局域网 IPv4 地址，优先选择 Wi-Fi/WLAN 接口，并排除常见虚拟网卡和 198.18/15 代理网段；不监听公网，不把正式图库作为静态目录暴露。IP 变化时需要重新配对。Windows 防火墙首次可能询问是否允许该应用访问专用网络，请允许专用网络访问；不要开放公用网络或做端口转发。

## 6. 移动端图片资源

目录：

```text
<图库>/previews/mobile/thumbs/{assetId}.jpg
<图库>/previews/mobile/display/{assetId}.jpg
```

| 类型 | 来源 | 长边 | 质量 |
| --- | --- | --- | --- |
| `thumbnail` | 调色图或原图 | 384 px | JPEG 82 |
| `edit_display` | 正式调色图 | 2400 px | JPEG 88 |
| `original_preview` | 原始扫描/原图 | 2400 px | JPEG 88 |
| `edit_original` | 正式调色原文件 | 原尺寸 | PNG/JPEG/WebP |

- PWA 继续使用 `edit_display`；原生 `GSFMobile` 查看大图时使用 `edit_original`。
- `edit_original` 只允许正式调色文件，不复制、不修改源文件；下载时流式返回并缓存 SHA-256。
- CR2 只读取内嵌 JPEG，不解析 RAW 像素。
- TIFF 复用现有 TIFF 预览流程生成独立预览。
- JPG/JPEG 原图生成独立移动查看副本。
- 资源生成不修改、覆盖或删除正式图库文件。
- 不提供 RAW、CR2、胶片扫描、正式 TIFF 的下载接口。
- 每个资源保存 SHA-256；手机下载后校验 `x-goshootfilm-sha256` 与本地哈希，失败不写入缓存。

## 7. 手机端缓存

- 原生 `GSFMobile`：元数据、缩略图、`cursor` 和统计保存在 Room；标星照片的正式调色原图在同步时自动预缓存到 App 私有目录，并标记为 pinned。
- 原生客户端不再在点击大图时联网下载；无本地缓存时提示“该图片无本地缓存”。长按本地图片可保存到系统相册或分享。
- PWA：元数据、缩略图、`cursor` 和统计保存在 IndexedDB；查看图按需下载。
- 缓存上限：2 GB、5 GB、10 GB、不自动清理，默认 5 GB。
- 超限时按 `lastAccess` 最近最少使用清理查看图；缩略图永不参与自动清理。
- 正在查看的资源加入活跃集合，不会被清理。
- “清理查看图缓存”只删除查看图，保留元数据和缩略图。
- 离线时继续展示已缓存内容；网络失败显示错误/离线提示，不显示为空数据。
- 原生 APK 自带界面资源，不依赖 `4173` 静态服务器；PWA 仍受局域网 HTTP 的 Service Worker 限制。

## 8. 本地运行

```powershell
# 桌面端
npm run tauri dev

# 手机端开发服务器（同一局域网内用手机访问电脑 IP:1422）
npm run dev:mobile

# 手机端生产构建
npm run build:mobile

# 预览手机端构建
npm run preview:mobile
```

桌面端设置：左侧 **设置 → 移动端同步 → 启动同步服务 → 启动配对**。

手机端配对：扫描二维码；若浏览器禁用摄像头，点击电脑端“复制配对信息”，在手机“手动输入”中粘贴 JSON。

### 在 Android 手机上使用当前 PWA

```powershell
npm run build:mobile
npm run preview:mobile
```

1. 记下 `preview` 输出的局域网地址（例如 `http://10.151.113.147:4173/`），不要使用 198.18.x.x 代理/VPN 地址。
2. 注意：`4173` 是手机页面，`47821` 是同步 API。直接在手机打开 `http://电脑IP:47821/` 会看到 404，这是正常的；API 健康检查地址是 `/api/v1/health`。
3. 手机与电脑连接同一 Wi-Fi，在 Android Chrome 打开该地址。
4. 浏览器菜单可“添加到主屏幕”；局域网 HTTP 下可能只是快捷方式，不是完整独立 PWA。
5. 电脑端启动同步服务并生成二维码；手机浏览器若无法调用摄像头，可用 Android 系统相机/扫码工具读取二维码文本，或把电脑端“复制配对信息”的内容发送到手机后粘贴。
6. 电脑端批准后，手机即可同步。

原生 APK 位于 `android/`，构建和发布步骤见 `android/README.md`。安装后应用名为 `GSFMobile`，不再依赖 `4173` 页面服务，也不需要手机浏览器扫码。

## 9. 验证结果

已通过：

- `npm run build`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml`：42 个单元测试 + 1 个 HTTP 集成测试 + 2 个配对集成测试；HTTP 测试覆盖 `/edit-originals/{assetId}` 和正式调色原图字节校验
- `npm run build:mobile`
- `:app:testDebugUnitTest`：7 个 Android 单元测试通过；`FilmCatalogInstrumentedTest` 和 `GallerySaverInstrumentedTest` 在 API 35 模拟器上通过
- `:app:assembleDebug` 和 `:app:assembleRelease` 通过
- 签名 release APK 通过 `apksigner verify`，manifest 显示 `com.k1taproject.goshootfilm`、`versionCode 30006`、`versionName 0.3.6`、label `GSFMobile`
- Android 模拟器安装并启动 release APK 成功

未完成：

- 真实 Android 设备扫码、配对、同步、增量、中断恢复、离线浏览、缓存清理、设备撤销验证
- 真机上的正式调色原图大文件下载、低存储和弱网测试
- HTTPS/TLS 与公网隔离之外的更强传输安全

## 10. 当前不实现

- 云同步、账号系统
- 双向编辑
- RAW/CR2/正式 TIFF 传输
- 复杂设备发现和 IP 变化自动迁移
- 多用户权限
