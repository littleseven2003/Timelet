import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Entry } from '../types/entry';
import {
  deleteEntry,
  listEntries,
  reorderEntries,
  saveEntry,
  setNearIsleEntry,
} from '../api/entries';

const entries = ref<Entry[]>([]);
const nearIsleEntryId = ref<string | null>(null);
const loaded = ref(false);
const loading = ref(false);
const loadError = ref('');
const syncError = ref('');
const error = computed(() => loadError.value || syncError.value);
const busy = ref(false);
let request = 0;
let users = 0;
let listening: Promise<void> | undefined;
let unlisten: UnlistenFn | undefined;

async function reload() {
  const version = ++request;
  loading.value = true;
  try {
    const result = await listEntries();
    if (version !== request) return;
    entries.value = result.entries;
    nearIsleEntryId.value = result.nearIsleEntryId ?? null;
    loaded.value = true;
    loadError.value = '';
  } catch (cause) {
    if (version === request) loadError.value = String(cause);
  } finally {
    if (version === request) loading.value = false;
  }
}

async function ensureChangeListener() {
  if (unlisten) return;
  if (listening) return listening;
  listening = listen('entries-changed', () => {
    void reload();
  })
    .then((stop) => {
      syncError.value = '';
      if (users) unlisten = stop;
      else stop();
    })
    .catch((cause) => {
      syncError.value = String(cause);
    })
    .finally(() => {
      listening = undefined;
    });
  return listening;
}

async function retry() {
  await ensureChangeListener();
  await reload();
}
async function mutate(action: () => Promise<void>) {
  if (busy.value) throw new Error('操作正在保存，请稍候');
  busy.value = true;
  try {
    await action();
    await reload();
  } catch (cause) {
    await reload();
    throw cause;
  } finally {
    busy.value = false;
  }
}

export function useEntries() {
  onMounted(() => {
    if (users++ === 0) {
      void retry();
      window.addEventListener('focus', retry);
    }
  });
  onBeforeUnmount(() => {
    if (--users === 0) {
      unlisten?.();
      unlisten = undefined;
      window.removeEventListener('focus', retry);
    }
  });
  const upsert = (
    entry: Entry,
    expectedUpdatedAt = entries.value.find((item) => item.id === entry.id)?.updatedAt,
    nearIsle?: boolean,
  ) => {
    const previous = Date.parse(expectedUpdatedAt ?? '');
    const updatedAt = new Date(
      Math.max(Date.now(), Number.isFinite(previous) ? previous + 1 : 0),
    ).toISOString();
    return mutate(() => saveEntry({ ...entry, updatedAt }, expectedUpdatedAt, nearIsle));
  };
  const remove = (id: string) => mutate(() => deleteEntry(id));
  const setNearIsle = (id?: string) => mutate(() => setNearIsleEntry(id));
  const reorder = (ids: string[], reset = false) => mutate(() => reorderEntries(ids, reset));
  const setArchive = async (id: string, archived: boolean) => {
    const target = entries.value.find((entry) => entry.id === id);
    if (target) await upsert({ ...target, archived, updatedAt: new Date().toISOString() });
  };
  const duplicate = async (id: string, suffix = '（副本）') => {
    const target = entries.value.find((entry) => entry.id === id);
    if (!target) return;
    const now = new Date().toISOString();
    await upsert({
      ...target,
      id: crypto.randomUUID(),
      name: `${target.name.slice(0, 30 - suffix.length)}${suffix}`,
      pinned: false,
      archived: false,
      sortIndex: undefined,
      createdAt: now,
      updatedAt: now,
    });
  };
  return {
    entries,
    nearIsleEntryId,
    loaded,
    loading,
    error,
    busy,
    reload: retry,
    upsert,
    remove,
    setNearIsle,
    reorder,
    archive: (id: string) => setArchive(id, true),
    restore: (id: string) => setArchive(id, false),
    duplicate,
    ensureChangeListener,
  };
}

export function createDraft(now = new Date()): Entry {
  return {
    id: crypto.randomUUID(),
    name: '',
    entryType: 'countdown',
    date: '',
    pinned: false,
    createdAt: now.toISOString(),
    updatedAt: now.toISOString(),
  };
}
