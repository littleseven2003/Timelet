<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Entry } from './types/entry';
import { useEntries } from './composables/useEntries';
import { getSettings } from './api/settings';
import { entryDays, entryDeadline, isExpiredCountdown, sortEntries } from './utils/entries';

const { t, locale } = useI18n();
const { entries, loaded, reload, ensureChangeListener } = useEntries();

// 「显示已过期条目」设置，默认显示
const showExpired = ref(true);

// 分钟级时钟：带时刻条目的展示与排序依赖当前时间，需定时失效缓存
const now = ref(Date.now());
let ticker: ReturnType<typeof setInterval> | undefined;

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

const sorted = computed(() => {
  void now.value;
  const visible = showExpired.value
    ? entries.value
    : entries.value.filter((entry) => !isExpiredCountdown(entry, now.value));
  return sortEntries(visible);
});

// 天数/时刻展示文案：倒计时（今天/剩余/已过期）与正计时
function daysText(entry: Entry): string {
  if (entry.time) return timedText(entry, now.value);
  const days = entryDays(entry);
  if (entry.entryType === 'elapsed') return t('panel.elapsed', { days });
  if (days === 0) return t('panel.today');
  if (days < 0) return t('panel.expired', { days: -days });
  return t('panel.daysLeft', { days });
}

// 带时刻条目按精确间隔展示：天+小时 → 小时+分 → 分钟
function timedText(entry: Entry, nowMs: number): string {
  const diffMinutes = Math.round((entryDeadline(entry).getTime() - nowMs) / 60_000);
  const expired = diffMinutes < 0;
  const abs = Math.abs(diffMinutes);
  const days = Math.floor(abs / 1440);
  const hours = Math.floor((abs % 1440) / 60);
  const minutes = abs % 60;

  if (days > 0) {
    return t(expired ? 'panel.ago.daysHours' : 'panel.left.daysHours', { d: days, h: hours });
  }
  if (hours > 0) {
    return t(expired ? 'panel.ago.hoursMinutes' : 'panel.left.hoursMinutes', { h: hours, m: minutes });
  }
  if (minutes > 0) {
    return t(expired ? 'panel.ago.minutes' : 'panel.left.minutes', { m: minutes });
  }
  return expired ? t('panel.expiredOnly') : t('panel.soon');
}

onMounted(async () => {
  reload();
  ensureChangeListener();
  try {
    showExpired.value = (await getSettings()).showExpired;
  } catch {
    // 读取失败时保持默认显示
  }
  ticker = setInterval(() => {
    now.value = Date.now();
  }, 30_000);
});

onUnmounted(() => clearInterval(ticker));
</script>

<template>
  <aside class="panel">
    <header class="panel__header">
      <span class="panel__date">{{ today.date }}</span>
      <span class="panel__weekday">{{ today.weekday }}</span>
    </header>

    <div v-if="loaded && sorted.length === 0" class="panel__empty">
      <p class="panel__empty-title">{{ t('panel.emptyTitle') }}</p>
      <p class="panel__empty-hint">{{ t('panel.emptyHint') }}</p>
    </div>

    <ul v-else class="panel__list">
      <li v-for="entry in sorted" :key="entry.id" class="panel-item">
        <span class="panel-item__color" :style="{ backgroundColor: entry.color }" />
        <span class="panel-item__name" :title="entry.name">{{ entry.name }}</span>
        <span
          class="panel-item__days"
          :style="{ color: entry.color }"
        >
          {{ daysText(entry) }}
        </span>
      </li>
    </ul>
  </aside>
</template>

<style scoped>
.panel {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: #f6f6f6;
  color: #1a1a1a;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid rgba(0, 0, 0, 0.12);
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

.panel__list {
  list-style: none;
  margin: 0;
  padding: 6px 8px;
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.panel-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 8px;
  border-radius: 8px;
}

.panel-item:hover {
  background-color: rgba(0, 0, 0, 0.04);
}

.panel-item__color {
  width: 4px;
  height: 26px;
  border-radius: 2px;
  flex-shrink: 0;
}

.panel-item__name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.panel-item__days {
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

@media (prefers-color-scheme: dark) {
  .panel {
    background-color: #2b2b2b;
    color: #f0f0f0;
    border-color: rgba(255, 255, 255, 0.14);
  }

  .panel__header {
    border-bottom-color: rgba(255, 255, 255, 0.1);
  }

  .panel-item:hover {
    background-color: rgba(255, 255, 255, 0.06);
  }
}
</style>
