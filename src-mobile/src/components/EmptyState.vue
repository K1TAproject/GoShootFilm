<script setup lang="ts">
defineProps<{
  text: string
  hint?: string
}>()
</script>

<template>
  <div class="empty-state">
    <div>{{ text }}</div>
    <div v-if="hint" class="small muted" style="margin-top: 6px">{{ hint }}</div>
  </div>
</template>
