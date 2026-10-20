package com.k1taproject.goshootfilm

import android.content.Context
import android.graphics.Bitmap
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.k1taproject.goshootfilm.data.GallerySaver
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FileOutputStream

@RunWith(AndroidJUnit4::class)
class GallerySaverInstrumentedTest {
    @Test
    fun savesOriginalBytesIntoMediaStoreAndCleansUp() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val source = File(context.cacheDir, "gsfmobile-save-test.png")
        val bitmap = Bitmap.createBitmap(2, 2, Bitmap.Config.ARGB_8888)
        try {
            FileOutputStream(source).use { output ->
                bitmap.compress(Bitmap.CompressFormat.PNG, 100, output)
            }
        } finally {
            bitmap.recycle()
        }
        val original = source.readBytes()
        val uri = GallerySaver.saveToGallery(context, source)
        try {
            assertNotNull(uri)
            val saved = context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
            assertEquals(original.size, saved?.size)
            assertEquals(original.toList(), saved?.toList())
        } finally {
            context.contentResolver.delete(uri, null, null)
            source.delete()
        }
    }
}
