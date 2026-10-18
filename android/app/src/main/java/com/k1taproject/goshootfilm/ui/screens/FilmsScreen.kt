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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.data.EntityRecord
import com.k1taproject.goshootfilm.data.FilmEntry
import com.k1taproject.goshootfilm.data.filmTypeLabel
import com.k1taproject.goshootfilm.data.obj
import com.k1taproject.goshootfilm.data.optionalText
import com.k1taproject.goshootfilm.data.text
import com.k1taproject.goshootfilm.ui.GsfBorder
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfSurface
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.ScreenHeader

@Composable
fun FilmsScreen(vm: GsfViewModel) {
    var query by remember { mutableStateOf("") }
    var selected by remember { mutableStateOf<FilmEntry?>(null) }
    var syncedFilms by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }
    var rolls by remember { mutableStateOf<List<EntityRecord>>(emptyList()) }

    LaunchedEffect(Unit) {
        syncedFilms = vm.entities("film")
        rolls = vm.entities("roll")
    }

    val filmByKey = syncedFilms.associateBy {
        val data = it.obj()
        "${data.text("brand")}|${data.text("name")}".lowercase()
    }
    val shotFilmIds = rolls.mapNotNull { it.obj().optionalText("filmSyncId") }.toSet()
    val entries = vm.filmCatalog.entries
        .filter { it.matches(query) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 18.dp, vertical = 18.dp),
    ) {
        ScreenHeader("Films", "胶卷目录与拍摄状态")
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = query,
            onValueChange = { query = it },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            placeholder = { Text("搜索品牌、型号或种类") },
            shape = RoundedCornerShape(12.dp),
        )
        Spacer(Modifier.height(12.dp))
        Text(
            text = "共 ${entries.size} 种",
            color = GsfMuted,
            style = MaterialTheme.typography.labelSmall,
        )
        Spacer(Modifier.height(8.dp))
        if (entries.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("没有匹配的胶卷", color = GsfMuted)
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Fixed(2),
                modifier = Modifier.weight(1f),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(entries, key = { "${it.brand}|${it.name}" }) { entry ->
                    val synced = filmByKey["${entry.brand}|${entry.name}".lowercase()]
                    val isShot = synced?.syncId?.let { it in shotFilmIds } == true
                    FilmCard(entry, isShot) { selected = entry }
                }
            }
        }
    }

    selected?.let { entry ->
        val synced = filmByKey["${entry.brand}|${entry.name}".lowercase()]
        val data = synced?.obj()
        val isShot = synced?.syncId?.let { it in shotFilmIds } == true
        AlertDialog(
            onDismissRequest = { selected = null },
            confirmButton = {
                TextButton(onClick = { selected = null }) { Text("关闭") }
            },
            title = { Text(entry.displayName) },
            text = {
                Column {
                    AsyncImage(
                        model = "file:///android_asset/${entry.fullAsset}",
                        contentDescription = null,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier
                            .fillMaxWidth()
                            .aspectRatio(2.05f)
                            .clip(RoundedCornerShape(10.dp)),
                    )
                    Spacer(Modifier.height(10.dp))
                    Text("ISO ${entry.iso} · ${filmTypeLabel(entry.type)}", color = GsfMuted)
                    Spacer(Modifier.height(6.dp))
                    Text(
                        if (isShot) "已拍摄" else "未拍摄",
                        color = if (isShot) GsfCyan else GsfMuted,
                        fontWeight = FontWeight.SemiBold,
                    )
                    data?.optionalText("note")?.let {
                        Spacer(Modifier.height(6.dp))
                        Text(it, color = GsfMuted, style = MaterialTheme.typography.bodySmall)
                    }
                }
            },
            containerColor = GsfSurface,
        )
    }
}

@Composable
private fun FilmCard(entry: FilmEntry, isShot: Boolean, onClick: () -> Unit) {
    Surface(
        color = GsfSurface,
        shape = RoundedCornerShape(12.dp),
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, GsfBorder, RoundedCornerShape(12.dp))
            .clickable(onClick = onClick),
    ) {
        Column {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(2.05f)
                    .background(GsfBorder),
            ) {
                AsyncImage(
                    model = "file:///android_asset/${entry.thumbAsset}",
                    contentDescription = entry.displayName,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(
                            Brush.verticalGradient(
                                listOf(Color.Transparent, Color.Black.copy(alpha = 0.55f)),
                            ),
                        ),
                )
                if (isShot) {
                    Text(
                        text = "已拍摄",
                        color = GsfCyan,
                        style = MaterialTheme.typography.labelSmall,
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .padding(8.dp)
                            .background(Color.Black.copy(alpha = 0.45f), RoundedCornerShape(999.dp))
                            .padding(horizontal = 7.dp, vertical = 3.dp),
                    )
                }
            }
            Column(modifier = Modifier.padding(10.dp)) {
                Text(
                    text = entry.displayName,
                    color = GsfText,
                    style = MaterialTheme.typography.bodySmall,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(3.dp))
                Text(
                    text = "ISO ${entry.iso} · ${filmTypeLabel(entry.type)}",
                    color = GsfMuted,
                    style = MaterialTheme.typography.labelSmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}
