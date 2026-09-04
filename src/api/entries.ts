import { invoke } from '@tauri-apps/api/core';
import type { Entry } from '../types/entry';

export interface EntrySnapshot {
  entries: Entry[];
  nearIsleEntryId?: string;
}

export const listEntries = () => invoke<EntrySnapshot>('entry_list');

export const saveEntry = (entry: Entry, expectedUpdatedAt?: string, nearIsle?: boolean) =>
  invoke<void>('entry_save', { entry, expectedUpdatedAt, nearIsle });

export const deleteEntry = (id: string) => invoke<void>('entry_delete', { id });

export const setNearIsleEntry = (id?: string) =>
  invoke<void>('entry_set_near_isle', { id: id ?? null });

/** 按给定顺序写入手动排序 */
export const reorderEntries = (ids: string[], reset = false) =>
  invoke<void>('entry_reorder', { ids, reset });
