<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()

const items = [
  { path: '/', key: 'home', label: '首页', icon: '⌂' },
  { path: '/equipment', key: 'equipment', label: '器材', icon: '◉' },
  { path: '/films', key: 'films', label: 'Films', icon: '▤' },
  { path: '/rolls', key: 'rolls', label: 'Rolls', icon: '◎' },
  { path: '/albums', key: 'albums', label: 'Albums', icon: '▧' },
]

const activeKey = computed(() => {
  if (route.path.startsWith('/equipment')) return 'equipment'
  if (route.path.startsWith('/films')) return 'films'
  if (route.path.startsWith('/rolls')) return 'rolls'
  if (route.path.startsWith('/albums')) return 'albums'
  return 'home'
})
</script>

<template>
  <nav class="bottom-nav" aria-label="主导航">
    <RouterLink
      v-for="item in items"
      :key="item.key"
      :to="item.path"
      :class="{ active: activeKey === item.key }"
    >
      <span class="nav-icon" aria-hidden="true">{{ item.icon }}</span>
      <span>{{ item.label }}</span>
    </RouterLink>
  </nav>
</template>
