import { invoke } from '@tauri-apps/api/core';
import type { Entry } from '../types/entry';

export const listEntries = () => invoke<Entry[]>('entry_list');

export const saveEntry = (entry: Entry) => invoke<void>('entry_save', { entry });

export const deleteEntry = (id: string) => invoke<void>('entry_delete', { id });
