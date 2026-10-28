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
    DownloadingEdits,
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
    val reason: String? = null,
)

private class CacheCapacityException(message: String) : SyncException(message)

private const val MIN_FREE_BYTES = 256L * 1024 * 1024

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
        val thumbnails = dao.allThumbnails().filter { it.available }
        val edits = dao.allEditOriginals().filter { it.available }

        val thumbnailResult = downloadThumbnails(thumbnails, onProgress)
        if (cancelRequested) throw SyncException("同步已取消")
        val editResult = downloadEditOriginals(edits, onProgress)
        val failedItems = thumbnailResult.failed + editResult.failed

        onProgress(SyncProgress(SyncPhase.Cleaning, "正在确认同步结果…"))
        settings.setLastSync(System.currentTimeMillis())
        val failureReason = listOfNotNull(thumbnailResult.reason, editResult.reason)
            .distinct()
            .joinToString("；")
        val finalMessage = when {
            failedItems > 0 && failureReason.isNotBlank() ->
                "资料已同步，$failedItems 张图片未完成缓存：$failureReason"
            failedItems > 0 -> "资料已同步，$failedItems 张图片未完成缓存，点击同步可重试"
            else -> "资料与图片均已同步"
        }
        onProgress(
            SyncProgress(
                phase = if (failedItems > 0) SyncPhase.Partial else SyncPhase.Completed,
                message = finalMessage,
                currentItems = thumbnailResult.completed + editResult.completed,
                totalItems = thumbnails.size + edits.size,
                downloadedBytes = thumbnailResult.bytes + editResult.bytes,
                totalBytes = knownTotalBytes(thumbnails + edits),
                fraction = if (failedItems == 0) 1f else null,
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
        assets: List<AssetRecord>,
        onProgress: (SyncProgress) -> Unit,
    ): DownloadBatchResult {
        val jobs = pendingAvailableAssets(assets)
        val current = settings.current()
        val credential = secure.credential() ?: throw SyncException("设备凭证已失效，请重新配对")
        var completedBytes = cachedBytes(assets)
        var completed = assets.size - jobs.size
        var failed = 0
        var reason: String? = null
        val totalBytes = knownTotalBytes(assets)
        val required = knownTotalBytes(jobs)
        if (required != null && required + MIN_FREE_BYTES > appContext.filesDir.usableSpace) {
            reason = "手机空间不足，请释放空间后重试"
            onProgress(
                downloadProgress(
                    SyncPhase.DownloadingThumbnails,
                    reason,
                    completed,
                    assets.size,
                    completedBytes,
                    totalBytes,
                    jobs.size,
                ),
            )
            return DownloadBatchResult(completedBytes, completed, jobs.size, reason)
        }
        onProgress(
            downloadProgress(
                SyncPhase.DownloadingThumbnails,
                if (jobs.isEmpty()) "缩略图已全部缓存" else "正在下载缩略图",
                completed,
                assets.size,
                completedBytes,
                totalBytes,
                failed,
            ),
        )
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
                        if (appContext.filesDir.usableSpace <= MIN_FREE_BYTES) {
                            throw CacheCapacityException("手机空间不足，请释放空间后重试")
                        }
                        onProgress(
                            downloadProgress(
                                SyncPhase.DownloadingThumbnails,
                                "正在下载缩略图 ${completed + 1} / ${assets.size}",
                                completed,
                                assets.size,
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
            } catch (error: CacheCapacityException) {
                reason = error.message
                failed += jobs.size - index
                break
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
                    "正在下载缩略图 $completed / ${assets.size}",
                    completed,
                    assets.size,
                    completedBytes,
                    totalBytes,
                    failed,
                ),
            )
        }
        return DownloadBatchResult(completedBytes, completed, failed, reason)
    }

    private suspend fun downloadEditOriginals(
        assets: List<AssetRecord>,
        onProgress: (SyncProgress) -> Unit,
    ): DownloadBatchResult {
        val jobs = pendingAvailableAssets(assets)
        var completedBytes = cachedBytes(assets)
        var completed = assets.size - jobs.size
        var failed = 0
        var reason: String? = null
        val totalBytes = knownTotalBytes(assets)
        val required = jobs.sumOf { it.byteSize ?: 0L }
        val cacheLimitBytes = settings.current().cacheLimitMb
            .takeIf { it > 0 }
            ?.toLong()
            ?.times(1024 * 1024)
        val preflightMessage = if (jobs.isEmpty()) {
            null
        } else {
            capacityMessage(
                cachedBytes = completedBytes,
                requiredBytes = required,
                cacheLimitBytes = cacheLimitBytes,
                usableSpace = appContext.filesDir.usableSpace,
            )
        }
        preflightMessage?.let { message ->
            onProgress(
                downloadProgress(
                    SyncPhase.DownloadingEdits,
                    message,
                    completed,
                    assets.size,
                    completedBytes,
                    totalBytes,
                    jobs.size,
                ),
            )
            return DownloadBatchResult(completedBytes, completed, jobs.size, message)
        }
        onProgress(
            downloadProgress(
                SyncPhase.DownloadingEdits,
                if (jobs.isEmpty()) "调色图已全部缓存" else "正在下载调色图",
                completed,
                assets.size,
                completedBytes,
                totalBytes,
                failed,
            ),
        )
        for ((index, asset) in jobs.withIndex()) {
            if (cancelRequested) throw SyncException("同步已取消")
            try {
                val file = assetLocks.getOrPut(asset.id) { Mutex() }.withLock {
                    val currentAsset = dao.assetById(asset.id) ?: asset
                    downloadEditOriginalInternal(currentAsset) { done, _ ->
                        if (appContext.filesDir.usableSpace <= MIN_FREE_BYTES) {
                            throw CacheCapacityException("手机剩余空间不足，请释放空间后重试")
                        }
                        capacityMessage(
                            cachedBytes = completedBytes,
                            requiredBytes = done,
                            cacheLimitBytes = cacheLimitBytes,
                            usableSpace = Long.MAX_VALUE,
                        )?.let { throw CacheCapacityException(it) }
                        onProgress(
                            downloadProgress(
                                SyncPhase.DownloadingEdits,
                                "正在下载调色图 ${completed + 1} / ${assets.size}",
                                completed,
                                assets.size,
                                completedBytes + done,
                                totalBytes,
                                failed,
                            ),
                        )
                    }
                }
                completedBytes += file.length()
                completed++
            } catch (error: CacheCapacityException) {
                reason = error.message
                failed += jobs.size - index
                break
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
                    SyncPhase.DownloadingEdits,
                    "正在下载调色图 $completed / ${assets.size}",
                    completed,
                    assets.size,
                    completedBytes,
                    totalBytes,
                    failed,
                ),
            )
        }
        return DownloadBatchResult(completedBytes, completed, failed, reason)
    }

    suspend fun downloadEditOriginal(
        assetId: String,
        onProgress: (Long, Long) -> Unit = { _, _ -> },
    ): File = assetLocks.getOrPut(assetId) { Mutex() }.withLock {
        val asset = dao.assetById(assetId) ?: throw SyncException("调色图不存在")
        val cachedBytes = dao.cachedViewBytes()
        val cacheLimitBytes = settings.current().cacheLimitMb
            .takeIf { it > 0 }
            ?.toLong()
            ?.times(1024 * 1024)
        if (asset.localPath == null) {
            capacityMessage(
                cachedBytes = cachedBytes,
                requiredBytes = asset.byteSize ?: 0L,
                cacheLimitBytes = cacheLimitBytes,
                usableSpace = appContext.filesDir.usableSpace,
            )?.let { throw SyncException(it) }
        }
        downloadEditOriginalInternal(asset) { done, total ->
            if (appContext.filesDir.usableSpace <= MIN_FREE_BYTES) {
                throw SyncException("手机剩余空间不足，请释放空间后重试")
            }
            capacityMessage(
                cachedBytes = cachedBytes,
                requiredBytes = done,
                cacheLimitBytes = cacheLimitBytes,
                usableSpace = Long.MAX_VALUE,
            )?.let { throw SyncException(it) }
            onProgress(done, total)
        }
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

    suspend fun cachedEditBytes(): Long = dao.cachedViewBytes()

    suspend fun setWifiOnly(value: Boolean) = settings.setWifiOnly(value)

    suspend fun setCacheLimitMb(value: Int) = mutex.withLock {
        settings.setCacheLimitMb(value)
        cleanupCache(value)
    }

    suspend fun clearEditCache() = mutex.withLock {
        for (asset in dao.cachedEditOriginals()) {
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

    private fun deleteTree(file: File) {
        if (!file.exists()) return
        file.walkBottomUp().forEach { runCatching { it.delete() } }
    }

    fun thumbnailFile(asset: AssetRecord): File? = asset.localPath?.let(::File)?.takeIf { it.isFile }

    fun editFile(asset: AssetRecord): File? = asset.localPath?.let(::File)?.takeIf { it.isFile }
}

internal fun inspectCachedAsset(asset: AssetRecord): Pair<String, Long>? {
    val path = asset.localPath ?: return null
    if (path.endsWith(".part", ignoreCase = true)) return null
    val file = File(path).takeIf { it.isFile } ?: return null
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
    val localHash = asset.localSha256 ?: return EditCacheStatus.NotCached
    val matchesCurrentVersion = asset.sha256.isNullOrBlank() || asset.sha256.equals(localHash, ignoreCase = true)
    return if (matchesCurrentVersion && file.isFile && file.length() == expectedSize) {
        EditCacheStatus.Cached
    } else {
        EditCacheStatus.NotCached
    }
}

internal fun pendingAvailableAssets(assets: List<AssetRecord>): List<AssetRecord> =
    assets.filter { it.available && it.localPath == null }

private fun cachedBytes(assets: List<AssetRecord>): Long = assets
    .filter { it.localPath != null }
    .sumOf { it.localByteSize ?: it.byteSize ?: 0L }

internal fun capacityMessage(
    cachedBytes: Long,
    requiredBytes: Long,
    cacheLimitBytes: Long?,
    usableSpace: Long,
): String? = when {
    cacheLimitBytes != null && cachedBytes + requiredBytes > cacheLimitBytes ->
        "调色图缓存限制不足，请在设置中提高限制或清理缓存后重试"
    requiredBytes + MIN_FREE_BYTES > usableSpace ->
        "手机剩余空间不足，请释放空间后重试"
    else -> null
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
