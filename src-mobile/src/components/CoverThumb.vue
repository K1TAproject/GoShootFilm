<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import type { PhotoVersion, SyncEntityType } from '../../../src/shared/sync'
import { getThumbnailBlob, store } from '../lib/store'

const props = defineProps<{
  entityType: SyncEntityType
  syncId: string
  version: PhotoVersion
  alt?: string
}>()

const url = ref('')
let loadToken = 0

async function load() {
  const token = ++loadToken
  if (url.value) {
    URL.revokeObjectURL(url.value)
    url.value = ''
  }
  try {
    const blob = await getThumbnailBlob(props.entityType, props.syncId, props.version)
    if (token !== loadToken) return
    if (blob) url.value = URL.createObjectURL(blob)
  } catch {
    // Keep the placeholder; a later sync can refresh it.
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
  <div class="cover-thumb">
    <img v-if="url" :src="url" :alt="alt ?? ''" loading="lazy" decoding="async" />
    <span v-else class="muted-2 small" style="display: grid; place-items: center; height: 100%">▦</span>
  </div>
</template>
