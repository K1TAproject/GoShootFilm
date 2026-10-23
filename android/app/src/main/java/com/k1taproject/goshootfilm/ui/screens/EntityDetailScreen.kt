package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.int
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.optionalText
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.InfoLine
import com.k1taproject.goshootfilm.ui.components.ScreenHeader

@Composable
fun EntityDetailScreen(
    vm: GsfViewModel,
    type: String,
    syncId: String,
    onBack: () -> Unit,
    onRoll: (String) -> Unit,
    onAlbum: (String) -> Unit,
) {
    var entity by remember { mutableStateOf<EntityRecord?>(null) }
    var rolls by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var albums by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    LaunchedEffect(type, syncId) {
        rolls = emptyList()
        albums = emptyList()
        val loaded = vm.entity(type, syncId)
        entity = loaded
        if (type == "camera" && loaded?.obj()?.text("cameraType") == "digital") {
            albums = vm.entities("digital_album")
                .filter { it.obj().text("cameraSyncId") == syncId }
                .sortedByDescending { it.obj().text("shotDate") }
        } else if (type == "camera") {
            rolls = vm.entities("roll")
                .filter { it.obj().text("cameraSyncId") == syncId }
                .sortedWith(
                    compareByDescending<EntityRecord> { it.obj().text("shotMonth") }
                        .thenByDescending { it.obj().int("index") ?: 0 },
                )
        }
    }
    val data = entity?.obj()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 18.dp, vertical = 14.dp),
    ) {
        TextButton(onClick = onBack) { Text("返回", color = GsfMuted) }
        val title = if (type == "camera") {
            listOf(data?.text("brand"), data?.text("model")).filterNotNull().filter { it.isNotBlank() }.joinToString(" ")
        } else {
            listOf(data?.text("brand"), data?.text("model")).filterNotNull().filter { it.isNotBlank() }.joinToString(" ")
        }
        ScreenHeader(
            title = title.ifBlank { "Equipment" },
            subtitle = if (type == "camera") "相机机身" else "器材",
        )
        Spacer(Modifier.height(14.dp))
        if (data == null) {
            Text("未找到器材", color = GsfMuted)
            return@Column
        }
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                if (type == "camera") {
                    InfoLine("类型", if (data.text("cameraType") == "digital") "数码机身" else "胶片机身")
                    if (data.text("cameraType") == "digital") {
                        InfoLine("CMOS", data.text("sensorFormat").ifBlank { "未记录" })
                    } else {
                        InfoLine("画幅", data.text("format").ifBlank { "135" })
                    }
                } else {
                    InfoLine("分类", if (data.text("category") == "lens") "镜头" else "其他")
                    if (data.text("category") == "lens") {
                        InfoLine("卡口", data.text("mount").ifBlank { "未记录" })
                    } else {
                        InfoLine("类型", data.text("subtype").ifBlank { "未记录" })
                    }
                }
                InfoLine("状态", if (data.text("status") == "active") "在用" else "闲置")
                if (type == "camera") {
                    if (data.text("cameraType") == "digital") {
                        InfoLine("相册", "${albums.size} 个")
                    } else {
                        InfoLine("拍摄卷", "${rolls.size} 卷")
                    }
                }
                data.optionalText("purchaseDate")?.let { InfoLine("购入日期", it) }
                data.optionalText("note")?.let { InfoLine("备注", it) }
            }
        }
        if (type == "camera" && data.text("cameraType") == "digital") {
            Spacer(Modifier.height(14.dp))
            Text(
                text = "关联 Albums",
                color = GsfMuted,
                style = MaterialTheme.typography.labelMedium,
            )
            Spacer(Modifier.height(8.dp))
            if (albums.isEmpty()) {
                Text(
                    "这台数码相机还没有相册",
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                )
            } else {
                albums.forEach { album ->
                    AlbumCard(album, vm) { onAlbum(album.syncId) }
                    Spacer(Modifier.height(8.dp))
                }
            }
        } else if (type == "camera") {
            Spacer(Modifier.height(14.dp))
            Text(
                text = "关联 Rolls",
                color = GsfMuted,
                style = MaterialTheme.typography.labelMedium,
            )
            Spacer(Modifier.height(8.dp))
            if (rolls.isEmpty()) {
                Text(
                    "这台相机还没有拍摄卷",
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                )
            } else {
                rolls.forEach { roll ->
                    RollCard(roll, vm) { onRoll(roll.syncId) }
                    Spacer(Modifier.height(8.dp))
                }
            }
        }
        Spacer(Modifier.height(16.dp))
        Text(
            "手机端只读，器材信息请在桌面端修改。",
            color = GsfMuted,
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Normal,
        )
    }
}
