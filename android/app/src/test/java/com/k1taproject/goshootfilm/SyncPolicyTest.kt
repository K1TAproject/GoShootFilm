package com.k1taproject.goshootfilm

import com.k1taproject.goshootfilm.data.AssetRecord
import com.k1taproject.goshootfilm.data.inspectCachedAsset
import com.k1taproject.goshootfilm.data.syncFraction
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.security.MessageDigest

class SyncPolicyTest {
    @Test
    fun cachedAssetIsReusedOnlyWhenFileHashAndSizeMatch() {
        val file = File.createTempFile("gsfmobile-cache", ".jpg")
        try {
            file.writeText("stable cached image")
            val hash = MessageDigest.getInstance("SHA-256")
                .digest(file.readBytes())
                .joinToString("") { "%02x".format(it) }
            val asset = asset(file, hash, file.length())

            val inspected = inspectCachedAsset(asset)
            assertEquals(hash, inspected?.first)
            assertEquals(file.length(), inspected?.second)

            file.writeText("damaged")
            assertNull(inspectCachedAsset(asset))
            file.delete()
            assertNull(inspectCachedAsset(asset))
        } finally {
            file.delete()
        }
    }

    @Test
    fun progressUsesItemsWhenTotalBytesAreUnknown() {
        assertEquals(0.5f, syncFraction(5, 10, 500, null) ?: -1f, 0.0001f)
        assertEquals(0.25f, syncFraction(1, 10, 250, 1000) ?: -1f, 0.0001f)
        assertNull(syncFraction(0, 0, 0, null))
        assertTrue((syncFraction(5, 1, 2000, 1000) ?: 0f) <= 1f)
    }

    private fun asset(file: File, hash: String, size: Long) = AssetRecord(
        id = "asset-1",
        entityType = "film_photo",
        entitySyncId = "photo-1",
        assetType = "thumbnail",
        version = "edit",
        mimeType = "image/jpeg",
        byteSize = size,
        sha256 = hash,
        available = true,
        localPath = file.absolutePath,
        localSha256 = hash,
        localByteSize = size,
        lastAccess = 0,
    )
}
