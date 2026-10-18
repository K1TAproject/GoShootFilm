# GoShootFilm 移动端同步（第一版）

本文记录手机端只读同步的实现范围、协议、安全边界和当前未完成项。

## 1. 当前状态

- 桌面端局域网同步服务、二维码配对、设备撤销、增量元数据同步和移动端派生图片：已实现。
- 手机界面：以 `src-mobile/` 的移动优先 PWA 实现，可在 Android Chrome 中打开。
- Android 原生 APK：**未完成**。当前开发环境没有 Android SDK、NDK、ADB，Java 只有 1.8，无法完成 Tauri/Capacitor Android 的 debug/release 构建和真实设备验证。
- 浏览器在局域网 HTTP 下通常不允许摄像头，因此 PWA 提供“手动粘贴配对信息”作为扫码替代。原生壳接入相机插件后才能稳定扫码。
- 浏览器在局域网 HTTP 下通常也不会注册 Service Worker，因此 PWA 的冷启动离线应用壳不可用；已打开的页面可以继续使用 IndexedDB 数据。完整冷启动离线浏览需要 Android 原生壳或 HTTPS。
- 配对凭证在浏览器中退化为 localStorage；原生壳需要注入 `window.GoShootFilmSecure` 桥接 Android Keystore 后才属于系统安全存储。

## 2. 代码组织

```text
src/
  shared/
    sync.ts              同步协议类型（桌面和手机共用）
    entities.ts          手机端实体数据类型
  components/
    MobileSyncSettings.vue   桌面设置中的移动端同步面板
src-mobile/               移动优先 PWA
  src/lib/db.ts           IndexedDB 本地数据库与事务
  src/lib/store.ts        同步状态、实体查询、按需下载、缓存 LRU
  src/lib/api.ts          配对、增量同步、资源下载 HTTP 客户端
  src/lib/secure.ts       凭证存储桥接（原生 Keystore / 浏览器回退）
  src/lib/hash.ts         SHA-256 校验（WebCrypto + 纯 JS 回退）
  src/views/              首页、器材、Films、Rolls、Albums、详情、设置、配对
src-tauri/src/sync/
  mod.rs                  服务生命周期、配对状态、设备凭证、Tauri 命令
  server.rs               axum 路由、鉴权、同步与资源接口
  data.rs                 增量 payload、统计、删除墓碑
  assets.rs               移动端缩略图/查看图生成与哈希
src-tauri/migrations/
  0007_mobile_sync.sql
  0008_mobile_sync_parent_updates.sql
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

## 4. 同步协议

所有接口均在 `http://<局域网地址>:<端口>/api/v1/` 下，协议版本为 `1`。

| 方法 | 路径 | 鉴权 | 说明 |
| --- | --- | --- | --- |
| GET | `/health` | 无 | 协议版本、服务指纹、应用版本、配对可用状态 |
| POST | `/pair/request` | 配对令牌 | 提交手机名称/平台，创建待确认请求 |
| GET | `/pair/status/{requestId}` | 请求 ID | 轮询 pending/approved/denied/expired，批准后一次性返回设备凭证 |
| GET | `/sync/changes?cursor=&limit=` | Bearer 设备凭证 | 返回增量变更、统计、下一游标、是否有更多 |
| GET | `/resources/{assetId}` | Bearer 设备凭证 | 下载派生 JPEG，响应带 `x-goshootfilm-sha256` |

同步实体类型：

- `camera`、`equipment`、`film`、`roll`、`film_photo`、`digital_album`、`digital_photo`

删除以 `{ deleted: true, syncId, revision }` 墓碑返回，手机在同一 IndexedDB 事务中删除旧实体和资源记录。

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
      ]
    }
  ]
}
```

服务端只返回派生资源 ID，不返回桌面图库绝对路径。

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

- CR2 只读取内嵌 JPEG，不解析 RAW 像素。
- TIFF 复用现有 TIFF 预览流程生成独立预览。
- JPG/JPEG 原图生成独立移动查看副本。
- 资源生成不修改、覆盖或删除正式图库文件。
- 不提供 RAW、CR2、正式 TIFF 的下载接口。
- 每个资源保存 SHA-256；手机下载后校验 `x-goshootfilm-sha256` 与本地哈希，失败不写入缓存。

## 7. 手机端缓存

- 元数据、全部缩略图、最新 `cursor` 和统计保存在 IndexedDB。
- 调色图/原图查看图按需下载。
- 缓存上限：2 GB、5 GB、10 GB、不自动清理，默认 5 GB。
- 超限时按 `lastAccess` 最近最少使用清理查看图；缩略图永不参与自动清理。
- 正在查看的资源加入活跃集合，不会被清理。
- “清理查看图缓存”只删除查看图，保留元数据和缩略图。
- 离线时继续展示已缓存内容；网络失败显示错误/离线提示，不显示为空数据。
- 注意：局域网 HTTP 下浏览器可能不注册 Service Worker，冷启动离线应用壳不可用；此限制需要 Android 原生壳或 HTTPS 解决。

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
2. 手机与电脑连接同一 Wi-Fi，在 Android Chrome 打开该地址。
3. 浏览器菜单可“添加到主屏幕”；局域网 HTTP 下可能只是快捷方式，不是完整独立 PWA。
4. 电脑端启动同步服务并生成二维码；手机浏览器若无法调用摄像头，可用 Android 系统相机/扫码工具读取二维码文本，或把电脑端“复制配对信息”的内容发送到手机后粘贴。
5. 电脑端批准后，手机即可同步。

当前没有 APK。若要像普通 Android 应用一样安装、使用系统安全存储和原生扫码，需要 Android SDK/Java 17 并增加原生壳；否则 HTTP PWA 的摄像头、Service Worker 和冷启动离线能力会受限。

## 9. 验证结果

已通过：

- `npm run build`
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml`：40 个单元测试 + 1 个 HTTP 集成测试 + 2 个配对集成测试
- `npm run build:mobile`
- Tauri 桌面开发启动，确认迁移 0007/0008 应用到现有数据库
- headless Edge（Android UA）加载手机 PWA、注入模拟数据并检查首页、Rolls、器材、Films、Albums、设置和照片查看器布局
- 手机端数据层与冷启动离线的区别已记录：IndexedDB 数据可离线使用，但 HTTP PWA 的 Service Worker 冷启动不可保证

未完成：

- Android debug/release APK 构建
- 真实 Android 设备扫码、配对、同步、增量、中断恢复、离线浏览、缓存清理、设备撤销验证
- Android Keystore 原生桥接
- HTTPS/TLS 与公网隔离之外的更强传输安全

## 10. 当前不实现

- 云同步、账号系统
- 双向编辑
- RAW/CR2/正式 TIFF 传输
- 复杂设备发现和 IP 变化自动迁移
- 多用户权限
