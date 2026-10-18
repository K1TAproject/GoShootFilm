package com.k1taproject.goshootfilm.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import coil.compose.AsyncImage
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfText
import java.io.File

@Composable
fun PhotoViewerDialog(
    vm: GsfViewModel,
    assetId: String,
    onDismiss: () -> Unit,
) {
    var file by remember(assetId) { mutableStateOf<File?>(null) }
    var loading by remember(assetId) { mutableStateOf(true) }
    var downloaded by remember(assetId) { mutableStateOf(0L) }
    var total by remember(assetId) { mutableStateOf(0L) }
    var error by remember(assetId) { mutableStateOf<String?>(null) }
    var retry by remember(assetId) { mutableStateOf(0) }
    var scale by remember(assetId) { mutableStateOf(1f) }
    var offset by remember(assetId) { mutableStateOf(Offset.Zero) }

    LaunchedEffect(assetId, retry) {
        loading = true
        error = null
        file = null
        downloaded = 0
        total = 0
        try {
            file = vm.downloadEditOriginal(assetId) { done, size ->
                downloaded = done
                total = size
            }
        } catch (failure: Exception) {
            error = failure.message ?: "无法下载调色原图"
        } finally {
            loading = false
        }
    }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false),
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black),
        ) {
            val current = file
            if (current != null) {
                AsyncImage(
                    model = current,
                    contentDescription = "调色原图",
                    contentScale = ContentScale.Fit,
                    modifier = Modifier
                        .fillMaxSize()
                        .graphicsLayer(
                            scaleX = scale,
                            scaleY = scale,
                            translationX = offset.x,
                            translationY = offset.y,
                        )
                        .pointerInput(assetId) {
                            detectTransformGestures { _, pan, zoom, _ ->
                                scale = (scale * zoom).coerceIn(1f, 6f)
                                offset = if (scale <= 1f) {
                                    Offset.Zero
                                } else {
                                    offset + pan
                                }
                            }
                        },
                )
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopCenter)
                    .background(Color.Black.copy(alpha = 0.55f))
                    .padding(horizontal = 8.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "调色原图",
                    color = GsfText,
                    style = MaterialTheme.typography.titleSmall,
                )
                TextButton(onClick = onDismiss) { Text("关闭", color = GsfCyan) }
            }
            if (loading) {
                Column(
                    modifier = Modifier.align(Alignment.Center),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    CircularProgressIndicator(color = GsfCyan)
                    Spacer(Modifier.height(10.dp))
                    Text(
                        text = if (total > 0) {
                            "正在下载 ${downloaded / 1024 / 1024} / ${total / 1024 / 1024} MB"
                        } else {
                            "正在下载正式调色原图…"
                        },
                        color = GsfMuted,
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
            error?.let { message ->
                Column(
                    modifier = Modifier
                        .align(Alignment.Center)
                        .padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(message, color = Color(0xFFF0A7AF))
                    Spacer(Modifier.height(8.dp))
                    TextButton(
                        onClick = {
                            error = null
                            retry++
                        },
                    ) { Text("重试", color = GsfCyan) }
                }
            }
        }
    }
}
