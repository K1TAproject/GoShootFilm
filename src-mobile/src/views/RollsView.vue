<script setup lang="ts">
import { computed, ref } from 'vue'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import CoverThumb from '../components/CoverThumb.vue'
import EmptyState from '../components/EmptyState.vue'
import { rolls, store, syncNow } from '../lib/store'

const search = ref('')
const allRolls = computed(() => rolls())
const filtered = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  if (!keyword) return allRolls.value
  return allRolls.value.filter((roll) =>
    [
      roll.data.cameraInfo,
      roll.data.filmInfo,
      roll.data.shotMonth,
      roll.data.city,
      String(roll.data.index),
    ]
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
      title="Rolls"
      subtitle="胶片拍摄卷与照片"
      settings
      @sync="syncNow"
    />
    <SyncNotice />

    <input v-model="search" class="input" type="search" placeholder="搜索卷号、机身、胶卷或地点" />

    <div class="grid-3" style="margin: 14px 0">
      <div class="stat-card">
        <span>拍摄卷</span>
        <strong>{{ store.stats.rollCount }}</strong>
      </div>
      <div class="stat-card">
        <span>胶片照片</span>
        <strong>{{ store.stats.photoCount }}</strong>
      </div>
      <div class="stat-card">
        <span>收藏</span>
        <strong>{{ store.stats.favoritePhotoCount }}</strong>
      </div>
    </div>

    <div v-if="filtered.length" class="stack">
      <RouterLink
        v-for="roll in filtered"
        :key="roll.syncId"
        class="list-card"
        :to="`/rolls/${roll.syncId}`"
      >
        <div v-if="roll.data.coverPhotoSyncId" class="cover-thumb">
          <CoverThumb
            entity-type="film_photo"
            :sync-id="roll.data.coverPhotoSyncId"
            :version="roll.data.coverVersion === 'edit' ? 'edit' : 'lab'"
            :alt="`Roll ${roll.data.index}`"
          />
        </div>
        <div v-else class="cover-thumb" style="display: grid; place-items: center; color: var(--muted-2)">◎</div>
        <div class="list-main">
          <strong>Roll #{{ String(roll.data.index).padStart(3, '0') }}</strong>
          <p>{{ roll.data.cameraInfo }}</p>
          <p>{{ roll.data.filmInfo }}</p>
          <p>
            {{ roll.data.shotMonth || '未记录月份' }}
            <template v-if="roll.data.city"> · {{ roll.data.city }}</template>
          </p>
        </div>
        <div class="list-aside">
          <div>{{ roll.data.photoCount }} 张</div>
          <div v-if="roll.data.coverVersion === 'edit'" class="small muted">有调色</div>
        </div>
      </RouterLink>
    </div>
    <EmptyState v-else text="没有符合条件的拍摄卷" hint="在电脑端 Rolls 中添加或导入照片后同步" />
  </main>
</template>
