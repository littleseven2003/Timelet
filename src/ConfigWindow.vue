<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DisplayUnit, Entry } from './types/entry';
import { ENTRY_COLORS } from './types/entry';
import { createDraft, useEntries } from './composables/useEntries';
import {
  entryDisplayValue,
  formatEntryText,
  groupForConfig,
  sortEntries,
} from './utils/entries';
import DateTimePicker from './components/DateTimePicker.vue';
import SegmentedControl from './components/SegmentedControl.vue';
import ToggleSwitch from './components/ToggleSwitch.vue';
import SettingsSection from './components/SettingsSection.vue';
import EntryTypeSymbol from './components/EntryTypeSymbol.vue';

type NavKey = 'entries' | 'settings' | 'about';

const { t } = useI18n();
const activeNav = ref<NavKey>('entries');
const { entries, loaded, reload, upsert, remove, reorder } = useEntries();

// 编辑态：editing 非空时内容区切换为表单
const editing = ref<Entry | null>(null);
const isNew = ref(false);
// 删除确认弹窗：记录待删除条目
const deleteTarget = ref<Entry | null>(null);
// 列表行右键菜单：定位与目标条目
const ctxMenu = ref<{ x: number; y: number; entry: Entry } | null>(null);

// 打开时按窗口尺寸收敛坐标，避免菜单越出窗口边缘
function openCtxMenu(event: MouseEvent, entry: Entry) {
  const width = 140;
  const height = 120;
  ctxMenu.value = {
    x: Math.min(event.clientX, window.innerWidth - width),
    y: Math.min(event.clientY, window.innerHeight - height),
    entry,
  };
}

async function togglePinned(entry: Entry) {
  await upsert({ ...entry, pinned: !entry.pinned, updatedAt: new Date().toISOString() });
}

const sorted = computed(() => sortEntries(entries.value));
const canSave = computed(() => !!editing.value?.name && !!editing.value?.date);

// 主窗口分组（设计文档 5.2）：今天 / 接下来 7 天 / 更晚；拖拽预览期间退回平铺
const groups = computed(() => groupForConfig(entries.value));
const flatGroups = computed(() => groups.value.flatMap((group) => group.items));

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

// 当前颜色不在预设色板中时高亮自定义取色块
const isCustomColor = computed(
  () => editing.value != null && !(ENTRY_COLORS as readonly string[]).includes(editing.value.color),
);

// 切换类型时，若仍在对应类型的默认色上则同步切换（倒计时=时屿蓝，正计时=累积岛绿）
const TYPE_DEFAULT_COLOR = { countdown: '#2a9cdb', elapsed: '#368e76' } as const;

function onTypeChange(next: 'countdown' | 'elapsed') {
  if (!editing.value) return;
  const previous = editing.value.entryType;
  if (editing.value.color === TYPE_DEFAULT_COLOR[previous]) {
    editing.value.color = TYPE_DEFAULT_COLOR[next];
  }
  editing.value.entryType = next;
}

// 分段控件选项集中定义
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

onMounted(() => window.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));

// 拖拽排序：拖动过程用本地预览列表渲染，落点后一次性持久化
const dragId = ref<string | null>(null);
const previewList = ref<Entry[] | null>(null);
const displayList = computed(() => previewList.value ?? flatGroups.value);

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
        v-for="key in ['entries', 'settings', 'about'] as NavKey[]"
        :key="key"
        class="config__nav-item"
        :class="{ 'config__nav-item--active': activeNav === key }"
        type="button"
        @click="activeNav = key"
      >
        <svg
          v-if="key === 'entries'"
          class="config__nav-icon"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        >
          <path d="M5.5 4h8M5.5 8h8M5.5 12h8" />
          <circle cx="2.5" cy="4" r="0.9" fill="currentColor" stroke="none" />
          <circle cx="2.5" cy="8" r="0.9" fill="currentColor" stroke="none" />
          <circle cx="2.5" cy="12" r="0.9" fill="currentColor" stroke="none" />
        </svg>
        <svg
          v-else-if="key === 'settings'"
          class="config__nav-icon"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        >
          <circle cx="8" cy="8" r="2.2" />
          <path
            d="M8 1.8v2M8 12.2v2M1.8 8h2M12.2 8h2M3.6 3.6l1.4 1.4M11 11l1.4 1.4M12.4 3.6 11 5M5 11l-1.4 1.4"
          />
        </svg>
        <svg
          v-else
          class="config__nav-icon"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        >
          <circle cx="8" cy="8" r="6" />
          <path d="M8 7.4v3.4" />
          <circle cx="8" cy="5" r="0.9" fill="currentColor" stroke="none" />
        </svg>
        {{ t(`config.nav.${key}`) }}
      </button>
      <div class="config__nav-brand">时屿 · Timelet</div>
    </nav>

    <main class="config__main">
      <!-- 条目管理 -->
      <template v-if="activeNav === 'entries'">
        <!-- 编辑表单 -->
        <form v-if="editing" class="entry-form" @submit.prevent="submit">
          <div class="entry-form__title-row">
            <h2 class="entry-form__title">
              {{ isNew ? t('config.addEntry') : t('config.editEntry') }}
            </h2>
            <!-- 危险操作与主操作分离：删除放在标题行右侧 -->
            <button
              v-if="!isNew"
              class="entry-form__delete"
              type="button"
              @click="deleteTarget = editing"
            >
              {{ t('config.delete') }}
            </button>
          </div>

          <!-- 实时预览：大数字 + 状态小字（带时刻条目回退文本）；时弧作为背景主意象 -->
          <div class="entry-preview">
            <svg
              class="entry-preview__arc"
              viewBox="0 0 96 96"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M20 74a40 40 0 1 1 56 0"
                stroke="currentColor"
                stroke-width="3"
                stroke-linecap="round"
              />
              <circle cx="76" cy="70" r="5" fill="currentColor" />
            </svg>
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
            <!-- 类型是影响左右两栏语义的总开关，通栏置顶 -->
            <div class="form-row form-row--column form-row--type">
              <div class="form-row">
                <span class="form-row__label">{{ t('config.fieldType') }}</span>
                <SegmentedControl
                  :model-value="editing.entryType"
                  :options="typeOptions"
                  class="form-row__seg"
                  @update:model-value="onTypeChange($event as 'countdown' | 'elapsed')"
                />
              </div>
              <span class="form-row__hint">
                {{
                  editing.entryType === 'countdown'
                    ? t('config.hintCountdown')
                    : t('config.hintElapsed')
                }}
              </span>
            </div>

            <!-- 左列：日期时间组件作为视觉锚点独占 -->
            <section class="form-section form-section--time">
              <h3 class="form-section__title">{{ t('config.sectionTime') }}</h3>
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
              </section>

              <section class="form-section form-section--appearance">
                <h3 class="form-section__title">{{ t('config.sectionAppearance') }}</h3>
                <div class="form-row form-row--column">
                  <span class="form-row__label">{{ t('config.fieldColor') }}</span>
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
                    <!-- 自定义取色：色板外的颜色命中此项 -->
                    <label
                      class="entry-form__color entry-form__color--custom"
                      :class="{ 'entry-form__color--active': isCustomColor }"
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
                <div class="form-row form-row--column form-row--grow">
                  <span class="form-row__label">{{ t('config.fieldNote') }}</span>
                  <textarea
                    v-model="editing.note"
                    class="entry-form__input entry-form__note"
                    :placeholder="t('config.notePlaceholder')"
                    maxlength="100"
                  />
                </div>
              </section>
            </div>
          </div>

          <div class="entry-form__footer">
            <span class="entry-form__helper" :class="{ 'entry-form__helper--active': !canSave }">
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

        <!-- 列表 -->
        <template v-else>
          <header class="entry-list__header">
            <h2 class="entry-list__title">{{ t('config.nav.entries') }}</h2>
            <button class="btn btn--primary" type="button" @click="openCreate">
              {{ t('config.addEntry') }}
            </button>
          </header>

          <div v-if="loaded && sorted.length === 0" class="entry-list__empty">
            <!-- 极简水面与小岛轮廓（设计语言 5.6），与面板空状态一致 -->
            <svg class="entry-list__empty-art" viewBox="0 0 120 44" fill="none" aria-hidden="true">
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
            <p>{{ t('config.emptyList') }}</p>
          </div>

          <ul v-if="dragId" class="entry-list">
            <!-- 拖拽中：平铺预览，保证跨组移动的视觉连续 -->
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
              <span class="entry-item__color" :style="{ backgroundColor: entry.color }" />
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
                  · {{ entry.date }}
                </span>
              </div>
              <span class="entry-item__days" :style="{ color: entry.color }">
                {{ formatEntryText(entry, Date.now(), (key, params) => t(key, params ?? {})) }}
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

          <template v-else>
            <section v-for="group in groups" :key="group.key" class="entry-group">
              <h3 class="entry-group__title">{{ t(`config.groups.${group.key}`) }}</h3>
              <ul class="entry-list">
                <li
                  v-for="entry in group.items"
                  :key="entry.id"
                  class="entry-item"
                  draggable="true"
                  @dragstart="onDragStart(entry.id)"
                  @contextmenu.prevent="openCtxMenu($event, entry)"
                >
                  <span class="entry-item__color" :style="{ backgroundColor: entry.color }" />
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
                      · {{ entry.date }}
                    </span>
                  </div>
                  <span class="entry-item__days" :style="{ color: entry.color }">
                    {{ formatEntryText(entry, Date.now(), (key, params) => t(key, params ?? {})) }}
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

    <!-- 列表行右键菜单：透明遮罩负责点击外部关闭 -->
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
            openEdit(ctxMenu.entry);
            ctxMenu = null;
          "
        >
          {{ t('config.edit') }}
        </button>
        <button
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
  display: flex;
  min-height: 100vh;
  background-color: var(--ts-bg);
  color: var(--ts-text);
}

.config__nav {
  width: 132px;
  flex-shrink: 0;
  padding: 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  border-right: 1px solid var(--ts-line);
  min-height: 100vh;
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
  display: flex;
  align-items: center;
  gap: 8px;
}

.config__nav-icon {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  opacity: 0.75;
}

.config__nav-brand {
  margin-top: auto;
  padding: 10px 12px 2px;
  font-size: 11px;
  opacity: 0.4;
  white-space: nowrap;
}

.config__nav-item:hover {
  background-color: rgba(42, 156, 219, 0.08);
}

.config__nav-item--active {
  background-color: rgba(42, 156, 219, 0.14);
  color: var(--ts-primary-text);
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

.entry-form__title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

/* 危险操作：幽灵样式与主操作区分离 */
.entry-form__delete {
  border: none;
  background: none;
  font-size: 12px;
  color: var(--ts-coral);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
}

.entry-form__delete:hover {
  background-color: rgba(201, 79, 85, 0.1);
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
  border: 1px solid var(--ts-line);
  border-radius: 10px;
  background-color: var(--ts-surface);
  cursor: grab;
}

.entry-item--dragging {
  opacity: 0.5;
}

.entry-item__symbol {
  flex-shrink: 0;
  opacity: 0.65;
}

.entry-item__color {
  width: 10px;
  height: 32px;
  /* 签名造型：底部圆弧的「岛屿」色条，呼应产品图标意象 */
  border-radius: 3px 3px 45% 45% / 3px 3px 30% 30%;
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
  /* 桌面惯例：操作按钮悬停浮现，降低行内噪音（键盘 focus 时同样可见） */
  opacity: 0;
  transition: opacity 0.15s ease-out;
}

.entry-item:hover .entry-item__actions,
.entry-item:focus-within .entry-item__actions {
  opacity: 1;
}

.entry-group + .entry-group {
  margin-top: 16px;
}

.entry-group__title {
  font-size: 12px;
  font-weight: 600;
  color: var(--ts-text-2);
  margin: 0 0 8px;
}

.entry-list__empty {
  text-align: center;
  opacity: 0.55;
  font-size: 13px;
  padding: 40px 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.entry-list__empty p {
  margin: 0;
}

.entry-list__empty-art {
  width: 120px;
  height: 44px;
}

.btn {
  border: 1px solid var(--ts-line);
  background-color: var(--ts-surface);
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
  color: inherit;
}

.btn:hover {
  border-color: var(--ts-primary);
}

.btn:disabled {
  opacity: 0.45;
  cursor: default;
}

.btn--primary {
  background-color: var(--ts-primary);
  border-color: var(--ts-primary);
  color: #fff;
}

.btn--primary:hover:not(:disabled) {
  filter: brightness(1.08);
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
  filter: brightness(1.08);
}

/* 列表行右键菜单 */
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 90;
}

.ctx-menu {
  position: fixed;
  min-width: 120px;
  padding: 4px;
  border-radius: 8px;
  background-color: var(--ts-surface);
  border: 1px solid var(--ts-line);
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.18);
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
  background-color: rgba(42, 156, 219, 0.08);
}

.ctx-menu__item--danger {
  color: var(--ts-coral);
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
  background-color: var(--ts-surface);
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
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border: 1px solid var(--ts-line);
  border-radius: 10px;
  background-color: var(--ts-surface);
  overflow: hidden;
}

/* 时弧：开放圆弧 + 目标端点，颜色随条目色，仅作背景意象 */
.entry-preview__arc {
  position: absolute;
  right: -14px;
  top: -30px;
  width: 96px;
  height: 96px;
  opacity: 0.14;
  pointer-events: none;
}

.entry-preview__color {
  width: 5px;
  height: 28px;
  /* 岛屿弧形签名造型 */
  border-radius: 2px 2px 45% 45% / 2px 2px 30% 30%;
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

/* 双栏网格：预览与类型通栏 → 左侧日历锚点 + 右侧紧凑字段列 */
.entry-form__body {
  display: grid;
  grid-template-columns: minmax(260px, 300px) minmax(0, 1fr);
  grid-template-areas:
    'preview preview'
    'type type'
    'time side';
  gap: 12px;
  align-items: stretch;
}

.entry-preview {
  grid-area: preview;
}

.form-row--type {
  grid-area: type;
}

.form-row--type .form-row__seg {
  flex: 1;
  max-width: 240px;
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
  border: 1px solid var(--ts-line);
  border-radius: 10px;
  background-color: var(--ts-surface);
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
  justify-content: flex-end;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
}

/* 保存前置提示：名称或日期未完成时说明原因 */
.entry-form__helper {
  flex: 1;
  font-size: 12px;
  opacity: 0.55;
}

.entry-form__helper--active {
  opacity: 0.85;
  color: var(--ts-amber);
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

.form-row--grow {
  flex: 1;
}

.form-row__label {
  font-size: 13px;
  opacity: 0.7;
}

.form-row__hint {
  font-size: 12px;
  color: var(--ts-text-2);
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
  border: 1px solid var(--ts-line);
  border-radius: 6px;
  padding: 7px 10px;
  font-size: 13px;
  background-color: var(--ts-surface);
  color: inherit;
}

.entry-form__colors {
  display: flex;
  flex-wrap: wrap;
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
  border-color: var(--ts-text);
}

/* 自定义取色块：虚线描边示意可自定义，内嵌隐藏的原生取色控件 */
.entry-form__color--custom {
  position: relative;
  border-style: dashed;
}

.entry-form__color--custom input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
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
    background-color: var(--ts-bg);
    color: var(--ts-text);
  }

  .config__nav {
    border-right-color: var(--ts-line);
  }

  .config__nav-item:hover {
    background-color: rgba(98, 185, 235, 0.08);
  }

  .config__nav-item--active {
    background-color: rgba(98, 185, 235, 0.16);
    color: var(--ts-primary-text);
  }

  .entry-item,
  .btn,
  .entry-form__input,
  .entry-preview,
  .form-section,
  .ctx-menu,
  .modal {
    background-color: var(--ts-surface);
    border-color: var(--ts-line);
  }

  .entry-form__footer {
    border-top-color: var(--ts-line);
  }

  .config__nav-item--active .config__nav-icon {
    opacity: 1;
  }
}
</style>
