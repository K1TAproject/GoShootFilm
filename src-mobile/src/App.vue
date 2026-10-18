<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import BottomNav from './components/BottomNav.vue'
import appIcon from './assets/app-icon.png'
import { init, store } from './lib/store'

const route = useRoute()
const showNav = computed(() => !route.meta.hideNav)

onMounted(() => {
  void init()
})
</script>

<template>
  <div class="app-root">
    <div v-if="!store.ready" class="boot-screen">
      <img class="boot-mark" :src="appIcon" alt="" />
      <span>正在打开本地档案…</span>
    </div>
    <template v-else>
      <RouterView />
      <BottomNav v-if="showNav" />
    </template>
  </div>
</template>
