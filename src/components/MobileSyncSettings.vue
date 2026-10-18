<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  MobileDevice,
  MobileSyncStatus,
  PairingRequest,
  PairingSession,
} from '../shared/sync'
import { errorMessage } from '../utils/errors'

const status = ref<MobileSyncStatus>()
const devices = ref<MobileDevice[]>([])
const requests = ref<PairingRequest[]>([])
const session = ref<PairingSession>()
const error = ref('')
const info = ref('')
const busy = ref('')
const now = ref(Date.now())
let refreshTimer: number | undefined

const sessionExpired = computed(() => {
  if (!session.value) return false
  return session.value.expiresAt * 1000 <= now.value
})

const sessionRemaining = computed(() => {
  if (!session.value) return 0
  return Math.max(0, Math.ceil(session.value.expiresAt - now.value / 1000))
})

function formatTime(value?: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString('zh-CN', { hour12: false })
}

function shortFingerprint(value?: string) {
  if (!value) return '—'
  return value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-8)}` : value
}

function platformLabel(platform: string) {
  if (platform === 'android') return 'Android'
  if (platform === 'ios') return 'iOS'
  if (platform === 'web') return 'Web'
  return platform
}

async function refresh() {
  now.value = Date.now()
  try {
    const [nextStatus, nextRequests, nextDevices] = await Promise.all([
      invoke<MobileSyncStatus>('get_mobile_sync_status'),
      invoke<PairingRequest[]>('get_mobile_pairing_requests'),
      invoke<MobileDevice[]>('get_mobile_devices'),
    ])
    status.value = nextStatus
    requests.value = nextRequests
    devices.value = nextDevices
  } catch (cause) {
    error.value = errorMessage(cause, '无法读取移动端同步状态')
  }
}

async function startService() {
  busy.value = 'start'
  error.value = ''
  info.value = ''
  try {
    status.value = await invoke<MobileSyncStatus>('start_mobile_sync')
    info.value = '同步服务已启动。请让手机和电脑连接同一局域网。'
    await refresh()
  } catch (cause) {
    error.value = errorMessage(cause, '启动同步服务失败')
  } finally {
    busy.value = ''
  }
}

async function stopService() {
  busy.value = 'stop'
  error.value = ''
  info.value = ''
  try {
    status.value = await invoke<MobileSyncStatus>('stop_mobile_sync')
    session.value = undefined
    info.value = '同步服务已停止，手机将无法继续连接。'
    await refresh()
  } catch (cause) {
    error.value = errorMessage(cause, '停止同步服务失败')
  } finally {
    busy.value = ''
  }
}

async function createPairing() {
  busy.value = 'pair'
  error.value = ''
  info.value = ''
  try {
    session.value = await invoke<PairingSession>('create_mobile_pairing')
    status.value = await invoke<MobileSyncStatus>('get_mobile_sync_status')
    info.value = '二维码 2 分钟内有效。手机扫码后，还需要在这里确认。'
  } catch (cause) {
    error.value = errorMessage(cause, '生成配对二维码失败')
  } finally {
    busy.value = ''
  }
}

async function approve(request: PairingRequest) {
  busy.value = `approve:${request.id}`
  error.value = ''
  try {
    await invoke<string>('approve_mobile_pairing', { requestId: request.id })
    info.value = `已批准 ${request.deviceName}，请在手机上完成配对。`
    await refresh()
  } catch (cause) {
    error.value = errorMessage(cause, '批准配对失败')
  } finally {
    busy.value = ''
  }
}

async function deny(request: PairingRequest) {
  busy.value = `deny:${request.id}`
  error.value = ''
  try {
    await invoke<string>('deny_mobile_pairing', { requestId: request.id })
    info.value = `已拒绝 ${request.deviceName} 的配对请求。`
    await refresh()
  } catch (cause) {
    error.value = errorMessage(cause, '拒绝配对失败')
  } finally {
    busy.value = ''
  }
}

async function revoke(device: MobileDevice) {
  if (!confirm(`确定撤销“${device.name}”吗？该设备会立即失去同步权限。`)) return
  busy.value = `revoke:${device.id}`
  error.value = ''
  try {
    await invoke<string>('revoke_mobile_device', { deviceId: device.id })
    info.value = `已撤销 ${device.name}。`
    await refresh()
  } catch (cause) {
    error.value = errorMessage(cause, '撤销设备失败')
  } finally {
    busy.value = ''
  }
}

async function copyPairing() {
  if (!session.value) return
  try {
    await navigator.clipboard.writeText(session.value.payload)
    info.value = '配对信息已复制，可在手机端手动粘贴。'
  } catch {
    error.value = '复制失败，请让手机直接扫描二维码。'
  }
}

onMounted(() => {
  void refresh()
  refreshTimer = window.setInterval(() => {
    now.value = Date.now()
    void refresh()
  }, 2500)
})

onBeforeUnmount(() => {
  if (refreshTimer !== undefined) window.clearInterval(refreshTimer)
})
</script>

<template>
  <section class="settings-section mobile-sync-section">
    <div class="mobile-sync-heading">
      <div>
        <h3>移动端同步</h3>
        <p>同一局域网内，把档案只读同步到 Android 手机</p>
      </div>
      <span class="sync-state" :class="{ running: status?.running }">
        <i></i>{{ status?.running ? '运行中' : '已停止' }}
      </span>
    </div>

    <div v-if="error" class="feedback-error" role="alert">{{ error }}</div>
    <div v-if="info" class="feedback-info" role="status">{{ info }}</div>

    <div class="sync-status-grid">
      <div>
        <span>服务地址</span>
        <strong>{{ status?.address || '未启动' }}</strong>
      </div>
      <div>
        <span>服务指纹</span>
        <strong :title="status?.fingerprint">{{ shortFingerprint(status?.fingerprint) }}</strong>
      </div>
      <div>
        <span>已配对设备</span>
        <strong>{{ status?.pairedDeviceCount ?? 0 }}</strong>
      </div>
      <div>
        <span>协议版本</span>
        <strong>v{{ status?.protocolVersion ?? 1 }}</strong>
      </div>
    </div>

    <p v-if="status?.localOnly" class="sync-hint warning">
      未检测到局域网地址，当前只在 127.0.0.1 上监听；请先连接 Wi-Fi 或网线后重新启动同步服务。
    </p>
    <p class="sync-hint">
      同步服务默认关闭，只监听当前局域网地址，不会把正式图库作为静态目录暴露。第一版请只在可信局域网内使用。
    </p>

    <div class="sync-actions">
      <button
        v-if="!status?.running"
        class="primary-btn"
        type="button"
        :disabled="busy !== ''"
        @click="startService"
      >
        {{ busy === 'start' ? '正在启动…' : '启动同步服务' }}
      </button>
      <button
        v-else
        class="secondary-btn"
        type="button"
        :disabled="busy !== ''"
        @click="stopService"
      >
        {{ busy === 'stop' ? '正在停止…' : '停止同步服务' }}
      </button>
      <button
        class="primary-btn"
        type="button"
        :disabled="!status?.running || busy !== ''"
        @click="createPairing"
      >
        {{ busy === 'pair' ? '正在生成…' : '启动配对' }}
      </button>
    </div>

    <div v-if="session && !sessionExpired" class="pairing-box">
      <div class="qr-frame" v-html="session.qrSvg"></div>
      <div class="pairing-copy">
        <strong>用手机扫描二维码</strong>
        <p>扫码后手机会提交配对请求，请在本页点击“批准”。</p>
        <dl>
          <div><dt>地址</dt><dd>{{ session.url }}</dd></div>
          <div><dt>指纹</dt><dd :title="session.fingerprint">{{ shortFingerprint(session.fingerprint) }}</dd></div>
          <div><dt>剩余</dt><dd>{{ sessionRemaining }} 秒</dd></div>
        </dl>
        <button class="secondary-btn compact-btn" type="button" @click="copyPairing">复制配对信息</button>
      </div>
    </div>
    <div v-else-if="session" class="pairing-expired">
      二维码已过期，请点击“启动配对”重新生成。
    </div>

    <div v-if="requests.length" class="pending-requests">
      <h4>等待确认</h4>
      <div v-for="request in requests" :key="request.id" class="pending-row">
        <div>
          <strong>{{ request.deviceName }}</strong>
          <span>{{ platformLabel(request.platform) }} · {{ formatTime(request.createdAt) }}</span>
        </div>
        <div class="pending-actions">
          <button
            class="primary-btn compact-btn"
            type="button"
            :disabled="busy !== ''"
            @click="approve(request)"
          >
            批准
          </button>
          <button
            class="secondary-btn compact-btn"
            type="button"
            :disabled="busy !== ''"
            @click="deny(request)"
          >
            拒绝
          </button>
        </div>
      </div>
    </div>

    <div class="device-list">
      <h4>已配对设备</h4>
      <p v-if="devices.length === 0" class="empty-hint">还没有配对设备。</p>
      <div v-for="device in devices" :key="device.id" class="device-row">
        <div>
          <strong>{{ device.name }}</strong>
          <span>{{ platformLabel(device.platform) }} · 配对 {{ formatTime(device.pairedAt) }}</span>
          <span>最近同步 {{ formatTime(device.lastSyncAt) }}</span>
        </div>
        <button
          class="danger-link"
          type="button"
          :disabled="busy !== ''"
          @click="revoke(device)"
        >
          撤销
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.mobile-sync-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.mobile-sync-heading h3 {
  margin: 0;
}

.mobile-sync-heading p {
  margin: 5px 0 0;
}

.sync-state {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: none;
  padding: 5px 9px;
  border: 1px solid #3a4252;
  border-radius: 999px;
  color: #9ca3af;
  font-size: 12px;
}

.sync-state i {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #6b7280;
}

.sync-state.running {
  border-color: #2f6b4f;
  color: #9be3c2;
}

.sync-state.running i {
  background: #34d399;
  box-shadow: 0 0 8px rgba(52, 211, 153, 0.7);
}

.sync-status-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 9px;
  margin-top: 14px;
}

.sync-status-grid > div {
  min-width: 0;
  padding: 9px 10px;
  border: 1px solid #28303d;
  border-radius: 7px;
  background: #11161e;
}

.sync-status-grid span,
.device-row span,
.pending-row span {
  display: block;
  color: #8b94a3;
  font-size: 12px;
}

.sync-status-grid strong {
  display: block;
  margin-top: 4px;
  overflow: hidden;
  color: #e5e7eb;
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sync-hint {
  margin: 12px 0 0;
  color: #7f8998;
  font-size: 12px;
  line-height: 1.6;
}

.sync-hint.warning {
  color: #e4cf94;
}

.sync-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
  margin-top: 14px;
}

.compact-btn {
  min-height: 30px;
  padding: 5px 10px;
  font-size: 12px;
}

.pairing-box {
  display: grid;
  grid-template-columns: minmax(180px, 220px) minmax(0, 1fr);
  gap: 16px;
  align-items: center;
  margin-top: 16px;
  padding: 14px;
  border: 1px solid #303846;
  border-radius: 9px;
  background: #10151d;
}

.qr-frame {
  padding: 9px;
  border-radius: 8px;
  background: #ffffff;
}

.qr-frame :deep(svg) {
  display: block;
  width: 100%;
  height: auto;
}

.pairing-copy strong {
  color: #f3f6fb;
  font-size: 14px;
}

.pairing-copy p {
  margin: 4px 0 10px;
}

.pairing-copy dl {
  display: grid;
  gap: 5px;
  margin: 0 0 10px;
  font-size: 12px;
}

.pairing-copy dl > div {
  display: flex;
  gap: 8px;
}

.pairing-copy dt {
  flex: none;
  color: #7f8998;
}

.pairing-copy dd {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: #c8d0dc;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pairing-expired {
  margin-top: 14px;
  padding: 10px 12px;
  border: 1px solid #67552c;
  border-radius: 7px;
  background: #211d14;
  color: #e4cf94;
  font-size: 12px;
}

.pending-requests,
.device-list {
  margin-top: 18px;
}

.pending-requests h4,
.device-list h4 {
  margin: 0 0 8px;
  color: #cbd3df;
  font-size: 13px;
}

.pending-row,
.device-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 9px 10px;
  border: 1px solid #28303d;
  border-radius: 7px;
  background: #11161e;
}

.pending-row + .pending-row,
.device-row + .device-row {
  margin-top: 6px;
}

.pending-row strong,
.device-row strong {
  display: block;
  color: #edf1f7;
  font-size: 13px;
}

.pending-actions {
  display: flex;
  gap: 7px;
  flex: none;
}

.danger-link {
  flex: none;
  border: 0;
  background: transparent;
  color: #f08a8a;
  cursor: pointer;
  font-size: 12px;
  padding: 5px;
}

.danger-link:hover {
  color: #ffb0b0;
}

.empty-hint {
  margin: 0;
  color: #7f8998;
  font-size: 12px;
}

@media (max-width: 560px) {
  .pairing-box {
    grid-template-columns: 1fr;
  }

  .qr-frame {
    width: min(220px, 60vw);
    margin: 0 auto;
  }
}
</style>
