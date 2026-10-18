<script setup lang="ts">
import { computed, ref } from 'vue'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import CoverThumb from '../components/CoverThumb.vue'
import EmptyState from '../components/EmptyState.vue'
import { albums, store, syncNow } from '../lib/store'

const search = ref('')
const allAlbums = computed(() => albums())
const filtered = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  if (!keyword) return allAlbums.value
  return allAlbums.value.filter((album) =>
    [album.data.title, album.data.cameraBrand, album.data.cameraModel, album.data.city]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(keyword),
  )
})
</script>

<template>
  <main class="page">
    <MobileHeader
      title="Albums"
      subtitle="数码原图与调色图"
      settings
      @sync="syncNow"
    />
    <SyncNotice />

    <input v-model="search" class="input" type="search" placeholder="搜索相册、机身或地点" />

    <div class="grid-3" style="margin: 14px 0">
      <div class="stat-card">
        <span>数码相册</span>
        <strong>{{ store.stats.digitalAlbumCount }}</strong>
      </div>
      <div class="stat-card">
        <span>数码照片</span>
        <strong>{{ store.stats.digitalPhotoCount }}</strong>
      </div>
      <div class="stat-card">
        <span>收藏</span>
        <strong>{{ store.stats.favoritePhotoCount }}</strong>
      </div>
    </div>

    <div v-if="filtered.length" class="stack">
      <RouterLink
        v-for="album in filtered"
        :key="album.syncId"
        class="list-card"
        :to="`/albums/${album.syncId}`"
      >
        <div v-if="album.data.coverPhotoSyncId" class="cover-thumb">
          <CoverThumb
            entity-type="digital_photo"
            :sync-id="album.data.coverPhotoSyncId"
            :version="album.data.coverVersion === 'edit' ? 'edit' : 'raw'"
            :alt="album.data.title"
          />
        </div>
        <div v-else class="cover-thumb" style="display: grid; place-items: center; color: var(--muted-2)">▧</div>
        <div class="list-main">
          <strong>{{ album.data.title }}</strong>
          <p>{{ album.data.cameraBrand || '未指定相机' }} {{ album.data.cameraModel || '' }}</p>
          <p>
            {{ album.data.shotDate || '未记录日期' }}
            <template v-if="album.data.city"> · {{ album.data.city }}</template>
          </p>
        </div>
        <div class="list-aside">
          <div>{{ album.data.photoCount }} 张</div>
          <div class="small muted">
            原图 {{ album.data.rawCount }} · 调色 {{ album.data.editCount }}
          </div>
        </div>
      </RouterLink>
    </div>
    <EmptyState v-else text="没有符合条件的数码相册" hint="在电脑端 Albums 中导入照片后同步" />
  </main>
</template>
