<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Entry, EntryType } from './types/entry';
import { useEntries } from './composables/useEntries';
import { getSettings, type AppSettings } from './api/settings';
import {
  effectiveDeadline,
  formatEntryText,
  groupedEntries,
  isExpiredCountdown,
  type EntrySection,
} from './utils/entries';
import EntryTypeSymbol from './components/EntryTypeSymbol.vue';

const { t, locale } = useI18n();
const { entries, loaded, reload, ensureChangeListener } = useEntries();

// 「显示已过期条目」设置，默认显示
const showExpired = ref(true);

// 分钟级时钟：带时刻条目的展示与排序依赖当前时间，需定时失效缓存
const now = ref(Date.now());
let ticker: ReturnType<typeof setInterval> | undefined;

// 品牌头日期：8月30日 · 星期五（依赖共享时钟，跨日自动刷新）
const today = computed(() => {
  const current = new Date(now.value);
  return {
    date: current.toLocaleDateString(locale.value, { month: 'long', day: 'numeric' }),
    weekday: current.toLocaleDateString(locale.value, { weekday: 'long' }),
  };
});

// 面板分区（设计文档 5.1）：此时 / 将至 / 历时 / 已过期
const groups = computed(() => {
  void now.value;
  const visible = showExpired.value
    ? entries.value
    : entries.value.filter((entry) => !isExpiredCountdown(entry, now.value));
  return groupedEntries(visible, now.value);
});

const sectionLabels: Record<EntrySection, string> = {
  now: 'panel.sections.now',
  soon: 'panel.sections.soon',
  elapsed: 'panel.sections.elapsed',
  past: 'panel.sections.past',
};

// 第二行类型标签与确切日期
function typeLabel(type: EntryType): string {
  return type === 'countdown' ? t('config.typeCountdown') : t('config.typeElapsed');
}

function dateLine(entry: Entry): string {
  const target = effectiveDeadline(entry, now.value);
  const date = target.toLocaleDateString(locale.value, { month: 'long', day: 'numeric' });
  return entry.time ? `${date} ${entry.time}` : date;
}

// 已过期条目使用到期珊瑚色（设计语言 6.4），不再沿用条目自选色
function daysColor(entry: Entry): string {
  return isExpiredCountdown(entry, now.value) ? 'var(--ts-coral)' : entry.color;
}

// 右键唤起原生菜单：条目上传入 id（编辑详情/打开主界面），空白处仅打开主界面
function showContextMenu(entryId: string | null) {
  invoke('show_panel_menu', { entryId }).catch(() => {
    /* 菜单唤起失败时静默 */
  });
}

// 悬停标题：备注拼在名称下方展示
function entryTitle(entry: Entry): string {
  return entry.note ? `${entry.name}\n${entry.note}` : entry.name;
}

// 左键单击条目：展开/收起缩略详情（同时只展开一条）
const expandedId = ref<string | null>(null);

function toggleExpanded(id: string) {
  expandedId.value = expandedId.value === id ? null : id;
}

// 面板内直达主界面的动作
function openCreate() {
  invoke('open_main_create').catch(() => {
    /* 静默 */
  });
}

function openMain() {
  invoke('open_main_window').catch(() => {
    /* 静默 */
  });
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
  // 设置保存后同步过期显示等偏好
  listen<AppSettings>('settings-changed', (event) => {
    showExpired.value = event.payload.showExpired;
  });
});

onUnmounted(() => clearInterval(ticker));
</script>

<template>
  <aside class="panel" @contextmenu.prevent="showContextMenu(null)">
    <header class="panel__header">
      <span class="panel__brand">{{ t('panel.brand') }}</span>
      <span class="panel__header-right">
        <span class="panel__date">{{ today.date }} · {{ today.weekday }}</span>
        <button
          class="panel__quick-add"
          type="button"
          :aria-label="t('panel.addEntry')"
          :title="t('panel.addEntry')"
          @click="openCreate"
        >
          ＋
        </button>
      </span>
    </header>

    <div v-if="loaded && groups.length === 0" class="panel__empty">
      <!-- 极简水面与小岛轮廓（设计语言 5.6），不做成场景插画 -->
      <svg class="panel__empty-art" viewBox="0 0 120 44" fill="none" aria-hidden="true">
        <path
          d="M40 32 Q60 10 80 32"
          stroke="var(--ts-primary)"
          stroke-width="2"
          stroke-linecap="round"
          opacity="0.55"
        />
        <path
          d="M6 33 Q34 29 60 33 T114 33"
          stroke="var(--ts-line)"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>
      <p class="panel__empty-title">{{ t('panel.emptyTitle') }}</p>
      <button class="panel__add" type="button" @click="openCreate">
        {{ t('panel.createFirst') }}
      </button>
    </div>

    <template v-else>
      <div class="panel__scroll">
        <section v-for="group in groups" :key="group.key" class="panel-section">
          <h3 class="panel-section__title">{{ t(sectionLabels[group.key]) }}</h3>
          <ul class="panel__list">
            <li
              v-for="entry in group.items"
              :key="entry.id"
              class="panel-item"
              :class="{ 'panel-item--expanded': expandedId === entry.id }"
              @click="toggleExpanded(entry.id)"
              @contextmenu.prevent.stop="showContextMenu(entry.id)"
            >
              <div class="panel-item__row">
                <span class="panel-item__color" :style="{ backgroundColor: entry.color }" />
                <EntryTypeSymbol :type="entry.entryType" class="panel-item__symbol" />
                <span class="panel-item__name" :title="entryTitle(entry)">{{ entry.name }}</span>
                <span class="panel-item__days" :style="{ color: daysColor(entry) }">
                  {{ formatEntryText(entry, now, t) }}
                </span>
              </div>
              <div class="panel-item__meta">
                {{ typeLabel(entry.entryType) }} · {{ dateLine(entry) }}
              </div>

              <!-- 缩略详情：备注与编辑入口 -->
              <div v-if="expandedId === entry.id" class="panel-item__detail" @click.stop>
                <p v-if="entry.note" class="panel-item__detail-note">{{ entry.note }}</p>
                <button
                  class="panel-item__detail-edit"
                  type="button"
                  @click="invoke('open_entry_editor', { id: entry.id })"
                >
                  {{ t('panel.editEntry') }}
                </button>
              </div>
            </li>
          </ul>
        </section>
      </div>

      <button class="panel__footer" type="button" @click="openMain">
        {{ t('panel.viewAll') }}
      </button>
    </template>
  </aside>
</template>

<style scoped>
.panel {
  animation: panel-in 0.18s ease-out;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--ts-bg);
  color: var(--ts-text);
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid var(--ts-line);
}

.panel__header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 12px 16px 10px;
  border-bottom: 1px solid var(--ts-line);
  flex-shrink: 0;
}

.panel__brand {
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: var(--ts-primary-text);
}

.panel__header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.panel__date {
  font-size: 12px;
  opacity: 0.6;
}

.panel__quick-add {
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 50%;
  background-color: rgba(42, 156, 219, 0.12);
  color: var(--ts-primary-text);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
}

.panel__quick-add:hover {
  background-color: rgba(42, 156, 219, 0.22);
}

.panel__scroll {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 8px;
}

.panel-section + .panel-section {
  margin-top: 10px;
}

.panel-section__title {
  font-size: 11px;
  font-weight: 600;
  color: var(--ts-text-2);
  margin: 0;
  padding: 8px 8px 4px;
  letter-spacing: 0.02em;
}

.panel__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.panel-item {
  display: flex;
  flex-direction: column;
  padding: 3px 8px;
  border-radius: 8px;
  cursor: pointer;
}

.panel-item__row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 0;
}

.panel-item:hover,
.panel-item--expanded {
  background-color: rgba(42, 156, 219, 0.07);
}

.panel-item__color {
  width: 4px;
  height: 24px;
  /* 岛屿弧形签名造型 */
  border-radius: 2px 2px 45% 45% / 2px 2px 30% 30%;
  flex-shrink: 0;
}

.panel-item__symbol {
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
  font-size: 13px;
  font-weight: 600;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.panel-item__meta {
  font-size: 11px;
  color: var(--ts-text-2);
  padding: 0 0 4px 21px;
}

.panel-item__detail {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 0 8px 21px;
}

.panel-item__detail-note {
  font-size: 12px;
  color: var(--ts-text-2);
  margin: 0;
  white-space: pre-wrap;
}

.panel-item__detail-edit {
  align-self: flex-start;
  border: none;
  background: none;
  font-size: 12px;
  color: var(--ts-primary-text);
  cursor: pointer;
  padding: 0;
}

.panel-item__detail-edit:hover {
  text-decoration: underline;
}

.panel__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  text-align: center;
}

.panel__empty-art {
  width: 120px;
  height: 44px;
  margin-bottom: 4px;
}

.panel__empty-title {
  font-size: 14px;
  font-weight: 500;
}

.panel__add {
  border: none;
  background: none;
  font-size: 13px;
  color: var(--ts-primary-text);
  cursor: pointer;
  padding: 8px 0;
}

.panel__add:hover {
  text-decoration: underline;
}

.panel__footer {
  border: none;
  border-top: 1px solid var(--ts-line);
  background: none;
  font-size: 13px;
  color: var(--ts-primary-text);
  cursor: pointer;
  padding: 10px 0;
  flex-shrink: 0;
}

.panel__footer:hover {
  background-color: rgba(42, 156, 219, 0.07);
}

@keyframes panel-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .panel {
    animation: none;
  }
}
</style>
