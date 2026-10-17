<script setup lang="ts">
withDefaults(defineProps<{
  title: string
  subtitle?: string
  showBack?: boolean
}>(), {
  subtitle: '',
  showBack: false,
})

defineEmits<{
  (event: 'back'): void
}>()
</script>

<template>
  <button v-if="showBack" class="back-button" type="button" @click="$emit('back')">← 返回</button>
  <header class="page-heading">
    <div>
      <h1>{{ title }}</h1>
      <p v-if="subtitle">{{ subtitle }}</p>
    </div>
    <div class="heading-actions">
      <slot />
    </div>
  </header>
</template>

<style scoped>
.page-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
}

.back-button {
  position: sticky;
  z-index: 20;
  top: 12px;
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  min-height: 38px;
  padding: 7px 14px;
  border: 1px solid rgba(148, 163, 184, 0.24);
  border-radius: 8px;
  background: rgba(15, 23, 42, 0.92);
  color: #93a4bd;
  font-size: 14px;
  font-weight: 600;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
  backdrop-filter: blur(12px);
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease, color 0.16s ease;
}

.back-button:hover {
  border-color: rgba(139, 123, 255, 0.5);
  background: rgba(139, 123, 255, 0.1);
  color: #f8fafc;
}

.back-button:focus-visible {
  outline: 2px solid rgba(95, 216, 198, 0.75);
  outline-offset: 2px;
}

h1 {
  margin: 0;
  color: #f8fafc;
  font-size: clamp(24px, 3vw, 32px);
  letter-spacing: -0.02em;
}

p {
  margin: 8px 0 0;
  color: #8f9bad;
  font-size: 14px;
}

.heading-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
}

@media (max-width: 640px) {
  .page-heading {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
