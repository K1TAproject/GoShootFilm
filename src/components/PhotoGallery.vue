<script setup lang="ts">
import { computed, onUnmounted, type ObjectDirective } from 'vue'
import type { LabPreviewState, Photo, PhotoVersion } from '../types'
import { photoImageUrl } from '../utils/photos'

const props = defineProps<{
  photos: Photo[]
  version: PhotoVersion
  previews: Record<string, LabPreviewState>
  imageErrors: Record<string, string>
  busy: boolean
}>()

const emit = defineEmits<{
  (event: 'view', photo: Photo): void
  (event: 'favorite', photo: Photo): void
  (event: 'delete', photoId: number): void
  (event: 'image-error', photo: Photo, version: PhotoVersion): void
  (event: 'request-preview', photo: Photo, version: PhotoVersion): void
  (event: 'retry-preview', photo: Photo, version: PhotoVersion): void
}>()

const sortedPhotos = computed(() => props.photos
  .filter(photo => props.version === 'edit' ? photo.editScanPath : photo.labScanPath)
  .sort((a, b) => (a.frameNumber ?? 1000) - (b.frameNumber ?? 1000) || a.id - b.id))

function previewKey(photo: Photo, version = props.version) {
  return `${version}:${photo.id}`
}

function previewFor(photo: Photo) {
  return props.previews[previewKey(photo)]
}

function sourceFor(photo: Photo) {
  return previewFor(photo)?.previewPath
}

function errorFor(photo: Photo) {
  return previewFor(photo)?.error || props.imageErrors[previewKey(photo)]
}

interface PreviewRequest {
  photo: Photo
  version: PhotoVersion
}

const observedRequests = new WeakMap<Element, PreviewRequest>()
const previewObserver = typeof IntersectionObserver === 'undefined'
  ? null
  : new IntersectionObserver(entries => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue
        const request = observedRequests.get(entry.target)
        if (!request) continue
        requestPreview(request)
        previewObserver?.unobserve(entry.target)
      }
    }, { rootMargin: '120px 0px' })

function requestPreview(request: PreviewRequest) {
  const key = previewKey(request.photo, request.version)
  const state = props.previews[key]
  if (state?.loading || state?.previewPath || state?.error) return
  emit('request-preview', request.photo, request.version)
}

const vRequestPreview: ObjectDirective<HTMLElement, PreviewRequest> = {
  mounted(element, binding) {
    observedRequests.set(element, binding.value)
    if (previewObserver) previewObserver.observe(element)
    else requestPreview(binding.value)
  },
  updated(element, binding) {
    observedRequests.set(element, binding.value)
    const previous = binding.oldValue
    const key = previewKey(binding.value.photo, binding.value.version)
    if (previous && previewKey(previous.photo, previous.version) === key && props.previews[key]) return
    if (previewObserver) previewObserver.observe(element)
    else requestPreview(binding.value)
  },
  unmounted(element) {
    previewObserver?.unobserve(element)
    observedRequests.delete(element)
  },
}

onUnmounted(() => previewObserver?.disconnect())

</script>

<template>
  <div v-if="sortedPhotos.length === 0" class="empty-state">当前版本还没有可显示的影像。</div>
  <div v-else class="photo-grid">
    <article
      v-for="photo in sortedPhotos"
      :key="photo.id"
      v-request-preview="{ photo, version }"
      class="photo-card"
    >
      <div class="photo-frame">
        <button
          v-if="sourceFor(photo) && !errorFor(photo)"
          type="button"
          class="image-button"
          :aria-label="`打开 Frame ${photo.frameNumber || '?'} ${version === 'lab' ? '原始扫描预览' : '调色图'}大图`"
          @click="emit('view', photo)"
        >
          <img
            :src="photoImageUrl(sourceFor(photo))"
            :alt="`Frame ${photo.frameNumber || '?'} ${version === 'lab' ? '原始扫描预览' : '调色图'}`"
            loading="lazy"
            decoding="async"
            fetchpriority="low"
            @error="emit('image-error', photo, version)"
          />
        </button>

        <div v-else-if="previewFor(photo)?.loading" class="photo-state">
          <span class="spinner"></span>
          <strong>正在准备影像缩略图</strong>
          <small>正式图库图片不会被修改</small>
        </div>

        <div v-else-if="errorFor(photo)" class="photo-state error-state">
          <strong>图片无法读取</strong>
          <small>{{ errorFor(photo) }}</small>
          <button type="button" @click="emit('retry-preview', photo, version)">重试预览</button>
        </div>

        <div v-else class="photo-state">
          <strong>等待加载缩略图</strong>
          <small>滚动到此处时加载缩略图</small>
        </div>

        <button
          class="photo-action delete-action"
          type="button"
          :disabled="busy"
          aria-label="删除照片记录"
          title="删除记录"
          @click.stop="emit('delete', photo.id)"
        >
          ×
        </button>
        <button
          class="photo-action favorite-action"
          type="button"
          :class="{ active: photo.isFavorite }"
          :aria-label="photo.isFavorite ? '取消收藏' : '收藏照片'"
          :title="photo.isFavorite ? '取消收藏' : '收藏照片'"
          @click.stop="emit('favorite', photo)"
        >
          {{ photo.isFavorite ? '★' : '☆' }}
        </button>
      </div>

      <footer class="photo-footer">
        <strong>Frame {{ photo.frameNumber || '?' }}</strong>
      </footer>
    </article>
  </div>
</template>

<style scoped>
.photo-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  gap: 14px;
}

.photo-card {
  min-width: 0;
  overflow: visible;
  border: 1px solid #29313d;
  border-radius: 9px;
  background: #10151c;
  content-visibility: auto;
  contain-intrinsic-size: 260px;
}

.photo-frame {
  position: relative;
  aspect-ratio: 4 / 3;
  overflow: hidden;
  border-radius: 8px 8px 0 0;
  background: #0b0f15;
}

.image-button {
  width: 100%;
  height: 100%;
  display: block;
  border: 0;
  background: transparent;
  padding: 0;
  cursor: zoom-in;
}

.image-button img {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: cover;
}

.photo-state {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 7px;
  padding: 18px;
  color: #b7c0cd;
  text-align: center;
}

.photo-state strong { font-size: 13px; }
.photo-state small { color: #7e8998; line-height: 1.4; }
.error-state { color: #fecaca; }
.error-state small { color: #f1a8ad; overflow-wrap: anywhere; }
.error-state button { padding: 5px 9px; font-size: 12px; }

.spinner {
  width: 22px;
  height: 22px;
  border: 2px solid #394454;
  border-top-color: #d3dbe5;
  border-radius: 999px;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.photo-action {
  position: absolute;
  top: 8px;
  width: 30px;
  height: 30px;
  opacity: 0;
  pointer-events: none;
  border: 1px solid #465165;
  border-radius: 999px;
  background: rgba(12, 17, 24, 0.88);
  color: #e5e7eb;
  cursor: pointer;
  transition: opacity 140ms ease, background-color 140ms ease;
}

.photo-card:hover .photo-action,
.photo-card:focus-within .photo-action {
  opacity: 1;
  pointer-events: auto;
}

.delete-action { left: 8px; font-size: 21px; line-height: 1; }
.delete-action:hover { background: rgba(127, 29, 29, 0.92); }
.favorite-action { right: 8px; }
.favorite-action.active { color: #f5d0fe; }

.photo-footer {
  position: relative;
  min-height: 42px;
  display: flex;
  align-items: center;
  padding: 10px 11px;
}

.photo-footer strong { display: block; color: #dfe5ec; font-size: 12px; }
.empty-state { color: #8f9bad; font-size: 13px; }

@media (hover: none) {
  .photo-action {
    opacity: 1;
    pointer-events: auto;
  }
}
</style>
