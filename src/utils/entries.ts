import type { DisplayUnit, Entry } from '../types/entry';

// 解析 ISO 日期（YYYY-MM-DD）为本地时区的当日零点
function parseLocalDate(isoDate: string): Date {
  const [year, month, day] = isoDate.split('-').map(Number);
  return new Date(year!, month! - 1, day!);
}

// 两个自然日之间的整月差（不足整月舍去）
function calendarMonthsBetween(from: Date, to: Date): number {
  let months = (to.getFullYear() - from.getFullYear()) * 12 + (to.getMonth() - from.getMonth());
  if (to.getDate() < from.getDate()) months -= 1;
  return months;
}

// 今日零点
export function startOfToday(): Date {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth(), now.getDate());
}

// 条目的生效时间点：带时刻为当日具体时分，纯日期为当日零点
export function entryDeadline(entry: Entry): Date {
  const base = parseLocalDate(entry.date);
  if (entry.time) {
    const [hour, minute] = entry.time.split(':').map(Number);
    base.setHours(hour!, minute!, 0, 0);
  }
  return base;
}

function isWorkday(date: Date): boolean {
  const weekday = date.getDay();
  return weekday >= 1 && weekday <= 5;
}

function atTime(day: Date, hour: number, minute: number): Date {
  const result = new Date(day);
  result.setHours(hour, minute, 0, 0);
  return result;
}

// 循环条目的下一次发生时间（每天/每个工作日），单次条目返回固定截止
export function effectiveDeadline(entry: Entry, nowMs: number): Date {
  if (!entry.time || !entry.repeat) return entryDeadline(entry);

  const [hour, minute] = entry.time.split(':').map(Number);
  let candidate = atTime(new Date(nowMs), hour!, minute!);
  if (entry.repeat === 'workday') {
    while (!isWorkday(candidate) || candidate.getTime() <= nowMs) {
      candidate = atTime(new Date(candidate.getTime() + 86_400_000), hour!, minute!);
    }
    return candidate;
  }
  if (candidate.getTime() <= nowMs) {
    candidate = atTime(new Date(candidate.getTime() + 86_400_000), hour!, minute!);
  }
  return candidate;
}

// 两个自然日相差的天数（本地时区，按日历日而非 24 小时制）
export function daysBetween(from: Date, toIsoDate: string): number {
  const target = parseLocalDate(toIsoDate);
  return Math.round((target.getTime() - from.getTime()) / 86_400_000);
}

// 条目展示天数：倒计时为剩余天数（负数表示已过期），正计时为已过天数；仅适用于纯日期条目
export function entryDays(entry: Entry, today: Date = startOfToday()): number {
  const diff = daysBetween(today, entry.date);
  return entry.entryType === 'elapsed' ? -diff : diff;
}

// 按展示单位换算条目数值（倒计时为剩余、正计时为已过；周向下取整、月/年按日历整月差）；仅纯日期条目
export function entryUnitValue(entry: Entry, unit: DisplayUnit): number | null {
  if (entry.time) return null;
  const days = entryDays(entry);
  if (unit === 'day') return days;
  if (unit === 'week') return Math.trunc(days / 7);

  const today = startOfToday();
  const from = entry.entryType === 'countdown' ? today : parseLocalDate(entry.date);
  const to = entry.entryType === 'countdown' ? parseLocalDate(entry.date) : today;
  const months = calendarMonthsBetween(from, to);
  return unit === 'month' ? months : Math.trunc(months / 12);
}

// 大数字预览用：返回当前展示单位的数值与单位（带时刻条目返回 null 走文本展示）
export function entryDisplayValue(entry: Entry): { value: number; unit: DisplayUnit } | null {
  const unit = entry.displayUnit ?? 'day';
  const value = entryUnitValue(entry, unit);
  return value == null ? null : { value, unit };
}

// 是否为已过期的倒计时条目：带时刻按时刻判定；纯日期在目标日全天有效，次日零点起算过期；循环条目永不过期
export function isExpiredCountdown(entry: Entry, nowMs: number = Date.now()): boolean {
  if (entry.entryType !== 'countdown') return false;
  if (entry.repeat) return false;
  const deadline = entryDeadline(entry).getTime();
  if (entry.time) return deadline < nowMs;
  return deadline + 86_400_000 <= nowMs;
}

// 展示文案翻译函数的最小形状（兼容 vue-i18n 的 t）
type Translate = (key: string, params?: Record<string, number>) => string;

// 条目展示文案：天数（今天/剩余/已过期/已过）、时刻精确间隔或按展示单位换算，供面板与编辑预览共用；
// 单位换算结果不足 1 时回退到按天展示
export function formatEntryText(entry: Entry, nowMs: number, t: Translate): string {
  if (!entry.date) return '';
  if (entry.time) return timedText(entry, nowMs, t);

  const days = entryDays(entry);
  const unit = entry.displayUnit ?? 'day';

  if (entry.entryType === 'elapsed') {
    // 起始日在未来：尚未开始，保留日期并说明剩余天数
    if (days < 0) return t('panel.notStarted', { days: -days });
    if (unit !== 'day') {
      const value = entryUnitValue(entry, unit) ?? 0;
      if (value > 0) return t(`panel.elapsedUnits.${unit}`, { n: value });
    }
    return t('panel.elapsed', { days });
  }

  if (days === 0) return t('panel.today');

  if (days > 0) {
    if (unit !== 'day') {
      const value = entryUnitValue(entry, unit) ?? 0;
      if (value > 0) return t(`panel.units.${unit}`, { n: value });
    }
    return t('panel.daysLeft', { days });
  }

  if (unit !== 'day') {
    const value = -(entryUnitValue(entry, unit) ?? 0);
    if (value > 0) return t(`panel.expiredUnits.${unit}`, { n: value });
  }
  return t('panel.expired', { days: -days });
}

// 带时刻条目按精确间隔展示（循环条目对齐下一次发生）：天+小时 → 小时+分 → 分钟
function timedText(entry: Entry, nowMs: number, t: Translate): string {
  const deadline = effectiveDeadline(entry, nowMs).getTime();
  const diffMinutes = Math.round((deadline - nowMs) / 60_000);
  const expired = diffMinutes < 0;
  const abs = Math.abs(diffMinutes);
  const days = Math.floor(abs / 1440);
  const hours = Math.floor((abs % 1440) / 60);
  const minutes = abs % 60;

  if (days > 0) {
    return t(expired ? 'panel.ago.daysHours' : 'panel.left.daysHours', { d: days, h: hours });
  }
  if (hours > 0) {
    return t(expired ? 'panel.ago.hoursMinutes' : 'panel.left.hoursMinutes', {
      h: hours,
      m: minutes,
    });
  }
  if (minutes > 0) {
    return t(expired ? 'panel.ago.minutes' : 'panel.left.minutes', { m: minutes });
  }
  return expired ? t('panel.expiredOnly') : t('panel.soon');
}


// 条目分区：此时（今天内到期）/ 将至（未来）/ 历时（正数日）/ 已过期（倒数日过期）
export type EntrySection = 'now' | 'soon' | 'elapsed' | 'past';

// 今天零点与明日零点（毫秒），供分区与分组判定
function dayBounds(nowMs: number) {
  const start = new Date(nowMs);
  const today0 = new Date(start.getFullYear(), start.getMonth(), start.getDate()).getTime();
  return { today0, tomorrow0: today0 + 86_400_000 };
}

export function sectionOf(entry: Entry, nowMs: number = Date.now()): EntrySection {
  if (entry.entryType === 'elapsed') return 'elapsed';
  const { today0, tomorrow0 } = dayBounds(nowMs);

  if (entry.repeat) {
    // 循环条目永不过期，按下一次发生是否在今天内判定
    const next = effectiveDeadline(entry, nowMs).getTime();
    return next < tomorrow0 ? 'now' : 'soon';
  }

  const dl = entryDeadline(entry).getTime();
  if (entry.time) {
    // 带时刻条目以当前时刻判定是否已过期
    return dl >= nowMs ? (dl < tomorrow0 ? 'now' : 'soon') : 'past';
  }
  // 纯日期条目按自然日判定
  return dl >= today0 ? (dl < tomorrow0 ? 'now' : 'soon') : 'past';
}

// 分区内排序：置顶优先 → 手动顺序（若存在）→ 截止时间
function compareWithin(a: Entry, b: Entry): number {
  if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
  const ma = a.sortIndex ?? Number.MAX_SAFE_INTEGER;
  const mb = b.sortIndex ?? Number.MAX_SAFE_INTEGER;
  if (ma !== mb) return ma - mb;
  return entryDeadline(a).getTime() - entryDeadline(b).getTime();
}

// 面板分组：此时 → 将至 → 历时 → 已过期（已过滤条目不传入）
export function groupedEntries(
  entries: Entry[],
  nowMs: number = Date.now(),
): { key: EntrySection; items: Entry[] }[] {
  const buckets: Record<EntrySection, Entry[]> = { now: [], soon: [], elapsed: [], past: [] };
  for (const entry of entries) buckets[sectionOf(entry, nowMs)].push(entry);
  const order: EntrySection[] = ['now', 'soon', 'elapsed', 'past'];
  return order
    .map((key) => ({ key, items: buckets[key].sort(compareWithin) }))
    .filter((group) => group.items.length > 0);
}

// 主窗口分组（设计文档 5.2）：今天 / 接下来 7 天 / 更晚（含正数日与已过期）
export type ConfigGroup = 'now' | 'week' | 'later';

export function groupForConfig(
  entries: Entry[],
  nowMs: number = Date.now(),
): { key: ConfigGroup; items: Entry[] }[] {
  const { tomorrow0 } = dayBounds(nowMs);
  const weekEnd = tomorrow0 + 6 * 86_400_000;
  const buckets: Record<ConfigGroup, Entry[]> = { now: [], week: [], later: [] };

  for (const entry of entries) {
    const section = sectionOf(entry, nowMs);
    if (section === 'now') {
      buckets.now.push(entry);
    } else if (section === 'soon') {
      (effectiveDeadline(entry, nowMs).getTime() < weekEnd ? buckets.week : buckets.later).push(
        entry,
      );
    } else {
      buckets.later.push(entry);
    }
  }

  const order: ConfigGroup[] = ['now', 'week', 'later'];
  return order
    .map((key) => ({ key, items: buckets[key].sort(compareWithin) }))
    .filter((group) => group.items.length > 0);
}

// 分组序：0 置顶、1 未到期倒计时、2 正计时、3 已过期倒计时；带时刻条目以当前时刻判定
function groupOf(entry: Entry, nowMs: number): number {
  if (entry.pinned) return 0;
  if (entry.entryType === 'countdown') {
    return effectiveDeadline(entry, nowMs).getTime() >= nowMs ? 1 : 3;
  }
  return 2;
}

// 排序：任一条目存在手动顺序时，整体按"置顶优先 + 手动顺序"；否则按自动规则：
// 未到期倒计时按截止升序 → 正计时按起始升序 → 已过期倒计时置底（最近过期在前）
export function sortEntries(entries: Entry[]): Entry[] {
  const manual = entries.some((entry) => entry.sortIndex != null);
  if (manual) {
    return [...entries].sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return orderValue(a) - orderValue(b);
    });
  }

  const now = Date.now();
  return [...entries].sort((a, b) => {
    const groupDiff = groupOf(a, now) - groupOf(b, now);
    if (groupDiff !== 0) return groupDiff;
    const timeDiff = effectiveDeadline(a, now).getTime() - effectiveDeadline(b, now).getTime();
    return groupOf(a, now) === 3 ? -timeDiff : timeDiff;
  });
}

// 手动顺序值，无手动顺序的条目排在末尾
function orderValue(entry: Entry): number {
  return entry.sortIndex ?? Number.MAX_SAFE_INTEGER;
}
