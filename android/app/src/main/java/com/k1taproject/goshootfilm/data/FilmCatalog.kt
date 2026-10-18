package com.k1taproject.goshootfilm.data

import android.content.Context
import java.io.BufferedReader
import java.io.InputStreamReader

data class FilmEntry(
    val brand: String,
    val name: String,
    val iso: Int,
    val type: String,
    val image: String,
) {
    val displayName: String
        get() = if (name.startsWith(brand, ignoreCase = true)) name else "$brand $name"

    val thumbAsset: String
        get() = "film-thumbs/${image.substringBeforeLast('.').ifBlank { "placeholder" }}.webp"

    val fullAsset: String
        get() = "film-stocks/${image.substringBeforeLast('.').ifBlank { "placeholder" }}.webp"

    fun matches(query: String): Boolean {
        val value = query.trim().lowercase()
        if (value.isEmpty()) return true
        return brand.lowercase().contains(value) ||
            name.lowercase().contains(value) ||
            type.lowercase().contains(value) ||
            filmTypeLabel(type).lowercase().contains(value)
    }
}

fun filmTypeLabel(type: String): String = when (type) {
    "B&W" -> "黑白"
    "Color Negative" -> "彩色负片"
    "Color Reversal", "Slide" -> "反转片"
    "C-41" -> "C-41"
    else -> type
}

class FilmCatalog(context: Context) {
    private val context = context.applicationContext
    val entries: List<FilmEntry> by lazy { load() }

    private fun load(): List<FilmEntry> {
        val result = mutableListOf<FilmEntry>()
        runCatching {
            context.assets.open("film-catalog.csv").use { stream ->
                BufferedReader(InputStreamReader(stream, Charsets.UTF_8)).useLines { lines ->
                    lines.drop(2).forEach { line ->
                        if (line.isBlank()) return@forEach
                        val columns = line.split(';')
                        if (columns.size < 5) return@forEach
                        val brand = columns[0].trim()
                        val name = columns[1].trim()
                        val iso = columns[2].trim().toIntOrNull() ?: 0
                        val type = columns[3].trim()
                        val image = columns[4].trim()
                        if (brand.isBlank() || name.isBlank() || image.isBlank()) return@forEach
                        result += FilmEntry(brand, name, iso, type, image)
                    }
                }
            }
        }
        return result
    }
}
