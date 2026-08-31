<script setup lang="ts">
import { computed, nextTick, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Entry } from './types/entry';
import { createDraft, useEntries } from './composables/useEntries';
import { useClock } from './composables/useClock';
import { useSettings } from './composables/useSettings';
import { featuredEntry, groupForConfig, sortEntries } from './utils/entries';
import IslandAtmosphere from './components/IslandAtmosphere.vue';
import FeaturedEntry from './components/FeaturedEntry.vue';
import EntryRow from './components/EntryRow.vue';
import EntryEditor from './components/EntryEditor.vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import EntryTypeSymbol from './components/EntryTypeSymbol.vue';
import InterfaceSymbol from './components/InterfaceSymbol.vue';
import SettingsSection from './components/SettingsSection.vue';
import AboutSection from './components/AboutSection.vue';
import appIcon from './assets/app-icon.png';

const { t, locale } = useI18n();
const {
  entries,
  loaded,
  loading,
  error,
  busy,
  reload,
  upsert,
  remove,
  archive,
  restore,
  duplicate,
  reorder,
} = useEntries();
const { error: settingsError, retry: retrySettings } = useSettings();
const now = useClock();
type Nav = 'now' | 'countdown' | 'elapsed' | 'archive' | 'settings' | 'about';
const nav = ref<Nav>('now');
const search = ref('');
const expanded = ref<string | null>(null);
const actionError = ref('');
const message = ref('');
const undo = ref<(() => Promise<void>) | null>(null);
const editing = ref<Entry | null>(null);
const snapshot = ref('');
const isNew = ref(false);
const editor = ref<InstanceType<typeof EntryEditor>>();
const dirty = computed(
  () => editing.value !== null && JSON.stringify(editing.value) !== snapshot.value,
);
const orderDraft = ref<Entry[] | null>(null);
const dragId = ref<string | null>(null);
const pendingDialog = ref<{
  title: string;
  text: string;
  confirm: string;
  danger?: boolean;
  run: () => Promise<void> | void;
} | null>(null);
const active = computed(() => entries.value.filter((entry) => !entry.archived));
const counts = computed(() => ({
  now: active.value.length,
  countdown: active.value.filter((e) => e.entryType === 'countdown').length,
  elapsed: active.value.filter((e) => e.entryType === 'elapsed').length,
  archive: entries.value.length - active.value.length,
}));
const today = computed(() =>
  new Date(now.value).toLocaleDateString(locale.value, {
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }),
);
const isList = computed(() => !['settings', 'about'].includes(nav.value));
const filtered = computed(() => {
  const query = search.value.trim().toLocaleLowerCase();
  return entries.value.filter((entry) => {
    if ((nav.value === 'archive') !== !!entry.archived) return false;
    if (['countdown', 'elapsed'].includes(nav.value) && entry.entryType !== nav.value) return false;
    return !query || `${entry.name}\n${entry.note ?? ''}`.toLocaleLowerCase().includes(query);
  });
});
const manual = computed(() => active.value.some((e) => e.sortIndex != null));
const canOrder = computed(
  () => nav.value === 'now' && !search.value.trim() && active.value.length > 1,
);
const featured = computed(() =>
  nav.value === 'now' && !search.value.trim() && !orderDraft.value
    ? featuredEntry(active.value, now.value)
    : null,
);
const groups = computed(() => {
  const list = filtered.value.filter((e) => e.id !== featured.value?.id);
  if (search.value.trim() || nav.value === 'archive' || manual.value)
    return [{ key: 'all', items: sortEntries(list, now.value) }];
  return groupForConfig(list, now.value);
});

function focusEntry(id?: string) {
  void nextTick(() => {
    const target = id
      ? Array.from(document.querySelectorAll<HTMLElement>('[data-entry-id]')).find(
          (el) => el.dataset.entryId === id,
        )
      : null;
    (target ?? document.querySelector<HTMLElement>('[data-create]'))?.focus();
  });
}
async function run(action: () => Promise<void>, success?: string) {
  actionError.value = '';
  try {
    await action();
    if (success) {
      message.value = success;
      undo.value = null;
    }
    return true;
  } catch (cause) {
    actionError.value = String(cause);
    return false;
  }
}
function leave(action: () => void | Promise<void>) {
  if (busy.value || pendingDialog.value) return;
  if (dirty.value || orderDraft.value) {
    pendingDialog.value = {
      title: t('config.unsavedTitle'),
      text: t('config.unsavedText'),
      confirm: t('config.discard'),
      run: async () => {
        editing.value = null;
        orderDraft.value = null;
        await action();
      },
    };
  } else {
    void action();
  }
}
function navigate(key: Nav) {
  leave(() => {
    nav.value = key;
    editing.value = null;
    orderDraft.value = null;
    search.value = '';
    expanded.value = null;
    actionError.value = '';
  });
}
function beginEdit(entry?: Entry) {
  if (entry && editing.value?.id === entry.id) return;
  leave(() => {
    if (!isList.value || nav.value === 'archive') nav.value = 'now';
    isNew.value = !entry;
    editing.value = entry ? { ...entry } : createDraft();
    if (!entry && nav.value === 'elapsed') {
      editing.value.entryType = 'elapsed';
    }
    snapshot.value = JSON.stringify(editing.value);
    actionError.value = '';
  });
}
function cancelEdit() {
  const id = editing.value?.id;
  leave(() => {
    editing.value = null;
    actionError.value = '';
    focusEntry(id);
  });
}
async function save(entry: Entry) {
  const original = JSON.parse(snapshot.value) as Entry;
  if (
    await run(() => upsert(entry, isNew.value ? undefined : original.updatedAt), t('common.saved'))
  ) {
    editing.value = null;
    undo.value = null;
    search.value = '';
    focusEntry(entry.id);
  }
}
async function togglePin(entry: Entry) {
  await run(
    () => upsert({ ...entry, pinned: !entry.pinned, updatedAt: new Date().toISOString() }),
    t('common.saved'),
  );
  focusEntry(entry.id);
}
async function setArchived(entry: Entry, archived: boolean) {
  if (
    await run(
      () => (archived ? archive(entry.id) : restore(entry.id)),
      t(archived ? 'config.archived' : 'config.restored'),
    )
  ) {
    undo.value = () => (archived ? restore(entry.id) : archive(entry.id));
    expanded.value = null;
    focusEntry();
  }
}
function askRemove(entry: Entry) {
  pendingDialog.value = {
    title: t('config.deleteConfirmTitle'),
    text: t('config.deleteConfirmText', { name: entry.name }),
    confirm: t('config.delete'),
    danger: true,
    run: async () => {
      await remove(entry.id);
      message.value = t('config.deleted');
      undo.value = async () => {
        if (entries.value.some((e) => e.id === entry.id)) throw new Error(t('config.undoConflict'));
        await upsert(entry);
      };
      expanded.value = null;
      focusEntry();
    },
  };
}
async function confirmDialog() {
  const dialog = pendingDialog.value;
  if (!dialog) return;
  if (
    await run(async () => {
      await dialog.run();
    })
  )
    pendingDialog.value = null;
}
async function performUndo() {
  if (undo.value && (await run(undo.value, t('config.undone')))) undo.value = null;
}
function startOrder() {
  orderDraft.value = sortEntries(active.value, now.value);
  expanded.value = null;
}
function moveOrder(id: string, delta: number) {
  const list = orderDraft.value;
  if (!list || busy.value) return;
  const from = list.findIndex((e) => e.id === id);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= list.length || list[from]!.pinned !== list[to]!.pinned) return;
  const next = [...list];
  [next[from], next[to]] = [next[to]!, next[from]!];
  orderDraft.value = next;
}
function dropOn(id: string) {
  const list = orderDraft.value;
  if (!list || !dragId.value) return;
  const from = list.findIndex((e) => e.id === dragId.value),
    to = list.findIndex((e) => e.id === id);
  if (from >= 0 && to >= 0 && list[from]!.pinned === list[to]!.pinned) {
    const next = [...list];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved!);
    orderDraft.value = next;
  }
  dragId.value = null;
}
async function finishOrder() {
  if (
    orderDraft.value &&
    (await run(() => reorder(orderDraft.value!.map((e) => e.id)), t('common.saved')))
  )
    orderDraft.value = null;
}
async function resetOrder() {
  await run(
    () =>
      reorder(
        active.value.map((e) => e.id),
        true,
      ),
    t('common.saved'),
  );
}
watch(entries, () => {
  if (orderDraft.value) {
    const current = active.value
      .map((e) => `${e.id}:${e.pinned}`)
      .sort()
      .join();
    const previous = orderDraft.value
      .map((e) => `${e.id}:${e.pinned}`)
      .sort()
      .join();
    if (current !== previous) {
      orderDraft.value = null;
      message.value = t('config.orderChanged');
    }
  }
});
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && !pendingDialog.value) {
    if (editing.value) cancelEdit();
    else expanded.value = null;
  }
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter' && editing.value) {
    event.preventDefault();
    editor.value?.submit();
  }
}

type PendingAction = { kind: 'create' } | { kind: 'edit'; id: string } | { kind: 'list' };
let stopAction: UnlistenFn | undefined, stopClose: UnlistenFn | undefined;
let disposed = false;
async function consumePending() {
  await run(async () => {
    const action = await invoke<PendingAction | null>('take_pending_action');
    if (!action) return;
    if (action.kind === 'list') {
      if (dirty.value || orderDraft.value) {
        message.value = t('config.draftKept');
        return;
      }
      navigate('now');
    } else if (action.kind === 'create') beginEdit();
    else {
      await reload();
      const entry = entries.value.find((e) => e.id === action.id && !e.archived);
      if (entry) beginEdit(entry);
    }
  });
}
onMounted(async () => {
  window.addEventListener('keydown', keydown);
  try {
    const stop = await listen('open-entry-action', () => {
      void consumePending();
    });
    if (disposed) {
      stop();
      return;
    }
    stopAction = stop;
    const close = await getCurrentWindow().onCloseRequested((event) => {
      if (busy.value) {
        event.preventDefault();
        return;
      }
      if (dirty.value || orderDraft.value) {
        event.preventDefault();
        leave(async () => {
          await run(() => getCurrentWindow().destroy());
        });
      }
    });
    if (disposed) {
      close();
      return;
    }
    stopClose = close;
    await consumePending();
  } catch (cause) {
    actionError.value = String(cause);
  }
});
onBeforeUnmount(() => {
  disposed = true;
  stopAction?.();
  stopClose?.();
  window.removeEventListener('keydown', keydown);
});
</script>

<template>
  <div class="workspace">
    <IslandAtmosphere :strength="editing ? 0.65 : 1" />
    <aside class="sidebar">
      <div class="brand">
        <img :src="appIcon" alt="" />
        <div>
          <strong>{{ t('app.name') }}</strong
          ><small>TIMELET</small>
        </div>
      </div>
      <nav :aria-label="t('config.navigation')">
        <button
          v-for="key in ['now', 'countdown', 'elapsed'] as const"
          :key="key"
          class="nav-item"
          :class="{ active: nav === key }"
          :aria-current="nav === key ? 'page' : undefined"
          type="button"
          @click="navigate(key)"
        >
          <InterfaceSymbol v-if="key === 'now'" name="now" /><EntryTypeSymbol
            v-else
            :type="key"
          /><span>{{ t(`config.nav.${key}`) }}</span
          ><small>{{ counts[key] }}</small>
        </button>
      </nav>
      <nav class="nav-low" :aria-label="t('config.otherNavigation')">
        <button
          v-for="key in ['archive', 'settings', 'about'] as const"
          :key="key"
          class="nav-item"
          :class="{ active: nav === key }"
          :aria-current="nav === key ? 'page' : undefined"
          type="button"
          @click="navigate(key)"
        >
          <InterfaceSymbol :name="key" /><span>{{ t(`config.nav.${key}`) }}</span
          ><small v-if="key === 'archive' && counts.archive">{{ counts.archive }}</small>
        </button>
      </nav>
    </aside>
    <main class="main-content">
      <div v-if="error" class="error-notice" role="alert">
        <strong>{{ t('common.loadError') }}</strong>
        <p>{{ error }}</p>
        <button class="btn" type="button" :disabled="loading" @click="reload">
          {{ t('common.retry') }}
        </button>
      </div>
      <div v-if="settingsError && nav !== 'settings'" class="error-notice" role="alert">
        {{ settingsError
        }}<button class="btn" type="button" @click="retrySettings">{{ t('common.retry') }}</button>
      </div>
      <div v-if="actionError && !editing && !pendingDialog" class="error-notice" role="alert">
        {{ actionError
        }}<button class="btn" type="button" @click="actionError = ''">
          {{ t('common.dismiss') }}
        </button>
      </div>
      <div v-if="message" class="feedback" role="status">
        <span>{{ message }}</span
        ><button v-if="undo" type="button" :disabled="busy || !!error" @click="performUndo">
          {{ t('config.undo') }}</button
        ><button
          type="button"
          :aria-label="t('common.dismiss')"
          @click="
            message = '';
            undo = null;
          "
        >
          ×
        </button>
      </div>
      <EntryEditor
        v-if="editing"
        ref="editor"
        v-model="editing"
        :is-new="isNew"
        :now="now"
        :busy="busy"
        :blocked="!!error || !loaded"
        :error="actionError"
        @save="save"
        @cancel="cancelEdit"
      />
      <template v-else-if="isList">
        <header class="page-header">
          <div>
            <span class="eyebrow">{{ today }}</span>
            <h1>{{ t(`config.nav.${nav}`) }}</h1>
          </div>
          <button
            class="btn btn--primary"
            data-create
            type="button"
            :disabled="busy || !loaded || !!error"
            @click="beginEdit()"
          >
            <InterfaceSymbol name="plus" />{{ t('config.addEntry') }}
          </button>
        </header>
        <div class="list-toolbar">
          <label class="search"
            ><InterfaceSymbol name="search" /><input
              v-model="search"
              :aria-label="t('config.searchPlaceholder')"
              :placeholder="t('config.searchPlaceholder')"
              type="search"
              :disabled="!!orderDraft" /></label
          ><button
            v-if="canOrder && !orderDraft"
            class="btn btn--quiet"
            type="button"
            :disabled="busy || !!error"
            @click="startOrder"
          >
            {{ t('config.reorder') }}</button
          ><button
            v-if="canOrder && manual && !orderDraft"
            class="btn btn--quiet"
            type="button"
            :disabled="busy || !!error"
            @click="resetOrder"
          >
            {{ t('config.autoOrder') }}
          </button>
        </div>
        <template v-if="orderDraft">
          <p class="order-hint">{{ t('config.orderHint') }}</p>
          <ol class="order-list">
            <li
              v-for="(entry, index) in orderDraft"
              :key="entry.id"
              :draggable="!busy"
              @dragstart="dragId = entry.id"
              @dragover.prevent
              @drop.prevent="dropOn(entry.id)"
              @dragend="dragId = null"
            >
              <span
                >{{ entry.name
                }}<small v-if="entry.pinned"> · {{ t('config.pinnedTag') }}</small></span
              ><button
                class="btn"
                type="button"
                :aria-label="t('config.moveUp', { name: entry.name })"
                :disabled="busy || index === 0 || orderDraft[index - 1]?.pinned !== entry.pinned"
                @click="moveOrder(entry.id, -1)"
              >
                ↑</button
              ><button
                class="btn"
                type="button"
                :aria-label="t('config.moveDown', { name: entry.name })"
                :disabled="
                  busy ||
                  index === orderDraft.length - 1 ||
                  orderDraft[index + 1]?.pinned !== entry.pinned
                "
                @click="moveOrder(entry.id, 1)"
              >
                ↓
              </button>
            </li>
          </ol>
          <div class="order-actions">
            <button class="btn" type="button" :disabled="busy" @click="orderDraft = null">
              {{ t('config.cancel') }}</button
            ><button
              class="btn btn--primary"
              type="button"
              :disabled="busy || !!error"
              @click="finishOrder"
            >
              {{ t('config.saveOrder') }}
            </button>
          </div>
        </template>
        <template v-else>
          <FeaturedEntry v-if="featured" :entry="featured" :now="now" @edit="beginEdit" />
          <p v-if="!loaded && !error" class="empty" role="status">{{ t('common.loading') }}</p>
          <div v-else-if="loaded && !filtered.length && !error" class="empty">
            <InterfaceSymbol name="now" />
            <p>
              {{
                t(
                  search.trim()
                    ? 'config.searchEmpty'
                    : nav === 'archive'
                      ? 'config.archiveEmpty'
                      : 'config.emptyList',
                )
              }}
            </p>
            <button v-if="search.trim()" class="btn" type="button" @click="search = ''">
              {{ t('config.clearSearch') }}</button
            ><button
              v-else-if="nav !== 'archive'"
              class="btn"
              type="button"
              :disabled="!!error"
              @click="beginEdit()"
            >
              {{ t('panel.createFirst') }}
            </button>
          </div>
          <section v-for="group in groups" :key="group.key" class="entry-group">
            <h2 v-if="group.key !== 'all'">{{ t(`config.groups.${group.key}`) }}</h2>
            <ul>
              <EntryRow
                v-for="entry in group.items"
                :key="entry.id"
                :entry="entry"
                :now="now"
                :expanded="expanded === entry.id"
                :disabled="busy || !!error"
                @expand="expanded = expanded === entry.id ? null : entry.id"
                @edit="beginEdit(entry)"
                @pin="togglePin(entry)"
                @archive="setArchived(entry, true)"
                @restore="setArchived(entry, false)"
                @duplicate="
                  run(() => duplicate(entry.id, t('config.copySuffix')), t('config.copied'))
                "
                @remove="askRemove(entry)"
              />
            </ul>
          </section>
        </template>
        <footer v-if="loaded" class="list-count">
          {{ filtered.length }} {{ t('config.entriesUnit') }}
        </footer>
      </template>
      <template v-else
        ><header class="page-header">
          <h1>{{ t(`config.nav.${nav}`) }}</h1>
        </header>
        <SettingsSection v-if="nav === 'settings'" /><AboutSection v-else
      /></template>
    </main>
    <ConfirmDialog
      v-if="pendingDialog"
      :title="pendingDialog.title"
      :confirm="pendingDialog.confirm"
      :danger="pendingDialog.danger"
      :busy="busy"
      @confirm="confirmDialog"
      @cancel="pendingDialog = null"
      ><p>{{ pendingDialog.text }}</p>
      <p v-if="actionError" class="error-notice" role="alert">{{ actionError }}</p></ConfirmDialog
    >
  </div>
</template>

<style scoped>
.workspace {
  position: relative;
  display: grid;
  grid-template-columns: 146px minmax(0, 1fr);
  height: 100dvh;
  min-height: 320px;
  background: var(--ts-surface);
  isolation: isolate;
  overflow: hidden;
}
.sidebar {
  position: relative;
  display: flex;
  flex-direction: column;
  background: var(--ts-rail);
  border-right: 1px solid var(--ts-line);
  padding: 25px 11px 16px;
  overflow-y: auto;
}
.brand {
  display: flex;
  gap: 8px;
  align-items: center;
  margin: 0 6px 35px;
}
.brand img {
  width: 29px;
  height: 29px;
}
.brand strong {
  font-size: 16px;
  letter-spacing: 0.15em;
  font-weight: 550;
}
.brand small {
  display: block;
  font-size: 8px;
  letter-spacing: 0.17em;
  color: var(--ts-text-2);
  margin-top: 4px;
}
nav {
  display: grid;
  gap: 5px;
}
.nav-low {
  margin-top: auto;
  padding-top: 40px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ts-text-2);
  border: 0;
  background: none;
  border-radius: 7px;
  padding: 10px 9px;
  font-size: 12px;
  cursor: pointer;
  width: 100%;
  text-align: left;
}
.nav-item svg {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}
.nav-item small {
  margin-left: auto;
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}
.nav-item.active {
  background: var(--ts-surface);
  color: var(--ts-blue);
}
.nav-item:hover {
  background: var(--ts-hover);
}
.main-content {
  position: relative;
  padding: 30px 34px 18px;
  overflow-y: auto;
  min-width: 0;
  scrollbar-gutter: stable;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 22px;
}
.page-header h1 {
  font-size: 26px;
  font-weight: 500;
  letter-spacing: 0.04em;
  margin: 8px 0 0;
}
.eyebrow {
  font-size: 11px;
  color: var(--ts-text-2);
}
.page-header .btn {
  font-size: 12px;
}
.list-toolbar {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}
.search {
  display: flex;
  gap: 9px;
  align-items: center;
  flex: 1;
  min-width: 150px;
  color: var(--ts-text-2);
}
.search input {
  border: 0;
  background: transparent;
  padding: 9px 0;
  width: 100%;
  min-width: 0;
  color: var(--ts-text);
  font-size: 12px;
}
.search input::placeholder {
  color: var(--ts-text-2);
}
.entry-group {
  margin-top: 25px;
}
.entry-group h2 {
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.08em;
  color: var(--ts-text-2);
  margin: 0 0 5px 7px;
}
.entry-group ul {
  padding: 0;
  margin: 0;
}
.list-count {
  color: var(--ts-text-2);
  font-size: 10px;
  padding: 22px 7px 4px;
}
.empty {
  text-align: center;
  padding: 50px 10px;
  color: var(--ts-text-2);
  font-size: 13px;
}
.empty svg {
  width: 50px;
  height: 50px;
  opacity: 0.6;
}
.feedback {
  display: flex;
  gap: 12px;
  align-items: center;
  color: var(--ts-teal);
  background: var(--ts-focus);
  padding: 10px 12px;
  margin-bottom: 16px;
  border-radius: 7px;
  font-size: 12px;
}
.feedback span {
  flex: 1;
}
.feedback button {
  color: inherit;
  border: 0;
  background: none;
  cursor: pointer;
}
.order-list {
  list-style: none;
  padding: 0;
}
.order-list li {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 4px;
  border-bottom: 1px solid var(--ts-line);
  font-size: 13px;
}
.order-list li > span {
  flex: 1;
  overflow-wrap: anywhere;
}
.order-list small {
  color: var(--ts-text-2);
}
.order-hint {
  font-size: 12px;
  color: var(--ts-text-2);
  line-height: 1.8;
}
.order-actions {
  display: flex;
  justify-content: flex-end;
  gap: 9px;
}
@media (max-width: 600px) {
  .workspace {
    grid-template-columns: 112px minmax(0, 1fr);
  }
  .main-content {
    padding: 22px 18px;
  }
  .sidebar {
    padding-inline: 5px;
  }
  .nav-item {
    gap: 5px;
    font-size: 11px;
  }
  .nav-item small {
    display: none;
  }
  .brand {
    margin-inline: 3px;
  }
  .brand img {
    width: 24px;
    height: 24px;
  }
  .brand strong {
    font-size: 14px;
  }
  .page-header {
    flex-wrap: wrap;
  }
}
@media (max-width: 390px) {
  .workspace {
    grid-template-columns: 86px minmax(0, 1fr);
  }
  .nav-item svg {
    display: none;
  }
  .brand img {
    display: none;
  }
  .main-content {
    padding-inline: 12px;
  }
}
</style>
