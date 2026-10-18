package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.components.ThumbnailFor

@Composable
fun RollsScreen(vm: GsfViewModel, onRoll: (String) -> Unit) {
    var rolls by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    LaunchedEffect(Unit) { rolls = vm.entities("roll") }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 18.dp, vertical = 18.dp),
    ) {
        ScreenHeader("Rolls", "胶片拍摄卷")
        Spacer(Modifier.height(14.dp))
        if (rolls.isEmpty()) {
            EmptyState("还没有同步任何拍摄卷")
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                items(
                    rolls.sortedWith(
                        compareByDescending<EntityRecord> { it.obj().text("shotMonth") }
                            .thenByDescending { it.obj().int("index") ?: 0 },
                    ),
                    key = { it.syncId },
                ) { roll ->
                    RollCard(roll, vm) { onRoll(roll.syncId) }
                }
            }
        }
    }
}

@Composable
fun AlbumsScreen(vm: GsfViewModel, onAlbum: (String) -> Unit) {
    var albums by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    LaunchedEffect(Unit) { albums = vm.entities("digital_album") }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 18.dp, vertical = 18.dp),
    ) {
        ScreenHeader("Albums", "数码相册")
        Spacer(Modifier.height(14.dp))
        if (albums.isEmpty()) {
            EmptyState("还没有同步任何相册")
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                items(
                    albums.sortedByDescending { it.obj().text("shotDate") },
                    key = { it.syncId },
                ) { album ->
                    AlbumCard(album, vm) { onAlbum(album.syncId) }
                }
            }
        }
    }
}

@Composable
private fun RollCard(roll: EntityRecord, vm: GsfViewModel, onClick: () -> Unit) {
    val data = roll.obj()
    GsfCard(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            ThumbnailFor(
                vm = vm,
                type = "film_photo",
                syncId = data.text("coverPhotoSyncId"),
                modifier = Modifier.size(66.dp),
            )
            Spacer(Modifier.size(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = data.text("filmInfo").ifBlank { "Roll" },
                    color = GsfText,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(3.dp))
                Text(
                    text = listOf(data.text("cameraInfo"), data.text("shotMonth"), data.text("city"))
                        .filter { it.isNotBlank() }
                        .joinToString(" · "),
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                data.optionalText("note")?.let {
                    Spacer(Modifier.height(3.dp))
                    Text(
                        it,
                        color = GsfMuted,
                        style = MaterialTheme.typography.labelSmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    "${data.int("photoCount") ?: 0}",
                    color = GsfCyan,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                Text("张", color = GsfMuted, style = MaterialTheme.typography.labelSmall)
            }
        }
    }
}

@Composable
private fun AlbumCard(album: EntityRecord, vm: GsfViewModel, onClick: () -> Unit) {
    val data = album.obj()
    GsfCard(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            ThumbnailFor(
                vm = vm,
                type = "digital_photo",
                syncId = data.text("coverPhotoSyncId"),
                modifier = Modifier.size(66.dp),
            )
            Spacer(Modifier.size(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = data.text("title").ifBlank { "Album" },
                    color = GsfText,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(3.dp))
                Text(
                    text = listOf(
                        listOf(data.text("cameraBrand"), data.text("cameraModel"))
                            .filter { it.isNotBlank() }
                            .joinToString(" "),
                        data.text("city"),
                    ).filter { it.isNotBlank() }.joinToString(" · "),
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    "${data.int("photoCount") ?: 0}",
                    color = GsfCyan,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                Text("张", color = GsfMuted, style = MaterialTheme.typography.labelSmall)
            }
        }
    }
}
