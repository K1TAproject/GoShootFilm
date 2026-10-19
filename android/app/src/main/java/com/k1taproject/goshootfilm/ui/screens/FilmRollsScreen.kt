package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
import com.k1taproject.goshootfilm.data.filmTypeLabel
import com.k1taproject.goshootfilm.data.int
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.optionalText
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.components.EmptyState
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.InfoLine
import com.k1taproject.goshootfilm.ui.components.ScreenHeader

@Composable
fun FilmRollsScreen(
    vm: GsfViewModel,
    filmSyncId: String,
    onBack: () -> Unit,
    onRoll: (String) -> Unit,
) {
    var film by remember { mutableStateOf<EntityRecord?>(null) }
    var rolls by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }

    LaunchedEffect(filmSyncId) {
        film = vm.entity("film", filmSyncId)
        rolls = vm.entities("roll")
            .filter { it.obj().text("filmSyncId") == filmSyncId }
            .sortedWith(
                compareByDescending<EntityRecord> { it.obj().text("shotMonth") }
                    .thenByDescending { it.obj().int("index") ?: 0 },
            )
    }

    val data = film?.obj()
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 18.dp, vertical = 14.dp),
    ) {
        TextButton(onClick = onBack) { Text("返回", color = GsfMuted) }
        ScreenHeader(
            title = data?.let { listOf(it.text("brand"), it.text("name")).filter { v -> v.isNotBlank() }.joinToString(" ") }
                .orEmpty()
                .ifBlank { "Film" },
            subtitle = "已拍摄胶卷 · 关联 Rolls",
        )
        Spacer(Modifier.height(12.dp))
        GsfCard(modifier = Modifier.fillMaxWidth()) {
            Column {
                InfoLine("ISO", data?.int("iso")?.toString() ?: "未记录")
                InfoLine("类型", data?.text("type")?.let(::filmTypeLabel).orEmpty().ifBlank { "未记录" })
                InfoLine("拍摄卷", "${rolls.size} 卷")
                data?.optionalText("note")?.let { InfoLine("备注", it) }
            }
        }
        Spacer(Modifier.height(14.dp))
        Text(
            text = if (rolls.isEmpty()) "暂无关联拍摄卷" else "点击拍摄卷查看照片",
            color = GsfMuted,
            style = MaterialTheme.typography.labelSmall,
        )
        Spacer(Modifier.height(8.dp))
        if (rolls.isEmpty()) {
            EmptyState("暂无关联拍摄卷")
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(rolls, key = { it.syncId }) { roll ->
                    RollCard(roll, vm) { onRoll(roll.syncId) }
                }
            }
        }
        Spacer(Modifier.height(8.dp))
        Text(
            "该列表来自桌面端 Rolls 的 filmSyncId 关联。",
            color = GsfCyan,
            style = MaterialTheme.typography.labelSmall,
            fontWeight = FontWeight.Normal,
        )
    }
}
