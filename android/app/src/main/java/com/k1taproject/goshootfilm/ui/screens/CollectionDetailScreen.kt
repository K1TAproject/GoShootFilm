package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import android.widget.Toast
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.data.EditCacheStatus
import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.bool
import com.k1taproject.goshootfilm.data.int
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.optionalText
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfBorder
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfDanger
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.EmptyState
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.InfoLine
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.components.ThumbnailFor
import kotlinx.coroutines.launch
import java.io.File

@Composable
fun CollectionDetailScreen(
    vm: GsfViewModel,
    kind: String,
    syncId: String,
    onBack: () -> Unit,
) {
    var parent by remember { mutableStateOf<EntityRecord?>(null) }
    var viewerFile by remember { mutableStateOf<File?>(null) }
    var downloadingPhotoId by remember { mutableStateOf<String?>(null) }
    var downloadedBytes by remember { mutableStateOf(0L) }
    var totalBytes by remember { mutableStateOf(0L) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val settings by vm.settings.collectAsStateWithLifecycle()
    val photoType = if (kind == "roll") "film_photo" else "digital_photo"
    val allPhotos by remember(photoType) { vm.entitiesFlow(photoType) }
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val photos = remember(allPhotos, kind, syncId) {
        if (kind == "roll") {
            allPhotos
                .filter { it.obj().text("rollSyncId") == syncId }
                .sortedBy { it.obj().int("frameNumber") ?: 0 }
        } else {
            allPhotos
                .filter { it.obj().text("albumSyncId") == syncId }
                .sortedBy { it.obj().text("pairingKey") }
        }
    }

    LaunchedEffect(kind, syncId) {
        parent = vm.entity(kind, syncId)
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp, vertical = 14.dp),
    ) {
        TextButton(onClick = onBack) { Text("返回", color = GsfMuted) }
        val data = parent?.obj()
        ScreenHeader(
            title = when (kind) {
                "roll" -> data?.text("filmInfo").orEmpty().ifBlank { "Roll" }
                else -> data?.text("title").orEmpty().ifBlank { "Album" }
            },
            subtitle = when (kind) {
                "roll" -> listOf(data?.text("cameraInfo"), data?.text("shotMonth"), data?.text("city"))
                    .filterNotNull().filter { it.isNotBlank() }.joinToString(" · ")
                else -> listOf(
                    listOf(data?.text("cameraBrand"), data?.text("cameraModel"))
                        .filterNotNull().filter { it.isNotBlank() }.joinToString(" "),
                    data?.text("city"),
                ).filterNotNull().filter { it.isNotBlank() }.joinToString(" · ")
            },
        )
        Spacer(Modifier.height(10.dp))
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                InfoLine(
                    "照片",
                    "${data?.int("photoCount") ?: photos.size} 张",
                )
                if (kind == "roll") {
                    InfoLine("胶卷类型", data?.text("filmType").orEmpty().ifBlank { "未记录" })
                } else {
                    InfoLine("调色图", "${data?.int("editCount") ?: 0} 张")
                }
                data?.optionalText("note")?.let { InfoLine("备注", it) }
            }
        }
        Spacer(Modifier.height(12.dp))
        Text(
            text = "点击照片查看正式调色原图",
            color = GsfMuted,
            style = MaterialTheme.typography.labelSmall,
        )
        Spacer(Modifier.height(6.dp))
        PhotoCacheLegend()
        Spacer(Modifier.height(8.dp))
        if (downloadingPhotoId != null) {
            Text("正在下载调色图…", color = GsfCyan, style = MaterialTheme.typography.labelSmall)
            if (totalBytes > 0) {
                LinearProgressIndicator(
                    progress = {
                        (downloadedBytes.toFloat() / totalBytes).coerceIn(0f, 1f)
                    },
                    modifier = Modifier.fillMaxWidth(),
                )
            } else {
                LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            }
            Spacer(Modifier.height(8.dp))
        }
        if (photos.isEmpty()) {
            EmptyState("这个集合还没有照片")
        } else {
            LazyVerticalGrid(
                columns = GridCells.Fixed(3),
                modifier = Modifier.weight(1f),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                items(photos, key = { it.syncId }) { photo ->
                    PhotoGridCard(
                        vm = vm,
                        photo = photo,
                        enabled = downloadingPhotoId == null,
                        onClick = {
                            scope.launch {
                                val asset = vm.editOriginal(photo.entityType, photo.syncId)
                                if (asset == null || !asset.available) {
                                    Toast.makeText(
                                        context,
                                        "该照片暂无调色图",
                                        Toast.LENGTH_SHORT,
                                    ).show()
                                    return@launch
                                }
                                val local = asset?.let { vm.localEditFile(it.id) }
                                if (local != null) {
                                    viewerFile = local
                                } else if (settings.serverUrl.isBlank()) {
                                    Toast.makeText(
                                        context,
                                        "调色图尚未下载到本机，连接电脑后可下载",
                                        Toast.LENGTH_SHORT,
                                    ).show()
                                } else {
                                    downloadingPhotoId = photo.syncId
                                    downloadedBytes = 0
                                    totalBytes = asset.byteSize ?: 0
                                    try {
                                        viewerFile = vm.downloadEditOriginal(asset.id) { done, total ->
                                            downloadedBytes = done
                                            totalBytes = total
                                        }
                                    } catch (error: Exception) {
                                        Toast.makeText(
                                            context,
                                            error.message ?: "调色图下载失败",
                                            Toast.LENGTH_LONG,
                                        ).show()
                                    } finally {
                                        downloadingPhotoId = null
                                    }
                                }
                            }
                        },
                    )
                }
            }
        }
    }

    viewerFile?.let { file ->
        PhotoViewerDialog(
            file = file,
            onDismiss = { viewerFile = null },
        )
    }
}

@Composable
private fun PhotoCacheLegend() {
    Row(
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CacheLegendItem("已缓存调色图", GsfCyan)
        CacheLegendItem("调色图未缓存", GsfDanger)
        CacheLegendItem("无调色图", GsfPurple)
    }
}

@Composable
private fun CacheLegendItem(label: String, color: Color) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .height(8.dp)
                .aspectRatio(1f)
                .clip(RoundedCornerShape(2.dp))
                .background(color),
        )
        Text(
            text = label,
            color = GsfMuted,
            style = MaterialTheme.typography.labelSmall,
        )
    }
}

@Composable
private fun PhotoGridCard(
    vm: GsfViewModel,
    photo: EntityRecord,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val data = photo.obj()
    val hasEdit = data.bool("editAvailable")
    val status by remember(photo.entityType, photo.syncId, hasEdit) {
        vm.editCacheStatus(photo.entityType, photo.syncId, hasEdit)
    }.collectAsStateWithLifecycle(initialValue = EditCacheStatus.Checking)
    val label = if (photo.entityType == "film_photo") {
        data.text("displayName").ifBlank { "Frame" }
    } else {
        data.text("pairingKey").ifBlank { "Photo" }
    }
    val borderColor = when (status) {
        EditCacheStatus.Cached -> GsfCyan
        EditCacheStatus.NotCached -> GsfDanger
        EditCacheStatus.NoEdit -> GsfPurple
        EditCacheStatus.Checking -> GsfBorder
    }
    val highlightColor = when (status) {
        EditCacheStatus.Cached -> GsfCyan.copy(alpha = 0.06f)
        EditCacheStatus.NotCached -> GsfDanger.copy(alpha = 0.06f)
        else -> Color.Transparent
    }
    val statusDescription = when (status) {
        EditCacheStatus.Cached -> "已缓存调色图"
        EditCacheStatus.NotCached -> "调色图未缓存"
        EditCacheStatus.NoEdit -> "无调色图"
        EditCacheStatus.Checking -> "正在确认调色图缓存状态"
    }
    val shape = RoundedCornerShape(8.dp)

    Column(
        modifier = Modifier
            .clip(shape)
            .background(highlightColor)
            .border(1.dp, borderColor, shape)
            .semantics(mergeDescendants = true) {
                contentDescription = "$label，$statusDescription"
            }
            .clickable(enabled = enabled, onClick = onClick),
    ) {
        ThumbnailFor(
            vm = vm,
            type = photo.entityType,
            syncId = photo.syncId,
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(1f),
        )
        Text(
            text = label,
            color = GsfMuted,
            style = MaterialTheme.typography.labelSmall,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 3.dp),
        )
    }
}
