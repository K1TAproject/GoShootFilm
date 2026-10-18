package com.k1taproject.goshootfilm.ui.screens

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.k1taproject.goshootfilm.GsfViewModel
import com.k1taproject.goshootfilm.PairState
import com.k1taproject.goshootfilm.scan.QrScanner
import com.k1taproject.goshootfilm.ui.GsfBorder
import com.k1taproject.goshootfilm.ui.GsfCyan
import com.k1taproject.goshootfilm.ui.GsfMuted
import com.k1taproject.goshootfilm.ui.GsfPurple
import com.k1taproject.goshootfilm.ui.GsfSurface
import com.k1taproject.goshootfilm.ui.GsfText
import com.k1taproject.goshootfilm.ui.components.GsfCard
import com.k1taproject.goshootfilm.ui.components.StatusChip

@Composable
fun PairScreen(vm: GsfViewModel) {
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    val pairState by vm.pairState.collectAsStateWithLifecycle()
    val settings by vm.settings.collectAsStateWithLifecycle()
    var manual by remember { mutableStateOf("") }
    var showManual by remember { mutableStateOf(false) }
    var hasCamera by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED,
        )
    }
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted -> hasCamera = granted }

    LaunchedEffect(Unit) {
        if (!hasCamera) permissionLauncher.launch(Manifest.permission.CAMERA)
    }

    Scaffold(containerColor = Color.Transparent) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 28.dp),
        ) {
            Text(
                text = "GSFMobile",
                color = GsfText,
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
            )
            Spacer(Modifier.height(6.dp))
            Text(
                text = "局域网只读客户端",
                color = GsfPurple,
                style = MaterialTheme.typography.titleSmall,
            )
            Spacer(Modifier.height(18.dp))
            Text(
                text = "在电脑端打开「移动端同步」，启动同步服务和配对，然后扫描二维码。",
                color = GsfMuted,
                style = MaterialTheme.typography.bodyMedium,
            )
            Spacer(Modifier.height(18.dp))

            val waiting = pairState is PairState.Waiting
            if (hasCamera && !waiting && !showManual) {
                QrScanner(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(310.dp),
                ) { raw ->
                    vm.pairFromPayload(raw)
                }
                Spacer(Modifier.height(10.dp))
                Text(
                    text = "将电脑端二维码放入取景框",
                    color = GsfMuted,
                    style = MaterialTheme.typography.bodySmall,
                )
            } else if (waiting) {
                GsfCard(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        CircularProgressIndicator(color = GsfCyan)
                        Spacer(Modifier.height(14.dp))
                        Text(
                            text = (pairState as PairState.Waiting).status,
                            color = GsfText,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.height(6.dp))
                        Text(
                            text = "请在电脑端点击「批准」",
                            color = GsfMuted,
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            } else {
                GsfCard(modifier = Modifier.fillMaxWidth()) {
                    Column {
                        Text("手动配对", color = GsfText, fontWeight = FontWeight.SemiBold)
                        Spacer(Modifier.height(6.dp))
                        Text(
                            "摄像头不可用时，粘贴电脑端「复制配对信息」的 JSON。",
                            color = GsfMuted,
                            style = MaterialTheme.typography.bodySmall,
                        )
                        Spacer(Modifier.height(10.dp))
                        OutlinedTextField(
                            value = manual,
                            onValueChange = { manual = it },
                            modifier = Modifier.fillMaxWidth(),
                            minLines = 4,
                            maxLines = 8,
                            placeholder = { Text("{\"v\":1,\"url\":\"...\",...}") },
                            shape = RoundedCornerShape(12.dp),
                        )
                        Spacer(Modifier.height(8.dp))
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            OutlinedButton(
                                onClick = {
                                    clipboard.getText()?.text?.let { manual = it }
                                },
                            ) { Text("粘贴") }
                            Button(
                                onClick = { vm.pairFromPayload(manual) },
                                enabled = manual.isNotBlank(),
                            ) { Text("提交配对") }
                        }
                    }
                }
            }

            Spacer(Modifier.height(14.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                TextButton(onClick = { showManual = !showManual }) {
                    Text(if (showManual) "使用摄像头" else "手动输入")
                }
                if (!hasCamera) {
                    TextButton(onClick = { permissionLauncher.launch(Manifest.permission.CAMERA) }) {
                        Text("允许相机")
                    }
                }
            }

            if (settings.pendingFingerprint.isNotBlank()) {
                Spacer(Modifier.height(12.dp))
                GsfCard(modifier = Modifier.fillMaxWidth()) {
                    Column {
                        StatusChip("服务指纹", GsfCyan)
                        Spacer(Modifier.height(8.dp))
                        Text(
                            text = settings.pendingFingerprint,
                            color = GsfMuted,
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            }

            when (val state = pairState) {
                is PairState.Failed -> {
                    Spacer(Modifier.height(12.dp))
                    GsfCard(modifier = Modifier.fillMaxWidth()) {
                        Column {
                            Text("配对失败", color = Color(0xFFF0A7AF), fontWeight = FontWeight.SemiBold)
                            Spacer(Modifier.height(4.dp))
                            Text(state.message, color = GsfMuted, style = MaterialTheme.typography.bodySmall)
                            Spacer(Modifier.height(8.dp))
                            OutlinedButton(onClick = { vm.consumePairError() }) { Text("重试") }
                        }
                    }
                }
                PairState.Paired -> {
                    Spacer(Modifier.height(12.dp))
                    Text("配对成功，正在准备同步…", color = GsfCyan)
                }
                else -> Unit
            }
        }
    }
}
