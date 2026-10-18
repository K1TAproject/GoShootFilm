<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import {
  clearViewCache,
  formatBytes,
  formatSyncTime,
  refreshStorageUsage,
  settings,
  store,
  syncNow,
  unpair,
  updateSettings,
} from '../lib/store'
import { usingNativeSecureStorage } from '../lib/secure'

const router = useRouter()

const cacheOptions = [
  { value: 2 * 1024 * 1024 * 1024, label: '2 GB' },
  { value: 5 * 1024 * 1024 * 1024, label: '5 GB' },
  { value: 10 * 1024 * 1024 * 1024, label: '10 GB' },
  { value: 0, label: '不自动清理' },
]

function changeCacheLimit(event: Event) {
  const value = Number((event.target as HTMLSelectElement).value)
  updateSettings({ cacheLimitBytes: value })
}

async function rePair() {
  if (!confirm('重新配对会清除当前配对凭证，但保留已缓存的档案。是否继续？')) return
  await unpair()
  await router.push('/pair')
}

async function disconnect() {
  if (!confirm('解除配对后手机将不能继续同步，已缓存的内容仍可离线查看。是否继续？')) return
  await unpair()
  store.syncNotice = '已解除配对，本地缓存仍可离线浏览'
}

onMounted(() => {
  void refreshStorageUsage()
})
</script>

<template>
  <main class="page page-detail">
    <MobileHeader title="设置" subtitle="同步、缓存与安全" back @sync="syncNow" />
    <SyncNotice />

    <section class="card card-pad">
      <div class="spread">
        <div>
          <span class="eyebrow">Pairing</span>
          <strong style="display: block; margin-top: 4px; font-size: 16px">
            {{ store.paired ? '已与电脑配对' : '尚未配对' }}
          </strong>
        </div>
        <span class="pill" :class="{ ok: store.paired }">
          {{ store.paired ? '已连接' : '离线' }}
        </span>
      </div>
      <dl class="detail-list">
        <div><dt>电脑地址</dt><dd>{{ store.serverUrl || '未设置' }}</dd></div>
        <div><dt>服务指纹</dt><dd style="max-width: 62%; overflow-wrap: anywhere">{{ store.fingerprint || '—' }}</dd></div>
        <div><dt>设备 ID</dt><dd style="max-width: 62%; overflow-wrap: anywhere">{{ store.deviceId || '—' }}</dd></div>
        <div><dt>上次同步</dt><dd>{{ formatSyncTime(store.lastSyncAt) }}</dd></div>
        <div>
          <dt>凭证存储</dt>
          <dd>{{ usingNativeSecureStorage() ? '系统安全存储' : '浏览器本地存储' }}</dd>
        </div>
      </dl>
      <p v-if="!usingNativeSecureStorage()" class="small muted" style="margin: 8px 0 0; line-height: 1.6">
        当前是浏览器/PWA 预览模式，配对凭证保存在 WebView 的本地存储中。Android 原生壳需要接入 Android Keystore 桥接后才会显示“系统安全存储”。
      </p>
      <div class="row" style="margin-top: 12px; flex-wrap: wrap">
        <button class="btn btn-primary" type="button" :disabled="store.syncing || !store.paired" @click="syncNow">
          {{ store.syncing ? '同步中…' : '手动同步' }}
        </button>
        <button class="btn btn-secondary" type="button" :disabled="store.syncing" @click="rePair">
          重新配对
        </button>
        <button class="btn btn-danger" type="button" :disabled="store.syncing || !store.paired" @click="disconnect">
          解除配对
        </button>
      </div>
    </section>

    <section class="card card-pad" style="margin-top: 12px">
      <h2 style="margin: 0 0 2px; font-size: 15px">同步偏好</h2>
      <div class="settings-row">
        <div>
          <strong>仅 Wi-Fi 同步</strong>
          <span>移动网络下不自动同步，也不会按需下载查看图。</span>
        </div>
        <button
          class="switch"
          :class="{ on: settings.wifiOnly }"
          type="button"
          role="switch"
          :aria-checked="settings.wifiOnly"
          aria-label="仅 Wi-Fi 同步"
          @click="updateSettings({ wifiOnly: !settings.wifiOnly })"
        ></button>
      </div>
      <div class="settings-row">
        <div>
          <strong>查看图缓存上限</strong>
          <span>超出后按最近最少使用清理查看图，不会删除元数据和缩略图。</span>
        </div>
        <select class="select" style="width: 128px" :value="settings.cacheLimitBytes" @change="changeCacheLimit">
          <option v-for="option in cacheOptions" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </select>
      </div>
      <div class="settings-row">
        <div>
          <strong>缓存占用</strong>
          <span>
            合计 {{ formatBytes(store.cacheBytes) }} · 缩略图 {{ formatBytes(store.thumbnailBytes) }} · 查看图 {{ formatBytes(store.displayBytes) }}
          </span>
        </div>
      </div>
      <button class="btn btn-secondary btn-block" type="button" @click="clearViewCache">
        清理查看图缓存
      </button>
      <p class="small muted" style="margin: 9px 0 0; line-height: 1.6">
        清理缓存只删除已下载的查看图，保留全部元数据、缩略图和已配对的离线内容。
      </p>
    </section>

    <section class="card card-pad" style="margin-top: 12px">
      <h2 style="margin: 0 0 2px; font-size: 15px">关于手机端</h2>
      <div class="settings-row">
        <div>
          <strong>只读浏览</strong>
          <span>手机端不提供新增、编辑、删除、导入和图库迁移功能。</span>
        </div>
      </div>
      <div class="settings-row">
        <div>
          <strong>协议版本</strong>
          <span>v1 · 与 GoShootFilm 桌面端局域网同步服务通信</span>
        </div>
      </div>
      <div class="settings-row">
        <div>
          <strong>当前范围</strong>
          <span>第一版不传输 RAW、CR2 或正式 TIFF 原件，也不支持云同步、账号和双向编辑。</span>
        </div>
      </div>
    </section>
  </main>
</template>
