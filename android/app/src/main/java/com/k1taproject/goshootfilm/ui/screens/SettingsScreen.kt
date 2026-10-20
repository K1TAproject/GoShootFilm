package com.k1taproject.goshootfilm.ui.screens

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.k1taproject.goshootfilm.BuildConfig
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.SyncState
import com.k1taproject.goshootfilm.UpdateState
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfDanger
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.InfoLine
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.components.SectionLabel
import com.k1taproject.goshootfilm.ui.components.StatusChip
import com.k1taproject.goshootfilm.ui.components.SyncProgressBar
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
fun SettingsScreen(vm: GsfViewModel, onBack: () -> Unit) {
    val context = LocalContext.current
    val settings by vm.settings.collectAsStateWithLifecycle()
    val syncState by vm.syncState.collectAsStateWithLifecycle()
    val updateState by vm.updateState.collectAsStateWithLifecycle()
    var confirmUnpair by remember { mutableStateOf(false) }
    var confirmClear by remember { mutableStateOf(false) }
    var confirmPinned by remember { mutableStateOf(false) }
    var cacheVersion by remember { mutableStateOf(0) }
    val cachedBytes by produceState(
        initialValue = 0L,
        cacheVersion,
        syncState,
        settings.cacheLimitMb,
    ) {
        value = vm.cachedBytes()
    }
    val pinnedBytes by produceState(
        initialValue = 0L,
        cacheVersion,
        syncState,
    ) {
        value = vm.cachedPinnedBytes()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 18.dp, vertical = 14.dp),
    ) {
        TextButton(onClick = onBack) { Text("返回", color = GsfMuted) }
        ScreenHeader("Settings", "GSFMobile 设置")
        Spacer(Modifier.height(14.dp))

        SectionLabel("服务器")
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                InfoLine("地址", settings.serverUrl.ifBlank { "未配对" })
                InfoLine("设备 ID", settings.deviceId.ifBlank { "未配对" })
                InfoLine("服务指纹", settings.serverFingerprint.ifBlank { "未记录" })
                InfoLine(
                    "上次同步",
                    if (settings.lastSync > 0) formatTime(settings.lastSync) else "尚未同步",
                )
            }
        }

        Spacer(Modifier.height(16.dp))
        SectionLabel("同步")
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text("仅 Wi-Fi 同步", color = GsfText)
                        Text(
                            "移动网络下不会自动同步",
                            color = GsfMuted,
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                    Switch(
                        checked = settings.wifiOnly,
                        onCheckedChange = { value ->
                            vm.setWifiOnly(value)
                        },
                    )
                }
                Spacer(Modifier.height(10.dp))
                when (val state = syncState) {
                    is SyncState.Running -> {
                        Text(state.message, color = GsfCyan)
                        Spacer(Modifier.height(8.dp))
                        SyncProgressBar(state.progress)
                    }
                    is SyncState.Failed -> Text(state.message, color = GsfDanger)
                    is SyncState.Done -> Text("同步完成：${formatTime(state.at)}", color = GsfMuted)
                    SyncState.Idle -> Text("尚未同步", color = GsfMuted)
                }
                Spacer(Modifier.height(8.dp))
                Button(onClick = { vm.sync() }, enabled = syncState !is SyncState.Running) {
                    Text("立即同步")
                }
            }
        }

        Spacer(Modifier.height(16.dp))
        SectionLabel("缓存")
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                Text(
                    "标星原图：${formatBytes(pinnedBytes)}",
                    color = GsfText,
                )
                Text(
                    "查看图缓存：${formatBytes(cachedBytes)}",
                    color = GsfText,
                )
                Spacer(Modifier.height(8.dp))
                val options = listOf(2048 to "2 GB", 5120 to "5 GB", 10240 to "10 GB", 0 to "不限制")
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    options.chunked(2).forEach { row ->
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            row.forEach { (value, label) ->
                                FilterChip(
                                    selected = settings.cacheLimitMb == value,
                                    onClick = { vm.setCacheLimit(value) },
                                    label = { Text(label) },
                                )
                            }
                        }
                    }
                }
                Spacer(Modifier.height(8.dp))
                OutlinedButton(onClick = {
                    vm.clearViewCache()
                    cacheVersion++
                }) {
                    Text("清理查看图缓存")
                }
                Spacer(Modifier.height(4.dp))
                OutlinedButton(onClick = { confirmPinned = true }) {
                    Text("清理标星原图", color = GsfDanger)
                }
                Spacer(Modifier.height(4.dp))
                Text(
                    "缩略图和元数据会保留；标星原图下次同步会重新下载",
                    color = GsfMuted,
                    style = MaterialTheme.typography.labelSmall,
                )
            }
        }

        Spacer(Modifier.height(16.dp))
        SectionLabel("更新")
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                when (val state = updateState) {
                    UpdateState.Checking -> Text("正在检查 GitHub Release…", color = GsfMuted)
                    is UpdateState.Available -> {
                        StatusChip("有新版本 ${state.info.versionName}", GsfCyan)
                        Spacer(Modifier.height(8.dp))
                        Text(
                            state.info.notes.take(600),
                            color = GsfMuted,
                            style = MaterialTheme.typography.bodySmall,
                        )
                        Spacer(Modifier.height(8.dp))
                        Button(
                            onClick = {
                                runCatching {
                                    context.startActivity(
                                        Intent(Intent.ACTION_VIEW, Uri.parse(state.info.apkUrl)),
                                    )
                                }
                            },
                        ) { Text("前往下载 APK") }
                        Spacer(Modifier.height(4.dp))
                        Text(
                            "SHA-256: ${state.info.sha256}",
                            color = GsfMuted,
                            style = MaterialTheme.typography.labelSmall,
                        )
                    }
                    UpdateState.Latest -> Text("已是最新版本", color = GsfCyan)
                    is UpdateState.Failed -> Text(state.message, color = GsfDanger)
                    UpdateState.Idle -> Text("手动检查 GitHub Release", color = GsfMuted)
                }
                Spacer(Modifier.height(8.dp))
                OutlinedButton(
                    onClick = { vm.checkForUpdate() },
                    enabled = updateState !is UpdateState.Checking,
                ) { Text("检查更新") }
            }
        }

        Spacer(Modifier.height(16.dp))
        SectionLabel("设备")
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                InfoLine("应用版本", "${BuildConfig.VERSION_NAME} (${BuildConfig.VERSION_CODE})")
                InfoLine("协议版本", "v1")
                InfoLine("包名", BuildConfig.APPLICATION_ID)
                Spacer(Modifier.height(10.dp))
                OutlinedButton(onClick = { confirmUnpair = true }) {
                    Text("解除配对", color = GsfDanger)
                }
                Spacer(Modifier.height(4.dp))
                TextButton(onClick = { confirmClear = true }) {
                    Text("清除本地同步数据", color = GsfMuted)
                }
            }
        }
        Spacer(Modifier.height(24.dp))
    }

    if (confirmUnpair) {
        AlertDialog(
            onDismissRequest = { confirmUnpair = false },
            title = { Text("解除配对？") },
            text = { Text("手机将无法继续同步，需要重新扫码配对。") },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmUnpair = false
                        vm.unpair()
                    },
                ) { Text("解除", color = GsfDanger) }
            },
            dismissButton = {
                TextButton(onClick = { confirmUnpair = false }) { Text("取消") }
            },
            containerColor = com.k1taproject.goshootfilm.ui.GsfSurface,
        )
    }
    if (confirmClear) {
        AlertDialog(
            onDismissRequest = { confirmClear = false },
            title = { Text("清除本地同步数据？") },
            text = { Text("将删除本地元数据和图片缓存；电脑端数据不会受影响。") },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmClear = false
                        vm.clearAllData()
                    },
                ) { Text("清除", color = GsfDanger) }
            },
            dismissButton = {
                TextButton(onClick = { confirmClear = false }) { Text("取消") }
            },
            containerColor = com.k1taproject.goshootfilm.ui.GsfSurface,
        )
    }
    if (confirmPinned) {
        AlertDialog(
            onDismissRequest = { confirmPinned = false },
            title = { Text("清理标星原图？") },
            text = { Text("将删除所有标星调色原图；下次同步会重新下载。") },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmPinned = false
                        vm.clearPinnedCache()
                        cacheVersion++
                    },
                ) { Text("清理", color = GsfDanger) }
            },
            dismissButton = {
                TextButton(onClick = { confirmPinned = false }) { Text("取消") }
            },
            containerColor = com.k1taproject.goshootfilm.ui.GsfSurface,
        )
    }
}

private fun formatTime(epochMillis: Long): String =
    SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.getDefault()).format(Date(epochMillis))

private fun formatBytes(bytes: Long): String {
    if (bytes < 1024) return "$bytes B"
    val kb = bytes / 1024.0
    if (kb < 1024) return "%.1f KB".format(kb)
    val mb = kb / 1024.0
    if (mb < 1024) return "%.1f MB".format(mb)
    return "%.2f GB".format(mb / 1024.0)
}
