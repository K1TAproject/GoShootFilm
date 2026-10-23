package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.optionalText
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfBorder
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfSurfaceRaised
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.EmptyState
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.ScreenHeader
import com.k1taproject.goshootfilm.ui.components.StatusChip

private enum class EquipmentFilter(val label: String) {
    All("全部"),
    Camera("机身"),
    Lens("镜头"),
    Other("其他"),
}

@Composable
fun EquipmentScreen(
    vm: GsfViewModel,
    onDetail: (String, String) -> Unit,
) {
    var cameras by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var equipment by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var rolls by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var albums by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var filter by remember { mutableStateOf(EquipmentFilter.All) }

    LaunchedEffect(Unit) {
        cameras = vm.entities("camera")
        equipment = vm.entities("equipment")
        rolls = vm.entities("roll")
        albums = vm.entities("digital_album")
    }

    val rollCountByCamera = rolls.groupingBy { it.obj().text("cameraSyncId") }.eachCount()
    val albumCountByCamera = albums.groupingBy { it.obj().text("cameraSyncId") }.eachCount()

    val rows = buildList {
        if (filter == EquipmentFilter.All || filter == EquipmentFilter.Camera) {
            cameras.sortedBy { it.obj().text("model") }.forEach { add(it to "camera") }
        }
        if (filter == EquipmentFilter.All || filter == EquipmentFilter.Lens || filter == EquipmentFilter.Other) {
            equipment
                .filter {
                    val category = it.obj().text("category")
                    when (filter) {
                        EquipmentFilter.Lens -> category == "lens"
                        EquipmentFilter.Other -> category != "lens"
                        else -> true
                    }
                }
                .sortedBy { it.obj().text("model") }
                .forEach { add(it to "equipment") }
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 18.dp, vertical = 18.dp),
    ) {
        ScreenHeader("Equipment", "相机机身、镜头与其他器材")
        Spacer(Modifier.height(14.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            EquipmentFilter.entries.forEach { entry ->
                FilterPill(
                    text = entry.label,
                    selected = filter == entry,
                    onClick = { filter = entry },
                )
            }
        }
        Spacer(Modifier.height(14.dp))
        if (rows.isEmpty()) {
            EmptyState("还没有同步器材")
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(rows, key = { it.first.entityType + it.first.syncId }) { (record, type) ->
                    val cameraData = record.obj()
                    val digital = type == "camera" && cameraData.text("cameraType") == "digital"
                    EquipmentCard(
                        record = record,
                        type = type,
                        collectionCount = when {
                            type != "camera" -> null
                            digital -> albumCountByCamera[record.syncId] ?: 0
                            else -> rollCountByCamera[record.syncId] ?: 0
                        },
                        collectionLabel = when {
                            type != "camera" -> null
                            digital -> "相册"
                            else -> "卷"
                        },
                    ) { onDetail(type, record.syncId) }
                }
            }
        }
    }
}

@Composable
private fun FilterPill(text: String, selected: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(999.dp),
        color = if (selected) GsfPurple.copy(alpha = 0.18f) else GsfSurfaceRaised,
        contentColor = if (selected) GsfPurple else GsfMuted,
        modifier = Modifier.then(
            if (selected) Modifier.background(Color.Transparent, RoundedCornerShape(999.dp))
            else Modifier,
        ),
    ) {
        Text(
            text = text,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            style = MaterialTheme.typography.labelMedium,
            fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
        )
    }
}

@Composable
private fun EquipmentCard(
    record: EntityRecord,
    type: String,
    collectionCount: Int?,
    collectionLabel: String?,
    onClick: () -> Unit,
) {
    val data = record.obj()
    val isCamera = type == "camera"
    val accent = if (isCamera) GsfPurple else GsfCyan
    GsfCard(modifier = Modifier.fillMaxWidth(), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(
                modifier = Modifier
                    .width(3.dp)
                    .height(64.dp)
                    .background(accent, RoundedCornerShape(99.dp)),
            )
            Spacer(Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    StatusChip(
                        if (isCamera) {
                            if (data.text("cameraType") == "digital") "数码机身" else "胶片机身"
                        } else if (data.text("category") == "lens") {
                            "镜头"
                        } else {
                            "其他"
                        },
                        accent,
                    )
                    StatusChip(
                        if (data.text("status") == "active") "在用" else "闲置",
                        GsfMuted,
                    )
                    if (collectionCount != null && collectionLabel != null) {
                        StatusChip("$collectionCount $collectionLabel", GsfCyan)
                    }
                }
                Spacer(Modifier.height(6.dp))
                Text(
                    text = listOf(data.text("brand"), data.text("model"))
                        .filter { it.isNotBlank() }
                        .joinToString(" ")
                        .ifBlank { "未命名器材" },
                    color = GsfText,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(2.dp))
                Text(
                    text = if (isCamera) {
                        if (data.text("cameraType") == "digital") {
                            data.text("sensorFormat").ifBlank { "CMOS 规格未记录" }
                        } else {
                            "${data.text("format").ifBlank { "135" }} 画幅"
                        }
                    } else if (data.text("category") == "lens") {
                        data.optionalText("mount")?.let { "卡口 $it" } ?: "卡口未记录"
                    } else {
                        data.optionalText("subtype") ?: "其他器材"
                    },
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                )
                data.optionalText("note")?.let { note ->
                    Spacer(Modifier.height(4.dp))
                    Text(
                        text = note,
                        color = GsfMuted,
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}
