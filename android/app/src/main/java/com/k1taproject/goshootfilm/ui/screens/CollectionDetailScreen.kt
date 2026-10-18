package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.int
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.optionalText
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.EmptyState
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.InfoLine
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.components.ThumbnailFor
import kotlinx.coroutines.launch

@Composable
fun CollectionDetailScreen(
    vm: GsfViewModel,
    kind: String,
    syncId: String,
    onBack: () -> Unit,
) {
    var parent by remember { mutableStateOf<EntityRecord?>(null) }
    var photos by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var viewerAssetId by remember { mutableStateOf<String?>(null) }
    var message by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(kind, syncId) {
        parent = vm.entity(kind, syncId)
        photos = if (kind == "roll") {
            vm.entities("film_photo")
                .filter { it.obj().text("rollSyncId") == syncId }
                .sortedBy { it.obj().int("frameNumber") ?: 0 }
        } else {
            vm.entities("digital_photo")
                .filter { it.obj().text("albumSyncId") == syncId }
                .sortedBy { it.obj().text("pairingKey") }
        }
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
        Spacer(Modifier.height(8.dp))
        message?.let {
            Text(it, color = GsfCyan, style = MaterialTheme.typography.bodySmall)
            Spacer(Modifier.height(6.dp))
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
                    val photoType = photo.entityType
                    Column(
                        modifier = Modifier.clickable {
                            scope.launch {
                                val asset = vm.editOriginal(photoType, photo.syncId)
                                if (asset == null) {
                                    message = "这张照片没有可用的正式调色原图"
                                } else {
                                    message = null
                                    viewerAssetId = asset.id
                                }
                            }
                        },
                    ) {
                        ThumbnailFor(
                            vm = vm,
                            type = photoType,
                            syncId = photo.syncId,
                            modifier = Modifier
                                .fillMaxWidth()
                                .aspectRatio(1f),
                        )
                        Text(
                            text = if (photoType == "film_photo") {
                                photo.obj().text("displayName").ifBlank { "Frame" }
                            } else {
                                photo.obj().text("pairingKey").ifBlank { "Photo" }
                            },
                            color = GsfMuted,
                            style = MaterialTheme.typography.labelSmall,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.padding(top = 3.dp),
                        )
                    }
                }
            }
        }
    }

    viewerAssetId?.let { assetId ->
        PhotoViewerDialog(
            vm = vm,
            assetId = assetId,
            onDismiss = { viewerAssetId = null },
        )
    }
}
