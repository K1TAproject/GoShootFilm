<script setup lang="ts">
withDefaults(defineProps<{
  title: string
  subtitle?: string
  showHome?: boolean
}>(), {
  subtitle: '',
  showHome: true,
})
</script>

<template>
  <header class="page-heading">
    <div>
      <RouterLink v-if="showHome" class="home-link" to="/">← 首页</RouterLink>
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

.home-link {
  display: inline-block;
  margin-bottom: 8px;
  color: #93a4bd;
  font-size: 13px;
  text-decoration: none;
}

.home-link:hover {
  color: #f8fafc;
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
