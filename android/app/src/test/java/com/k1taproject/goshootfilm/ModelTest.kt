package com.k1taproject.goshootfilm

import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.FilmEntry
import com.k1taproject.goshootfilm.data.displayTitle
import com.k1taproject.goshootfilm.data.filmTypeLabel
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.text
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ModelTest {
    @Test
    fun filmSearchMatchesBrandModelAndType() {
        val film = FilmEntry("Kodak", "Portra 400", 400, "Color Negative", "kodak-portra-400.png")
        assertTrue(film.matches("kodak"))
        assertTrue(film.matches("portra"))
        assertTrue(film.matches("彩色负片"))
        assertFalse(film.matches("黑白"))
    }

    @Test
    fun filmTypeLabelsMatchDesktopVocabulary() {
        assertEquals("黑白", filmTypeLabel("B&W"))
        assertEquals("彩色负片", filmTypeLabel("Color Negative"))
        assertEquals("反转片", filmTypeLabel("Color Reversal"))
    }

    @Test
    fun entityTitlePrefersBrandAndModel() {
        val camera = EntityRecord(
            entityType = "camera",
            syncId = "camera-1",
            revision = 1,
            deleted = false,
            dataJson = """{"brand":"Nikon","model":"F2"}""",
            updatedAt = 0,
        )
        assertEquals("Nikon F2", camera.displayTitle())
        assertEquals("Nikon", camera.obj().text("brand"))
    }
}
