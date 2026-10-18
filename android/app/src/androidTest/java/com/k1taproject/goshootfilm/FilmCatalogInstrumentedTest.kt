package com.k1taproject.goshootfilm

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.k1taproject.goshootfilm.data.FilmCatalog
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FilmCatalogInstrumentedTest {
    @Test
    fun catalogAndBundledFilmImagesAreAvailable() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val catalog = FilmCatalog(context)
        assertTrue("film catalog should not be empty", catalog.entries.size > 100)
        val entry = catalog.entries.first()
        context.assets.open(entry.thumbAsset).use { stream ->
            assertTrue(stream.readBytes().isNotEmpty())
        }
        context.assets.open(entry.fullAsset).use { stream ->
            assertTrue(stream.readBytes().isNotEmpty())
        }
    }
}
