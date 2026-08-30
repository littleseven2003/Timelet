<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DisplayUnit, Entry } from './types/entry';
import { ENTRY_COLORS } from './types/entry';
import { createDraft, useEntries } from './composables/useEntries';
import { entryDisplayValue, formatEntryText, sortEntries } from './utils/entries';
import DateTimePicker from './components/DateTimePicker.vue';
import SettingsSection from './components/SettingsSection.vue';

type NavKey = 'entries' | 'settings' | 'about';

const { t } = useI18n();
const activeNav = ref<NavKey>('entries');
const { entries, loaded, reload, upsert, remove, reorder } = useEntries();

// 编辑态：editing 非空时内容区切换为表单
const editing = ref<Entry | null>(null);
const isNew = ref(false);
// 删除确认弹窗：记录待删除条目
const deleteTarget = ref<Entry | null>(null);

const sorted = computed(() => sortEntries(entries.value));
const canSave = computed(() => !!editing.value?.name && !!editing.value?.date);

// 预览卡文案：随名称/类型/日期输入实时变化
const previewText = computed(() => {
  if (!editing.value) return '';
  return formatEntryText(editing.value, Date.now(), (key, params) => t(key, params ?? {}));
});

// 大数字预览：纯日期条目返回数值与单位，带时刻条目回退文本展示
const previewValue = computed(() => {
  if (!editing.value) return null;
  return entryDisplayValue(editing.value);
});

// 大数字上方的状态小字
const previewStatus = computed(() => {
  const value = previewValue.value;
  if (!value || !editing.value) return '';
  if (editing.value.entryType === 'elapsed') return t('config.previewElapsed');
  return value.value >= 0 ? t('config.previewLeft') : t('config.previewAgo');
});

// 新建/编辑时自动聚焦名称输入
const nameInput = ref<HTMLInputElement | null>(null);
watch(editing, (value) => {
  if (value) void nextTick(() => nameInput.value?.focus());
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
  deleteTarget.value = null;
}

function cancelEdit() {
  editing.value = null;
  deleteTarget.value = null;
}

// 开关时刻精度：开启时给默认时刻，关闭时移除并清空循环
function toggleTime(enabled: boolean) {
  if (!editing.value) return;
  editing.value.time = enabled ? (editing.value.time ?? '09:00') : undefined;
  if (!enabled) editing.value.repeat = undefined;
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
  // 从编辑表单内删除时同步关闭表单
  if (editing.value?.id === id) cancelEdit();
}

// 从面板"编辑详情"或"新增条目"进入：数据就绪后应用动作
async function applyPendingAction(action: PendingAction) {
  activeNav.value = 'entries';
  if (!loaded.value) await reload();
  if (action.kind === 'create') {
    openCreate();
    return;
  }
  const entry = entries.value.find((item) => item.id === action.id);
  if (entry) openEdit(entry);
}

// 面板动作载荷：新建或编辑指定条目
type PendingAction = { kind: 'create' } | { kind: 'edit'; id: string };

onMounted(async () => {
  // 窗口先于面板动作创建时，取走暂存的待执行动作
  const pending = await invoke<PendingAction | null>('take_pending_action');
  if (pending) await applyPendingAction(pending);
  // 窗口已存在时后续动作通过事件送达
  listen<PendingAction>('open-entry-action', (event) => {
    void applyPendingAction(event.payload);
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

          <!-- 实时预览：大数字 + 状态小字（带时刻条目回退文本） -->
          <div class="entry-preview">
            <span class="entry-preview__color" :style="{ backgroundColor: editing.color }" />
            <div class="entry-preview__main">
              <span class="entry-preview__name">{{ editing.name || t('config.previewName') }}</span>
              <span v-if="previewText" class="entry-preview__date">{{ previewText }}</span>
            </div>
            <div v-if="previewValue" class="entry-preview__big" :style="{ color: editing.color }">
              <span class="entry-preview__status">{{ previewStatus }}</span>
              <span class="entry-preview__number">
                {{ Math.abs(previewValue.value) }}
                <small>{{ t(`config.unit.${previewValue.unit}`) }}</small>
              </span>
            </div>
            <span v-else class="entry-preview__days" :style="{ color: editing.color }">
              {{ t('config.previewDays') }}
            </span>
          </div>

          <div class="entry-form__body">
            <!-- 左列：日期时间组件作为视觉锚点独占 -->
            <section class="form-section form-section--time">
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

              <template v-if="editing.time">
                <span class="entry-form__label">{{ t('config.fieldRepeat') }}</span>
                <div class="entry-form__options">
                  <button
                    v-for="rule in (['none', 'daily', 'workday'] as const)"
                    :key="rule"
                    type="button"
                    class="entry-form__option"
                    :class="{ 'entry-form__option--active': (editing.repeat ?? 'none') === rule }"
                    @click="editing.repeat = rule === 'none' ? undefined : rule"
                  >
                    {{ t(`config.repeat.${rule}`) }}
                  </button>
                </div>
              </template>

              <template v-if="!editing.time">
                <span class="entry-form__label">{{ t('config.fieldUnit') }}</span>
                <div class="entry-form__options">
                  <button
                    v-for="unit in (['day', 'week', 'month', 'year'] as DisplayUnit[])"
                    :key="unit"
                    type="button"
                    class="entry-form__option"
                    :class="{ 'entry-form__option--active': (editing.displayUnit ?? 'day') === unit }"
                    @click="editing.displayUnit = unit"
                  >
                    {{ t(`config.unit.${unit}`) }}
                  </button>
                </div>
              </template>
            </section>

            <!-- 右列：紧凑字段纵向排布，备注自动伸展补齐高度 -->
            <div class="entry-form__side">
              <section class="form-section">
                <h3 class="form-section__title">{{ t('config.sectionBasic') }}</h3>
              <input
                ref="nameInput"
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

              <section class="form-section form-section--appearance">
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
                  <span>{{ t('config.fieldPinned') }}（{{ editing.pinned ? t('config.pinnedOn') : t('config.pinnedOff') }}）</span>
                </label>
                <span class="entry-form__label">{{ t('config.fieldNote') }}</span>
                <textarea
                  v-model="editing.note"
                  class="entry-form__input entry-form__note"
                  :placeholder="t('config.notePlaceholder')"
                  maxlength="100"
                />
              </section>
            </div>
          </div>

          <div class="entry-form__footer">
            <button
              v-if="!isNew"
              class="btn btn--danger"
              type="button"
              @click="deleteTarget = editing"
            >
              {{ t('config.delete') }}
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
              <span class="entry-item__days" :style="{ color: entry.color }">
                {{ formatEntryText(entry, Date.now(), (key, params) => t(key, params ?? {})) }}
              </span>
              <div class="entry-item__actions">
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
  max-width: 780px;
  margin: 0 auto;
  padding: 20px 24px;
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
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
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

.btn--danger-solid {
  background-color: #d64545;
  border-color: #d64545;
  color: #fff;
}

.btn--danger-solid:hover {
  background-color: #e05555;
  border-color: #e05555;
}

/* 删除确认弹窗 */
.modal-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  width: 320px;
  padding: 18px 20px;
  border-radius: 12px;
  background-color: #fff;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.2);
}

.modal__title {
  font-size: 15px;
  font-weight: 600;
  margin: 0 0 8px;
}

.modal__text {
  font-size: 13px;
  opacity: 0.7;
  margin: 0 0 16px;
  line-height: 1.5;
}

.modal__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.btn--small {
  padding: 4px 10px;
  font-size: 12px;
}

.entry-form {
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

.entry-preview__main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.entry-preview__date {
  font-size: 12px;
  opacity: 0.55;
}

/* 大数字预览（LikeDay 风格） */
.entry-preview__big {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0;
}

.entry-preview__status {
  font-size: 11px;
  opacity: 0.55;
}

.entry-preview__number {
  font-size: 30px;
  font-weight: 700;
  line-height: 1.1;
  font-variant-numeric: tabular-nums;
}

.entry-preview__number small {
  font-size: 13px;
  font-weight: 500;
  margin-left: 2px;
}

.entry-item__days {
  font-size: 13px;
  font-weight: 600;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

/* 双栏网格：预览通栏 → 左侧日历锚点 + 右侧紧凑字段列 */
.entry-form__body {
  display: grid;
  grid-template-columns: 300px 1fr;
  grid-template-areas:
    'preview preview'
    'time side';
  gap: 12px;
  align-items: stretch;
}

.entry-preview {
  grid-area: preview;
}

.form-section--time {
  grid-area: time;
}

.entry-form__side {
  grid-area: side;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.form-section--appearance {
  flex: 1;
}

.form-section--appearance .entry-form__note {
  flex: 1;
  min-height: 56px;
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

.entry-form__note {
  resize: none;
  font-family: inherit;
  line-height: 1.5;
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

  .modal {
    background-color: #333;
  }

  .btn--danger-solid {
    background-color: #d64545;
    border-color: #d64545;
    color: #fff;
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
