<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { LabPreview } from '../types'

const props = defineProps<{ photoId?: number; version?: 'raw' | 'edit' }>()
const source = ref('')
const failed = ref(false)

async function load() {
  source.value = ''
  failed.value = false
  if (!props.photoId || !props.version) return
  try {
    const result = await invoke<LabPreview>('get_digital_photo_preview', {
      photoId: props.photoId,
      version: props.version,
      thumbnail: true,
    })
    source.value = convertFileSrc(result.previewPath)
  } catch {
    failed.value = true
  }
}

watch(() => [props.photoId, props.version], load)
onMounted(load)
</script>

<template>
  <div class="cover">
    <img v-if="source && !failed" :src="source" alt="相册封面" loading="lazy" decoding="async" @error="failed = true" />
    <span v-else>{{ failed ? '封面无法读取' : 'DIGITAL ALBUM' }}</span>
  </div>
</template>

<style scoped>
.cover { aspect-ratio: 3 / 2; display: grid; place-items: center; overflow: hidden; background: #0d1219; color: #596575; font-size: 12px; letter-spacing: .08em; }
img { width: 100%; height: 100%; display: block; object-fit: cover; }
</style>
