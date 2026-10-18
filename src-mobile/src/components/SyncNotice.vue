<script setup lang="ts">
import { store, syncNow } from '../lib/store'
</script>

<template>
  <div v-if="store.syncError" class="feedback-error sync-banner" role="alert">
    <span>{{ store.syncError }}</span>
    <button type="button" :disabled="store.syncing" @click="syncNow">重试</button>
  </div>
  <div v-else-if="store.syncing" class="feedback-info sync-banner" role="status">
    <span>正在同步…</span>
  </div>
  <div v-else-if="!store.online" class="feedback-info sync-banner" role="status">
    <span>离线模式：当前显示本地缓存</span>
  </div>
  <div v-else-if="store.syncNotice" class="feedback-info sync-banner" role="status">
    <span>{{ store.syncNotice }}</span>
  </div>
</template>
