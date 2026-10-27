package com.k1taproject.goshootfilm

import com.k1taproject.goshootfilm.data.ApiClient
import com.k1taproject.goshootfilm.data.bool
import com.k1taproject.goshootfilm.data.int
import com.k1taproject.goshootfilm.data.text
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.security.MessageDigest

class ApiClientTest {
    @Test
    fun healthAndPairingUseTheDesktopApiShape() {
        val server = MockWebServer()
        server.enqueue(
            MockResponse().setBody(
                """{"appVersion":"0.3.0","protocolVersion":1,"serverFingerprint":"abc"}""",
            ),
        )
        server.enqueue(
            MockResponse().setBody("""{"requestId":"req-1","expiresAt":123}"""),
        )
        server.start()
        try {
            val base = "http://127.0.0.1:${server.port}"
            val client = ApiClient()
            val health = client.health(base)
            assertEquals("0.3.0", health.appVersion)
            assertEquals(1, health.protocolVersion)
            assertEquals("abc", health.fingerprint)

            val request = client.pairRequest(base, "token-1", "Pixel")
            assertEquals("req-1", request.requestId)
            server.takeRequest()
            val recorded = server.takeRequest()
            assertEquals("/api/v1/pair/request", recorded.path)
            assertTrue(recorded.body.readUtf8().contains("\"platform\":\"android\""))
        } finally {
            server.shutdown()
        }
    }

    @Test
    fun changesAndThumbnailDownloadCarryAuthAndVerifyHash() {
        val bytes = "jpeg-placeholder".toByteArray()
        val hash = MessageDigest.getInstance("SHA-256")
            .digest(bytes)
            .joinToString("") { "%02x".format(it) }
        val server = MockWebServer()
        server.enqueue(
            MockResponse().setBody(
                """{"protocolVersion":1,"cursor":7,"resetRequired":false,"hasMore":false,"stats":{},"changes":[]}""",
            ),
        )
        server.enqueue(
            MockResponse()
                .setBody(String(bytes))
                .setHeader("Content-Type", "image/jpeg")
                .setHeader("x-goshootfilm-sha256", hash),
        )
        server.start()
        try {
            val base = "http://127.0.0.1:${server.port}"
            val client = ApiClient()
            val changes = client.changes(base, "credential", 0)
            assertEquals(1, changes.int("protocolVersion"))
            assertEquals(7L, changes["cursor"]?.toString()?.toLong())
            assertEquals(false, changes.bool("resetRequired"))
            val changesRequest = server.takeRequest()
            assertEquals("Bearer credential", changesRequest.getHeader("Authorization"))
            assertTrue(changesRequest.path.orEmpty().contains("cursor=0"))

            val target = File.createTempFile("gsfmobile-test", ".jpg")
            try {
                val result = client.downloadThumbnail(
                    base,
                    "credential",
                    "asset-1",
                    target,
                    hash,
                ) { _, _ -> }
                assertEquals(hash, result.sha256)
                assertEquals(hash, target.readBytes().let {
                    MessageDigest.getInstance("SHA-256").digest(it)
                        .joinToString("") { byte -> "%02x".format(byte) }
                })
            } finally {
                target.delete()
            }
            val downloadRequest = server.takeRequest()
            assertEquals("/api/v1/resources/asset-1", downloadRequest.path)
            assertEquals("Bearer credential", downloadRequest.getHeader("Authorization"))
        } finally {
            server.shutdown()
        }
    }

    @Test
    fun failedHashCheckKeepsPreviousCacheAndRemovesPartFile() {
        val server = MockWebServer()
        server.enqueue(MockResponse().setBody("corrupt replacement"))
        server.start()
        val target = File.createTempFile("gsfmobile-atomic", ".jpg")
        try {
            target.writeText("previous valid cache")
            assertThrows(Exception::class.java) {
                ApiClient().downloadThumbnail(
                    "http://127.0.0.1:${server.port}",
                    "credential",
                    "asset-2",
                    target,
                    "not-the-real-hash",
                ) { _, _ -> }
            }
            assertEquals("previous valid cache", target.readText())
            assertFalse(File(target.parentFile, target.name + ".part").exists())
        } finally {
            target.delete()
            server.shutdown()
        }
    }
}
