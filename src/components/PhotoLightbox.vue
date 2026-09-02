<script setup lang="ts">
import type { PhotoVersion } from '../types'

defineProps<{
  open: boolean
  source?: string
  frameNumber?: number
  version: PhotoVersion
}>()

const emit = defineEmits<{
  (event: 'close'): void
  (event: 'image-error'): void
}>()
</script>

<template>
  <div v-if="open" class="lightbox-overlay" @click.self="emit('close')">
    <header>
      <div>
        <strong>Frame {{ frameNumber || '?' }}</strong>
        <span>{{ version === 'lab' ? '原始扫描预览（非原 TIFF）' : '调色图' }}</span>
      </div>
      <button class="close-btn" aria-label="关闭大图" @click="emit('close')">×</button>
    </header>
    <div class="lightbox-content">
      <img v-if="source" :src="source" :alt="`Frame ${frameNumber || '?'} 大图`" @error="emit('image-error')" />
    </div>
  </div>
</template>

<style scoped>
.lightbox-overlay {
  position: fixed;
  inset: 0;
  z-index: 10010;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  background: rgba(2, 5, 9, 0.94);
}

header {
  display: flex;
  align-items: center;
  gap: 14px;
  border-bottom: 1px solid #2a323e;
  padding: 13px 18px;
}

header > div:first-child { margin-right: auto; }
header strong { display: block; color: #f8fafc; }
header span { display: block; margin-top: 3px; color: #929eae; font-size: 11px; }
button { border: 1px solid #3a4555; border-radius: 6px; background: #1b222c; color: #d6dde6; padding: 7px 10px; cursor: pointer; }
.close-btn { border: 0; background: transparent; font-size: 24px; }
.lightbox-content {
  min-width: 0;
  min-height: 0;
  display: grid;
  place-items: center;
  overflow: hidden;
  padding: 18px;
}

img {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: block;
  object-fit: contain;
}

@media (max-width: 760px) {
  header { align-items: flex-start; flex-wrap: wrap; }
}
</style>
