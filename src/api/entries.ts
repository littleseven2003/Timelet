import { invoke } from '@tauri-apps/api/core';
import type { Entry } from '../types/entry';

export const listEntries = () => invoke<Entry[]>('entry_list');

export const saveEntry = (entry: Entry, expectedUpdatedAt?: string) =>
  invoke<void>('entry_save', { entry, expectedUpdatedAt });

export const deleteEntry = (id: string) => invoke<void>('entry_delete', { id });

/** 按给定顺序写入手动排序 */
export const reorderEntries = (ids: string[], reset = false) =>
  invoke<void>('entry_reorder', { ids, reset });
