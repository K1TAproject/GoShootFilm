<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { photoImageUrl } from '../utils/photos'

const props = defineProps<{
  rollId: number
  hasCover: boolean
  alt: string
}>()

const root = ref<HTMLElement>()
const source = ref('/film-stocks/placeholder.svg')
let observer: IntersectionObserver | undefined
let requested = false

async function loadCover() {
  if (requested || !props.hasCover) return
  requested = true
  try {
    source.value = photoImageUrl(await invoke<string>('get_roll_cover_preview', { rollId: props.rollId }))
  } catch {
    source.value = '/film-stocks/placeholder.svg'
  }
}

function usePlaceholder() {
  source.value = '/film-stocks/placeholder.svg'
}

onMounted(() => {
  if (!root.value || !('IntersectionObserver' in window)) {
    void loadCover()
    return
  }
  observer = new IntersectionObserver(entries => {
    if (entries.some(entry => entry.isIntersecting)) {
      observer?.disconnect()
      void loadCover()
    }
  }, { rootMargin: '240px' })
  observer.observe(root.value)
})

onBeforeUnmount(() => observer?.disconnect())
</script>

<template>
  <div ref="root" class="roll-cover">
    <img :src="source" :alt="alt" loading="lazy" decoding="async" @error="usePlaceholder" />
  </div>
</template>

<style scoped>
.roll-cover {
  width: 100%;
  height: 166px;
  min-height: 0;
  overflow: hidden;
  border-right: 1px solid #262c38;
  background: #0f131b;
}

img {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: cover;
}
</style>
