<script setup lang="ts">
defineProps<{
  label: string
  value: string | number
  hint?: string
}>()
</script>

<template>
  <article class="stat-card">
    <span>{{ label }}</span>
    <strong>{{ value }}</strong>
    <small v-if="hint">{{ hint }}</small>
  </article>
</template>

<style scoped>
.stat-card {
  min-width: 0;
  border: 1px solid #27303c;
  border-radius: 12px;
  background: linear-gradient(145deg, #171c24, #12161d);
  padding: 18px;
}

span,
small {
  color: #8f9bad;
}

span {
  display: block;
  font-size: 13px;
}

strong {
  display: block;
  margin-top: 10px;
  color: #f8fafc;
  font-size: 28px;
}

small {
  display: block;
  margin-top: 6px;
  font-size: 12px;
}
</style>
