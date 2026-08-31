export type EntryType = 'countdown' | 'elapsed';

/** 条目天数展示单位 */
export type DisplayUnit = 'day' | 'week' | 'month' | 'year';

/** 循环规则（仅带时刻的条目生效） */
export type RepeatRule = 'daily' | 'workday';

export interface Entry {
  id: string;
  name: string;
  /** countdown：距目标日；elapsed：自起始日已过 */
  entryType: EntryType;
  /** ISO 日期（YYYY-MM-DD） */
  date: string;
  /** 可选时刻（HH:mm），缺失表示纯日期条目 */
  time?: string;
  /** 循环规则，缺失表示单次 */
  repeat?: RepeatRule;
  /** 展示单位，缺失按天 */
  displayUnit?: DisplayUnit;
  /** 选填备注，面板悬停时展示 */
  note?: string;
  /** 手动排序值（拖拽后生成），缺失表示按自动规则排序 */
  sortIndex?: number;
  /** 归档后离开活动视图，可恢复；缺失表示活动条目 */
  archived?: boolean;
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
}
