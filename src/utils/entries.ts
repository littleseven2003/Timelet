import type { Entry } from '../types/entry';

// 解析 ISO 日期（YYYY-MM-DD）为本地时区的当日零点
function parseLocalDate(isoDate: string): Date {
  const [year, month, day] = isoDate.split('-').map(Number);
  return new Date(year!, month! - 1, day!);
}

// 今日零点
export function startOfToday(): Date {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth(), now.getDate());
}

// 两个自然日相差的天数（本地时区，按日历日而非 24 小时制）
export function daysBetween(from: Date, toIsoDate: string): number {
  const target = parseLocalDate(toIsoDate);
  return Math.round((target.getTime() - from.getTime()) / 86_400_000);
}

// 条目展示天数：倒计时为剩余天数（负数表示已过期），正计时为已过天数
export function entryDays(entry: Entry, today: Date = startOfToday()): number {
  const diff = daysBetween(today, entry.date);
  return entry.entryType === 'elapsed' ? -diff : diff;
}

// 分组序：0 置顶、1 未过期倒计时、2 正计时、3 已过期倒计时
function groupOf(entry: Entry): number {
  const days = entryDays(entry);
  if (entry.pinned) return 0;
  if (entry.entryType === 'countdown') return days >= 0 ? 1 : 3;
  return 2;
}

// 面板与配置列表共用排序：置顶优先 → 未过期倒计时按剩余升序 → 正计时按起始日期升序 → 已过期倒计时置底（最近过期在前）
export function sortEntries(entries: Entry[]): Entry[] {
  return [...entries].sort((a, b) => {
    const groupDiff = groupOf(a) - groupOf(b);
    if (groupDiff !== 0) return groupDiff;
    if (groupOf(a) === 0) return 0;
    const dateDiff = a.date.localeCompare(b.date);
    return groupOf(a) === 3 ? -dateDiff : dateDiff;
  });
}
