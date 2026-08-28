<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Entry } from './types/entry';
import { ENTRY_COLORS } from './types/entry';
import { createDraft, useEntries } from './composables/useEntries';
import { formatEntryText, sortEntries } from './utils/entries';
import DateTimePicker from './components/DateTimePicker.vue';
import SettingsSection from './components/SettingsSection.vue';

type NavKey = 'entries' | 'settings' | 'about';

const { t } = useI18n();
const activeNav = ref<NavKey>('entries');
const { entries, loaded, reload, upsert, remove, reorder } = useEntries();

// 编辑态：editing 非空时内容区切换为表单
const editing = ref<Entry | null>(null);
const isNew = ref(false);
// 两步删除确认：记录待确认删除的条目 id
const deletingId = ref<string | null>(null);

const sorted = computed(() => sortEntries(entries.value));
const canSave = computed(() => !!editing.value?.name && !!editing.value?.date);

// 预览卡文案：随名称/类型/日期输入实时变化
const previewText = computed(() => {
  if (!editing.value) return '';
  return formatEntryText(editing.value, Date.now(), (key, params) => t(key, params ?? {}));
});

// 拖拽排序：拖动过程用本地预览列表渲染，落点后一次性持久化
const dragId = ref<string | null>(null);
const previewList = ref<Entry[] | null>(null);
const displayList = computed(() => previewList.value ?? sorted.value);

function onDragStart(id: string) {
  dragId.value = id;
}

function onDragEnter(id: string) {
  if (!dragId.value || dragId.value === id) return;
  const base = previewList.value ?? sorted.value;
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

function openCreate() {
  isNew.value = true;
  editing.value = createDraft();
}

function openEdit(entry: Entry) {
  isNew.value = false;
  editing.value = { ...entry };
  deletingId.value = null;
}

function cancelEdit() {
  editing.value = null;
  deletingId.value = null;
}

// 开关时刻精度：开启时给默认时刻，关闭时移除
function toggleTime(enabled: boolean) {
  if (!editing.value) return;
  editing.value.time = enabled ? (editing.value.time ?? '09:00') : undefined;
}

async function submit() {
  if (!editing.value || !canSave.value) return;
  const entry = { ...editing.value, updatedAt: new Date().toISOString() };
  await upsert(entry);
  cancelEdit();
}

async function confirmRemove() {
  if (!deletingId.value) return;
  const id = deletingId.value;
  deletingId.value = null;
  await remove(id);
  // 从编辑表单内删除时同步关闭表单
  if (editing.value?.id === id) cancelEdit();
}

// 从面板"编辑详情"进入：数据就绪后按 id 打开编辑表单
async function openEditorById(id: string) {
  if (activeNav.value !== 'entries') activeNav.value = 'entries';
  if (!loaded.value) await reload();
  const entry = entries.value.find((item) => item.id === id);
  if (entry) openEdit(entry);
}

onMounted(async () => {
  await reload();
  // 窗口先于面板右键动作创建时，取走暂存的待编辑条目
  const pendingId = await invoke<string | null>('take_pending_edit');
  if (pendingId) await openEditorById(pendingId);
  // 窗口已存在时后续动作通过事件送达
  listen<string>('open-entry-editor', (event) => {
    void openEditorById(event.payload);
  });
});
</script>

<template>
  <div class="config">
    <nav class="config__nav">
      <button
        v-for="key in (['entries', 'settings', 'about'] as NavKey[])"
        :key="key"
        class="config__nav-item"
        :class="{ 'config__nav-item--active': activeNav === key }"
        type="button"
        @click="activeNav = key"
      >
        {{ t(`config.nav.${key}`) }}
      </button>
    </nav>

    <main class="config__main">
      <!-- 条目管理 -->
      <template v-if="activeNav === 'entries'">
        <!-- 编辑表单 -->
        <form v-if="editing" class="entry-form" @submit.prevent="submit">
          <h2 class="entry-form__title">
            {{ isNew ? t('config.addEntry') : t('config.editEntry') }}
          </h2>

          <!-- 实时预览：与面板条目样式一致 -->
          <div class="entry-preview">
            <span class="entry-preview__color" :style="{ backgroundColor: editing.color }" />
            <span class="entry-preview__name">{{ editing.name || t('config.previewName') }}</span>
            <span class="entry-preview__days" :style="{ color: editing.color }">
              {{ previewText || t('config.previewDays') }}
            </span>
          </div>

          <div class="entry-form__body">
            <section class="form-section">
              <h3 class="form-section__title">{{ t('config.sectionBasic') }}</h3>
              <input
                v-model="editing.name"
                class="entry-form__input entry-form__input--hero"
                type="text"
                :placeholder="t('config.namePlaceholder')"
                maxlength="30"
              />
              <div class="entry-form__options">
                <button
                  type="button"
                  class="entry-form__option"
                  :class="{ 'entry-form__option--active': editing.entryType === 'countdown' }"
                  @click="editing.entryType = 'countdown'"
                >
                  {{ t('config.typeCountdown') }}
                </button>
                <button
                  type="button"
                  class="entry-form__option"
                  :class="{ 'entry-form__option--active': editing.entryType === 'elapsed' }"
                  @click="editing.entryType = 'elapsed'"
                >
                  {{ t('config.typeElapsed') }}
                </button>
              </div>
            </section>

            <section class="form-section">
              <h3 class="form-section__title">{{ t('config.sectionTime') }}</h3>
              <span class="entry-form__label">
                {{ editing.entryType === 'countdown' ? t('config.targetDate') : t('config.startDate') }}
              </span>
              <DateTimePicker
                :date="editing.date"
                :time="editing.time ?? null"
                :with-time="!!editing.time"
                @update:date="editing.date = $event"
                @update:time="editing.time = $event"
              />
              <label class="entry-form__toggle">
                <input
                  type="checkbox"
                  :checked="!!editing.time"
                  @change="toggleTime(($event.target as HTMLInputElement).checked)"
                />
                <span>{{ t('config.includeTime') }}</span>
              </label>
            </section>

            <section class="form-section">
              <h3 class="form-section__title">{{ t('config.sectionAppearance') }}</h3>
              <span class="entry-form__label">{{ t('config.fieldColor') }}</span>
              <div class="entry-form__colors">
                <button
                  v-for="color in ENTRY_COLORS"
                  :key="color"
                  type="button"
                  class="entry-form__color"
                  :class="{ 'entry-form__color--active': editing.color === color }"
                  :style="{ backgroundColor: color }"
                  :aria-label="color"
                  @click="editing.color = color"
                />
              </div>
              <label class="entry-form__toggle">
                <input v-model="editing.pinned" type="checkbox" />
                <span>{{ t('config.fieldPinned') }}</span>
              </label>
            </section>
          </div>

          <div class="entry-form__footer">
            <button
              v-if="!isNew"
              class="btn btn--danger"
              type="button"
              @click="deletingId === editing.id ? confirmRemove() : (deletingId = editing.id)"
            >
              {{ deletingId === editing.id ? t('config.confirmDelete') : t('config.delete') }}
            </button>
            <span class="entry-form__spacer" />
            <button class="btn" type="button" @click="cancelEdit">
              {{ t('config.cancel') }}
            </button>
            <button class="btn btn--primary" type="submit" :disabled="!canSave">
              {{ t('config.save') }}
            </button>
          </div>
        </form>

        <!-- 列表 -->
        <template v-else>
          <header class="entry-list__header">
            <h2 class="entry-list__title">{{ t('config.nav.entries') }}</h2>
            <button class="btn btn--primary" type="button" @click="openCreate">
              {{ t('config.addEntry') }}
            </button>
          </header>

          <p v-if="loaded && sorted.length === 0" class="entry-list__empty">
            {{ t('config.emptyList') }}
          </p>

          <ul class="entry-list">
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
            >
              <span class="entry-item__color" :style="{ backgroundColor: entry.color }" />
              <div class="entry-item__info">
                <span class="entry-item__name">
                  {{ entry.name }}
                  <span v-if="entry.pinned" class="entry-item__pin">{{ t('config.pinnedTag') }}</span>
                </span>
                <span class="entry-item__meta">
                  {{ entry.entryType === 'countdown' ? t('config.typeCountdown') : t('config.typeElapsed') }}
                  · {{ entry.date }}
                </span>
              </div>
              <div class="entry-item__actions">
                <button class="btn btn--small" type="button" @click="openEdit(entry)">
                  {{ t('config.edit') }}
                </button>
                <button
                  v-if="deletingId !== entry.id"
                  class="btn btn--small btn--danger"
                  type="button"
                  @click="deletingId = entry.id"
                >
                  {{ t('config.delete') }}
                </button>
                <button
                  v-else
                  class="btn btn--small btn--danger"
                  type="button"
                  @click="confirmRemove"
                >
                  {{ t('config.confirmDelete') }}
                </button>
              </div>
            </li>
          </ul>
        </template>
      </template>

      <!-- 通用设置 -->
      <template v-else-if="activeNav === 'settings'">
        <SettingsSection />
      </template>

      <!-- 关于（M5 实装） -->
      <template v-else>
        <h2 class="config__placeholder-title">{{ t('config.nav.about') }}</h2>
        <p class="config__placeholder">{{ t('config.aboutPlaceholder') }}</p>
      </template>
    </main>
  </div>
</template>

<style scoped>
.config {
  display: flex;
  min-height: 100vh;
  background-color: #f6f6f6;
  color: #1a1a1a;
}

.config__nav {
  width: 132px;
  flex-shrink: 0;
  padding: 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
}

.config__nav-item {
  border: none;
  background: none;
  text-align: left;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  color: inherit;
}

.config__nav-item:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

.config__nav-item--active {
  background-color: rgba(0, 145, 255, 0.12);
  color: #0067c0;
  font-weight: 500;
}

.config__main {
  flex: 1;
  padding: 16px 20px;
  overflow-y: auto;
}

.entry-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.entry-list__title,
.config__placeholder-title,
.entry-form__title {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.entry-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.entry-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  background-color: #fff;
  cursor: grab;
}

.entry-item--dragging {
  opacity: 0.5;
}

.entry-item__color {
  width: 10px;
  height: 32px;
  border-radius: 3px;
  flex-shrink: 0;
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
  color: #0067c0;
  margin-left: 6px;
}

.entry-item__meta {
  font-size: 12px;
  opacity: 0.55;
}

.entry-item__actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.entry-list__empty {
  text-align: center;
  opacity: 0.55;
  font-size: 13px;
  padding: 40px 0;
}

.btn {
  border: 1px solid rgba(0, 0, 0, 0.12);
  background-color: #fff;
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
  color: inherit;
}

.btn:hover {
  border-color: rgba(0, 0, 0, 0.24);
}

.btn:disabled {
  opacity: 0.45;
  cursor: default;
}

.btn--primary {
  background-color: #0067c0;
  border-color: #0067c0;
  color: #fff;
}

.btn--primary:hover:not(:disabled) {
  background-color: #0072d4;
}

.btn--danger {
  color: #d33;
}

.btn--small {
  padding: 4px 10px;
  font-size: 12px;
}

.entry-form {
  max-width: 560px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 实时预览卡：结构对齐面板条目 */
.entry-preview {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background-color: #fff;
}

.entry-preview__color {
  width: 4px;
  height: 28px;
  border-radius: 2px;
  flex-shrink: 0;
}

.entry-preview__name {
  flex: 1;
  min-width: 0;
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-preview__days {
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.entry-form__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background-color: #fff;
}

.form-section__title {
  font-size: 12px;
  font-weight: 600;
  opacity: 0.55;
  margin: 0;
}

.entry-form__input--hero {
  font-size: 16px;
  font-weight: 600;
  padding: 10px 12px;
}

.entry-form__footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
}

.entry-form__spacer {
  flex: 1;
}

.entry-form__field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.entry-form__field--row {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.entry-form__label {
  font-size: 13px;
  opacity: 0.7;
}

.entry-form__include-time,
.entry-form__toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  margin-top: 2px;
  cursor: pointer;
}

.entry-form__input {
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 6px;
  padding: 7px 10px;
  font-size: 13px;
  background-color: #fff;
  color: inherit;
}

.entry-form__options {
  display: flex;
  gap: 8px;
}

.entry-form__option {
  border: 1px solid rgba(0, 0, 0, 0.12);
  background: #fff;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 13px;
  cursor: pointer;
  color: inherit;
}

.entry-form__option--active {
  border-color: #0067c0;
  color: #0067c0;
  background-color: rgba(0, 145, 255, 0.08);
  font-weight: 500;
}

.entry-form__colors {
  display: flex;
  gap: 8px;
}

.entry-form__color {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
}

.entry-form__color--active {
  border-color: rgba(0, 0, 0, 0.5);
}

.entry-form__actions {
  display: flex;
  gap: 8px;
}

.config__placeholder-title {
  margin-bottom: 8px;
}

.config__placeholder {
  font-size: 13px;
  opacity: 0.55;
}

@media (prefers-color-scheme: dark) {
  .config {
    background-color: #262626;
    color: #f0f0f0;
  }

  .config__nav {
    border-right-color: rgba(255, 255, 255, 0.1);
  }

  .config__nav-item:hover {
    background-color: rgba(255, 255, 255, 0.06);
  }

  .config__nav-item--active {
    background-color: rgba(0, 145, 255, 0.2);
    color: #6cb8ff;
  }

  .entry-item,
  .btn,
  .entry-form__input,
  .entry-form__option,
  .entry-preview,
  .form-section {
    background-color: #333;
    border-color: rgba(255, 255, 255, 0.12);
  }

  .entry-form__footer {
    border-top-color: rgba(255, 255, 255, 0.1);
  }

  .btn--primary {
    background-color: #0067c0;
    border-color: #0067c0;
    color: #fff;
  }

  .entry-form__option--active {
    color: #6cb8ff;
    border-color: #6cb8ff;
  }

  .entry-form__color--active {
    border-color: rgba(255, 255, 255, 0.7);
  }
}
</style>
