import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import type { Entry } from '../types/entry';
import { deleteEntry, listEntries, reorderEntries, saveEntry } from '../api/entries';

// 模块级共享状态：同一窗口内的多个组件使用同一份数据
const entries = ref<Entry[]>([]);
const loaded = ref(false);
const loading = ref(false);

async function reload() {
  loading.value = true;
  try {
    entries.value = await listEntries();
    loaded.value = true;
  } finally {
    loading.value = false;
  }
}

// 其他窗口（如配置窗口）修改数据后，Rust 侧广播 entries-changed 事件
let listening = false;
async function ensureChangeListener() {
  if (listening) return;
  listening = true;
  await listen('entries-changed', () => {
    void reload();
  });
}

export function useEntries() {
  async function upsert(entry: Entry) {
    await saveEntry(entry);
    await reload();
  }

  async function remove(id: string) {
    await deleteEntry(id);
    await reload();
  }

  async function reorder(ids: string[]) {
    await reorderEntries(ids);
    await reload();
  }

  return { entries, loaded, loading, reload, upsert, remove, reorder, ensureChangeListener };
}

// 由条目字段构造一条新记录的默认值（id 与时间戳在此生成）
export function createDraft(now = new Date()): Entry {
  return {
    id: crypto.randomUUID(),
    name: '',
    entryType: 'countdown',
    date: '',
    color: '#2a9cdb',
    pinned: false,
    createdAt: now.toISOString(),
    updatedAt: now.toISOString(),
  };
}
