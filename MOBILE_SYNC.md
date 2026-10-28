# GoShootFilm 移动端同步（第一版）

本文记录手机端只读同步的实现范围、协议、安全边界和当前未完成项。

## 1. 当前状态

- 桌面端局域网同步服务、二维码配对、设备撤销、增量元数据同步和移动端派生图片：已实现。
- Android 原生客户端：`android/` 下的 Kotlin + Compose 工程已实现并可构建 debug/release APK；包名 `com.k1taproject.goshootfilm`，应用名 `GSFMobile`。
- 原生客户端使用 CameraX + ZXing 扫码、Android Keystore 保存凭证、Room 保存元数据，并可从 `/edit-originals/{assetId}` 下载正式调色原图。
- 原生客户端始终进入主界面；未配对或电脑离线时仍可浏览 Room 档案和已缓存图片，并可从首页或设置主动进入配对页。
- 同步会依次显示连接、检查、更新档案、核对缓存、下载缩略图、下载标星调色图和整理缓存；未知字节数时按项目数量计算进度，单张失败会显示“部分完成”。
- Films 的已拍摄名片可进入该胶卷的关联 Rolls；Equipment 的相机名片显示拍摄卷数，详情中可进入具体 Rolls。
- 原生客户端仍需真实 Android 设备验证；模拟器没有摄像头，无法验证扫码和弱网/存储边界。

## 2. 代码组织

```text
src/
  shared/
    sync.ts              同步协议类型（桌面和手机共用）
    entities.ts          手机端实体数据类型
  components/
    MobileSyncSettings.vue   桌面设置中的移动端同步面板
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
- 不修改 `mobile_assets` 的 CHECK 约束和既有 `edit_display` 协议行为；
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

删除以 `{ deleted: true, syncId, revision }` 墓碑返回；原生客户端在 Room 事务中删除旧实体和资源记录。

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
        { "id": "…", "assetType": "thumbnail", "version": "edit", "sha256": null, "available": true }
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

服务端只返回资源 ID 和描述符，不返回桌面图库绝对路径。`available` 表示正式源文件存在并可生成资源，不表示派生 JPEG 已经生成；首次请求 `/resources/{assetId}` 时可以按需生成。`sha256` 可能暂为空，手机仍以下载响应头和实际计算结果为准校验。

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

手机在发送设备凭证前先访问 `/health` 校验协议与服务指纹。重新配对批准前，旧地址、旧凭证、Room 数据和图片缓存都保持不变；同一服务指纹配对成功后只把游标重置为 0 做稳定 ID 的完整元数据核对，不会无条件重下图片。不同服务指纹必须由用户明确确认。

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

- 原生 `GSFMobile` 查看大图时使用 `edit_original`；既有 `edit_display` 资源类型继续保留协议兼容性。
- `edit_original` 只允许正式调色文件，不复制、不修改源文件；下载时流式返回并缓存 SHA-256。
- CR2 只读取内嵌 JPEG，不解析 RAW 像素。
- TIFF 复用现有 TIFF 预览流程生成独立预览。
- JPG/JPEG 原图生成独立移动查看副本。
- 资源生成不修改、覆盖或删除正式图库文件。
- 不提供 RAW、CR2、胶片扫描、正式 TIFF 的下载接口。
- 每个资源保存 SHA-256；手机下载后校验 `x-goshootfilm-sha256` 与本地哈希，失败不写入缓存。

## 7. 手机端缓存

- 原生 `GSFMobile`：元数据、缩略图、`cursor` 和统计保存在 Room；标星照片的正式调色原图在同步时自动预缓存到 App 私有目录，并标记为 pinned。
- 非标星调色图在用户点击时按需下载；本地已有且哈希、大小有效时立即打开。同一资源的重复点击由 asset ID 锁去重。
- 缓存上限：2 GB、5 GB、10 GB、不自动清理，默认 5 GB。
- 超限时按 `lastAccess` 最近最少使用清理查看图；缩略图永不参与自动清理。
- 正在查看的资源加入活跃集合，不会被清理。
- “清理查看图缓存”只删除查看图，保留元数据和缩略图。
- 离线时继续展示已缓存内容；网络失败显示错误/离线提示，不显示为空数据。
- 下载先写入 `.part`，校验实际 SHA-256 后原子替换缓存，并保存 `localPath`、实际哈希、实际字节数和 `lastAccess`。解除配对不会清理这些内容。
- 原生 APK 自带界面资源，不依赖额外的网页静态服务器。

## 8. 本地运行

```powershell
# 桌面端
npm run tauri dev
```

桌面端设置：左侧 **设置 → 移动端同步 → 启动同步服务 → 启动配对**。

手机端配对：在 GSFMobile 中扫描二维码；如摄像头不可用，可点击电脑端“复制配对信息”，在手机“手动输入”中粘贴 JSON。

原生 APK 位于 `android/`，构建和发布步骤见 `android/README.md`。安装后应用名为 `GSFMobile`。

## 9. 验证结果

已通过：

- `npm run build`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml`：42 个单元测试 + 1 个 HTTP 集成测试 + 3 个配对集成测试；覆盖服务重建后的旧凭证鉴权、资源按需生成和源文件变化后的失效重建
- `:app:testDebugUnitTest`：10 个 Android 单元测试通过；覆盖原子下载、缓存哈希/大小核对和未知字节数时的项目进度
- `:app:assembleDebug` 通过
- 当前源码版本为 `0.3.9`（Android `versionCode 30009`）；发布签名与真机安装仍需在发布候选构建后复核。

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
