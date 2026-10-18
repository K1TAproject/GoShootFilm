<script setup lang="ts">
import { computed, ref } from 'vue'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import EmptyState from '../components/EmptyState.vue'
import type { FilmEntity } from '../../../src/shared/entities'
import {
  filmTitle,
  filmTypeLabel,
  films,
  rollsForFilm,
  store,
  syncNow,
  type TypedEntity,
} from '../lib/store'

const search = ref('')
const activeType = ref('all')
const selected = ref<TypedEntity<FilmEntity>>()

const allFilms = computed(() => films())
const types = computed(() => {
  const values = new Set(allFilms.value.map((film) => film.data.type))
  return ['all', ...values]
})
const filtered = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  return allFilms.value.filter((film) => {
    if (activeType.value !== 'all' && film.data.type !== activeType.value) return false
    if (!keyword) return true
    return `${film.data.brand} ${film.data.name}`.toLowerCase().includes(keyword)
  })
})
const relatedRolls = computed(() =>
  selected.value ? rollsForFilm(selected.value.syncId) : [],
)

function typeLabel(type: string) {
  return type === 'all' ? '全部' : filmTypeLabel(type)
}

function isShot(filmSyncId: string) {
  return rollsForFilm(filmSyncId).length > 0
}

function statusLabel(shot: boolean) {
  return shot ? '已拍摄' : '未拍摄'
}

function close() {
  selected.value = undefined
}
</script>

<template>
  <main class="page">
    <MobileHeader
      title="Films"
      subtitle="胶卷资料与拍摄状态"
      settings
      @sync="syncNow"
    />
    <SyncNotice />

    <input v-model="search" class="input" type="search" placeholder="搜索品牌或型号" />

    <div class="chips" style="margin-top: 10px">
      <button
        v-for="type in types"
        :key="type"
        class="chip"
        :class="{ active: activeType === type }"
        type="button"
        @click="activeType = type"
      >
        {{ typeLabel(type) }}
      </button>
    </div>

    <div class="grid-3" style="margin: 14px 0">
      <div class="stat-card">
        <span>胶卷种类</span>
        <strong>{{ store.stats.filmCount }}</strong>
      </div>
      <div class="stat-card">
        <span>已拍摄</span>
        <strong>{{ store.stats.shotFilmCount }}</strong>
      </div>
      <div class="stat-card">
        <span>当前结果</span>
        <strong>{{ filtered.length }}</strong>
      </div>
    </div>

    <div v-if="filtered.length" class="stack">
      <button
        v-for="film in filtered"
        :key="film.syncId"
        class="list-card"
        type="button"
        @click="selected = film"
      >
        <div class="cover-thumb" style="display: grid; place-items: center; color: var(--accent)">▤</div>
        <div class="list-main">
          <strong>{{ filmTitle(film.data) }}</strong>
          <p>ISO {{ film.data.iso }} · {{ filmTypeLabel(film.data.type) }}</p>
          <p v-if="film.data.note">{{ film.data.note }}</p>
        </div>
        <span class="pill" :class="{ ok: isShot(film.syncId) }">
          {{ statusLabel(isShot(film.syncId)) }}
        </span>
      </button>
    </div>
    <EmptyState v-else text="没有符合条件的胶卷" hint="可以清除搜索或切换类型筛选" />

    <div v-if="selected" class="modal-overlay" @click.self="close">
      <section class="modal-sheet">
        <div class="spread">
          <div>
            <span class="eyebrow">Film</span>
            <h2>{{ filmTitle(selected.data) }}</h2>
          </div>
          <button class="header-button" type="button" aria-label="关闭" @click="close">×</button>
        </div>
        <dl class="detail-list">
          <div><dt>品牌</dt><dd>{{ selected.data.brand }}</dd></div>
          <div><dt>型号</dt><dd>{{ selected.data.name }}</dd></div>
          <div><dt>ISO</dt><dd>{{ selected.data.iso }}</dd></div>
          <div><dt>类型</dt><dd>{{ filmTypeLabel(selected.data.type) }}</dd></div>
          <div><dt>状态</dt><dd>{{ statusLabel(isShot(selected.syncId)) }}</dd></div>
          <div v-if="selected.data.note"><dt>备注</dt><dd>{{ selected.data.note }}</dd></div>
        </dl>
        <template v-if="relatedRolls.length">
          <h2 class="section-title" style="margin-top: 18px; font-size: 15px">关联拍摄卷</h2>
          <div class="stack" style="margin-top: 8px">
            <RouterLink
              v-for="roll in relatedRolls"
              :key="roll.syncId"
              class="list-card"
              :to="`/rolls/${roll.syncId}`"
              @click="close"
            >
              <div class="list-main">
                <strong>Roll #{{ String(roll.data.index).padStart(3, '0') }}</strong>
                <p>{{ roll.data.cameraInfo }} · {{ roll.data.shotMonth || '未记录月份' }}</p>
              </div>
              <span class="pill">{{ roll.data.photoCount }} 张</span>
            </RouterLink>
          </div>
        </template>
      </section>
    </div>
  </main>
</template>
