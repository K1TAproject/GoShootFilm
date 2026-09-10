<script setup lang="ts">
defineProps<{
  open: boolean
  currentVersion?: string
  nextVersion?: string
  notes?: string
  progress?: number
  status?: string
  error?: string
  busy: boolean
}>()

const emit = defineEmits<{
  (event: 'install'): void
  (event: 'later'): void
  (event: 'retry'): void
}>()
</script>

<template>
  <div v-if="open" class="update-overlay">
    <section class="update-dialog" role="dialog" aria-modal="true" aria-label="软件更新">
      <h2>{{ nextVersion ? '发现新版本' : '检查更新' }}</h2>
      <div v-if="nextVersion" class="version-row">
        <span>当前版本 {{ currentVersion }}</span>
        <strong>新版本 {{ nextVersion }}</strong>
      </div>
      <p v-if="nextVersion" class="notes">
        {{ notes || '本次更新未提供版本说明。' }}
      </p>
      <div v-if="progress !== undefined" class="progress-track" role="progressbar" :aria-valuenow="progress">
        <span :style="{ width: `${progress}%` }"></span>
      </div>
      <p v-if="status" class="update-status">{{ status }}</p>
      <div v-if="error" class="feedback-error" role="alert">{{ error }}</div>
      <div class="update-actions">
        <button v-if="error" type="button" :disabled="busy" @click="emit('retry')">重试</button>
        <button v-else type="button" :disabled="busy" @click="emit('install')">立即更新</button>
        <button type="button" :disabled="busy" @click="emit('later')">稍后提醒</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.update-overlay { position: fixed; z-index: 1100; inset: 0; display: grid; place-items: center; padding: 20px; background: rgba(4, 7, 11, 0.78); }
.update-dialog { width: min(520px, 100%); padding: 22px; border: 1px solid #303846; border-radius: 10px; background: #151922; }
.update-dialog h2 { margin: 0 0 14px; }
.version-row { display: flex; justify-content: space-between; gap: 16px; color: #9ca3af; font-size: 13px; }
.version-row strong { color: #e5e7eb; }
.notes { max-height: 180px; overflow: auto; white-space: pre-wrap; color: #b6c0cc; line-height: 1.55; }
.progress-track { height: 7px; overflow: hidden; border-radius: 999px; background: #28303c; }
.progress-track span { display: block; height: 100%; background: #a78bfa; transition: width 120ms linear; }
.update-status { color: #aeb8c6; font-size: 13px; }
.update-actions { display: flex; justify-content: flex-end; gap: 9px; margin-top: 18px; }
</style>
