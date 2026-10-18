<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import type { PhotoVersion, SyncEntityType } from '../../../src/shared/sync'
import { getDisplayBlob, markAssetActive } from '../lib/store'

const props = defineProps<{
  open: boolean
  entityType: SyncEntityType
  syncId: string
  version: PhotoVersion
  title: string
  subtitle?: string
  favorite?: boolean
  versions: { value: PhotoVersion; label: string }[]
}>()

const emit = defineEmits<{
  close: []
}>()

const currentVersion = ref<PhotoVersion>(props.version)
const url = ref('')
const loading = ref(false)
const error = ref('')
let activeAssetId = ''
let loadToken = 0

function release() {
  if (url.value) URL.revokeObjectURL(url.value)
  url.value = ''
  if (activeAssetId) {
    markAssetActive(activeAssetId, false)
    activeAssetId = ''
  }
}

async function load() {
  if (!props.open) return
  const token = ++loadToken
  release()
  loading.value = true
  error.value = ''
  try {
    const result = await getDisplayBlob(
      props.entityType,
      props.syncId,
      currentVersion.value,
    )
    if (token !== loadToken) return
    activeAssetId = result.asset.id
    markAssetActive(activeAssetId, true)
    url.value = URL.createObjectURL(result.blob)
  } catch (cause) {
    if (token === loadToken) {
      error.value = cause instanceof Error ? cause.message : '图片加载失败'
    }
  } finally {
    if (token === loadToken) loading.value = false
  }
}

function close() {
  release()
  emit('close')
}

function setVersion(version: PhotoVersion) {
  if (version === currentVersion.value) return
  currentVersion.value = version
}

watch(
  () => props.open,
  (open) => {
    document.body.style.overflow = open ? 'hidden' : ''
    if (open) {
      currentVersion.value = props.version
      void load()
    } else {
      release()
    }
  },
  { immediate: true },
)

watch(currentVersion, () => void load())

onBeforeUnmount(() => {
  release()
  document.body.style.overflow = ''
})
</script>

<template>
  <div v-if="open" class="photo-viewer" role="dialog" aria-modal="true">
    <div class="viewer-head">
      <div>
        <strong>{{ title }} <span v-if="favorite" style="color: #e6c971">★</span></strong>
        <span v-if="subtitle">{{ subtitle }}</span>
      </div>
      <button class="viewer-close" type="button" aria-label="关闭" @click="close">×</button>
    </div>

    <div class="viewer-image">
      <img v-if="url" :src="url" :alt="title" />
      <span v-else-if="loading" class="muted small">正在读取{{ currentVersion === 'edit' ? '调色图' : '原图预览' }}…</span>
      <div v-else-if="error" class="empty-state" style="max-width: 320px">
        <div>{{ error }}</div>
        <button class="btn btn-secondary" type="button" style="margin-top: 10px" @click="load">
          重试
        </button>
      </div>
    </div>

    <div class="viewer-bottom">
      <div v-if="versions.length > 1" class="viewer-versions">
        <button
          v-for="item in versions"
          :key="item.value"
          class="chip"
          :class="{ active: currentVersion === item.value }"
          type="button"
          @click="setVersion(item.value)"
        >
          {{ item.label }}
        </button>
      </div>
      <p class="viewer-note">
        手机端显示的是独立生成的查看图，不是 RAW、CR2 或正式原始文件。
      </p>
    </div>
  </div>
</template>
