package com.k1taproject.goshootfilm.data

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.room.withTransaction
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.onStart
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit

open class SyncException(message: String) : Exception(message)

class DifferentServerException(message: String) : SyncException(message)

enum class SyncPhase {
    Connecting,
    Checking,
    Updating,
    VerifyingCache,
    DownloadingThumbnails,
    DownloadingPinned,
    Cleaning,
    Completed,
    Partial,
}

enum class EditCacheStatus {
    Checking,
    Cached,
    NotCached,
    NoEdit,
}

data class SyncProgress(
    val phase: SyncPhase,
    val message: String,
    val currentItems: Int = 0,
    val totalItems: Int = 0,
    val downloadedBytes: Long = 0,
    val totalBytes: Long? = null,
    val fraction: Float? = null,
    val failedItems: Int = 0,
)

data class PairingPayload(
    val protocolVersion: Int,
    val url: String,
    val token: String,
    val fingerprint: String,
    val expiresAt: Long,
    val serverName: String,
)

private data class DownloadBatchResult(
    val bytes: Long,
    val completed: Int,
    val failed: Int,
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
    private val assetLocks = ConcurrentHashMap<String, Mutex>()
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

    fun thumbnail(type: String, syncId: String): Flow<AssetRecord?> =
        dao.observeThumbnail(type, syncId)

    suspend fun editOriginal(type: String, syncId: String): AssetRecord? =
        dao.asset(type, syncId, "edit", "edit_original")

    fun editCacheStatus(type: String, syncId: String, hasEdit: Boolean): Flow<EditCacheStatus> {
        if (!hasEdit) return flowOf(EditCacheStatus.NoEdit)
        return dao.observeAsset(type, syncId, "edit", "edit_original")
            .map { asset -> editCacheStatus(hasEdit = true, asset = asset) }
            .onStart { emit(EditCacheStatus.Checking) }
            .distinctUntilChanged()
            .flowOn(Dispatchers.IO)
    }

    suspend fun localEditFile(assetId: String): File? {
        val asset = dao.assetById(assetId) ?: return null
        if (localFileIsValid(asset)) return asset.localPath?.let(::File)
        if (asset.localPath != null) dao.clearLocalPath(asset.id)
        return null
    }

    suspend fun currentSettings(): AppSettings = settings.current()

    fun cancelSync() {
        cancelRequested = true
    }

    suspend fun sync(onProgress: (SyncProgress) -> Unit = {}) = mutex.withLock {
        cancelRequested = false
        val current = settings.current()
        val credential = secure.credential()
        if (current.serverUrl.isBlank() || credential.isNullOrBlank()) {
            throw SyncException("当前未配对，请先连接电脑")
        }
        if (current.wifiOnly && !isOnWifi()) {
            throw SyncException("当前不是 Wi-Fi 网络；请连接与电脑相同的 Wi-Fi 后再同步")
        }

        onProgress(SyncProgress(SyncPhase.Connecting, "正在连接电脑…"))
        validateServer(current)

        var cursor = current.cursor
        var pageCount = 0
        var changedItems = 0
        var resetHandled = false
        onProgress(SyncProgress(SyncPhase.Checking, "正在检查缺失信息…"))
        while (pageCount++ < 500) {
            if (cancelRequested) throw SyncException("同步已取消")
            val response = try {
                withContext(Dispatchers.IO) {
                    api.changes(current.serverUrl, credential, cursor)
                }
            } catch (error: ApiException) {
                if (error.status == 401) {
                    throw SyncException("配对已失效或设备已撤销，请重新配对")
                }
                throw SyncException(error.message)
            } catch (_: Exception) {
                throw SyncException("电脑离线或同步服务未启动")
            }
            val protocol = response.int("protocolVersion") ?: 0
            if (protocol != 1) throw SyncException("手机端与电脑端协议不兼容")
            if (response.bool("resetRequired")) {
                if (resetHandled || cursor == 0L) throw SyncException("桌面端无法提供完整同步数据")
                // 游标失效只重做稳定 ID 的元数据核对；本地档案和图片缓存必须保留。
                cursor = 0
                resetHandled = true
                settings.setCursor(0)
                onProgress(SyncProgress(SyncPhase.Checking, "正在重新核对全部档案…"))
                continue
            }
            val changes = response["changes"]?.jsonArray
                ?: throw SyncException("同步响应缺少 changes")
            val pageStart = changedItems
            db.withTransaction {
                for ((index, element) in changes.withIndex()) {
                    applyChange(element.jsonObject)
                    onProgress(
                        SyncProgress(
                            SyncPhase.Updating,
                            "正在更新档案 ${pageStart + index + 1} 项",
                            currentItems = pageStart + index + 1,
                        ),
                    )
                }
            }
            changedItems += changes.size
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

        onProgress(SyncProgress(SyncPhase.VerifyingCache, "正在核对本地缓存…"))
        auditLocalResources()
        reconcilePinnedAssets()
        val thumbnailJobs = dao.allThumbnails().filter { asset ->
            asset.available && asset.localPath == null
        }
        val pinnedJobs = dao.pinnedEditOriginals().filter { asset ->
            asset.available && asset.localPath == null
        }

        val thumbnailResult = downloadThumbnails(thumbnailJobs, onProgress)
        if (cancelRequested) throw SyncException("同步已取消")
        val pinnedResult = downloadPinnedEditOriginals(pinnedJobs, onProgress)
        val failedItems = thumbnailResult.failed + pinnedResult.failed

        onProgress(SyncProgress(SyncPhase.Cleaning, "正在整理缓存…"))
        settings.setLastSync(System.currentTimeMillis())
        cleanupCache(settings.current().cacheLimitMb)
        val hasDownloads = thumbnailJobs.isNotEmpty() || pinnedJobs.isNotEmpty()
        val finalMessage = when {
            failedItems > 0 -> "同步完成，$failedItems 张图片下载失败，可再次同步重试"
            changedItems == 0 && !hasDownloads -> "已是最新"
            else -> "同步完成"
        }
        onProgress(
            SyncProgress(
                phase = if (failedItems > 0) SyncPhase.Partial else SyncPhase.Completed,
                message = finalMessage,
                currentItems = thumbnailResult.completed + pinnedResult.completed,
                totalItems = thumbnailJobs.size + pinnedJobs.size,
                downloadedBytes = thumbnailResult.bytes + pinnedResult.bytes,
                totalBytes = knownTotalBytes(thumbnailJobs + pinnedJobs),
                fraction = 1f,
                failedItems = failedItems,
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
                val sameHash = hash == null || old?.sha256 == hash || old?.localSha256 == hash
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
                        localSha256 = if (sameHash) old?.localSha256 else null,
                        localByteSize = if (sameHash) old?.localByteSize else null,
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
                        localPath = if (hash == null || old?.sha256 == hash || old?.localSha256 == hash) {
                            old?.localPath
                        } else {
                            null
                        },
                        localSha256 = if (hash == null || old?.sha256 == hash || old?.localSha256 == hash) {
                            old?.localSha256
                        } else {
                            null
                        },
                        localByteSize = if (hash == null || old?.sha256 == hash || old?.localSha256 == hash) {
                            old?.localByteSize
                        } else {
                            null
                        },
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
        onProgress: (SyncProgress) -> Unit,
    ): DownloadBatchResult {
        val current = settings.current()
        val credential = secure.credential() ?: throw SyncException("设备凭证已失效，请重新配对")
        var completedBytes = 0L
        var completed = 0
        var failed = 0
        val totalBytes = knownTotalBytes(jobs)
        for ((index, asset) in jobs.withIndex()) {
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
                    ) { done, _ ->
                        onProgress(
                            downloadProgress(
                                SyncPhase.DownloadingThumbnails,
                                "正在下载缩略图 ${index + 1} / ${jobs.size}",
                                index,
                                jobs.size,
                                completedBytes + done,
                                totalBytes,
                                failed,
                            ),
                        )
                    }
                }
                dao.markDownloaded(
                    asset.id,
                    result.path,
                    result.sha256,
                    result.byteSize,
                    System.currentTimeMillis(),
                )
                completedBytes += result.byteSize
                completed++
            } catch (error: ApiException) {
                if (error.status == 401) {
                    throw SyncException("配对已失效或设备已撤销，请重新配对")
                }
                failed++
            } catch (_: Exception) {
                // A single thumbnail failure must not abort the whole sync.
                failed++
            }
            onProgress(
                downloadProgress(
                    SyncPhase.DownloadingThumbnails,
                    "正在下载缩略图 ${index + 1} / ${jobs.size}",
                    index + 1,
                    jobs.size,
                    completedBytes,
                    totalBytes,
                    failed,
                ),
            )
        }
        return DownloadBatchResult(completedBytes, completed, failed)
    }

    private suspend fun downloadPinnedEditOriginals(
        jobs: List<AssetRecord>,
        onProgress: (SyncProgress) -> Unit,
    ): DownloadBatchResult {
        val required = jobs.sumOf { it.byteSize ?: 0L }
        val free = appContext.filesDir.usableSpace
        if (required > 0 && required + 256L * 1024 * 1024 > free) {
            throw SyncException(
                "手机空间不足：标星调色图需要 ${formatBytes(required)}，请在设置中清理缓存或释放空间",
            )
        }
        var completedBytes = 0L
        var completed = 0
        var failed = 0
        val totalBytes = knownTotalBytes(jobs)
        for ((index, asset) in jobs.withIndex()) {
            if (cancelRequested) throw SyncException("同步已取消")
            try {
                val file = downloadEditOriginalInternal(asset) { done, _ ->
                    onProgress(
                        downloadProgress(
                            SyncPhase.DownloadingPinned,
                            "正在下载标星调色图 ${index + 1} / ${jobs.size}",
                            index,
                            jobs.size,
                            completedBytes + done,
                            totalBytes,
                            failed,
                        ),
                    )
                }
                completedBytes += file.length()
                completed++
            } catch (error: ApiException) {
                if (error.status == 401) {
                    throw SyncException("配对已失效或设备已撤销，请重新配对")
                }
                failed++
            } catch (_: Exception) {
                failed++
            }
            onProgress(
                downloadProgress(
                    SyncPhase.DownloadingPinned,
                    "正在下载标星调色图 ${index + 1} / ${jobs.size}",
                    index + 1,
                    jobs.size,
                    completedBytes,
                    totalBytes,
                    failed,
                ),
            )
        }
        return DownloadBatchResult(completedBytes, completed, failed)
    }

    suspend fun downloadEditOriginal(
        assetId: String,
        onProgress: (Long, Long) -> Unit = { _, _ -> },
    ): File = assetLocks.getOrPut(assetId) { Mutex() }.withLock {
        val asset = dao.assetById(assetId) ?: throw SyncException("调色图不存在")
        val file = downloadEditOriginalInternal(asset, onProgress)
        cleanupCache(settings.current().cacheLimitMb)
        file
    }

    private suspend fun downloadEditOriginalInternal(
        asset: AssetRecord,
        onProgress: (Long, Long) -> Unit,
    ): File {
        val cached = asset.localPath?.let(::File)
        if (cached != null && localFileIsValid(asset)) {
            dao.markAccessed(asset.id, System.currentTimeMillis())
            return cached
        }
        if (asset.localPath != null) dao.clearLocalPath(asset.id)
        if (!asset.available) throw SyncException("该照片暂无调色图")
        val current = settings.current()
        val credential = secure.credential() ?: throw SyncException("设备凭证已失效，请重新配对")
        if (current.serverUrl.isBlank()) {
            throw SyncException("调色图尚未下载到本机，连接电脑后可下载")
        }
        validateServer(current)
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
        dao.markDownloaded(
            asset.id,
            result.path,
            result.sha256,
            result.byteSize,
            System.currentTimeMillis(),
        )
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
            val size = asset.localByteSize ?: File(path).length()
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

    suspend fun beginPairing(rawPayload: String, allowDifferentServer: Boolean = false): String {
        val payload = parsePairingPayload(rawPayload)
        if (payload.protocolVersion != 1) throw SyncException("手机端与电脑端协议不兼容")
        if (payload.expiresAt > 0 && payload.expiresAt * 1000 < System.currentTimeMillis()) {
            throw SyncException("二维码已过期，请在电脑端重新生成")
        }
        val health = try {
            withContext(Dispatchers.IO) { api.health(payload.url) }
        } catch (_: Exception) {
            throw SyncException("电脑离线或同步服务未启动")
        }
        if (health.fingerprint.isNotBlank() &&
            payload.fingerprint.isNotBlank() &&
            health.fingerprint != payload.fingerprint
        ) {
            throw SyncException("服务指纹不一致，请确认连接的是自己的电脑")
        }
        val current = settings.current()
        if (!allowDifferentServer && current.serverFingerprint.isNotBlank() &&
            payload.fingerprint.isNotBlank() &&
            current.serverFingerprint != payload.fingerprint
        ) {
            throw DifferentServerException("检测到另一个数据源。确认后才会切换连接，本地数据不会被自动合并或删除。")
        }
        settings.setPendingPairing(payload.url, payload.fingerprint)
        return try {
            withContext(Dispatchers.IO) {
                api.pairRequest(payload.url, payload.token, current.deviceName)
            }.requestId
        } catch (error: Exception) {
            settings.clearPendingPairing()
            throw error
        }
    }

    suspend fun waitForPairing(
        requestId: String,
        onStatus: (String) -> Unit = {},
    ): PairStatus {
        val pending = settings.current()
        val base = pending.pendingServerUrl.ifBlank { pending.serverUrl }
        if (base.isBlank()) throw SyncException("配对地址缺失")
        try {
            repeat(90) {
                val status = withContext(Dispatchers.IO) { api.pairStatus(base, requestId) }
                onStatus(status.status)
                when (status.status) {
                    "approved" -> {
                        val credential = status.credential ?: throw SyncException("电脑端未返回设备凭证")
                        val deviceId = status.deviceId ?: throw SyncException("电脑端未返回设备 ID")
                        val previousCredential = secure.credential()
                        try {
                            secure.saveCredential(credential)
                            settings.setPairing(
                                base,
                                pending.pendingFingerprint,
                                deviceId,
                                resetCursor = true,
                            )
                        } catch (error: Exception) {
                            if (previousCredential == null) {
                                secure.clear()
                            } else {
                                secure.saveCredential(previousCredential)
                            }
                            throw error
                        }
                        return status
                    }
                    "denied" -> throw SyncException("电脑端拒绝了配对请求")
                    "expired", "credentialExpired" -> throw SyncException("配对已过期，请重新生成二维码")
                    "pending" -> delay(TimeUnit.SECONDS.toMillis(2))
                    else -> delay(TimeUnit.SECONDS.toMillis(2))
                }
            }
            throw SyncException("等待电脑端批准超时")
        } finally {
            settings.clearPendingPairing()
        }
    }

    suspend fun unpair() {
        cancelSync()
        mutex.withLock {
            secure.clear()
            settings.clearPairing()
        }
    }

    suspend fun cancelPairing() {
        settings.clearPendingPairing()
    }

    private suspend fun validateServer(current: AppSettings): HealthInfo {
        val health = try {
            withContext(Dispatchers.IO) { api.health(current.serverUrl) }
        } catch (_: Exception) {
            throw SyncException("电脑离线或同步服务未启动；若电脑地址已变化，请重新扫码更新连接")
        }
        if (health.protocolVersion != 1) {
            throw SyncException("手机端与电脑端协议不兼容")
        }
        if (current.serverFingerprint.isNotBlank() &&
            health.fingerprint.isNotBlank() &&
            current.serverFingerprint != health.fingerprint
        ) {
            throw SyncException("服务器指纹不匹配，未发送设备凭证；请确认电脑地址或重新配对")
        }
        return health
    }

    private suspend fun auditLocalResources() {
        for (asset in dao.allAssets()) {
            if (asset.localPath == null) continue
            val inspected = inspectLocalFile(asset)
            if (inspected == null) {
                dao.clearLocalPath(asset.id)
            } else {
                dao.markDownloaded(
                    asset.id,
                    asset.localPath,
                    inspected.first,
                    inspected.second,
                    asset.lastAccess,
                )
            }
        }
    }

    private suspend fun localFileIsValid(asset: AssetRecord): Boolean =
        inspectLocalFile(asset) != null

    private suspend fun inspectLocalFile(asset: AssetRecord): Pair<String, Long>? =
        withContext(Dispatchers.IO) { inspectCachedAsset(asset) }

    private fun knownTotalBytes(assets: List<AssetRecord>): Long? {
        if (assets.isEmpty() || assets.any { it.byteSize == null }) return null
        return assets.sumOf { it.byteSize ?: 0L }
    }

    private fun downloadProgress(
        phase: SyncPhase,
        message: String,
        currentItems: Int,
        totalItems: Int,
        downloadedBytes: Long,
        totalBytes: Long?,
        failedItems: Int,
    ): SyncProgress {
        val fraction = syncFraction(currentItems, totalItems, downloadedBytes, totalBytes)
        return SyncProgress(
            phase = phase,
            message = message,
            currentItems = currentItems,
            totalItems = totalItems,
            downloadedBytes = downloadedBytes,
            totalBytes = totalBytes,
            fraction = fraction,
            failedItems = failedItems,
        )
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

internal fun inspectCachedAsset(asset: AssetRecord): Pair<String, Long>? {
    val file = asset.localPath?.let(::File)?.takeIf { it.isFile } ?: return null
    val size = file.length()
    val expectedSize = asset.localByteSize ?: asset.byteSize
    if (expectedSize != null && expectedSize != size) return null
    val digest = MessageDigest.getInstance("SHA-256")
    file.inputStream().use { input ->
        val buffer = ByteArray(64 * 1024)
        while (true) {
            val read = input.read(buffer)
            if (read <= 0) break
            digest.update(buffer, 0, read)
        }
    }
    val hash = digest.digest().joinToString("") { "%02x".format(it) }
    val expectedHash = asset.localSha256 ?: asset.sha256
    if (expectedHash != null && !expectedHash.equals(hash, ignoreCase = true)) return null
    return hash to size
}

internal fun editCacheStatus(hasEdit: Boolean, asset: AssetRecord?): EditCacheStatus {
    if (!hasEdit) return EditCacheStatus.NoEdit
    if (asset == null || asset.assetType != "edit_original" || asset.version != "edit") {
        return EditCacheStatus.NotCached
    }
    val path = asset.localPath ?: return EditCacheStatus.NotCached
    if (path.endsWith(".part", ignoreCase = true)) return EditCacheStatus.NotCached
    val file = File(path)
    val expectedSize = asset.localByteSize ?: return EditCacheStatus.NotCached
    val hasVerifiedMetadata = !asset.localSha256.isNullOrBlank()
    return if (hasVerifiedMetadata && file.isFile && file.length() == expectedSize) {
        EditCacheStatus.Cached
    } else {
        EditCacheStatus.NotCached
    }
}

internal fun syncFraction(
    currentItems: Int,
    totalItems: Int,
    downloadedBytes: Long,
    totalBytes: Long?,
): Float? = if (totalBytes != null && totalBytes > 0) {
    (downloadedBytes.toFloat() / totalBytes).coerceIn(0f, 1f)
} else if (totalItems > 0) {
    (currentItems.toFloat() / totalItems).coerceIn(0f, 1f)
} else {
    null
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
