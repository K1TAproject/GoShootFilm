package com.k1taproject.goshootfilm

import com.k1taproject.goshootfilm.data.SyncException
import com.k1taproject.goshootfilm.data.parsePairingPayload
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class PairingPayloadTest {
    @Test
    fun parsesDesktopQrPayload() {
        val payload = parsePairingPayload(
            """{"v":1,"url":"http://10.0.0.2:47821/","token":"abc","fingerprint":"fp","expiresAt":1999999999,"serverName":"GoShootFilm"}""",
        )
        assertEquals(1, payload.protocolVersion)
        assertEquals("http://10.0.0.2:47821", payload.url)
        assertEquals("abc", payload.token)
        assertEquals("fp", payload.fingerprint)
        assertEquals(1999999999L, payload.expiresAt)
    }

    @Test
    fun rejectsMalformedOrIncompletePayloads() {
        assertThrows(SyncException::class.java) { parsePairingPayload("not-json") }
        assertThrows(SyncException::class.java) {
            parsePairingPayload("""{"v":1,"url":"http://10.0.0.2:47821"}""")
        }
    }
}
