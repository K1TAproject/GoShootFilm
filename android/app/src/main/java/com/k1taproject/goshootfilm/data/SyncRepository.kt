package com.k1taproject.goshootfilm.data

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.room.withTransaction
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import java.io.File
import java.util.concurrent.TimeUnit

class SyncException(message: String) : Exception(message)

data class SyncProgress(
    val message: String,
    val fraction: Float? = null,
)

data class PairingPayload(
    val protocolVersion: Int,
    val url: String,
    val token: String,
    val fingerprint: String,
    val expiresAt: Long,
    val serverName: String,
)

class SyncRepository(
    context: Context,
    private val db: AppDatabase,
    private val settings: SettingsStore,
    private val secure: SecureStore,
    private val api: ApiClient = ApiClient(),
) {
    private val appContext = context.applicationContext
    private val dao = db.dao()
    private val mutex = Mutex()
    private val cacheRoot = File(appContext.filesDir, "mobile-cache")
    private val connectivity = appContext.getSystemService(ConnectivityManager::class.java)
    private val thumbnailRoot = File(cacheRoot, "thumbs")
    private val editRoot = File(cacheRoot, "edits")
    @Volatile
    private var cancelRequested = false

    fun settingsFlow(): Flow<AppSettings> = settings.settings

    fun entities(type: String): Flow<List<EntityRecord>> = dao.observeEntities(type)

    suspend fun entity(type: String, syncId: String): EntityRecord? = dao.entity(type, syncId)

    suspend fun entitiesNow(type: String): List<EntityRecord> = dao.entities(type)

    suspend fun allEntities(): List<EntityRecord> = dao.allEntities()

    suspend fun thumbnail(type: String, syncId: String): AssetRecord? =
        dao.asset(type, syncId, "edit", "thumbnail")
            ?: dao.asset(type, syncId, "lab", "thumbnail")
            ?: dao.asset(type, syncId, "raw", "thumbnail")

    suspend fun editOriginal(type: String, syncId: String): AssetRecord? =
        dao.asset(type, syncId, "edit", "edit_original")

    suspend fun localEditFile(assetId: String): File? =
        dao.assetById(assetId)?.localPath?.let(::File)?.takeIf { it.isFile }

    suspend fun currentSettings(): AppSettings = settings.current()

    fun cancelSync() {
        cancelRequested = true
    }

    suspend fun sync(onProgress: (SyncProgress) -> Unit = {}) = mutex.withLock {
        cancelRequested = false
        val current = settings.current()
        val credential = secure.credential()
        if (current.serverUrl.isBlank() || credential.isNullOrBlank()) {
            throw SyncException("请先与电脑端配对")
        }
        if (current.wifiOnly && !isOnWifi()) {
            throw SyncException("当前不是 Wi-Fi 网络；请连接与电脑相同的 Wi-Fi 后再同步")
        }
        var cursor = current.cursor
        var pageCount = 0
        while (pageCount++ < 500) {
            if (cancelRequested) throw SyncException("同步已取消")
            onProgress(SyncProgress("正在同步元数据…"))
            val response = withContext(Dispatchers.IO) {
                api.changes(current.serverUrl, credential, cursor)
            }
            val protocol = response.int("protocolVersion") ?: 0
            if (protocol != 1) throw SyncException("手机端与电脑端协议不兼容")
            val resetRequired = response.bool("resetRequired")
            if (resetRequired) {
                db.withTransaction {
                    dao.clearEntities()
                    dao.clearAssets()
                    dao.clearMeta()
                }
                deleteTree(thumbnailRoot)
                deleteTree(editRoot)
                cursor = 0
                settings.setCursor(0)
                onProgress(SyncProgress("电脑端要求重新同步…"))
                continue
            }
            val changes = response["changes"]?.jsonArray
                ?: throw SyncException("同步响应缺少 changes")
            db.withTransaction {
                for (element in changes) {
                    applyChange(element.jsonObject)
                }
            }
            val nextCursor = response.long("cursor") ?: cursor
            val hasMore = response.bool("hasMore")
            cursor = nextCursor
            settings.setCursor(cursor)
            response["stats"]?.let { stats ->
                dao.putMeta(MetaRecord("stats", stats.toString()))
            }
            if (!hasMore) break
            if (nextCursor == current.cursor && pageCount > 1) break
        }
        reconcilePinnedAssets()
        val thumbnailJobs = dao.allThumbnails().filter { asset ->
            asset.available && asset.localPath?.let(::File)?.isFile != true
        }
        val pinnedJobs = dao.pinnedEditOriginals().filter { asset ->
            asset.available && asset.localPath?.let(::File)?.isFile != true
        }
        val totalBytes = (thumbnailJobs + pinnedJobs)
            .sumOf { (it.byteSize ?: 1L).coerceAtLeast(1L) }
        var downloadedBytes = 0L
        var lastReportedFraction = -1f
        fun report(message: String, totalDownloaded: Long) {
            val fraction = if (totalBytes > 0) {
                (totalDownloaded.toFloat() / totalBytes).coerceIn(0f, 1f)
            } else {
                null
            }
            if (fraction != null && lastReportedFraction >= 0f &&
                fraction - lastReportedFraction < 0.001f
            ) {
                return
            }
            lastReportedFraction = fraction ?: -1f
            onProgress(SyncProgress(message, fraction))
        }

        if (thumbnailJobs.isNotEmpty()) {
            onProgress(SyncProgress("正在下载缩略图…", 0f))
            downloadedBytes = downloadThumbnails(thumbnailJobs) { totalSoFar ->
                report("正在下载缩略图…", totalSoFar)
            }
        }
        if (cancelRequested) throw SyncException("同步已取消")
        var failedPinned = 0
        if (pinnedJobs.isNotEmpty()) {
            val startFraction = if (totalBytes > 0) {
                (downloadedBytes.toFloat() / totalBytes).coerceIn(0f, 1f)
            } else {
                0f
            }
            onProgress(SyncProgress("正在下载标星调色图…", startFraction))
            val baseBytes = downloadedBytes
            val result = downloadPinnedEditOriginals(pinnedJobs) { currentBytes ->
                report("正在下载标星调色图…", baseBytes + currentBytes)
            }
            downloadedBytes = baseBytes + result.first
            failedPinned = result.second
        }
        settings.setLastSync(System.currentTimeMillis())
        cleanupCache(settings.current().cacheLimitMb)
        onProgress(
            SyncProgress(
                if (failedPinned > 0) "同步完成，部分标星原图下载失败" else "同步完成",
                1f,
            ),
        )
    }

    private suspend fun applyChange(change: JsonObject) {
        val type = change.text("entityType")
        val syncId = change.text("syncId")
        if (type.isBlank() || syncId.isBlank()) return
        val deleted = change.bool("deleted")
        if (deleted) {
            dao.deleteEntity(type, syncId)
            dao.deleteAssetsFor(type, syncId)
            return
        }
        val revision = change.long("revision") ?: 1
        val data = change["data"] as? JsonObject
            ?: throw SyncException("同步数据缺少 data")
        dao.upsertEntity(
            EntityRecord(
                entityType = type,
                syncId = syncId,
                revision = revision,
                deleted = false,
                dataJson = data.toString(),
                updatedAt = System.currentTimeMillis(),
            ),
        )
        val assets = change["assets"]?.jsonArray
        if (assets != null) {
            for (element in assets) {
                val asset = element.jsonObject
                val id = asset.text("id")
                if (id.isBlank()) continue
                val hash = asset.optionalText("sha256")
                val old = dao.assetById(id)
                val sameHash = old?.sha256 == hash
                dao.upsertAsset(
                    AssetRecord(
                        id = id,
                        entityType = asset.text("entityType").ifBlank { type },
                        entitySyncId = asset.text("entitySyncId").ifBlank { syncId },
                        assetType = asset.text("assetType"),
                        version = asset.text("version").ifBlank { "edit" },
                        mimeType = asset.text("mimeType").ifBlank { "image/jpeg" },
                        byteSize = asset.long("byteSize"),
                        sha256 = hash,
                        available = asset.bool("available"),
                        localPath = if (sameHash) old?.localPath else null,
                        lastAccess = old?.lastAccess ?: 0,
                        pinned = old?.pinned ?: false,
                    ),
                )
            }
        }
        val edit = change["editOriginal"] as? JsonObject
        if (edit != null) {
            val id = edit.text("id")
            if (id.isNotBlank()) {
                val hash = edit.optionalText("sha256")
                val old = dao.assetById(id)
                dao.upsertAsset(
                    AssetRecord(
                        id = id,
                        entityType = edit.text("entityType").ifBlank { type },
                        entitySyncId = edit.text("entitySyncId").ifBlank { syncId },
                        assetType = "edit_original",
                        version = "edit",
                        mimeType = edit.text("mimeType").ifBlank { "image/jpeg" },
                        byteSize = edit.long("byteSize"),
                        sha256 = hash,
                        available = edit.bool("available"),
                        localPath = if (old?.sha256 == hash) old?.localPath else null,
                        lastAccess = old?.lastAccess ?: 0,
                        pinned = data.bool("isFavorite"),
                    ),
                )
            }
        }
    }

    private suspend fun reconcilePinnedAssets() {
        for (entity in dao.allEntities()) {
            if (entity.entityType != "film_photo" && entity.entityType != "digital_photo") continue
            val asset = dao.asset(entity.entityType, entity.syncId, "edit", "edit_original") ?: continue
            val favorite = entity.obj().bool("isFavorite")
            if (asset.pinned != favorite) {
                dao.setPinned(asset.id, favorite)
            }
        }
    }

    private suspend fun downloadThumbnails(
        jobs: List<AssetRecord>,
        onProgress: (Long) -> Unit,
    ): Long {
        val current = settings.current()
        val credential = secure.credential() ?: throw SyncException("设备凭证已失效，请重新配对")
        var completedBytes = 0L
        for (asset in jobs) {
            if (cancelRequested) throw SyncException("同步已取消")
            val target = File(thumbnailRoot, "${asset.id}.jpg")
            try {
                val result = withContext(Dispatchers.IO) {
                    api.downloadThumbnail(
                        current.serverUrl,
                        credential,
                        asset.id,
                        target,
                        asset.sha256,
                    ) { done, _ -> onProgress(completedBytes + done) }
                }
                dao.markDownloaded(asset.id, result.path, System.currentTimeMillis())
                completedBytes += result.byteSize
                onProgress(completedBytes)
            } catch (_: Exception) {
                // A single thumbnail failure must not abort the whole sync.
            }
        }
        return completedBytes
    }

    private suspend fun downloadPinnedEditOriginals(
        jobs: List<AssetRecord>,
        onProgress: (Long) -> Unit,
    ): Pair<Long, Int> {
        val required = jobs.sumOf { it.byteSize ?: 0L }
        val free = appContext.filesDir.usableSpace
        if (required > 0 && required + 256L * 1024 * 1024 > free) {
            throw SyncException(
                "手机空间不足：标星调色图需要 ${formatBytes(required)}，请在设置中清理缓存或释放空间",
            )
        }
        var completedBytes = 0L
        var failed = 0
        for (asset in jobs) {
            if (cancelRequested) throw SyncException("同步已取消")
            try {
                val file = downloadEditOriginalInternal(asset) { done, _ ->
                    onProgress(completedBytes + done)
                }
                completedBytes += file.length()
                onProgress(completedBytes)
            } catch (_: Exception) {
                failed++
            }
        }
        return completedBytes to failed
    }

    suspend fun downloadEditOriginal(
        assetId: String,
        onProgress: (Long, Long) -> Unit = { _, _ -> },
    ): File = mutex.withLock {
        val asset = dao.assetById(assetId) ?: throw SyncException("调色原图不存在")
        val file = downloadEditOriginalInternal(asset, onProgress)
        cleanupCache(settings.current().cacheLimitMb)
        file
    }

    private suspend fun downloadEditOriginalInternal(
        asset: AssetRecord,
        onProgress: (Long, Long) -> Unit,
    ): File {
        val cached = asset.localPath?.let(::File)
        if (cached != null && cached.isFile) {
            dao.markAccessed(asset.id, System.currentTimeMillis())
            return cached
        }
        val current = settings.current()
        val credential = secure.credential() ?: throw SyncException("设备凭证已失效，请重新配对")
        val extension = when (asset.mimeType) {
            "image/png" -> "png"
            "image/webp" -> "webp"
            else -> "jpg"
        }
        val target = File(editRoot, "${asset.id}.$extension")
        val result = withContext(Dispatchers.IO) {
            api.downloadEditOriginal(
                current.serverUrl,
                credential,
                asset.id,
                target,
                asset.sha256,
                onProgress,
            )
        }
        dao.markDownloaded(asset.id, result.path, System.currentTimeMillis())
        return File(result.path)
    }

    suspend fun markThumbnailAccess(asset: AssetRecord) {
        if (asset.lastAccess == 0L) dao.markAccessed(asset.id, System.currentTimeMillis())
    }

    suspend fun cachedBytes(): Long = dao.cachedViewBytes()

    suspend fun cachedPinnedBytes(): Long = dao.cachedPinnedBytes()

    suspend fun setWifiOnly(value: Boolean) = settings.setWifiOnly(value)

    suspend fun setCacheLimitMb(value: Int) {
        settings.setCacheLimitMb(value)
        cleanupCache(value)
    }

    suspend fun clearViewCache() = mutex.withLock {
        for (asset in dao.cachedEditOriginals()) {
            asset.localPath?.let { runCatching { File(it).delete() } }
            dao.clearLocalPath(asset.id)
        }
    }

    suspend fun clearPinnedEditOriginals() = mutex.withLock {
        for (asset in dao.pinnedEditOriginals()) {
            asset.localPath?.let { runCatching { File(it).delete() } }
            dao.clearLocalPath(asset.id)
        }
    }

    suspend fun cleanupCache(limitMb: Int) = withContext(Dispatchers.IO) {
        if (limitMb <= 0) return@withContext
        val limit = limitMb.toLong() * 1024 * 1024
        var total = dao.cachedViewBytes()
        if (total <= limit) return@withContext
        for (asset in dao.cachedEditOriginals()) {
            if (total <= limit) break
            val path = asset.localPath ?: continue
            val size = asset.byteSize ?: File(path).length()
            if (runCatching { File(path).delete() }.getOrDefault(false)) {
                dao.clearLocalPath(asset.id)
                total -= size
            }
        }
    }

    suspend fun clearAllData() = mutex.withLock {
        db.withTransaction {
            dao.clearEntities()
            dao.clearAssets()
            dao.clearMeta()
        }
        deleteTree(cacheRoot)
        settings.setCursor(0)
        settings.setLastSync(0)
    }

    suspend fun beginPairing(rawPayload: String): String {
        val payload = parsePairingPayload(rawPayload)
        if (payload.protocolVersion != 1) throw SyncException("手机端与电脑端协议不兼容")
        if (payload.expiresAt > 0 && payload.expiresAt * 1000 < System.currentTimeMillis()) {
            throw SyncException("二维码已过期，请在电脑端重新生成")
        }
        val health = withContext(Dispatchers.IO) { api.health(payload.url) }
        if (health.fingerprint.isNotBlank() &&
            payload.fingerprint.isNotBlank() &&
            health.fingerprint != payload.fingerprint
        ) {
            throw SyncException("服务指纹不一致，请确认连接的是自己的电脑")
        }
        val current = settings.current()
        settings.setPendingPairing(payload.url, payload.fingerprint)
        val request = withContext(Dispatchers.IO) {
            api.pairRequest(payload.url, payload.token, current.deviceName)
        }
        return request.requestId
    }

    suspend fun waitForPairing(
        requestId: String,
        onStatus: (String) -> Unit = {},
    ): PairStatus {
        val pending = settings.current()
        val base = pending.pendingServerUrl.ifBlank { pending.serverUrl }
        if (base.isBlank()) throw SyncException("配对地址缺失")
        repeat(90) {
            val status = withContext(Dispatchers.IO) { api.pairStatus(base, requestId) }
            onStatus(status.status)
            when (status.status) {
                "approved" -> {
                    val credential = status.credential ?: throw SyncException("电脑端未返回设备凭证")
                    val deviceId = status.deviceId ?: throw SyncException("电脑端未返回设备 ID")
                    secure.saveCredential(credential)
                    settings.setPairing(base, pending.pendingFingerprint, deviceId)
                    return status
                }
                "denied" -> throw SyncException("电脑端拒绝了配对请求")
                "expired", "credentialExpired" -> throw SyncException("配对已过期，请重新生成二维码")
                "pending" -> delay(TimeUnit.SECONDS.toMillis(2))
                else -> delay(TimeUnit.SECONDS.toMillis(2))
            }
        }
        throw SyncException("等待电脑端批准超时")
    }

    suspend fun unpair() {
        secure.clear()
        settings.clearPairing()
    }

    private fun isOnWifi(): Boolean {
        val manager = connectivity ?: return true
        val network = manager.activeNetwork ?: return false
        val capabilities = manager.getNetworkCapabilities(network) ?: return false
        return capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
    }

    private fun formatBytes(bytes: Long): String {
        val mb = bytes / 1024.0 / 1024.0
        return if (mb < 1024) "%.1f MB".format(mb) else "%.2f GB".format(mb / 1024.0)
    }

    private fun deleteTree(file: File) {
        if (!file.exists()) return
        file.walkBottomUp().forEach { runCatching { it.delete() } }
    }

    fun thumbnailFile(asset: AssetRecord): File? = asset.localPath?.let(::File)?.takeIf { it.isFile }

    fun editFile(asset: AssetRecord): File? = asset.localPath?.let(::File)?.takeIf { it.isFile }
}

internal fun parsePairingPayload(raw: String): PairingPayload {
    val json = runCatching { AppJson.parseToJsonElement(raw.trim()).jsonObject }
        .getOrElse { throw SyncException("配对信息不是有效 JSON") }
    val version = json.int("v") ?: json.int("protocolVersion") ?: 1
    val url = json.text("url").trimEnd('/')
    val token = json.text("token")
    val fingerprint = json.text("fingerprint")
    val expiresAt = json.long("expiresAt") ?: 0
    if (url.isBlank() || token.isBlank()) throw SyncException("配对信息缺少地址或令牌")
    return PairingPayload(
        protocolVersion = version,
        url = url,
        token = token,
        fingerprint = fingerprint,
        expiresAt = expiresAt,
        serverName = json.text("serverName").ifBlank { "GoShootFilm" },
    )
}
