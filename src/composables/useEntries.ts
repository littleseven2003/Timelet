import { ref } from 'vue';
import type { Entry } from '../types/entry';
import { deleteEntry, listEntries, saveEntry } from '../api/entries';

// 模块级共享状态：同一窗口内的多个组件使用同一份数据
const entries = ref<Entry[]>([]);
const loaded = ref(false);
const loading = ref(false);

export function useEntries() {
  async function reload() {
    loading.value = true;
    try {
      entries.value = await listEntries();
      loaded.value = true;
    } finally {
      loading.value = false;
    }
  }

  async function upsert(entry: Entry) {
    await saveEntry(entry);
    await reload();
  }

  async function remove(id: string) {
    await deleteEntry(id);
    await reload();
  }

  return { entries, loaded, loading, reload, upsert, remove };
}

// 由条目字段构造一条新记录的默认值（id 与时间戳在此生成）
export function createDraft(now = new Date()): Entry {
  return {
    id: crypto.randomUUID(),
    name: '',
    entryType: 'countdown',
    date: '',
    color: '#0091ff',
    pinned: false,
    createdAt: now.toISOString(),
    updatedAt: now.toISOString(),
  };
}
