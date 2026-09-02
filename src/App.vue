<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

function openRollFromRelation(rollId: number) {
  void router.push({ name: 'rolls', query: { roll: String(rollId) } })
}
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <RouterLink class="brand-block" to="/">
        <div class="app-mark">G</div>
        <div>
          <div class="app-title">GoShootFilm</div>
          <div class="app-caption">Film archive</div>
        </div>
      </RouterLink>
      <nav class="nav-tabs">
        <RouterLink to="/" active-class="" exact-active-class="router-link-exact-active">Home</RouterLink>
        <RouterLink to="/cameras">Cameras</RouterLink>
        <RouterLink to="/films">Films</RouterLink>
        <RouterLink to="/rolls">Rolls</RouterLink>
      </nav>
    </aside>

    <main class="main-content">
      <RouterView v-slot="{ Component }">
        <component :is="Component" @jump-to-roll="openRollFromRelation" />
      </RouterView>
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
  background: #10141a;
  padding: 22px 16px;
}

.brand-block {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 4px 20px;
  border-bottom: 1px solid #242833;
  text-decoration: none;
}

.app-mark {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  background: #e5e7eb;
  color: #111827;
  font-weight: 800;
}

.app-title {
  color: #f9fafb;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0;
}

.app-caption {
  margin-top: 2px;
  color: #6f7b8c;
  font-size: 11px;
}

.nav-tabs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 18px;
}

.nav-tabs a {
  width: 100%;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  padding: 10px 12px;
  text-align: left;
  text-decoration: none;
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
}

.nav-tabs a:hover,
.nav-tabs a.router-link-exact-active {
  background: #1a1f2a;
  border-color: #2f3746;
  color: #f9fafb;
}

.main-content {
  width: 100%;
  min-width: 0;
  padding: clamp(20px, 4vw, 44px);
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
