<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from '../components/PageHeader.vue'
import StatCard from '../components/StatCard.vue'
import type { DashboardStats } from '../types'
import { errorMessage as formatError } from '../utils/errors'

const stats = ref<DashboardStats | null>(null)
const isLoading = ref(false)
const visibleError = ref('')

const filmProgress = computed(() => {
  if (!stats.value?.filmCount) return 0
  return Math.round((stats.value.shotFilmCount / stats.value.filmCount) * 100)
})

async function fetchDashboard() {
  isLoading.value = true
  visibleError.value = ''
  try {
    stats.value = await invoke<DashboardStats>('get_dashboard_stats')
  } catch (error) {
    visibleError.value = formatError(error, '无法读取首页统计')
  } finally {
    isLoading.value = false
  }
}

onMounted(fetchDashboard)
</script>

<template>
  <section class="home-page">
    <PageHeader
      title="Film Archive"
      subtitle="掌握拍摄进度"
      :show-home="false"
    />

    <div v-if="visibleError" class="feedback-error" role="alert">{{ visibleError }}</div>
    <div v-else-if="isLoading" class="feedback-info">正在汇总档案…</div>

    <div class="feature-grid">
      <RouterLink class="feature-card cameras-feature" to="/cameras">
        <span>设备档案</span>
        <strong>Cameras</strong>
        <small>查看与管理全部相机 →</small>
      </RouterLink>
      <RouterLink class="feature-card films-feature" to="/films">
        <span>胶卷资料</span>
        <strong>Films</strong>
        <small>整理胶卷型号和拍摄状态 →</small>
      </RouterLink>
      <RouterLink class="feature-card rolls-feature" to="/rolls">
        <span>拍摄记录</span>
        <strong>Rolls</strong>
        <small>浏览胶卷与照片 →</small>
      </RouterLink>
    </div>

    <template v-if="stats">
      <div class="stats-grid">
        <StatCard label="相机" :value="stats.cameraCount" hint="已登记设备" />
        <StatCard label="已拍摄卷数" :value="stats.rollCount" hint="全部 Rolls" />
        <StatCard label="已归档照片" :value="stats.photoCount" hint="应用图库中的照片" />
        <StatCard label="收藏照片" :value="stats.favoritePhotoCount" hint="标记为收藏" />
      </div>

      <section class="dashboard-panel progress-panel">
        <div class="panel-heading">
          <div>
            <span>胶卷拍摄进度</span>
            <strong>{{ stats.shotFilmCount }} / {{ stats.filmCount }} 种</strong>
          </div>
          <b>{{ filmProgress }}%</b>
        </div>
        <div class="progress-track" role="progressbar" :aria-valuenow="filmProgress" aria-valuemin="0" aria-valuemax="100">
          <span :style="{ width: `${filmProgress}%` }"></span>
        </div>

      </section>

      <section class="dashboard-panel">
        <div class="panel-heading">
          <div>
            <span>全部相机</span>
            <strong>设备概览</strong>
          </div>
          <RouterLink to="/cameras">管理设备</RouterLink>
        </div>
        <div v-if="stats.cameras.length" class="camera-list">
          <RouterLink v-for="camera in stats.cameras" :key="camera.id" to="/cameras" class="camera-list-item">
            <div>
              <strong>{{ camera.brand }} {{ camera.model }}</strong>
              <span>{{ camera.format || '135' }} 画幅</span>
            </div>
            <small>{{ camera.status === 'active' ? '在用' : '闲置' }}</small>
          </RouterLink>
        </div>
        <div v-else class="empty-state">还没有相机，前往 Cameras 添加第一台设备。</div>
      </section>
    </template>
  </section>
</template>

<style scoped>
.home-page {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.feature-grid,
.stats-grid {
  display: grid;
  gap: 14px;
}

.feature-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.stats-grid {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.feature-card {
  position: relative;
  min-height: 180px;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  overflow: hidden;
  border: 1px solid #2b3441;
  border-radius: 14px;
  padding: 20px;
  color: #f8fafc;
  text-decoration: none;
  transition: transform 0.18s ease, border-color 0.18s ease;
}

.feature-card::before {
  content: '';
  position: absolute;
  width: 150px;
  height: 150px;
  top: -55px;
  right: -30px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.08);
}

.feature-card:hover {
  transform: translateY(-3px);
  border-color: #657084;
}

.cameras-feature { background: linear-gradient(145deg, #293346, #171c25); }
.films-feature { background: linear-gradient(145deg, #3b3125, #1b1917); }
.rolls-feature { background: linear-gradient(145deg, #2b3840, #171d21); }

.feature-card span,
.feature-card small {
  color: #b7c0cd;
}

.feature-card span { font-size: 12px; }
.feature-card strong { margin: 6px 0 12px; font-size: 26px; }
.feature-card small { font-size: 13px; }

.dashboard-panel {
  border: 1px solid #27303c;
  border-radius: 12px;
  background: #141920;
  padding: 20px;
}

.panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.panel-heading span,
.progress-panel p {
  color: #8f9bad;
  font-size: 12px;
}

.panel-heading strong {
  display: block;
  margin-top: 5px;
  color: #f1f5f9;
  font-size: 18px;
}

.panel-heading b {
  color: #d6dde7;
  font-size: 22px;
}

.panel-heading a {
  color: #aebbc9;
  font-size: 13px;
  text-decoration: none;
}

.progress-track {
  height: 9px;
  overflow: hidden;
  margin-top: 18px;
  border-radius: 999px;
  background: #242c37;
}

.progress-track span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #8092a8, #d8e0e8);
}

.progress-panel p { margin: 10px 0 0; }

.camera-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 10px;
  margin-top: 16px;
}

.camera-list-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border: 1px solid #262e39;
  border-radius: 9px;
  background: #10151c;
  padding: 13px;
  color: inherit;
  text-decoration: none;
}

.camera-list-item strong { display: block; color: #e5e7eb; font-size: 14px; }
.camera-list-item span { display: block; margin-top: 5px; color: #7f8a99; font-size: 12px; }
.camera-list-item small { color: #a9b5c3; }

.empty-state { margin-top: 16px; color: #8f9bad; font-size: 13px; }

@media (max-width: 900px) {
  .feature-grid { grid-template-columns: 1fr; }
  .feature-card { min-height: 140px; }
  .stats-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

@media (max-width: 520px) {
  .stats-grid { grid-template-columns: 1fr; }
}
</style>
