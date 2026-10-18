<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'
import { useRouter } from 'vue-router'
import appIcon from '../assets/app-icon.png'
import type { PairingQrPayload } from '../../../src/shared/sync'
import { pairWithPayload, settings, updateSettings } from '../lib/store'

interface DetectedBarcode {
  rawValue: string
}

interface BarcodeDetectorLike {
  detect(source: CanvasImageSource): Promise<DetectedBarcode[]>
}

type BarcodeDetectorConstructor = new (options?: {
  formats?: string[]
}) => BarcodeDetectorLike

const router = useRouter()
const videoRef = ref<HTMLVideoElement | null>(null)
const scanning = ref(false)
const cameraError = ref('')
const manualMode = ref(false)
const manualText = ref('')
const busy = ref(false)
const statusText = ref('')
const error = ref('')
let stream: MediaStream | undefined
let detector: BarcodeDetectorLike | undefined
let scanTimer: number | undefined

const deviceName = computed({
  get: () => settings.deviceName,
  set: (value: string) => updateSettings({ deviceName: value.slice(0, 80) }),
})

const detectorConstructor = () =>
  (window as unknown as { BarcodeDetector?: BarcodeDetectorConstructor }).BarcodeDetector

function stopScan() {
  scanning.value = false
  if (scanTimer !== undefined) {
    window.clearTimeout(scanTimer)
    scanTimer = undefined
  }
  if (stream) {
    for (const track of stream.getTracks()) track.stop()
    stream = undefined
  }
  if (videoRef.value) videoRef.value.srcObject = null
  detector = undefined
}

async function scanLoop() {
  if (!scanning.value || !videoRef.value || !detector) return
  try {
    const codes = await detector.detect(videoRef.value)
    const raw = codes[0]?.rawValue
    if (raw) {
      stopScan()
      await connect(raw)
      return
    }
  } catch {
    // Ignore a single failed frame and keep scanning.
  }
  scanTimer = window.setTimeout(() => void scanLoop(), 350)
}

async function startScan() {
  error.value = ''
  statusText.value = ''
  cameraError.value = ''
  manualMode.value = false
  const Detector = detectorConstructor()
  if (!Detector) {
    cameraError.value =
      '当前浏览器不支持二维码识别。请使用电脑端的“复制配对信息”，再切换到手动输入。'
    manualMode.value = true
    return
  }
  if (!navigator.mediaDevices?.getUserMedia) {
    cameraError.value =
      '当前页面无法访问摄像头（局域网 HTTP 通常会禁用摄像头）。请使用电脑端“复制配对信息”后手动粘贴。'
    manualMode.value = true
    return
  }
  scanning.value = true
  await nextTick()
  try {
    stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: { ideal: 'environment' } },
      audio: false,
    })
    if (!videoRef.value) {
      stopScan()
      cameraError.value = '摄像头画面初始化失败，请改用手动粘贴。'
      manualMode.value = true
      return
    }
    videoRef.value.srcObject = stream
    await videoRef.value.play()
    detector = new Detector({ formats: ['qr_code'] })
    scanning.value = true
    void scanLoop()
  } catch {
    stopScan()
    cameraError.value =
      '无法打开摄像头。请检查浏览器权限；也可以使用电脑端“复制配对信息”后手动粘贴。'
    manualMode.value = true
  }
}

function parsePayload(raw: string): PairingQrPayload {
  let value: unknown
  try {
    value = JSON.parse(raw)
  } catch {
    throw new Error('配对信息不是有效的 JSON')
  }
  if (!value || typeof value !== 'object') throw new Error('配对信息格式不正确')
  const payload = value as Partial<PairingQrPayload>
  if (
    payload.v !== 1 ||
    typeof payload.url !== 'string' ||
    typeof payload.token !== 'string' ||
    typeof payload.fingerprint !== 'string' ||
    typeof payload.expiresAt !== 'number'
  ) {
    throw new Error('配对信息缺少必要字段或版本不兼容')
  }
  return {
    v: payload.v,
    url: payload.url,
    token: payload.token,
    fingerprint: payload.fingerprint,
    expiresAt: payload.expiresAt,
    serverName: typeof payload.serverName === 'string' ? payload.serverName : 'GoShootFilm',
  }
}

async function connect(raw: string) {
  if (!raw.trim()) {
    error.value = '请先扫描二维码或粘贴配对信息'
    return
  }
  busy.value = true
  error.value = ''
  statusText.value = '正在提交配对请求…'
  try {
    const payload = parsePayload(raw.trim())
    statusText.value = '等待电脑端确认…'
    await pairWithPayload(payload)
    statusText.value = '配对成功，正在打开档案…'
    window.setTimeout(() => void router.replace('/'), 350)
  } catch (cause) {
    statusText.value = ''
    error.value = cause instanceof Error ? cause.message : '配对失败，请重试'
  } finally {
    busy.value = false
    stopScan()
  }
}

onBeforeUnmount(stopScan)
</script>

<template>
  <main class="pair-screen">
    <div class="pair-hero">
      <img :src="appIcon" alt="" />
      <div>
        <span class="eyebrow">GoShootFilm Mobile</span>
        <h1>配对电脑</h1>
        <p>手机只读浏览电脑上的胶片与数码档案</p>
      </div>
    </div>

    <section class="card card-pad stack">
      <label class="field">
        <span>设备名称</span>
        <input v-model="deviceName" class="input" maxlength="80" placeholder="我的 Android 手机" />
      </label>
      <div class="pair-actions">
        <button class="btn btn-primary" type="button" :disabled="busy || scanning" @click="startScan">
          {{ scanning ? '正在扫描…' : '扫描二维码' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="busy" @click="manualMode = !manualMode">
          {{ manualMode ? '收起手动输入' : '手动输入' }}
        </button>
        <button v-if="scanning" class="btn btn-secondary" type="button" @click="stopScan">停止扫描</button>
      </div>
    </section>

    <div v-if="scanning || cameraError" class="scan-frame">
      <video ref="videoRef" muted playsinline v-show="scanning"></video>
      <div v-if="scanning" class="scan-guide"></div>
      <div v-else class="scan-placeholder">{{ cameraError }}</div>
    </div>

    <section v-if="manualMode" class="card card-pad stack">
      <label class="field">
        <span>配对信息</span>
        <textarea
          v-model="manualText"
          class="textarea"
          placeholder="在电脑端点击“复制配对信息”，把 JSON 粘贴到这里"
        ></textarea>
      </label>
      <button
        class="btn btn-primary btn-block"
        type="button"
        :disabled="busy || !manualText.trim()"
        @click="connect(manualText)"
      >
        {{ busy ? '正在配对…' : '使用配对信息' }}
      </button>
    </section>

    <div v-if="statusText || error" class="pair-status" :class="{ busy: busy && !error, error }">
      {{ error || statusText }}
    </div>

    <p class="pair-note">
      电脑端打开 设置 → 移动端同步 → 启动同步服务 → 启动配对。手机需与电脑在同一局域网。
      扫码后电脑端还需要点击“批准”，配对令牌 2 分钟内有效且只能使用一次。
    </p>
    <p class="pair-note">
      浏览器在局域网 HTTP 下通常会禁止摄像头，此时请使用电脑端的“复制配对信息”手动粘贴。
      Android 原生壳接入相机和 Keystore 后可直接扫码并安全保存凭证。
    </p>
  </main>
</template>
