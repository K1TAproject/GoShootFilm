package com.k1taproject.goshootfilm.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

val GsfBg = Color(0xFF080B10)
val GsfSurface = Color(0xFF10151D)
val GsfSurfaceRaised = Color(0xFF1A2130)
val GsfBorder = Color(0xFF262E3A)
val GsfText = Color(0xFFF3F6FB)
val GsfMuted = Color(0xFF8D97AA)
val GsfPurple = Color(0xFF9B87F5)
val GsfCyan = Color(0xFF55D6BE)
val GsfDanger = Color(0xFFF0A7AF)

private val scheme = darkColorScheme(
    primary = GsfPurple,
    onPrimary = Color(0xFF0C0E15),
    secondary = GsfCyan,
    onSecondary = Color(0xFF06201B),
    background = GsfBg,
    onBackground = GsfText,
    surface = GsfSurface,
    onSurface = GsfText,
    surfaceVariant = GsfSurfaceRaised,
    onSurfaceVariant = GsfMuted,
    outline = GsfBorder,
    error = GsfDanger,
)

@Composable
fun GsfTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = scheme,
        typography = MaterialTheme.typography,
        content = content,
    )
}
