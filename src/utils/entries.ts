import type { DisplayUnit, Entry } from '../types/entry';

const DAY = 86_400_000;
export function parseLocalDate(iso: string): Date {
  const [year, month, day] = iso.split('-').map(Number);
  const result = new Date(0);
  result.setFullYear(year!, month! - 1, day!);
  result.setHours(0, 0, 0, 0);
  return result;
}

export function isValidDate(iso: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(iso) || Number(iso.slice(0, 4)) < 1) return false;
  const date = parseLocalDate(iso);
  return Number.isFinite(date.getTime()) && dateIso(date) === iso;
}

export function dateIso(date: Date): string {
  return `${String(date.getFullYear()).padStart(4, '0')}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

export function startOfToday(nowMs = Date.now()): Date {
  const day = new Date(nowMs);
  day.setHours(0, 0, 0, 0);
  return day;
}

function addDays(date: Date, days: number): Date {
  const result = new Date(date);
  result.setDate(result.getDate() + days);
  return result;
}

// 以本地年月日构造日历序号，避免夏令时的 23/25 小时影响计日。
function ordinal(date: Date): number {
  const day = new Date(0);
  day.setUTCFullYear(date.getFullYear(), date.getMonth(), date.getDate());
  day.setUTCHours(0, 0, 0, 0);
  return day.getTime() / DAY;
}

export function daysBetween(from: Date, toIso: string): number {
  return ordinal(parseLocalDate(toIso)) - ordinal(from);
}

export function entryDeadline(entry: Entry): Date {
  const date = parseLocalDate(entry.date);
  if (entry.time) {
    const [hour, minute] = entry.time.split(':').map(Number);
    date.setHours(hour!, minute!, 0, 0);
  }
  return date;
}

// 从当前日与起始日的较晚者开始；日历递增避免夏令时回拨时停留在同一天。
export function effectiveDeadline(entry: Entry, nowMs: number): Date {
  const anchor = entryDeadline(entry);
  if (entry.entryType !== 'countdown' || !entry.time || !entry.repeat) return anchor;
  const [hour, minute] = entry.time.split(':').map(Number);
  let candidate = startOfToday(Math.max(anchor.getTime(), nowMs));
  candidate.setHours(hour!, minute!, 0, 0);
  while (
    candidate.getTime() < nowMs ||
    candidate.getTime() < anchor.getTime() ||
    (entry.repeat === 'workday' && (candidate.getDay() === 0 || candidate.getDay() === 6))
  ) {
    candidate = addDays(candidate, 1);
    candidate.setHours(hour!, minute!, 0, 0);
  }
  return candidate;
}

export function effectiveDateIso(entry: Entry, nowMs = Date.now()): string {
  return dateIso(effectiveDeadline(entry, nowMs));
}

export function effectiveTime(entry: Entry, nowMs: number): string {
  const date = effectiveDeadline(entry, nowMs);
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
}

export function entryDays(entry: Entry, today = startOfToday()): number {
  const days = daysBetween(today, entry.date);
  return entry.entryType === 'elapsed' ? -days : days;
}

function calendarMonthsBetween(from: Date, to: Date): number {
  if (from > to) return -calendarMonthsBetween(to, from);
  let months = (to.getFullYear() - from.getFullYear()) * 12 + to.getMonth() - from.getMonth();
  if (to.getDate() < from.getDate()) months--;
  return months;
}

export function entryUnitValue(entry: Entry, unit: DisplayUnit, nowMs = Date.now()): number | null {
  if (entry.time) return null;
  const today = startOfToday(nowMs);
  const days = entryDays(entry, today);
  if (unit === 'day') return days;
  if (unit === 'week') return Math.trunc(days / 7);
  const months =
    calendarMonthsBetween(today, parseLocalDate(entry.date)) *
    (entry.entryType === 'elapsed' ? -1 : 1);
  return (unit === 'month' ? months : Math.trunc(months / 12)) || 0;
}

export function entryDisplayValue(
  entry: Entry,
  nowMs = Date.now(),
): { value: number; unit: DisplayUnit } | null {
  const unit = entry.displayUnit ?? 'day';
  const value = entryUnitValue(entry, unit, nowMs);
  return value == null ? null : { value, unit };
}

export function isExpiredCountdown(entry: Entry, nowMs = Date.now()): boolean {
  if (entry.entryType !== 'countdown' || (entry.time && entry.repeat)) return false;
  return entry.time
    ? entryDeadline(entry).getTime() < nowMs
    : addDays(entryDeadline(entry), 1).getTime() <= nowMs;
}

type Translate = (key: string, params?: Record<string, string | number>) => string;
export function formatEntryText(entry: Entry, nowMs: number, t: Translate): string {
  if (!isValidDate(entry.date)) return '';
  if (entry.time) return timedText(entry, nowMs, t);
  const days = entryDays(entry, startOfToday(nowMs));
  const unit = entry.displayUnit ?? 'day';
  const value = entryUnitValue(entry, unit, nowMs) ?? 0;
  if (entry.entryType === 'elapsed') {
    if (days < 0) return t('panel.notStarted', { days: -days });
    if (unit !== 'day' && value > 0) return t(`panel.elapsedUnits.${unit}`, { n: value });
    return t('panel.elapsed', { days });
  }
  if (days === 0) return t('panel.today');
  if (unit !== 'day' && Math.abs(value) > 0) {
    return t(`panel.${days > 0 ? 'units' : 'expiredUnits'}.${unit}`, { n: Math.abs(value) });
  }
  return t(days > 0 ? 'panel.daysLeft' : 'panel.expired', { days: Math.abs(days) });
}

function compactDuration(unit: DisplayUnit, value: number, t: Translate): string {
  return t(`panel.compact.duration.${unit}`, { n: value });
}

function compactTimedDuration(diff: number, t: Translate): string {
  const minutesTotal = Math.floor(Math.abs(diff) / 60_000);
  const days = Math.floor(minutesTotal / 1440);
  const hours = Math.floor((minutesTotal % 1440) / 60);
  const minutes = minutesTotal % 60;
  if (days > 0) return t('panel.compact.duration.daysHours', { d: days, h: hours });
  if (hours > 0) return t('panel.compact.duration.hoursMinutes', { h: hours, m: minutes });
  if (minutes > 0) return t('panel.compact.duration.minutes', { m: minutes });
  return '';
}

// 快捷面板只保留辨认条目所需的最短时间语义，完整叙述留在主窗口。
export function formatCompactEntryText(entry: Entry, nowMs: number, t: Translate): string {
  if (!isValidDate(entry.date)) return '';
  if (entry.time) {
    const diff = effectiveDeadline(entry, nowMs).getTime() - nowMs;
    const duration = compactTimedDuration(diff, t);
    if (!duration) {
      if (entry.entryType === 'elapsed') {
        return t(diff > 0 ? 'panel.compact.soon' : 'panel.compact.justStarted');
      }
      return t(diff < 0 ? 'panel.compact.expiredOnly' : 'panel.compact.soon');
    }
    if (entry.entryType === 'elapsed') {
      return t(diff > 0 ? 'panel.compact.after' : 'panel.compact.since', { duration });
    }
    return diff < 0 ? t('panel.compact.overdue', { duration }) : duration;
  }

  const days = entryDays(entry, startOfToday(nowMs));
  if (entry.entryType === 'countdown' && days === 0) return t('panel.compact.today');
  if (entry.entryType === 'elapsed' && days === 0) return t('panel.compact.justStarted');
  const preferredUnit = entry.displayUnit ?? 'day';
  const preferredValue = entryUnitValue(entry, preferredUnit, nowMs) ?? 0;
  const unit = preferredUnit !== 'day' && Math.abs(preferredValue) > 0 ? preferredUnit : 'day';
  const value = unit === 'day' ? Math.abs(days) : Math.abs(preferredValue);
  const duration = compactDuration(unit, value, t);
  if (entry.entryType === 'elapsed') {
    return t(days < 0 ? 'panel.compact.after' : 'panel.compact.since', { duration });
  }
  return days < 0 ? t('panel.compact.overdue', { duration }) : duration;
}

export function formatCompactEntryMeta(entry: Entry, nowMs: number, t: Translate): string {
  if (!isValidDate(entry.date)) return '';
  if (entry.repeat && entry.time && entry.entryType === 'countdown') {
    return `${t(`config.repeat.${entry.repeat}`)} · ${effectiveTime(entry, nowMs)}`;
  }
  const date = effectiveDeadline(entry, nowMs);
  const now = new Date(nowMs);
  const key =
    date.getFullYear() === now.getFullYear()
      ? 'panel.compact.monthDay'
      : 'panel.compact.yearMonthDay';
  const dateText = t(key, {
    year: date.getFullYear(),
    month: date.getMonth() + 1,
    day: date.getDate(),
  });
  return entry.time ? `${dateText} · ${effectiveTime(entry, nowMs)}` : dateText;
}

function timedText(entry: Entry, nowMs: number, t: Translate): string {
  const diff = effectiveDeadline(entry, nowMs).getTime() - nowMs;
  const abs = Math.floor(Math.abs(diff) / 60_000);
  const days = Math.floor(abs / 1440);
  const hours = Math.floor((abs % 1440) / 60);
  const minutes = abs % 60;
  const prefix =
    entry.entryType === 'elapsed'
      ? diff > 0
        ? 'notStartedTime'
        : 'elapsedTime'
      : diff < 0
        ? 'ago'
        : 'left';
  if (days > 0) return t(`panel.${prefix}.daysHours`, { d: days, h: hours });
  if (hours > 0) return t(`panel.${prefix}.hoursMinutes`, { h: hours, m: minutes });
  if (minutes > 0) return t(`panel.${prefix}.minutes`, { m: minutes });
  if (entry.entryType === 'elapsed')
    return t(diff > 0 ? 'panel.notStartedSoon' : 'panel.justStarted');
  return t(diff < 0 ? 'panel.expiredOnly' : 'panel.soon');
}

export type EntrySection = 'now' | 'soon' | 'elapsed' | 'later' | 'past';
const SECTION_ORDER: EntrySection[] = ['now', 'soon', 'elapsed', 'later', 'past'];
export function sectionOf(entry: Entry, nowMs = Date.now()): EntrySection {
  if (entry.entryType === 'elapsed') return 'elapsed';
  if (isExpiredCountdown(entry, nowMs)) return 'past';
  const days = daysBetween(new Date(nowMs), effectiveDateIso(entry, nowMs));
  return days <= 0 ? 'now' : days <= 7 ? 'soon' : 'later';
}

function compareWithin(a: Entry, b: Entry, nowMs: number): number {
  if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
  const manual =
    (a.sortIndex ?? Number.MAX_SAFE_INTEGER) - (b.sortIndex ?? Number.MAX_SAFE_INTEGER);
  if (manual) return manual;
  const diff = effectiveDeadline(a, nowMs).getTime() - effectiveDeadline(b, nowMs).getTime();
  return (sectionOf(a, nowMs) === 'past' ? -diff : diff) || a.id.localeCompare(b.id);
}

export function groupedEntries(
  entries: Entry[],
  nowMs = Date.now(),
): { key: EntrySection; items: Entry[] }[] {
  return SECTION_ORDER.map((key) => ({
    key,
    items: entries
      .filter((entry) => sectionOf(entry, nowMs) === key)
      .sort((a, b) => compareWithin(a, b, nowMs)),
  })).filter((group) => group.items.length > 0);
}

export type ConfigGroup = 'now' | 'week' | 'elapsed' | 'later' | 'past';
export function groupForConfig(
  entries: Entry[],
  nowMs = Date.now(),
): { key: ConfigGroup; items: Entry[] }[] {
  return groupedEntries(entries, nowMs).map((group) => ({
    ...group,
    key: group.key === 'soon' ? 'week' : group.key,
  }));
}

export function sortEntries(entries: Entry[], nowMs = Date.now()): Entry[] {
  const manual = entries.some((entry) => entry.sortIndex != null);
  return [...entries].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    if (!manual) {
      const section =
        SECTION_ORDER.indexOf(sectionOf(a, nowMs)) - SECTION_ORDER.indexOf(sectionOf(b, nowMs));
      if (section) return section;
    }
    return compareWithin(a, b, nowMs);
  });
}

// 只有有效、非循环、未过期的置顶倒数日可以承载真实进度。
export function entryProgress(
  entry: Entry,
  nowMs: number,
): { progress: number; days: number; start: string } | null {
  if (
    entry.archived ||
    entry.entryType !== 'countdown' ||
    entry.repeat ||
    isExpiredCountdown(entry, nowMs)
  )
    return null;
  const start = new Date(entry.createdAt);
  if (!Number.isFinite(start.getTime()) || !isValidDate(entry.date)) return null;
  const span = entry.time
    ? entryDeadline(entry).getTime() - start.getTime()
    : daysBetween(start, entry.date);
  const passed = entry.time ? nowMs - start.getTime() : ordinal(new Date(nowMs)) - ordinal(start);
  if (!Number.isFinite(span) || span <= 0) return null;
  return {
    progress: Math.max(0, Math.min(1, passed / span)),
    days: daysBetween(new Date(nowMs), entry.date),
    start: dateIso(start),
  };
}

export function featuredEntry(entries: Entry[], nowMs: number): Entry | null {
  return (
    sortEntries(
      entries.filter((entry) => entry.pinned && entryProgress(entry, nowMs)),
      nowMs,
    )[0] ?? null
  );
}

export function panelSelection(entries: Entry[], nowMs: number, showExpired: boolean, limit = 6) {
  const visible = entries.filter(
    (entry) => !entry.archived && (showExpired || !isExpiredCountdown(entry, nowMs)),
  );
  const featured = featuredEntry(visible, nowMs);
  const rest = visible.filter((entry) => entry.id !== featured?.id);
  const cap = Math.max(5, Math.min(8, Number.isFinite(limit) ? Math.trunc(limit) : 6));
  const selected = groupedEntries(rest, nowMs)
    .flatMap((group) => group.items)
    .slice(0, cap - (featured ? 1 : 0));
  return { featured, groups: groupedEntries(selected, nowMs) };
}
