<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DisplayUnit, Entry } from './types/entry';
import { ENTRY_COLORS } from './types/entry';
import { createDraft, useEntries } from './composables/useEntries';
import {
  effectiveDateIso,
  formatEntryText,
  groupForConfig,
  sortEntries,
} from './utils/entries';
import { getSettings, type AppSettings, type ThemeMode } from './api/settings';
import DateTimePicker from './components/DateTimePicker.vue';
import SegmentedControl from './components/SegmentedControl.vue';
import ToggleSwitch from './components/ToggleSwitch.vue';
import EntryTypeSymbol from './components/EntryTypeSymbol.vue';
import SettingsSection from './components/SettingsSection.vue';
import AboutSection from './components/AboutSection.vue';
import appIcon from './assets/app-icon.png';

const { t, locale } = useI18n();
const {
  entries,
  loaded,
  reload,
  upsert,
  remove,
  reorder,
  archive,
  restore,
  duplicate,
  ensureChangeListener,
} = useEntries();

// ---------- 导航与视图 ----------
type NavKey = 'now' | 'countdown' | 'elapsed' | 'archive' | 'settings' | 'about';
const activeNav = ref<NavKey>('now');
const searchQuery = ref('');

const now = ref(Date.now());
let ticker: ReturnType<typeof setInterval> | undefined;

const today = computed(() => {
  const current = new Date(now.value);
  return {
    date: current.toLocaleDateString(locale.value, { month: 'long', day: 'numeric' }),
    weekday: current.toLocaleDateString(locale.value, { weekday: 'long' }),
  };
});

// ---------- 数据视图 ----------
const activeEntries = computed(() => entries.value.filter((entry) => !entry.archived));
const archiveEntries = computed(() =>
  sortEntries(entries.value.filter((entry) => entry.archived)),
);

function searchHit(entry: Entry): boolean {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query) return true;
  return (
    entry.name.toLowerCase().includes(query) || (entry.note ?? '').toLowerCase().includes(query)
  );
}

const navCounts = computed(() => ({
  now: activeEntries.value.length,
  countdown: activeEntries.value.filter((entry) => entry.entryType === 'countdown').length,
  elapsed: activeEntries.value.filter((entry) => entry.entryType === 'elapsed').length,
  archive: entries.value.filter((entry) => entry.archived).length,
}));

// 当前视图条目：导航类型过滤 + 搜索
const viewEntries = computed(() => {
  const base =
    activeNav.value === 'countdown'
      ? activeEntries.value.filter((entry) => entry.entryType === 'countdown')
      : activeNav.value === 'elapsed'
        ? activeEntries.value.filter((entry) => entry.entryType === 'elapsed')
        : activeNav.value === 'archive'
          ? archiveEntries.value
          : activeEntries.value;
  return base.filter(searchHit);
});

// 主列表分组：今天 / 接下来 7 天 / 更远（5.2）
const groups = computed(() => {
  void now.value;
  return groupForConfig(viewEntries.value, now.value);
});

const isPlainView = computed(
  () => activeNav.value === 'archive' || searchQuery.value.trim().length > 0,
);
const flatView = computed(() => sortEntries(viewEntries.value));

const viewCount = computed(() =>
  isPlainView.value
    ? flatView.value.length
    : groups.value.reduce((sum, group) => sum + group.items.length, 0),
);

// 侧栏导航项（此时 / 倒数日 / 正数日 / 归档）
const navItems = computed(() => [
  { key: 'now' as NavKey, label: t('config.nav.now'), count: navCounts.value.now },
  { key: 'countdown' as NavKey, label: t('config.typeCountdown'), count: navCounts.value.countdown },
  { key: 'elapsed' as NavKey, label: t('config.typeElapsed'), count: navCounts.value.elapsed },
  { key: 'archive' as NavKey, label: t('config.nav.archive'), count: navCounts.value.archive },
]);

// 近屿摘要：仅「此时」且未搜索时展示一个置顶倒数日（5.3）
const featured = computed(() => {
  if (activeNav.value !== 'now' || searchQuery.value.trim()) return null;
  const today0 = new Date(now.value).setHours(0, 0, 0, 0);
  const candidates = sortEntries(
    activeEntries.value.filter(
      (entry) =>
        entry.pinned &&
        entry.entryType === 'countdown' &&
        !entry.repeat &&
        new Date(`${entry.date}T00:00:00`).getTime() >= today0,
    ),
  );
  return candidates[0] ?? null;
});

// 时弧进度：起点为记录日期，终点为目标日期（5.7）
const featuredArc = computed(() => {
  const entry = featured.value;
  if (!entry) return null;
  const start = new Date(entry.createdAt);
  start.setHours(0, 0, 0, 0);
  const target = new Date(`${entry.date}T00:00:00`);
  const today0 = new Date(now.value).setHours(0, 0, 0, 0);
  const span = target.getTime() - start.getTime();
  if (span <= 0) return null;
  const progress = Math.min(1, Math.max(0, (today0 - start.getTime()) / span));
  const circumference = 2 * Math.PI * 52;
  return {
    dash: `${circumference * 0.75 * progress} ${circumference}`,
    days: Math.round((target.getTime() - today0) / 86_400_000),
  };
});

const featuredDate = computed(() => {
  if (!featured.value) return '';
  const target = new Date(`${featured.value.date}T00:00:00`);
  return `${target.toLocaleDateString(locale.value, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })} · ${target.toLocaleDateString(locale.value, { weekday: 'short' })}`;
});

// ---------- 主题与设置同步 ----------
function applyTheme(mode?: ThemeMode) {
  const root = document.documentElement;
  if (!mode || mode === 'system') {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = mode;
  }
}

function onSettings(next: AppSettings) {
  applyTheme(next.theme);
}

// ---------- 编辑态 ----------
const editing = ref<Entry | null>(null);
const isNew = ref(false);
const deleteTarget = ref<Entry | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);

const canSave = computed(() => !!editing.value?.name && !!editing.value?.date);

const previewText = computed(() => {
  if (!editing.value) return '';
  return formatEntryText(editing.value, now.value, (key, params) => t(key, params ?? {}));
});

const typeOptions = computed(() => [
  { value: 'countdown', label: t('config.typeCountdown') },
  { value: 'elapsed', label: t('config.typeElapsed') },
]);
const unitOptions = computed(() =>
  (['day', 'week', 'month', 'year'] as DisplayUnit[]).map((unit) => ({
    value: unit,
    label: t(`config.unit.${unit}`),
  })),
);
const repeatOptions = computed(() => [
  { value: 'none', label: t('config.repeat.none') },
  { value: 'daily', label: t('config.repeat.daily') },
  { value: 'workday', label: t('config.repeat.workday') },
]);

const TYPE_DEFAULT_COLOR = { countdown: '#2a9cdb', elapsed: '#368e76' } as const;

function onTypeChange(next: 'countdown' | 'elapsed') {
  if (!editing.value) return;
  const previous = editing.value.entryType;
  if (editing.value.color === TYPE_DEFAULT_COLOR[previous]) {
    editing.value.color = TYPE_DEFAULT_COLOR[next];
  }
  editing.value.entryType = next;
}

function toggleTime(enabled: boolean) {
  if (!editing.value) return;
  editing.value.time = enabled ? (editing.value.time ?? '09:00') : undefined;
  if (!enabled) editing.value.repeat = undefined;
}

const isCustomColor = computed(
  () => editing.value != null && !(ENTRY_COLORS as readonly string[]).includes(editing.value.color),
);

function openCreate() {
  isNew.value = true;
  editing.value = createDraft();
}

function openEdit(entry: Entry) {
  isNew.value = false;
  editing.value = { ...entry };
  deleteTarget.value = null;
}

function cancelEdit() {
  editing.value = null;
  deleteTarget.value = null;
}

async function submit() {
  if (!editing.value || !canSave.value) return;
  const entry = { ...editing.value, updatedAt: new Date().toISOString() };
  await upsert(entry);
  cancelEdit();
}

async function confirmRemove() {
  if (!deleteTarget.value) return;
  const id = deleteTarget.value.id;
  deleteTarget.value = null;
  await remove(id);
  if (editing.value?.id === id) cancelEdit();
}

// ---------- 生命周期与右键菜单 ----------
const ctxMenu = ref<{ x: number; y: number; entry: Entry; archived: boolean } | null>(null);

function openCtxMenu(event: MouseEvent, entry: Entry) {
  const width = 150;
  const height = 190;
  ctxMenu.value = {
    x: Math.min(event.clientX, window.innerWidth - width),
    y: Math.min(event.clientY, window.innerHeight - height),
    entry,
    archived: !!entry.archived,
  };
}

async function togglePinned(entry: Entry) {
  await upsert({ ...entry, pinned: !entry.pinned, updatedAt: new Date().toISOString() });
}

// Esc：先关右键菜单，再关弹窗，再退出编辑态
function onKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  if (ctxMenu.value) {
    ctxMenu.value = null;
  } else if (deleteTarget.value) {
    deleteTarget.value = null;
  } else if (editing.value) {
    cancelEdit();
  }
}

// ---------- 拖拽排序 ----------
const dragId = ref<string | null>(null);
const previewList = ref<Entry[] | null>(null);
const displayList = computed(() => previewList.value ?? flatView.value);

function onDragStart(id: string) {
  dragId.value = id;
}

function onDragEnter(id: string) {
  if (!dragId.value || dragId.value === id) return;
  const base = previewList.value ?? flatView.value;
  const from = base.findIndex((entry) => entry.id === dragId.value);
  const to = base.findIndex((entry) => entry.id === id);
  if (from < 0 || to < 0) return;
  const next = [...base];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved!);
  previewList.value = next;
}

async function onDrop() {
  const list = previewList.value;
  dragId.value = null;
  previewList.value = null;
  if (list) await reorder(list.map((entry) => entry.id));
}

function onDragEnd() {
  dragId.value = null;
  previewList.value = null;
}

// ---------- 自动聚焦 ----------
watch(editing, (value) => {
  if (value) void nextTick(() => nameInput.value?.focus());
});

onMounted(() => {
  ticker = setInterval(() => {
    now.value = Date.now();
  }, 30_000);
  window.addEventListener('keydown', onKeydown);
});

onBeforeUnmount(() => {
  clearInterval(ticker);
  window.removeEventListener('keydown', onKeydown);
});

// 面板动作：新建或编辑指定条目
type PendingAction = { kind: 'create' } | { kind: 'edit'; id: string };

async function applyPendingAction(action: PendingAction) {
  activeNav.value = 'now';
  searchQuery.value = '';
  if (!loaded.value) await reload();
  if (action.kind === 'create') {
    openCreate();
    return;
  }
  const entry = entries.value.find((item) => item.id === action.id);
  if (entry && !entry.archived) openEdit(entry);
}

onMounted(async () => {
  await reload();
  ensureChangeListener();
  try {
    onSettings(await getSettings());
  } catch {
    /* 读取失败保持默认 */
  }
  listen<AppSettings>('settings-changed', (event) => onSettings(event.payload));
  const pending = await invoke<PendingAction | null>('take_pending_action');
  if (pending) await applyPendingAction(pending);
  listen<PendingAction>('open-entry-action', (event) => {
    void applyPendingAction(event.payload);
  });
});
</script>

<template>
  <div class="config">
    <!-- 整页背景意象（6.8）：海雾 / 远屿 / 潮线，静态低对比 -->
    <div class="atmosphere" aria-hidden="true">
      <svg viewBox="0 0 800 310" preserveAspectRatio="xMaxYMax slice" focusable="false">
        <path
          class="atmosphere__isle-far"
          d="M240 310c72-30 115-60 193-65 91-6 98-70 188-74 78-4 126 29 179 8v131Z"
        />
        <path
          class="atmosphere__isle-near"
          d="M370 310c62-19 108-34 160-31 68-4 80 43 147 48 58 4 92-16 123-6v73Z"
        />
        <path
          class="atmosphere__waterline"
          d="M221 285c83-17 119-59 210-63 91-4 104-71 186-75 88-5 126 30 183 8"
        />
        <path
          class="atmosphere__waterline"
          d="M455 298c57-11 103-10 140-34 63-41 129-17 205-40"
          opacity=".65"
        />
        <path
          class="atmosphere__waterline"
          d="M45 289c39-5 68-3 94-9m-56 22c40-3 62-5 89-12"
          opacity=".55"
        />
      </svg>
    </div>

    <div class="shell">
      <!-- 侧栏 -->
      <aside class="sidebar">
        <div class="brand">
          <img class="brand__icon" :src="appIcon" alt="" />
          <div>
            <div class="brand__name">时屿</div>
            <div class="brand__wordmark">TIMELET</div>
          </div>
        </div>

        <nav class="nav">
          <button
            v-for="item in navItems"
            :key="item.key"
            class="nav__item"
            :class="{ 'nav__item--active': activeNav === item.key }"
            type="button"
            @click="activeNav = item.key"
          >
            {{ item.label }}
            <small>{{ item.count }}</small>
          </button>
        </nav>

        <div class="nav nav--low">
          <button
            class="nav__item"
            :class="{ 'nav__item--active': activeNav === 'settings' }"
            type="button"
            @click="activeNav = 'settings'"
          >
            {{ t('config.nav.settings') }}
          </button>
          <button
            class="nav__item"
            :class="{ 'nav__item--active': activeNav === 'about' }"
            type="button"
            @click="activeNav = 'about'"
          >
            {{ t('config.nav.about') }}
          </button>
        </div>
      </aside>

      <!-- 内容区 -->
      <main class="main">
        <!-- 条目视图（此时 / 倒数日 / 正数日 / 归档） -->
        <template v-if="activeNav !== 'settings' && activeNav !== 'about'">
          <!-- 编辑表单：单列紧凑（5.5） -->
          <form v-if="editing" class="editor" @submit.prevent="submit">
            <div class="editor__title-row">
              <h2 class="editor__title">
                {{ isNew ? t('config.addEntry') : t('config.editEntry') }}
              </h2>
              <button
                v-if="!isNew"
                class="editor__delete"
                type="button"
                @click="deleteTarget = editing"
              >
                {{ t('config.delete') }}
              </button>
            </div>

            <div class="form-row form-row--type">
              <span class="form-row__label">{{ t('config.fieldType') }}</span>
              <SegmentedControl
                :model-value="editing.entryType"
                :options="typeOptions"
                class="form-row__seg"
                @update:model-value="onTypeChange($event as 'countdown' | 'elapsed')"
              />
            </div>

            <div class="form-row form-row--column">
              <span class="form-row__label">{{ t('config.fieldName') }}</span>
              <input
                ref="nameInput"
                v-model="editing.name"
                class="form-input form-input--hero"
                type="text"
                :placeholder="t('config.namePlaceholder')"
                maxlength="30"
              />
            </div>

            <div class="form-row form-row--column">
              <span class="form-row__label">
                {{
                  editing.entryType === 'countdown'
                    ? t('config.targetDate')
                    : t('config.startDate')
                }}
              </span>
              <DateTimePicker
                :date="editing.date"
                :time="editing.time ?? null"
                :with-time="!!editing.time"
                :past="editing.entryType === 'elapsed'"
                @update:date="editing.date = $event"
                @update:time="editing.time = $event"
              />
            </div>

            <div class="form-row">
              <span class="form-row__label">{{ t('config.includeTime') }}</span>
              <ToggleSwitch
                :model-value="!!editing.time"
                @update:model-value="toggleTime($event)"
              />
            </div>

            <div v-if="editing.time" class="form-row">
              <span class="form-row__label">{{ t('config.fieldRepeat') }}</span>
              <SegmentedControl
                :model-value="editing.repeat ?? 'none'"
                :options="repeatOptions"
                @update:model-value="
                  editing.repeat = $event === 'none' ? undefined : ($event as 'daily' | 'workday')
                "
              />
            </div>

            <div v-else class="form-row">
              <span class="form-row__label">{{ t('config.fieldUnit') }}</span>
              <SegmentedControl
                :model-value="editing.displayUnit ?? 'day'"
                :options="unitOptions"
                @update:model-value="editing.displayUnit = $event as DisplayUnit"
              />
            </div>

            <!-- 紧凑语义预览：类型 · 标题 · 相对时间 · 确切日期（5.5），承载在轻屿面上 -->
            <div class="semantic-preview">
              <EntryTypeSymbol :type="editing.entryType" class="semantic-preview__symbol" />
              <div class="semantic-preview__main">
                <span class="semantic-preview__name">
                  {{ editing.name || t('config.previewName') }}
                </span>
                <span class="semantic-preview__text">
                  {{ previewText || t('config.previewDays') }}
                  <template v-if="editing.date"> · {{ effectiveDateIso(editing) }}</template>
                </span>
              </div>
            </div>

            <details class="more" :open="!!(editing.note || editing.pinned)">
              <summary class="more__summary">{{ t('config.showMore') }}</summary>
              <div class="form-row form-row--column">
                <span class="form-row__label">{{ t('config.fieldColor') }}</span>
                <div class="color-list">
                  <button
                    v-for="color in ENTRY_COLORS"
                    :key="color"
                    type="button"
                    class="color-dot"
                    :class="{ 'color-dot--active': editing.color === color }"
                    :style="{ backgroundColor: color }"
                    :aria-label="color"
                    @click="editing.color = color"
                  />
                  <label
                    class="color-dot color-dot--custom"
                    :class="{ 'color-dot--active': isCustomColor }"
                    :style="{ backgroundColor: editing.color }"
                    :title="t('config.customColor')"
                  >
                    <input
                      type="color"
                      :value="editing.color"
                      @input="editing.color = ($event.target as HTMLInputElement).value"
                    />
                  </label>
                </div>
              </div>
              <div class="form-row">
                <span class="form-row__label">{{ t('config.fieldPinned') }}</span>
                <ToggleSwitch v-model="editing.pinned" />
              </div>
              <div class="form-row form-row--column">
                <span class="form-row__label">{{ t('config.fieldNote') }}</span>
                <textarea
                  v-model="editing.note"
                  class="form-input form-input--note"
                  :placeholder="t('config.notePlaceholder')"
                  maxlength="100"
                />
              </div>
            </details>

            <div class="editor__footer">
              <span class="editor__helper" :class="{ 'editor__helper--active': !canSave }">
                {{ canSave ? '' : t('config.saveHint') }}
              </span>
              <button class="btn" type="button" @click="cancelEdit">
                {{ t('config.cancel') }}
              </button>
              <button class="btn btn--primary" type="submit" :disabled="!canSave">
                {{ isNew ? t('config.createEntry') : t('config.saveChanges') }}
              </button>
            </div>
          </form>

          <!-- 列表视图 -->
          <template v-else>
            <header class="page-head">
              <div>
                <h2 class="page-head__title">
                  {{ activeNav === 'now' ? t('config.nav.now') : t(`config.nav.${activeNav}`) }}
                </h2>
                <div class="page-head__date">{{ today.date }} · {{ today.weekday }}</div>
              </div>
              <button class="btn btn--primary" type="button" @click="openCreate">
                {{ t('config.addEntry') }}
              </button>
            </header>

            <!-- 近屿摘要：仅此时、未搜索（5.3） -->
            <div v-if="featured" class="feature">
              <svg
                class="feature__surface"
                viewBox="0 0 400 47"
                preserveAspectRatio="none"
                aria-hidden="true"
              >
                <path d="M0 47 L0 34 Q60 22 130 30 T260 26 T400 24 L400 47 Z" fill="currentColor" />
              </svg>
              <div class="feature__copy">
                <span class="feature__label">{{ t('config.featuredLabel') }}</span>
                <h3 class="feature__title">{{ featured.name }}</h3>
                <span class="feature__date">{{ featuredDate }}</span>
              </div>
              <svg v-if="featuredArc" class="feature__arc" viewBox="0 0 120 110" fill="none">
                <circle
                  cx="60"
                  cy="60"
                  r="52"
                  stroke="var(--ts-line)"
                  stroke-width="5"
                  stroke-dasharray="245 327"
                  stroke-linecap="round"
                  transform="rotate(135 60 60)"
                />
                <circle
                  cx="60"
                  cy="60"
                  r="52"
                  stroke="currentColor"
                  stroke-width="5"
                  stroke-linecap="round"
                  :stroke-dasharray="featuredArc.dash"
                  transform="rotate(135 60 60)"
                />
              </svg>
              <div v-if="featuredArc" class="feature__days" :style="{ color: featured.color }">
                {{ featuredArc.days }}
                <small>{{ t('config.unit.day') }}</small>
              </div>
            </div>

            <!-- 搜索 -->
            <input
              v-model="searchQuery"
              class="search"
              type="search"
              :placeholder="t('config.searchPlaceholder')"
            />

            <div v-if="loaded && viewCount === 0" class="list-empty">
              <svg class="list-empty__art" viewBox="0 0 120 44" fill="none" aria-hidden="true">
                <path
                  d="M40 32 Q60 10 80 32"
                  stroke="var(--ts-brand)"
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
              <p>
                {{
                  searchQuery.trim()
                    ? t('config.searchEmpty')
                    : activeNav === 'archive'
                      ? t('config.archiveEmpty')
                      : t('config.emptyList')
                }}
              </p>
            </div>

            <!-- 拖拽中：平铺预览 -->
            <ul v-if="dragId" class="entry-list">
              <li
                v-for="entry in displayList"
                :key="entry.id"
                class="entry-item"
                :class="{ 'entry-item--dragging': entry.id === dragId }"
                draggable="true"
                @dragstart="onDragStart(entry.id)"
                @dragenter="onDragEnter(entry.id)"
                @dragover.prevent
                @drop.prevent="onDrop"
                @dragend="onDragEnd"
                @contextmenu.prevent="openCtxMenu($event, entry)"
              >
                <EntryTypeSymbol :type="entry.entryType" class="entry-item__symbol" />
                <div class="entry-item__info">
                  <span class="entry-item__name">
                    {{ entry.name }}
                    <span v-if="entry.pinned" class="entry-item__pin">{{
                      t('config.pinnedTag')
                    }}</span>
                  </span>
                  <span class="entry-item__meta">
                    {{
                      entry.entryType === 'countdown'
                        ? t('config.typeCountdown')
                        : t('config.typeElapsed')
                    }}
                    · {{ effectiveDateIso(entry) }}
                  </span>
                </div>
                <span class="entry-item__days" :style="{ color: entry.color }">
                  {{ formatEntryText(entry, now, (key, params) => t(key, params ?? {})) }}
                </span>
                <div class="entry-item__actions" @mousedown.stop>
                  <button class="btn btn--small" type="button" @click="openEdit(entry)">
                    {{ t('config.edit') }}
                  </button>
                  <button
                    class="btn btn--small btn--danger"
                    type="button"
                    @click="deleteTarget = entry"
                  >
                    {{ t('config.delete') }}
                  </button>
                </div>
              </li>
            </ul>

            <!-- 分组列表 -->
            <template v-else>
              <section v-for="group in groups" :key="group.key" class="entry-group">
                <h3 class="entry-group__title">
                  {{ t(`config.groups.${group.key}`) }}
                </h3>
                <ul class="entry-list">
                  <li
                    v-for="entry in group.items"
                    :key="entry.id"
                    class="entry-item"
                    draggable="true"
                    @dragstart="onDragStart(entry.id)"
                    @contextmenu.prevent="openCtxMenu($event, entry)"
                  >
                    <EntryTypeSymbol :type="entry.entryType" class="entry-item__symbol" />
                    <div class="entry-item__info">
                      <span class="entry-item__name">
                        {{ entry.name }}
                        <span v-if="entry.pinned" class="entry-item__pin">{{
                          t('config.pinnedTag')
                        }}</span>
                      </span>
                      <span class="entry-item__meta">
                        {{
                          entry.entryType === 'countdown'
                            ? t('config.typeCountdown')
                            : t('config.typeElapsed')
                        }}
                        · {{ effectiveDateIso(entry) }}
                      </span>
                    </div>
                    <span class="entry-item__days" :style="{ color: entry.color }">
                      {{ formatEntryText(entry, now, (key, params) => t(key, params ?? {})) }}
                    </span>
                    <div class="entry-item__actions" @mousedown.stop>
                      <button class="btn btn--small" type="button" @click="openEdit(entry)">
                        {{ t('config.edit') }}
                      </button>
                      <button
                        class="btn btn--small btn--danger"
                        type="button"
                        @click="deleteTarget = entry"
                      >
                        {{ t('config.delete') }}
                      </button>
                    </div>
                  </li>
                </ul>
              </section>
            </template>

            <footer class="list-foot">
              {{ viewCount }} {{ t('config.entriesUnit') }}
            </footer>
          </template>
        </template>

        <!-- 通用设置 -->
        <template v-else-if="activeNav === 'settings'">
          <header class="page-head">
            <h2 class="page-head__title">{{ t('config.nav.settings') }}</h2>
          </header>
          <SettingsSection />
        </template>

        <!-- 关于时屿 -->
        <template v-else>
          <header class="page-head">
            <h2 class="page-head__title">{{ t('config.nav.about') }}</h2>
          </header>
          <AboutSection />
        </template>
      </main>
    </div>

    <!-- 列表行右键菜单 -->
    <div
      v-if="ctxMenu"
      class="ctx-overlay"
      @click="ctxMenu = null"
      @contextmenu.prevent="ctxMenu = null"
    >
      <div class="ctx-menu" :style="{ left: `${ctxMenu.x}px`, top: `${ctxMenu.y}px` }">
        <button
          class="ctx-menu__item"
          type="button"
          @click="
            ctxMenu.archived ? restore(ctxMenu.entry.id) : openEdit(ctxMenu.entry);
            ctxMenu = null;
          "
        >
          {{ ctxMenu.archived ? t('config.restore') : t('config.edit') }}
        </button>
        <button
          v-if="!ctxMenu.archived"
          class="ctx-menu__item"
          type="button"
          @click="
            duplicate(ctxMenu.entry.id);
            ctxMenu = null;
          "
        >
          {{ t('config.duplicate') }}
        </button>
        <button
          v-if="!ctxMenu.archived"
          class="ctx-menu__item"
          type="button"
          @click="
            togglePinned(ctxMenu.entry);
            ctxMenu = null;
          "
        >
          {{ ctxMenu.entry.pinned ? t('config.unpin') : t('config.pinIt') }}
        </button>
        <button
          v-if="!ctxMenu.archived"
          class="ctx-menu__item"
          type="button"
          @click="
            archive(ctxMenu.entry.id);
            ctxMenu = null;
          "
        >
          {{ t('config.archiveAction') }}
        </button>
        <button
          v-else
          class="ctx-menu__item ctx-menu__item--danger"
          type="button"
          @click="
            deleteTarget = ctxMenu.entry;
            ctxMenu = null;
          "
        >
          {{ t('config.delete') }}
        </button>
      </div>
    </div>

    <!-- 删除确认弹窗 -->
    <div v-if="deleteTarget" class="modal-overlay" @click.self="deleteTarget = null">
      <div class="modal">
        <h3 class="modal__title">{{ t('config.deleteConfirmTitle') }}</h3>
        <p class="modal__text">{{ t('config.deleteConfirmText', { name: deleteTarget.name }) }}</p>
        <div class="modal__actions">
          <button class="btn" type="button" @click="deleteTarget = null">
            {{ t('config.cancel') }}
          </button>
          <button class="btn btn--danger-solid" type="button" @click="confirmRemove">
            {{ t('config.delete') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config {
  position: relative;
  display: flex;
  min-height: 100vh;
  background-color: var(--ts-surface);
  color: var(--ts-text);
  isolation: isolate;
}

/* 整页背景意象（6.8）：限定窗口边界，无指针事件 */
.atmosphere {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
  z-index: 0;
}

.atmosphere svg {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 100%;
  height: auto;
  opacity: 0.6;
}

.atmosphere__isle-far {
  fill: var(--ts-isle-far);
  opacity: 0.38;
}

.atmosphere__isle-near {
  fill: var(--ts-isle-near);
  opacity: 0.23;
}

.atmosphere__waterline {
  stroke: var(--ts-current);
  fill: none;
  stroke-width: 1.4;
  opacity: 0.27;
}

.shell {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: 146px minmax(0, 1fr);
  width: 100%;
}

/* ---------- 侧栏 ---------- */
.sidebar {
  background-color: var(--ts-rail);
  border-right: 1px solid var(--ts-line);
  padding: 25px 12px 18px;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.brand {
  display: flex;
  gap: 9px;
  align-items: center;
  margin: 0 8px 26px;
}

.brand__icon {
  width: 33px;
  height: 33px;
}

.brand__name {
  font-weight: 500;
  letter-spacing: 0.16em;
  font-size: 17px;
}

.brand__wordmark {
  font-size: 9px;
  letter-spacing: 0.2em;
  color: var(--ts-text-2);
  margin-top: 1px;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.nav--low {
  margin-top: auto;
  padding-top: 30px;
}

.nav__item {
  border: 0;
  display: flex;
  align-items: center;
  gap: 9px;
  background: transparent;
  padding: 9px 10px;
  border-radius: 9px;
  color: var(--ts-text-2);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}

.nav__item small {
  margin-left: auto;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  opacity: 0.8;
}

.nav__item:hover {
  background-color: var(--ts-hover);
}

.nav__item--active {
  background-color: var(--ts-surface);
  color: var(--ts-blue);
  box-shadow: 0 1px 3px var(--ts-shadow);
}

/* ---------- 主内容 ---------- */
.main {
  padding: 23px 25px 18px;
  min-width: 0;
  overflow-y: auto;
}

.page-head {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.page-head__title {
  font-size: 22px;
  font-weight: 500;
  letter-spacing: 0.02em;
  margin: 0;
}

.page-head__date {
  color: var(--ts-text-2);
  font-size: 12px;
  margin-top: 3px;
}

/* ---------- 按钮 ---------- */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--ts-line);
  border-radius: 8px;
  background-color: var(--ts-surface);
  padding: 7px 11px;
  font-size: 12px;
  white-space: nowrap;
  cursor: pointer;
  color: inherit;
}

.btn:hover {
  background-color: var(--ts-hover);
}

.btn--primary {
  background-color: var(--ts-button);
  color: var(--ts-on-button);
  border-color: transparent;
}

.btn--primary:hover {
  filter: brightness(1.06);
  background-color: var(--ts-button);
}

.btn--small {
  padding: 4px 9px;
}

.btn--danger {
  color: var(--ts-coral);
}

.btn--danger-solid {
  background-color: var(--ts-coral);
  border-color: var(--ts-coral);
  color: #fff;
}

.btn--danger-solid:hover {
  background-color: var(--ts-coral);
  filter: brightness(1.06);
}

/* ---------- 近屿摘要 ---------- */
.feature {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 151px;
  align-items: center;
  min-height: 167px;
  margin-bottom: 20px;
  background-color: var(--ts-focus);
  border-radius: 12px 12px 28px 12px;
  overflow: hidden;
  padding: 19px 15px 18px 20px;
}

.feature__surface {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 47px;
  color: var(--ts-island);
  pointer-events: none;
}

.feature__copy {
  position: relative;
  z-index: 1;
  min-width: 0;
}

.feature__label {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--ts-blue);
  font-size: 11px;
  margin-bottom: 11px;
  letter-spacing: 0.03em;
}

.feature__title {
  font-size: 20px;
  font-weight: 500;
  letter-spacing: 0.025em;
  margin: 0;
  overflow-wrap: anywhere;
}

.feature__date {
  font-size: 12px;
  color: var(--ts-text-2);
  margin-top: 5px;
  display: block;
}

.feature__arc {
  position: relative;
  z-index: 1;
  width: 144px;
  height: 134px;
  margin: -20px -6px -30px 0;
  color: var(--ts-blue);
}

.feature__days {
  position: relative;
  z-index: 1;
  font-size: 40px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.feature__days small {
  font-size: 14px;
  font-weight: 400;
  color: var(--ts-text-2);
  margin-left: 3px;
}

/* ---------- 搜索 ---------- */
.search {
  width: 100%;
  border: 1px solid var(--ts-line);
  border-radius: 8px;
  background-color: var(--ts-surface);
  color: inherit;
  font-size: 13px;
  padding: 8px 12px;
  margin-bottom: 14px;
}

.search:focus {
  outline: 2px solid var(--ts-brand);
  outline-offset: -1px;
}

/* ---------- 列表 ---------- */
.entry-group + .entry-group {
  margin-top: 16px;
}

.entry-group__title {
  font-size: 12px;
  font-weight: 600;
  color: var(--ts-text-2);
  margin: 0 0 8px;
}

.entry-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.entry-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 9px;
  cursor: grab;
}

.entry-item:hover {
  background-color: var(--ts-hover);
}

.entry-item--dragging {
  opacity: 0.5;
}

.entry-item__symbol {
  flex-shrink: 0;
  opacity: 0.7;
}

.entry-item__info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.entry-item__name {
  font-size: 14px;
  font-weight: 500;
}

.entry-item__pin {
  font-size: 11px;
  color: var(--ts-blue);
  margin-left: 6px;
}

.entry-item__meta {
  font-size: 12px;
  color: var(--ts-text-2);
}

.entry-item__days {
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.entry-item__actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.15s ease-out;
}

.entry-item:hover .entry-item__actions,
.entry-item:focus-within .entry-item__actions {
  opacity: 1;
}

.list-foot {
  margin-top: 16px;
  text-align: center;
  font-size: 12px;
  color: var(--ts-text-2);
}

.list-empty {
  text-align: center;
  padding: 40px 0;
  font-size: 13px;
  color: var(--ts-text-2);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.list-empty p {
  margin: 0;
}

.list-empty__art {
  width: 120px;
  height: 44px;
}

/* ---------- 编辑表单（单列紧凑，5.5） ---------- */
.editor {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 560px;
}

.editor__title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.editor__title {
  font-size: 22px;
  font-weight: 500;
  margin: 0;
}

.editor__date {
  font-size: 12px;
  color: var(--ts-text-2);
  margin-bottom: 10px;
}

.editor__delete {
  border: none;
  background: none;
  font-size: 12px;
  color: var(--ts-coral);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
}

.editor__delete:hover {
  background-color: rgba(183, 70, 80, 0.1);
}

.form-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.form-row--column {
  flex-direction: column;
  align-items: stretch;
}

.form-row__label {
  font-size: 13px;
  color: var(--ts-text-2);
}

.form-row--type {
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
}

.form-row--type .form-row {
  justify-content: flex-start;
}

.form-row__seg {
  max-width: 300px;
}

.form-input {
  border: 1px solid var(--ts-line);
  border-radius: 8px;
  padding: 8px 11px;
  font-size: 13px;
  background-color: var(--ts-surface);
  color: inherit;
}

.form-input--hero {
  font-size: 15px;
  font-weight: 500;
  padding: 10px 12px;
}

.form-input:focus {
  outline: 2px solid var(--ts-brand);
  outline-offset: -1px;
}

.form-input--note {
  resize: none;
  font-family: inherit;
  line-height: 1.5;
}

/* 紧凑语义预览（轻屿面） */
.semantic-preview {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  background-color: var(--ts-focus);
  border-radius: 10px 10px 22px 10px;
}

.semantic-preview__symbol {
  color: var(--ts-text-2);
}

.semantic-preview__main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.semantic-preview__name {
  font-size: 14px;
  font-weight: 500;
}

.semantic-preview__text {
  font-size: 12px;
  color: var(--ts-text-2);
}

/* 更多选项折叠 */
.more {
  border-top: 1px solid var(--ts-line);
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.more__summary {
  font-size: 12px;
  color: var(--ts-text-2);
  cursor: pointer;
  align-self: flex-start;
}

.more__summary:hover {
  color: var(--ts-text);
}

.more[open] .more__summary {
  margin-bottom: 2px;
}

.more[open] > summary::after {
  content: ' ▴';
}

.more > summary::after {
  content: ' ▾';
}

.color-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.color-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
}

.color-dot--active {
  border-color: var(--ts-text);
}

.color-dot--custom {
  position: relative;
  border-style: dashed;
  border-color: var(--ts-line);
}

.color-dot--custom input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
}

.editor__footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid var(--ts-line);
}

.editor__helper {
  flex: 1;
  font-size: 12px;
  color: var(--ts-text-2);
}

.editor__helper--active {
  color: var(--ts-amber);
  opacity: 0.95;
}

/* ---------- 右键菜单与弹窗 ---------- */
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 90;
}

.ctx-menu {
  position: fixed;
  min-width: 130px;
  padding: 4px;
  border-radius: 9px;
  background-color: var(--ts-surface);
  border: 1px solid var(--ts-line);
  box-shadow: 0 8px 28px var(--ts-shadow);
  display: flex;
  flex-direction: column;
}

.ctx-menu__item {
  border: none;
  background: none;
  text-align: left;
  font-size: 13px;
  color: inherit;
  padding: 7px 12px;
  border-radius: 6px;
  cursor: pointer;
}

.ctx-menu__item:hover {
  background-color: var(--ts-hover);
}

.ctx-menu__item--danger {
  color: var(--ts-coral);
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(23, 35, 45, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  width: 320px;
  padding: 18px 20px;
  border-radius: 12px;
  background-color: var(--ts-surface);
  box-shadow: 0 12px 40px var(--ts-shadow);
}

.modal__title {
  font-size: 15px;
  font-weight: 600;
  margin: 0 0 8px;
}

.modal__text {
  font-size: 13px;
  color: var(--ts-text-2);
  margin: 0 0 16px;
  line-height: 1.5;
}

.modal__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
