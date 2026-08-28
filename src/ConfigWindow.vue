<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Entry } from './types/entry';
import { ENTRY_COLORS } from './types/entry';
import { createDraft, useEntries } from './composables/useEntries';
import { sortEntries } from './utils/entries';

type NavKey = 'entries' | 'settings' | 'about';

const { t } = useI18n();
const activeNav = ref<NavKey>('entries');
const { entries, loaded, reload, upsert, remove } = useEntries();

// 编辑态：editing 非空时内容区切换为表单
const editing = ref<Entry | null>(null);
const isNew = ref(false);
// 两步删除确认：记录待确认删除的条目 id
const deletingId = ref<string | null>(null);

const sorted = computed(() => sortEntries(entries.value));
const canSave = computed(() => !!editing.value?.name && !!editing.value?.date);

onMounted(() => reload());

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

async function submit() {
  if (!editing.value || !canSave.value) return;
  const entry = { ...editing.value, updatedAt: new Date().toISOString() };
  await upsert(entry);
  cancelEdit();
}

async function confirmRemove() {
  if (!deletingId.value) return;
  await remove(deletingId.value);
  deletingId.value = null;
}
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

          <label class="entry-form__field">
            <span class="entry-form__label">{{ t('config.fieldName') }}</span>
            <input
              v-model="editing.name"
              class="entry-form__input"
              type="text"
              :placeholder="t('config.namePlaceholder')"
              maxlength="30"
            />
          </label>

          <div class="entry-form__field">
            <span class="entry-form__label">{{ t('config.fieldType') }}</span>
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
          </div>

          <label class="entry-form__field">
            <span class="entry-form__label">
              {{ editing.entryType === 'countdown' ? t('config.targetDate') : t('config.startDate') }}
            </span>
            <input v-model="editing.date" class="entry-form__input" type="date" />
          </label>

          <div class="entry-form__field">
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
          </div>

          <label class="entry-form__field entry-form__field--row">
            <input v-model="editing.pinned" type="checkbox" />
            <span>{{ t('config.fieldPinned') }}</span>
          </label>

          <div class="entry-form__actions">
            <button class="btn btn--primary" type="submit" :disabled="!canSave">
              {{ t('config.save') }}
            </button>
            <button class="btn" type="button" @click="cancelEdit">
              {{ t('config.cancel') }}
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
            <li v-for="entry in sorted" :key="entry.id" class="entry-item">
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

      <!-- 通用设置（M3 实装） -->
      <template v-else-if="activeNav === 'settings'">
        <h2 class="config__placeholder-title">{{ t('config.nav.settings') }}</h2>
        <p class="config__placeholder">{{ t('config.settingsPlaceholder') }}</p>
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
  max-width: 420px;
  display: flex;
  flex-direction: column;
  gap: 14px;
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
  .entry-form__option {
    background-color: #333;
    border-color: rgba(255, 255, 255, 0.12);
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
