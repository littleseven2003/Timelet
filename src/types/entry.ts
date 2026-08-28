export type EntryType = 'countdown' | 'elapsed';

export interface Entry {
  id: string;
  name: string;
  /** countdown：距目标日；elapsed：自起始日已过 */
  entryType: EntryType;
  /** ISO 日期（YYYY-MM-DD） */
  date: string;
  /** 可选时刻（HH:mm），缺失表示纯日期条目 */
  time?: string;
  /** 手动排序值（拖拽后生成，M3-2 启用），缺失表示按自动规则排序 */
  sortIndex?: number;
  color: string;
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
}

/** 预设色板（8 色），新增颜色需同步设计文档 */
export const ENTRY_COLORS = [
  '#e5484d',
  '#f76b15',
  '#ffb224',
  '#30a46c',
  '#12a594',
  '#0091ff',
  '#6e56cf',
  '#d6409f',
] as const;
