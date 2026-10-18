<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import PhotoTile from '../components/PhotoTile.vue'
import PhotoViewer from '../components/PhotoViewer.vue'
import EmptyState from '../components/EmptyState.vue'
import type { DigitalAlbumEntity, DigitalPhotoEntity } from '../../../src/shared/entities'
import type { PhotoVersion } from '../../../src/shared/sync'
import {
  findEntity,
  photosForAlbum,
  syncNow,
  type TypedEntity,
} from '../lib/store'

const route = useRoute()
const album = computed(() =>
  findEntity<DigitalAlbumEntity>('digital_album', String(route.params.id ?? '')),
)
const photos = computed(() => (album.value ? photosForAlbum(album.value.syncId) : []))
const activeVersion = ref<PhotoVersion>('edit')
const selectedPhoto = ref<TypedEntity<DigitalPhotoEntity>>()
const viewerOpen = ref(false)

const availableVersions = computed(() => {
  const versions: { value: PhotoVersion; label: string }[] = []
  if (photos.value.some((photo) => photo.data.editAvailable)) {
    versions.push({ value: 'edit', label: '调色图' })
  }
  if (photos.value.some((photo) => photo.data.rawAvailable)) {
    versions.push({ value: 'raw', label: '原图预览' })
  }
  return versions
})

const visiblePhotos = computed(() =>
  photos.value.filter((photo) =>
    activeVersion.value === 'edit' ? photo.data.editAvailable : photo.data.rawAvailable,
  ),
)

const viewerVersions = computed(() => {
  const photo = selectedPhoto.value
  if (!photo) return []
  const versions: { value: PhotoVersion; label: string }[] = []
  if (photo.data.editAvailable) versions.push({ value: 'edit', label: '调色图' })
  if (photo.data.rawAvailable) versions.push({ value: 'raw', label: '原图预览' })
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

function openPhoto(photo: TypedEntity<DigitalPhotoEntity>) {
  selectedPhoto.value = photo
  if (activeVersion.value === 'edit' && !photo.data.editAvailable) activeVersion.value = 'raw'
  if (activeVersion.value === 'raw' && !photo.data.rawAvailable) activeVersion.value = 'edit'
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
      :title="album?.data.title ?? 'Album'"
      subtitle="数码相册"
      back
      @sync="syncNow"
    />
    <SyncNotice />

    <template v-if="album">
      <section class="card detail-hero">
        <span class="eyebrow">Digital album</span>
        <h1>{{ album.data.title }}</h1>
        <p>
          {{ album.data.cameraBrand || '未指定相机' }} {{ album.data.cameraModel || '' }}
        </p>
        <div class="detail-meta">
          <span class="pill">{{ album.data.shotDate || '未记录日期' }}</span>
          <span v-if="album.data.city" class="pill">{{ album.data.city }}</span>
          <span class="pill">{{ album.data.photoCount }} 张</span>
          <span class="pill">{{ album.data.editCount }} 调色</span>
        </div>
        <p v-if="album.data.note" style="margin-top: 10px">{{ album.data.note }}</p>
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
          entity-type="digital_photo"
          :sync-id="photo.syncId"
          :version="activeVersion"
          :label="photo.data.pairingKey"
          :favorite="photo.data.isFavorite"
          @open="openPhoto(photo)"
        />
      </div>
      <EmptyState
        v-else
        text="这个版本还没有照片"
        hint="在电脑端导入原图或调色图后同步，或者切换到另一个版本"
      />

      <p class="small muted" style="margin: 14px 2px 0; line-height: 1.6">
        原图预览来自 CR2 内嵌 JPEG、TIFF 预览或压缩副本，不是 RAW 原件。
      </p>
    </template>

    <EmptyState v-else text="找不到这个数码相册" hint="可能已在电脑端删除，请返回列表刷新" />

    <PhotoViewer
      v-if="selectedPhoto"
      :open="viewerOpen"
      entity-type="digital_photo"
      :sync-id="selectedPhoto.syncId"
      :version="activeVersion"
      :title="selectedPhoto.data.pairingKey"
      :subtitle="`${album?.data.title ?? ''}${selectedPhoto.data.isFavorite ? ' · ★ 收藏' : ''}`"
      :favorite="selectedPhoto.data.isFavorite"
      :versions="viewerVersions"
      @close="closeViewer"
    />
  </main>
</template>
