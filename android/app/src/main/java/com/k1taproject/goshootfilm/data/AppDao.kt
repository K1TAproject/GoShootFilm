package com.k1taproject.goshootfilm.data

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow

@Dao
interface AppDao {
    @Query("SELECT * FROM entities WHERE entityType = :type AND deleted = 0")
    fun observeEntities(type: String): Flow<List<EntityRecord>>

    @Query("SELECT * FROM entities WHERE entityType = :type AND deleted = 0")
    suspend fun entities(type: String): List<EntityRecord>

    @Query("SELECT * FROM entities WHERE entityType = :type AND syncId = :syncId LIMIT 1")
    suspend fun entity(type: String, syncId: String): EntityRecord?

    @Query("SELECT * FROM entities WHERE deleted = 0")
    suspend fun allEntities(): List<EntityRecord>

    @Upsert
    suspend fun upsertEntity(entity: EntityRecord)

    @Query("DELETE FROM entities WHERE entityType = :type AND syncId = :syncId")
    suspend fun deleteEntity(type: String, syncId: String)

    @Query("DELETE FROM entities")
    suspend fun clearEntities()

    @Query("SELECT * FROM assets WHERE entityType = :type AND entitySyncId = :syncId")
    suspend fun assetsFor(type: String, syncId: String): List<AssetRecord>

    @Query("SELECT * FROM assets WHERE entityType = :type AND entitySyncId = :syncId AND version = :version AND assetType = :assetType LIMIT 1")
    suspend fun asset(type: String, syncId: String, version: String, assetType: String): AssetRecord?

    @Query("SELECT * FROM assets WHERE entityType = :type AND entitySyncId = :syncId AND version = :version AND assetType = :assetType LIMIT 1")
    fun observeAsset(type: String, syncId: String, version: String, assetType: String): Flow<AssetRecord?>

    @Query("SELECT * FROM assets WHERE entityType = :type AND entitySyncId = :syncId AND assetType = 'thumbnail' AND (available = 1 OR localPath IS NOT NULL) ORDER BY CASE version WHEN 'edit' THEN 0 WHEN 'lab' THEN 1 WHEN 'raw' THEN 2 ELSE 3 END LIMIT 1")
    fun observeThumbnail(type: String, syncId: String): Flow<AssetRecord?>

    @Query("SELECT * FROM assets WHERE id = :id LIMIT 1")
    suspend fun assetById(id: String): AssetRecord?

    @Query("SELECT * FROM assets WHERE assetType = 'thumbnail'")
    suspend fun allThumbnails(): List<AssetRecord>

    @Query("SELECT * FROM assets")
    suspend fun allAssets(): List<AssetRecord>

    @Query("SELECT * FROM assets WHERE assetType = 'edit_original' ORDER BY lastAccess ASC")
    suspend fun allEditOriginals(): List<AssetRecord>

    @Query("SELECT * FROM assets WHERE assetType = 'edit_original' AND pinned = 1")
    suspend fun pinnedEditOriginals(): List<AssetRecord>

    @Query("SELECT * FROM assets WHERE localPath IS NOT NULL AND assetType = 'edit_original' AND pinned = 0 ORDER BY lastAccess ASC")
    suspend fun cachedEditOriginals(): List<AssetRecord>

    @Upsert
    suspend fun upsertAsset(asset: AssetRecord)

    @Query("UPDATE assets SET localPath = :path, localSha256 = :sha256, localByteSize = :byteSize, lastAccess = :now WHERE id = :id")
    suspend fun markDownloaded(
        id: String,
        path: String,
        sha256: String,
        byteSize: Long,
        now: Long,
    )

    @Query("UPDATE assets SET lastAccess = :now WHERE id = :id")
    suspend fun markAccessed(id: String, now: Long)

    @Query("UPDATE assets SET pinned = :pinned WHERE id = :id")
    suspend fun setPinned(id: String, pinned: Boolean)

    @Query("UPDATE assets SET localPath = NULL, localSha256 = NULL, localByteSize = NULL WHERE id = :id")
    suspend fun clearLocalPath(id: String)

    @Query("DELETE FROM assets WHERE entityType = :type AND entitySyncId = :syncId")
    suspend fun deleteAssetsFor(type: String, syncId: String)

    @Query("DELETE FROM assets")
    suspend fun clearAssets()

    @Query("SELECT COALESCE(SUM(localByteSize), 0) FROM assets WHERE localPath IS NOT NULL AND assetType = 'edit_original' AND pinned = 0")
    suspend fun cachedViewBytes(): Long

    @Query("SELECT COALESCE(SUM(localByteSize), 0) FROM assets WHERE localPath IS NOT NULL AND assetType = 'edit_original' AND pinned = 1")
    suspend fun cachedPinnedBytes(): Long

    @Query("SELECT * FROM meta WHERE `key` = :key LIMIT 1")
    suspend fun meta(key: String): MetaRecord?

    @Upsert
    suspend fun putMeta(meta: MetaRecord)

    @Query("DELETE FROM meta")
    suspend fun clearMeta()
}
