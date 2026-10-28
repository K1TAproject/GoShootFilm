package com.k1taproject.goshootfilm.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import coil.compose.AsyncImage
import kotlin.math.roundToInt
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.data.AssetRecord
import com.k1taproject.goshootfilm.data.SyncProgress
import com.k1taproject.goshootfilm.ui.GsfBorder
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfSurface
import com.k1taproject.goshootfilm.ui.GsfText
import java.io.File

@Composable
fun ScreenHeader(
    title: String,
    subtitle: String? = null,
    action: (@Composable () -> Unit)? = null,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                color = GsfText,
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            if (!subtitle.isNullOrBlank()) {
                Spacer(Modifier.height(4.dp))
                Text(
                    text = subtitle,
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
        action?.invoke()
    }
}

@Composable
fun GsfCard(
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    val shape = RoundedCornerShape(14.dp)
    val base = modifier
        .clip(shape)
        .background(GsfSurface)
        .border(1.dp, GsfBorder, shape)
    if (onClick != null) {
        Surface(onClick = onClick, modifier = base, color = Color.Transparent) {
            Box(Modifier.padding(14.dp)) { content() }
        }
    } else {
        Box(base.padding(14.dp)) { content() }
    }
}

@Composable
fun StatusChip(text: String, accent: Color = GsfPurple) {
    Surface(
        color = accent.copy(alpha = 0.12f),
        contentColor = accent,
        shape = RoundedCornerShape(999.dp),
        modifier = Modifier.border(1.dp, accent.copy(alpha = 0.35f), RoundedCornerShape(999.dp)),
    ) {
        Text(
            text = text,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
            style = MaterialTheme.typography.labelSmall,
        )
    }
}

@Composable
fun EmptyState(text: String) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(160.dp)
            .clip(RoundedCornerShape(12.dp))
            .border(1.dp, GsfBorder, RoundedCornerShape(12.dp)),
        contentAlignment = Alignment.Center,
    ) {
        Text(text, color = GsfMuted)
    }
}

@Composable
fun LocalThumbnail(
    path: String?,
    modifier: Modifier = Modifier,
) {
    var decodeFailed by remember(path) { mutableStateOf(false) }
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(10.dp))
            .background(GsfBorder.copy(alpha = 0.35f)),
        contentAlignment = Alignment.Center,
    ) {
        if (path != null && File(path).isFile && !decodeFailed) {
            AsyncImage(
                model = File(path),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
                onError = { decodeFailed = true },
            )
        } else {
            Text(
                if (decodeFailed) "图片无法读取" else "GSF",
                color = GsfMuted,
                style = MaterialTheme.typography.labelSmall,
            )
        }
    }
}

@Composable
fun rememberThumbnail(vm: GsfViewModel, type: String, syncId: String): AssetRecord? {
    val flow = remember(type, syncId) { vm.thumbnail(type, syncId) }
    val asset by flow.collectAsStateWithLifecycle(initialValue = null)
    return asset
}

@Composable
fun ThumbnailFor(
    vm: GsfViewModel,
    type: String,
    syncId: String,
    modifier: Modifier = Modifier,
) {
    val asset = rememberThumbnail(vm, type, syncId)
    LocalThumbnail(asset?.localPath, modifier)
}

@Composable
fun CoverBadge(text: String, accent: Color = GsfCyan) {
    Surface(
        color = accent,
        contentColor = Color(0xFF06201B),
        shape = RoundedCornerShape(6.dp),
    ) {
        Text(
            text = text,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            style = MaterialTheme.typography.labelSmall,
            fontWeight = FontWeight.Bold,
        )
    }
}

@Composable
fun InfoLine(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label, color = GsfMuted, style = MaterialTheme.typography.bodySmall)
        Text(
            value,
            color = GsfText,
            style = MaterialTheme.typography.bodySmall,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
fun SyncProgressBar(progress: Float?, modifier: Modifier = Modifier) {
    Column(modifier = modifier.fillMaxWidth()) {
        if (progress == null) {
            LinearProgressIndicator(
                modifier = Modifier.fillMaxWidth(),
                color = GsfCyan,
                trackColor = GsfBorder.copy(alpha = 0.6f),
            )
        } else {
            val value = progress.coerceIn(0f, 1f)
            LinearProgressIndicator(
                progress = { value },
                modifier = Modifier.fillMaxWidth(),
                color = GsfCyan,
                trackColor = GsfBorder.copy(alpha = 0.6f),
            )
            Spacer(Modifier.height(4.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                Text(
                    text = "${(value * 100).roundToInt()}%",
                    color = GsfCyan,
                    style = MaterialTheme.typography.labelSmall,
                )
            }
        }
    }
}

@Composable
fun SyncProgressSummary(progress: SyncProgress, modifier: Modifier = Modifier) {
    Column(modifier = modifier.fillMaxWidth()) {
        SyncProgressBar(progress.fraction)
        if (progress.totalItems > 0) {
            Text(
                text = "${progress.currentItems} / ${progress.totalItems}" +
                    if (progress.failedItems > 0) " · 未完成 ${progress.failedItems}" else "",
                color = GsfMuted,
                style = MaterialTheme.typography.labelSmall,
            )
        }
        if (progress.downloadedBytes > 0 || progress.totalBytes != null) {
            Text(
                text = if (progress.totalBytes != null) {
                    "已缓存 ${formatBytes(progress.downloadedBytes)} / ${formatBytes(progress.totalBytes)}"
                } else {
                    "已缓存 ${formatBytes(progress.downloadedBytes)}"
                },
                color = GsfMuted,
                style = MaterialTheme.typography.labelSmall,
            )
        }
    }
}

private fun formatBytes(bytes: Long): String {
    if (bytes < 1024) return "$bytes B"
    val kb = bytes / 1024.0
    if (kb < 1024) return "%.1f KB".format(kb)
    val mb = kb / 1024.0
    return if (mb < 1024) "%.1f MB".format(mb) else "%.2f GB".format(mb / 1024.0)
}

@Composable
fun SectionLabel(text: String) {
    Text(
        text = text,
        color = GsfMuted,
        style = MaterialTheme.typography.labelMedium,
        modifier = Modifier.padding(vertical = 4.dp),
    )
}
