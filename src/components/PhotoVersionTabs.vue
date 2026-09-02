<script setup lang="ts">
import type { PhotoVersion } from '../types'

defineProps<{
  modelValue: PhotoVersion
  editCount: number
  labCount: number
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: PhotoVersion): void
}>()
</script>

<template>
  <div class="version-tabs" role="tablist" aria-label="图片版本">
    <button
      type="button"
      role="tab"
      :aria-selected="modelValue === 'edit'"
      :class="{ active: modelValue === 'edit' }"
      @click="emit('update:modelValue', 'edit')"
    >
      调色图 <span>{{ editCount }}</span>
    </button>
    <button
      type="button"
      role="tab"
      :aria-selected="modelValue === 'lab'"
      :class="{ active: modelValue === 'lab' }"
      @click="emit('update:modelValue', 'lab')"
    >
      原始扫描 <span>{{ labCount }}</span>
    </button>
  </div>
</template>

<style scoped>
.version-tabs {
  display: inline-flex;
  gap: 4px;
  border: 1px solid #303846;
  border-radius: 9px;
  background: #0f131b;
  padding: 4px;
}

button {
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: #8f9bad;
  padding: 8px 12px;
  cursor: pointer;
}

button.active {
  background: #283140;
  color: #f8fafc;
}

span {
  margin-left: 4px;
  color: #b9c4d1;
  font-size: 12px;
}
</style>
