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

// 条目的生效时间点：带时刻为当日具体时分，纯日期为当日零点
export function entryDeadline(entry: Entry): Date {
  const base = parseLocalDate(entry.date);
  if (entry.time) {
    const [hour, minute] = entry.time.split(':').map(Number);
    base.setHours(hour!, minute!, 0, 0);
  }
  return base;
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

// 是否为已过期的倒计时条目（带时刻按时刻判定，纯日期按自然日判定）
export function isExpiredCountdown(entry: Entry, nowMs: number = Date.now()): boolean {
  if (entry.entryType !== 'countdown') return false;
  return entryDeadline(entry).getTime() < nowMs;
}

// 分组序：0 置顶、1 未到期倒计时、2 正计时、3 已过期倒计时；带时刻条目以当前时刻判定
function groupOf(entry: Entry, now: number): number {
  if (entry.pinned) return 0;
  if (entry.entryType === 'countdown') {
    return entryDeadline(entry).getTime() >= now ? 1 : 3;
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
    const timeDiff = entryDeadline(a).getTime() - entryDeadline(b).getTime();
    return groupOf(a, now) === 3 ? -timeDiff : timeDiff;
  });
}

// 手动顺序值，无手动顺序的条目排在末尾
function orderValue(entry: Entry): number {
  return entry.sortIndex ?? Number.MAX_SAFE_INTEGER;
}
