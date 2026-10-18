<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import type { SyncEntityType, PhotoVersion } from '../../../src/shared/sync'
import { getThumbnailBlob, store } from '../lib/store'

const props = withDefaults(
  defineProps<{
    entityType: SyncEntityType
    syncId: string
    version: PhotoVersion
    label?: string
    favorite?: boolean
    cover?: boolean
  }>(),
  {
    label: '',
    favorite: false,
    cover: false,
  },
)

defineEmits<{
  open: []
}>()

const url = ref('')
const failed = ref(false)
let loadToken = 0

async function load() {
  const token = ++loadToken
  failed.value = false
  if (url.value) {
    URL.revokeObjectURL(url.value)
    url.value = ''
  }
  try {
    const blob = await getThumbnailBlob(props.entityType, props.syncId, props.version)
    if (token !== loadToken) return
    if (blob) url.value = URL.createObjectURL(blob)
    else failed.value = true
  } catch {
    if (token === loadToken) failed.value = true
  }
}

watch(
  () => [props.entityType, props.syncId, props.version, store.dataRevision] as const,
  () => void load(),
  { immediate: true },
)

onBeforeUnmount(() => {
  loadToken += 1
  if (url.value) URL.revokeObjectURL(url.value)
})
</script>

<template>
  <button
    class="photo-tile"
    :class="{ cover }"
    type="button"
    :aria-label="label || '查看照片'"
    @click="$emit('open')"
  >
    <img v-if="url" :src="url" :alt="label" loading="lazy" decoding="async" />
    <span v-else class="tile-placeholder">{{ failed ? '▦' : '…' }}</span>
    <span v-if="favorite" class="tile-favorite" aria-label="收藏">★</span>
    <span v-if="label" class="tile-label">{{ label }}</span>
  </button>
</template>
