<script setup lang="ts">
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
</script>

<template>
  <main class="placeholder">
    <h1>Timelet（{{ t('app.name') }}）</h1>
    <p>{{ t('app.tagline') }}</p>
  </main>
</template>

<style scoped>
.placeholder {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

h1 {
  font-size: 24px;
  font-weight: 600;
}
</style>
