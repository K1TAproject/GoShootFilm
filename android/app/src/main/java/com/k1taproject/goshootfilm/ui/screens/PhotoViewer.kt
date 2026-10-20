package com.k1taproject.goshootfilm.ui.screens

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.core.content.ContextCompat
import coil.compose.AsyncImage
import com.k1taproject.goshootfilm.data.GallerySaver
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfSurface
import com.k1taproject.goshootfilm.ui.GsfText
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

@Composable
fun PhotoViewerDialog(
    file: File,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var showActions by remember { mutableStateOf(false) }
    var saving by remember { mutableStateOf(false) }
    var pendingSave by remember { mutableStateOf(false) }
    var scale by remember { mutableStateOf(1f) }
    var offset by remember { mutableStateOf(Offset.Zero) }

    fun saveToGallery() {
        if (saving) return
        saving = true
        scope.launch {
            try {
                withContext(Dispatchers.IO) {
                    GallerySaver.saveToGallery(context, file)
                }
                Toast.makeText(context, "已保存到相册", Toast.LENGTH_SHORT).show()
            } catch (error: Exception) {
                Toast.makeText(
                    context,
                    "保存失败：${error.message ?: "无法写入相册"}",
                    Toast.LENGTH_LONG,
                ).show()
            } finally {
                saving = false
            }
        }
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (granted && pendingSave) {
            pendingSave = false
            saveToGallery()
        } else {
            pendingSave = false
            Toast.makeText(context, "需要存储权限才能保存到相册", Toast.LENGTH_SHORT).show()
        }
    }

    fun requestSave() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.WRITE_EXTERNAL_STORAGE) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            pendingSave = true
            permissionLauncher.launch(Manifest.permission.WRITE_EXTERNAL_STORAGE)
        } else {
            saveToGallery()
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
            AsyncImage(
                model = file,
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
                    .combinedClickable(
                        onClick = {},
                        onLongClick = { showActions = true },
                    )
                    .pointerInput(file) {
                        detectTransformGestures { _, pan, zoom, _ ->
                            scale = (scale * zoom).coerceIn(1f, 6f)
                            offset = if (scale <= 1f) Offset.Zero else offset + pan
                        }
                    },
            )
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
                Row {
                    TextButton(onClick = { showActions = true }) {
                        Text("更多", color = GsfCyan)
                    }
                    TextButton(onClick = onDismiss) { Text("关闭", color = GsfText) }
                }
            }
            Text(
                text = "长按图片保存或分享",
                color = GsfMuted,
                style = MaterialTheme.typography.labelSmall,
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .background(Color.Black.copy(alpha = 0.55f))
                    .padding(horizontal = 12.dp, vertical = 6.dp),
            )
            if (saving) {
                Column(
                    modifier = Modifier.align(Alignment.Center),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    CircularProgressIndicator(color = GsfCyan)
                    Spacer(Modifier.height(8.dp))
                    Text("正在保存到相册…", color = GsfText)
                }
            }
        }
    }

    if (showActions) {
        AlertDialog(
            onDismissRequest = { showActions = false },
            title = { Text("图片操作") },
            text = {
                Column {
                    TextButton(
                        onClick = {
                            showActions = false
                            requestSave()
                        },
                    ) { Text("保存到相册") }
                    TextButton(
                        onClick = {
                            showActions = false
                            runCatching { GallerySaver.share(context, file) }
                                .onFailure {
                                    Toast.makeText(context, "分享失败", Toast.LENGTH_SHORT).show()
                                }
                        },
                    ) { Text("分享") }
                }
            },
            confirmButton = {
                TextButton(onClick = { showActions = false }) { Text("取消") }
            },
            containerColor = GsfSurface,
        )
    }
}
