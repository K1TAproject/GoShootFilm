<script setup lang="ts">
import { computed } from 'vue'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import CoverThumb from '../components/CoverThumb.vue'
import EmptyState from '../components/EmptyState.vue'
import { albums, formatSyncTime, rolls, store, syncNow } from '../lib/store'

const recentRolls = computed(() => rolls().slice(0, 3))
const recentAlbums = computed(() => albums().slice(0, 3))
const filmProgress = computed(() => {
  if (!store.stats.filmCount) return 0
  return Math.round((store.stats.shotFilmCount / store.stats.filmCount) * 100)
})

</script>

<template>
  <main class="page">
    <MobileHeader
      title="GoShootFilm"
      subtitle="本地档案 · 手机只读"
      settings
      @sync="syncNow"
    />
    <SyncNotice />

    <div v-if="!store.paired" class="card card-pad stack" style="margin-bottom: 14px">
      <div class="spread">
        <div>
          <strong style="font-size: 14px">尚未与电脑配对</strong>
          <p class="small muted" style="margin: 4px 0 0">
            配对后即可同步元数据和缩略图，或在离线时浏览已缓存内容。
          </p>
        </div>
        <RouterLink class="btn btn-primary" to="/pair">去配对</RouterLink>
      </div>
    </div>

    <div class="grid-2">
      <div class="stat-card">
        <span>器材总数</span>
        <strong>{{ store.stats.equipmentCount }}</strong>
        <small>机身、镜头与其他器材</small>
      </div>
      <div class="stat-card">
        <span>已拍摄卷</span>
        <strong>{{ store.stats.rollCount }}</strong>
        <small>全部 Rolls</small>
      </div>
      <div class="stat-card">
        <span>数码相册</span>
        <strong>{{ store.stats.digitalAlbumCount }}</strong>
        <small>手机只读浏览</small>
      </div>
      <div class="stat-card">
        <span>已归档照片</span>
        <strong>{{ store.stats.archivedPhotoCount }}</strong>
        <small>胶片与数码记录</small>
      </div>
    </div>

    <section class="card card-pad" style="margin-top: 12px">
      <div class="spread">
        <div>
          <span class="eyebrow">Film progress</span>
          <strong style="display: block; margin-top: 4px">
            {{ store.stats.shotFilmCount }} / {{ store.stats.filmCount }} 种胶卷
          </strong>
        </div>
        <b style="color: var(--accent)">{{ filmProgress }}%</b>
      </div>
      <div
        style="height: 6px; margin-top: 10px; overflow: hidden; border-radius: 99px; background: #0d1219"
      >
        <span
          :style="{ display: 'block', height: '100%', width: `${filmProgress}%`, background: '#b8a46f' }"
        ></span>
      </div>
      <p class="small muted" style="margin: 9px 0 0">
        收藏 {{ store.stats.favoritePhotoCount }} 张 · 上次同步 {{ formatSyncTime(store.lastSyncAt) }}
      </p>
    </section>

    <section class="section-title" style="margin-top: 18px">
      <h2>最近拍摄卷</h2>
      <RouterLink to="/rolls">全部</RouterLink>
    </section>
    <div v-if="recentRolls.length" class="stack" style="margin-top: 8px">
      <RouterLink
        v-for="roll in recentRolls"
        :key="roll.syncId"
        class="list-card"
        :to="`/rolls/${roll.syncId}`"
      >
        <div v-if="roll.data.coverPhotoSyncId" class="cover-thumb">
          <CoverThumb
            entity-type="film_photo"
            :sync-id="roll.data.coverPhotoSyncId ?? ''"
            :version="roll.data.coverVersion === 'edit' ? 'edit' : 'lab'"
            :alt="`Roll ${roll.data.index}`"
          />
        </div>
        <div v-else class="cover-thumb" style="display: grid; place-items: center; color: var(--muted-2)">◎</div>
        <div class="list-main">
          <strong>Roll #{{ String(roll.data.index).padStart(3, '0') }}</strong>
          <p>{{ roll.data.cameraInfo }} · {{ roll.data.filmInfo }}</p>
          <p>{{ roll.data.shotMonth || '未记录月份' }}{{ roll.data.city ? ` · ${roll.data.city}` : '' }}</p>
        </div>
        <div class="list-aside">{{ roll.data.photoCount }} 张</div>
      </RouterLink>
    </div>
    <EmptyState v-else text="还没有拍摄卷" hint="在电脑端添加 Rolls 后同步到手机" />

    <section class="section-title" style="margin-top: 18px">
      <h2>最近数码相册</h2>
      <RouterLink to="/albums">全部</RouterLink>
    </section>
    <div v-if="recentAlbums.length" class="stack" style="margin-top: 8px">
      <RouterLink
        v-for="album in recentAlbums"
        :key="album.syncId"
        class="list-card"
        :to="`/albums/${album.syncId}`"
      >
        <div v-if="album.data.coverPhotoSyncId" class="cover-thumb">
          <CoverThumb
            entity-type="digital_photo"
            :sync-id="album.data.coverPhotoSyncId ?? ''"
            :version="album.data.coverVersion === 'edit' ? 'edit' : 'raw'"
            :alt="album.data.title"
          />
        </div>
        <div v-else class="cover-thumb" style="display: grid; place-items: center; color: var(--muted-2)">▧</div>
        <div class="list-main">
          <strong>{{ album.data.title }}</strong>
          <p>{{ album.data.cameraBrand || '未指定相机' }} {{ album.data.cameraModel || '' }}</p>
          <p>{{ album.data.shotDate || '未记录日期' }}{{ album.data.city ? ` · ${album.data.city}` : '' }}</p>
        </div>
        <div class="list-aside">{{ album.data.photoCount }} 张</div>
      </RouterLink>
    </div>
    <EmptyState v-else text="还没有数码相册" hint="在电脑端创建 Albums 后同步到手机" />
  </main>
</template>
