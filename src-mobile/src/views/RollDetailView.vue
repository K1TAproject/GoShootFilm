<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import PhotoTile from '../components/PhotoTile.vue'
import PhotoViewer from '../components/PhotoViewer.vue'
import EmptyState from '../components/EmptyState.vue'
import type { FilmPhotoEntity, RollEntity } from '../../../src/shared/entities'
import type { PhotoVersion } from '../../../src/shared/sync'
import {
  findEntity,
  photosForRoll,
  syncNow,
  type TypedEntity,
} from '../lib/store'

const route = useRoute()
const roll = computed(() =>
  findEntity<RollEntity>('roll', String(route.params.id ?? '')),
)
const photos = computed(() => (roll.value ? photosForRoll(roll.value.syncId) : []))
const activeVersion = ref<PhotoVersion>('edit')
const selectedPhoto = ref<TypedEntity<FilmPhotoEntity>>()
const viewerOpen = ref(false)

const availableVersions = computed(() => {
  const versions: { value: PhotoVersion; label: string }[] = []
  if (photos.value.some((photo) => photo.data.editAvailable)) {
    versions.push({ value: 'edit', label: '调色图' })
  }
  if (photos.value.some((photo) => photo.data.labAvailable)) {
    versions.push({ value: 'lab', label: '原始扫描' })
  }
  return versions
})

const visiblePhotos = computed(() =>
  photos.value.filter((photo) =>
    activeVersion.value === 'edit' ? photo.data.editAvailable : photo.data.labAvailable,
  ),
)

const viewerVersions = computed(() => {
  const photo = selectedPhoto.value
  if (!photo) return []
  const versions: { value: PhotoVersion; label: string }[] = []
  if (photo.data.editAvailable) versions.push({ value: 'edit', label: '调色图' })
  if (photo.data.labAvailable) versions.push({ value: 'lab', label: '原始扫描' })
  return versions
})

watch(
  availableVersions,
  (versions) => {
    if (versions.length && !versions.some((version) => version.value === activeVersion.value)) {
      activeVersion.value = versions[0]?.value ?? 'edit'
    }
  },
  { immediate: true },
)

function openPhoto(photo: TypedEntity<FilmPhotoEntity>) {
  selectedPhoto.value = photo
  if (activeVersion.value === 'edit' && !photo.data.editAvailable) activeVersion.value = 'lab'
  if (activeVersion.value === 'lab' && !photo.data.labAvailable) activeVersion.value = 'edit'
  viewerOpen.value = true
}

function closeViewer() {
  viewerOpen.value = false
  selectedPhoto.value = undefined
}
</script>

<template>
  <main class="page page-detail">
    <MobileHeader
      :title="roll ? `Roll #${String(roll.data.index).padStart(3, '0')}` : 'Roll'"
      subtitle="胶片拍摄卷"
      back
      @sync="syncNow"
    />
    <SyncNotice />

    <template v-if="roll">
      <section class="card detail-hero">
        <span class="eyebrow">Film roll</span>
        <h1>Roll #{{ String(roll.data.index).padStart(3, '0') }}</h1>
        <p>{{ roll.data.cameraInfo }} · {{ roll.data.filmInfo }}</p>
        <div class="detail-meta">
          <span class="pill">{{ roll.data.shotMonth || '未记录月份' }}</span>
          <span v-if="roll.data.city" class="pill">{{ roll.data.city }}</span>
          <span class="pill">{{ roll.data.photoCount }} 张</span>
        </div>
        <p v-if="roll.data.note" style="margin-top: 10px">{{ roll.data.note }}</p>
      </section>

      <div class="chips" style="margin: 14px 0 10px">
        <button
          v-for="version in availableVersions"
          :key="version.value"
          class="chip"
          :class="{ active: activeVersion === version.value }"
          type="button"
          @click="activeVersion = version.value"
        >
          {{ version.label }}
        </button>
      </div>

      <div v-if="visiblePhotos.length" class="photo-grid">
        <PhotoTile
          v-for="photo in visiblePhotos"
          :key="photo.syncId"
          entity-type="film_photo"
          :sync-id="photo.syncId"
          :version="activeVersion"
          :label="photo.data.displayName || `Frame ${photo.data.frameNumber ?? ''}`"
          :favorite="photo.data.isFavorite"
          @open="openPhoto(photo)"
        />
      </div>
      <EmptyState
        v-else
        text="这个版本还没有照片"
        hint="在电脑端导入照片后同步，或者切换到另一个版本"
      />

      <p class="small muted" style="margin: 14px 2px 0; line-height: 1.6">
        手机端只显示生成的缩略图和查看图，不会下载 RAW、CR2 或正式 TIFF 原件。
      </p>
    </template>

    <EmptyState v-else text="找不到这个拍摄卷" hint="可能已在电脑端删除，请返回列表刷新" />

    <PhotoViewer
      v-if="selectedPhoto"
      :open="viewerOpen"
      entity-type="film_photo"
      :sync-id="selectedPhoto.syncId"
      :version="activeVersion"
      :title="selectedPhoto.data.displayName || `Frame ${selectedPhoto.data.frameNumber ?? ''}`"
      :subtitle="`${roll?.data.filmInfo ?? ''}${selectedPhoto.data.isFavorite ? ' · ★ 收藏' : ''}`"
      :favorite="selectedPhoto.data.isFavorite"
      :versions="viewerVersions"
      @close="closeViewer"
    />
  </main>
</template>
