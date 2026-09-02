<script setup lang="ts">
import { computed } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { LabPreviewState, Photo, PhotoVersion } from '../types'

const props = defineProps<{
  photos: Photo[]
  version: PhotoVersion
  previews: Record<number, LabPreviewState>
  imageErrors: Record<string, string>
  busy: boolean
}>()

const emit = defineEmits<{
  (event: 'view', photo: Photo, source: string): void
  (event: 'switch-version', version: PhotoVersion): void
  (event: 'favorite', photo: Photo): void
  (event: 'delete', photoId: number): void
  (event: 'image-error', photo: Photo, version: PhotoVersion): void
  (event: 'open-original', photo: Photo): void
  (event: 'reveal-original', photo: Photo): void
  (event: 'save-original', photo: Photo): void
}>()

const sortedPhotos = computed(() => [...props.photos].sort((a, b) => (a.frameNumber ?? 1000) - (b.frameNumber ?? 1000) || a.id - b.id))

function imageUrl(path?: string) {
  if (!path) return ''
  if (path.startsWith('/')) return path
  return convertFileSrc(path)
}

function sourceFor(photo: Photo) {
  return props.version === 'edit'
    ? photo.editScanPath
    : props.previews[photo.id]?.previewPath
}

function errorFor(photo: Photo) {
  return props.version === 'lab'
    ? props.previews[photo.id]?.error || props.imageErrors[`lab:${photo.id}`]
    : props.imageErrors[`edit:${photo.id}`]
}

function hasVersion(photo: Photo) {
  return props.version === 'edit' ? Boolean(photo.editScanPath) : Boolean(photo.labScanPath)
}
</script>

<template>
  <div v-if="sortedPhotos.length === 0" class="empty-state">这个 Roll 还没有照片记录。</div>
  <div v-else class="photo-grid">
    <article v-for="photo in sortedPhotos" :key="photo.id" class="photo-card">
      <div class="photo-frame">
        <button
          v-if="sourceFor(photo) && !errorFor(photo)"
          type="button"
          class="image-button"
          :aria-label="`打开 Frame ${photo.frameNumber || '?'} ${version === 'lab' ? '原始扫描预览' : '调色图'}大图`"
          @click="emit('view', photo, sourceFor(photo)!)"
        >
          <img
            :src="imageUrl(sourceFor(photo))"
            :alt="`Frame ${photo.frameNumber || '?'} ${version === 'lab' ? '原始扫描预览' : '调色图'}`"
            @error="emit('image-error', photo, version)"
          />
        </button>

        <div v-else-if="version === 'lab' && hasVersion(photo) && previews[photo.id]?.loading" class="photo-state">
          <span class="spinner"></span>
          <strong>正在生成原始扫描预览</strong>
          <small>原件不会被修改</small>
        </div>

        <div v-else-if="errorFor(photo)" class="photo-state error-state">
          <strong>图片无法读取</strong>
          <small>{{ errorFor(photo) }}</small>
        </div>

        <div v-else class="photo-state missing-state">
          <strong>Frame {{ photo.frameNumber || '?' }} 暂无{{ version === 'edit' ? '调色图' : '原始扫描' }}</strong>
          <button
            v-if="version === 'edit' && photo.labScanPath"
            type="button"
            @click="emit('switch-version', 'lab')"
          >
            查看原始扫描
          </button>
          <button
            v-else-if="version === 'lab' && photo.editScanPath"
            type="button"
            @click="emit('switch-version', 'edit')"
          >
            查看调色图
          </button>
        </div>

        <span class="version-badge">{{ version === 'lab' ? '原始扫描预览' : '调色图' }}</span>
        <button
          class="photo-action favorite-action"
          :class="{ active: photo.isFavorite }"
          :aria-label="photo.isFavorite ? '取消收藏' : '收藏照片'"
          @click="emit('favorite', photo)"
        >
          {{ photo.isFavorite ? '★' : '☆' }}
        </button>
      </div>

      <footer class="photo-footer">
        <div>
          <strong>Frame {{ photo.frameNumber || '?' }}</strong>
          <span>{{ version === 'lab' ? (photo.labOriginalName || '原始扫描') : '调色图' }}</span>
        </div>
        <details v-if="version === 'lab' && photo.labScanPath" class="original-menu">
          <summary>原件操作</summary>
          <div>
            <button type="button" @click="emit('open-original', photo)">打开原始 TIFF</button>
            <button type="button" @click="emit('reveal-original', photo)">在文件夹中显示</button>
            <button type="button" @click="emit('save-original', photo)">另存原始 TIFF…</button>
          </div>
        </details>
        <button class="record-delete" type="button" :disabled="busy" @click="emit('delete', photo.id)">删除记录</button>
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
.photo-state button { border: 1px solid #3b4657; border-radius: 6px; background: #202936; color: #d7dee7; padding: 7px 9px; cursor: pointer; }
.error-state { color: #fecaca; }
.error-state small { color: #f1a8ad; overflow-wrap: anywhere; }

.spinner {
  width: 22px;
  height: 22px;
  border: 2px solid #394454;
  border-top-color: #d3dbe5;
  border-radius: 999px;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.version-badge {
  position: absolute;
  left: 8px;
  bottom: 8px;
  border: 1px solid rgba(105, 119, 138, 0.65);
  border-radius: 999px;
  background: rgba(10, 14, 20, 0.84);
  color: #d2dae4;
  padding: 4px 7px;
  font-size: 10px;
}

.photo-action {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 30px;
  height: 30px;
  border: 1px solid #465165;
  border-radius: 999px;
  background: rgba(12, 17, 24, 0.88);
  color: #e5e7eb;
  cursor: pointer;
}

.favorite-action.active { color: #f5d0fe; }

.photo-footer {
  position: relative;
  min-height: 58px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 11px;
}

.photo-footer > div:first-child { min-width: 0; margin-right: auto; }
.photo-footer strong { display: block; color: #dfe5ec; font-size: 12px; }
.photo-footer span { display: block; margin-top: 3px; color: #7e8998; font-size: 10px; }
.record-delete { border: 0; background: transparent; color: #aa747a; padding: 4px; font-size: 11px; cursor: pointer; }

.original-menu { position: relative; }
.original-menu summary { color: #9eacbb; font-size: 11px; cursor: pointer; list-style: none; }
.original-menu summary::-webkit-details-marker { display: none; }
.original-menu > div {
  position: absolute;
  right: 0;
  bottom: calc(100% + 8px);
  width: 158px;
  z-index: 8;
  display: grid;
  gap: 3px;
  border: 1px solid #3a4555;
  border-radius: 8px;
  background: #171d26;
  padding: 5px;
  box-shadow: 0 12px 34px rgba(0, 0, 0, 0.42);
}
.original-menu button { border: 0; border-radius: 5px; background: transparent; color: #d2d8e1; padding: 8px; text-align: left; cursor: pointer; }
.original-menu button:hover { background: #252e3b; }
.empty-state { color: #8f9bad; font-size: 13px; }
</style>
