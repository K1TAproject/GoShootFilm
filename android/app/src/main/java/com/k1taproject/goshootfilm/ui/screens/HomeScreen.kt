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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.ConnectionState
import com.k1taproject.goshootfilm.SyncState
import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.SyncPhase
import com.k1taproject.goshootfilm.data.int
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfDanger
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfSurfaceRaised
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.EmptyState
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.components.SectionLabel
import com.k1taproject.goshootfilm.ui.components.SyncProgressSummary
import com.k1taproject.goshootfilm.ui.components.ThumbnailFor
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
fun HomeScreen(
    vm: GsfViewModel,
    onSettings: () -> Unit,
    onRoll: (String) -> Unit,
    onAlbum: (String) -> Unit,
    onOpenTab: (String) -> Unit,
    onPair: () -> Unit,
) {
    val settings by vm.settings.collectAsStateWithLifecycle()
    val syncState by vm.syncState.collectAsStateWithLifecycle()
    val connectionState by vm.connectionState.collectAsStateWithLifecycle()
    var counts by remember { mutableStateOf<Map<String, Int>>(emptyMap()) }
    var rolls by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var albums by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }

    LaunchedEffect(settings.serverUrl, settings.lastSync) {
        counts = listOf("camera", "equipment", "film", "roll", "film_photo", "digital_album", "digital_photo")
            .associateWith { vm.entities(it).size }
        rolls = vm.entities("roll").sortedByDescending { it.obj().int("index") ?: 0 }
        albums = vm.entities("digital_album")
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 18.dp, vertical = 18.dp),
    ) {
        ScreenHeader(
            title = "Home",
            subtitle = "桌面端的只读镜像",
            action = {
                TextButton(onClick = onSettings) {
                    Text("Settings", color = GsfMuted)
                }
            },
        )
        Spacer(Modifier.height(16.dp))

        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column {
                        Text(
                            text = when (val state = syncState) {
                                is SyncState.Running -> syncPhaseLabel(state.progress.phase)
                                is SyncState.Done -> state.message
                                is SyncState.Failed ->
                                    if (state.message == "同步已取消") "同步已取消" else "同步失败"
                                SyncState.Idle -> connectionLabel(connectionState)
                            },
                            color = GsfText,
                            fontWeight = FontWeight.SemiBold,
                        )
                        Text(
                            text = when (val state = syncState) {
                                is SyncState.Running -> state.progress.message
                                is SyncState.Done -> formatTime(state.at)
                                is SyncState.Failed -> state.message
                                SyncState.Idle -> connectionMessage(connectionState)
                            },
                            color = if (syncState is SyncState.Failed) GsfDanger else GsfMuted,
                            style = MaterialTheme.typography.bodySmall,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    if (syncState is SyncState.Running) {
                        TextButton(onClick = { vm.cancelSync() }) {
                            Text("取消", color = GsfMuted)
                        }
                    } else {
                        Button(onClick = if (settings.serverUrl.isBlank()) onPair else ({ vm.sync() })) {
                            Text(if (settings.serverUrl.isBlank()) "连接电脑" else "同步")
                        }
                    }
                }
                if (syncState is SyncState.Running) {
                    val running = syncState as SyncState.Running
                    Spacer(Modifier.height(10.dp))
                    SyncProgressSummary(running.progress)
                }
                if (settings.lastSync > 0) {
                    Spacer(Modifier.height(10.dp))
                    Text(
                        "上次同步：${formatTime(settings.lastSync)}",
                        color = GsfMuted,
                        style = MaterialTheme.typography.labelSmall,
                    )
                }
            }
        }

        Spacer(Modifier.height(18.dp))
        SectionLabel("概览")
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatBox("相机", counts["camera"] ?: 0, Modifier.weight(1f)) {
                onOpenTab("equipment")
            }
            StatBox("器材", counts["equipment"] ?: 0, Modifier.weight(1f)) {
                onOpenTab("equipment")
            }
        }
        Spacer(Modifier.height(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatBox("Rolls", counts["roll"] ?: 0, Modifier.weight(1f)) {
                onOpenTab("rolls")
            }
            StatBox("Albums", counts["digital_album"] ?: 0, Modifier.weight(1f)) {
                onOpenTab("albums")
            }
        }
        Spacer(Modifier.height(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            StatBox("胶片照片", counts["film_photo"] ?: 0, Modifier.weight(1f)) {
                onOpenTab("rolls")
            }
            StatBox("数码照片", counts["digital_photo"] ?: 0, Modifier.weight(1f)) {
                onOpenTab("albums")
            }
        }

        Spacer(Modifier.height(20.dp))
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SectionLabel("最近的 Rolls")
            TextButton(onClick = { onOpenTab("rolls") }) { Text("全部", color = GsfPurple) }
        }
        if (rolls.isEmpty()) {
            EmptyState("还没有同步任何 Roll")
        } else {
            rolls.take(3).forEach { roll ->
                RollRow(roll, vm) { onRoll(roll.syncId) }
                Spacer(Modifier.height(8.dp))
            }
        }

        Spacer(Modifier.height(14.dp))
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SectionLabel("最近的 Albums")
            TextButton(onClick = { onOpenTab("albums") }) { Text("全部", color = GsfPurple) }
        }
        if (albums.isEmpty()) {
            EmptyState("还没有同步任何 Album")
        } else {
            albums.take(3).forEach { album ->
                AlbumRow(album, vm) { onAlbum(album.syncId) }
                Spacer(Modifier.height(8.dp))
            }
        }
        Spacer(Modifier.height(24.dp))
    }
}

@Composable
private fun StatBox(
    label: String,
    value: Int,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
) {
    GsfCard(modifier = modifier, onClick = onClick) {
        Column {
            Text(value.toString(), color = GsfText, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            Text(label, color = GsfMuted, style = MaterialTheme.typography.labelSmall)
        }
    }
}

@Composable
private fun RollRow(roll: EntityRecord, vm: GsfViewModel, onClick: () -> Unit) {
    val data = roll.obj()
    GsfCard(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            ThumbnailFor(
                vm = vm,
                type = "film_photo",
                syncId = data.text("coverPhotoSyncId"),
                modifier = Modifier.size(58.dp),
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
                Text(
                    text = listOf(data.text("cameraInfo"), data.text("shotMonth"), data.text("city"))
                        .filter { it.isNotBlank() }
                        .joinToString(" · "),
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Text(
                text = "${data.int("photoCount") ?: 0} 张",
                color = GsfCyan,
                style = MaterialTheme.typography.labelMedium,
            )
        }
    }
}

@Composable
private fun AlbumRow(album: EntityRecord, vm: GsfViewModel, onClick: () -> Unit) {
    val data = album.obj()
    GsfCard(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            ThumbnailFor(
                vm = vm,
                type = "digital_photo",
                syncId = data.text("coverPhotoSyncId"),
                modifier = Modifier.size(58.dp),
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
                Text(
                    text = listOf(data.text("cameraBrand") + " " + data.text("cameraModel"), data.text("city"))
                        .filter { it.isNotBlank() }
                        .joinToString(" · "),
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Text(
                text = "${data.int("photoCount") ?: 0} 张",
                color = GsfCyan,
                style = MaterialTheme.typography.labelMedium,
            )
        }
    }
}

private fun formatTime(epochMillis: Long): String =
    SimpleDateFormat("MM-dd HH:mm", Locale.getDefault()).format(Date(epochMillis))

private fun syncPhaseLabel(phase: SyncPhase): String = when (phase) {
    SyncPhase.Connecting -> "正在连接电脑"
    SyncPhase.Checking -> "正在检查缺失信息"
    SyncPhase.Updating -> "正在更新档案"
    SyncPhase.VerifyingCache -> "正在核对本地缓存"
    SyncPhase.DownloadingThumbnails -> "正在下载缩略图"
    SyncPhase.DownloadingEdits -> "正在下载调色图"
    SyncPhase.Cleaning -> "正在确认同步结果"
    SyncPhase.Completed -> "同步完成"
    SyncPhase.Partial -> "同步部分完成"
}

private fun connectionLabel(state: ConnectionState): String = when (state) {
    ConnectionState.Unpaired -> "当前未配对"
    ConnectionState.Connecting -> "正在连接电脑"
    ConnectionState.Online -> "电脑在线"
    is ConnectionState.Offline -> "离线浏览"
    is ConnectionState.CredentialInvalid -> "配对已失效"
    ConnectionState.Repairing -> "正在重新配对"
}

private fun connectionMessage(state: ConnectionState): String = when (state) {
    ConnectionState.Unpaired -> "本地内容仍可浏览；连接电脑后可同步"
    ConnectionState.Connecting -> "正在验证电脑地址与服务指纹"
    ConnectionState.Online -> "已连接，可以开始同步"
    is ConnectionState.Offline -> state.message
    is ConnectionState.CredentialInvalid -> state.message
    ConnectionState.Repairing -> "批准成功前会保留原连接与本地缓存"
}
