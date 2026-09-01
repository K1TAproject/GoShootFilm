<script setup lang="ts">
import { ref } from 'vue'
import Cameras from './views/Cameras.vue'
import Films from './views/Films.vue'
import Rolls from './views/Rolls.vue'

const currentTab = ref<'cameras' | 'films' | 'rolls'>('cameras')
const activeRollId = ref<number | null>(null)

function switchTab(tab: 'cameras' | 'films' | 'rolls') {
  currentTab.value = tab
  if (tab !== 'rolls') {
    activeRollId.value = null
  }
}

function openRollFromRelation(rollId: number) {
  activeRollId.value = rollId
  currentTab.value = 'rolls'
}

function openRollsList() {
  activeRollId.value = null
  currentTab.value = 'rolls'
}
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="brand-block">
        <div class="app-title">GoShootFilm</div>
      </div>
      <nav class="nav-tabs">
        <button :class="{ active: currentTab === 'cameras' }" @click="switchTab('cameras')">Cameras</button>
        <button :class="{ active: currentTab === 'films' }" @click="switchTab('films')">Films</button>
        <button :class="{ active: currentTab === 'rolls' }" @click="openRollsList">Rolls</button>
      </nav>
    </aside>

    <main class="main-content">
      <Cameras v-if="currentTab === 'cameras'" @jump-to-roll="openRollFromRelation" />
      <Films v-else-if="currentTab === 'films'" @jump-to-roll="openRollFromRelation" />
      <Rolls v-else-if="currentTab === 'rolls'" :initialRollId="activeRollId" />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  width: 100%;
  min-width: 0;
  min-height: 100vh;
  display: grid;
  grid-template-columns: 180px minmax(0, 1fr);
  background: #0f1115;
  color: #e5e7eb;
  overflow-x: hidden;
}

.sidebar {
  min-width: 0;
  min-height: 100vh;
  border-right: 1px solid #242833;
  background: #11141a;
  padding: 22px 16px;
}

.brand-block {
  padding: 4px 4px 20px;
  border-bottom: 1px solid #242833;
}

.app-title {
  color: #f9fafb;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0;
}

.nav-tabs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 18px;
}

.nav-tabs button {
  width: 100%;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  padding: 10px 12px;
  text-align: left;
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
}

.nav-tabs button:hover,
.nav-tabs button.active {
  background: #1a1f2a;
  border-color: #2f3746;
  color: #f9fafb;
}

.main-content {
  width: 100%;
  min-width: 0;
  padding: 28px;
  overflow-x: hidden;
}

@media (max-width: 720px) {
  .app-layout {
    grid-template-columns: 1fr;
  }

  .sidebar {
    min-height: auto;
    border-right: 0;
    border-bottom: 1px solid #242833;
  }

  .nav-tabs {
    flex-direction: row;
  }

  .main-content {
    padding: 18px;
  }
}
</style>
