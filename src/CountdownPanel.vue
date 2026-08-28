<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

const { t, locale } = useI18n();

// 当天日期与星期，跟随 i18n 语言展示
const today = computed(() => {
  const now = new Date();
  return {
    date: now.toLocaleDateString(locale.value, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    }),
    weekday: now.toLocaleDateString(locale.value, { weekday: 'long' }),
  };
});
</script>

<template>
  <aside class="panel">
    <header class="panel__header">
      <span class="panel__date">{{ today.date }}</span>
      <span class="panel__weekday">{{ today.weekday }}</span>
    </header>

    <div class="panel__empty">
      <p class="panel__empty-title">{{ t('panel.emptyTitle') }}</p>
      <p class="panel__empty-hint">{{ t('panel.emptyHint') }}</p>
    </div>
  </aside>
</template>

<style scoped>
.panel {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: #f6f6f6;
  color: #1a1a1a;
}

.panel__header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 14px 16px 10px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}

.panel__date {
  font-size: 15px;
  font-weight: 600;
}

.panel__weekday {
  font-size: 13px;
  opacity: 0.6;
}

.panel__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 24px;
  text-align: center;
}

.panel__empty-title {
  font-size: 14px;
  font-weight: 500;
}

.panel__empty-hint {
  font-size: 12px;
  opacity: 0.55;
}

@media (prefers-color-scheme: dark) {
  .panel {
    background-color: #2b2b2b;
    color: #f0f0f0;
  }

  .panel__header {
    border-bottom-color: rgba(255, 255, 255, 0.1);
  }
}
</style>
