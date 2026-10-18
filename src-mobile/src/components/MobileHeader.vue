<script setup lang="ts">
import { useRouter } from 'vue-router'
import { store } from '../lib/store'

defineProps<{
  title: string
  subtitle?: string
  back?: boolean
  settings?: boolean
}>()

const emit = defineEmits<{
  back: []
  sync: []
}>()

const router = useRouter()

function goBack() {
  emit('back')
  if (window.history.length > 1) router.back()
  else void router.replace('/')
}
</script>

<template>
  <header class="mobile-header">
    <div class="mobile-header-main">
      <button v-if="back" class="header-button" type="button" aria-label="返回" @click="goBack">
        ‹
      </button>
      <div>
        <h1>{{ title }}</h1>
        <p v-if="subtitle">{{ subtitle }}</p>
      </div>
    </div>
    <div class="row">
      <button
        class="header-button"
        type="button"
        aria-label="同步"
        :disabled="store.syncing || !store.paired"
        @click="emit('sync')"
      >
        {{ store.syncing ? '…' : '↻' }}
      </button>
      <button
        v-if="settings"
        class="header-button"
        type="button"
        aria-label="设置"
        @click="router.push('/settings')"
      >
        ⚙
      </button>
    </div>
  </header>
</template>
