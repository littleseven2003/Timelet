import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import process from 'node:process';
import ts from 'typescript';

// 使用项目已有编译器，使回归用例也能在 Node 20 上运行。
const source = readFileSync(new URL('./entries.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 },
});
const u = await import(`data:text/javascript,${encodeURIComponent(outputText)}`);
const entry = (extra = {}) => ({
  id: 'a',
  name: '日期',
  entryType: 'countdown',
  date: '2026-09-07',
  pinned: false,
  createdAt: '2026-08-01T12:00:00+08:00',
  updatedAt: '2026-08-01T12:00:00+08:00',
  ...extra,
});
const translate = (key, params) => ({ key, ...params });
const compactMessages = {
  'config.repeat.daily': '每天',
  'config.repeat.workday': '周一至周五',
  'panel.compact.today': '今天',
  'panel.compact.soon': '即将',
  'panel.compact.expiredOnly': '已到时',
  'panel.compact.justStarted': '刚刚',
  'panel.compact.since': '已{duration}',
  'panel.compact.after': '{duration}后',
  'panel.compact.overdue': '超{duration}',
  'panel.compact.monthDay': '{month}月{day}日',
  'panel.compact.yearMonthDay': '{year}年{month}月{day}日',
  'panel.compact.duration.day': '{n}天',
  'panel.compact.duration.week': '{n}周',
  'panel.compact.duration.month': '{n}个月',
  'panel.compact.duration.year': '{n}年',
  'panel.compact.duration.daysHours': '{d}天{h}时',
  'panel.compact.duration.hoursMinutes': '{h}时{m}分',
  'panel.compact.duration.minutes': '{m}分',
};
const compactTranslate = (key, params = {}) =>
  compactMessages[key].replace(/\{(\w+)\}/g, (_, name) => String(params[name]));

for (const timezone of ['Asia/Shanghai', 'America/New_York']) {
  test(`${timezone}：自然日、七天边界和跨日文案共用传入时钟`, () => {
    process.env.TZ = timezone;
    const now = new Date('2026-08-31T23:59:00').getTime();
    assert.equal(u.isExpiredCountdown(entry({ date: '2026-08-31' }), now), false);
    assert.equal(u.sectionOf(entry({ date: '2026-09-07' }), now), 'soon');
    assert.equal(u.sectionOf(entry({ date: '2026-09-08' }), now), 'later');
    assert.equal(u.sectionOf(entry({ entryType: 'elapsed' }), now), 'elapsed');
    assert.deepEqual(u.formatEntryText(entry({ date: '2026-09-01' }), now, translate), {
      key: 'panel.daysLeft',
      days: 1,
    });
    assert.deepEqual(u.formatEntryText(entry({ date: '2026-09-01' }), now + 120000, translate), {
      key: 'panel.today',
    });
    assert.deepEqual(u.formatEntryText(entry({ entryType: 'elapsed' }), now, translate), {
      key: 'panel.notStarted',
      days: 7,
    });
  });
}

test('夏令时回拨不提前过期或卡住循环，春季跳时按本地时区顺延', () => {
  process.env.TZ = 'America/New_York';
  const autumn = new Date('2026-11-01T23:30:00').getTime();
  assert.equal(u.isExpiredCountdown(entry({ date: '2026-11-01' }), autumn), false);
  const loop = entry({ date: '2026-01-01', time: '00:30', repeat: 'daily' });
  assert.equal(u.effectiveDateIso(loop, new Date('2026-11-01T01:00:00').getTime()), '2026-11-02');
  const spring = new Date('2026-03-08T00:00:00').getTime();
  assert.equal(u.daysBetween(new Date(spring), '2026-03-09'), 1);
  assert.equal(
    u.effectiveTime(entry({ date: '2026-03-01', time: '02:30', repeat: 'daily' }), spring),
    '03:30',
  );
  assert.equal(
    u
      .effectiveDeadline(entry({ date: '2026-03-01', time: '02:30', repeat: 'daily' }), spring)
      .getHours(),
    3,
  );
});

test('未来循环保留锚点，周末顺延，确切日期与相对值一致', () => {
  process.env.TZ = 'Asia/Shanghai';
  const now = new Date('2026-08-31T12:00:00').getTime();
  const loop = entry({ date: '2036-09-06', time: '09:00', repeat: 'workday' });
  const next = u.effectiveDeadline(loop, now);
  assert.ok(next >= u.entryDeadline(loop));
  assert.ok(next.getDay() >= 1 && next.getDay() <= 5);
  const exact = entry({ date: '2026-08-31', time: '12:00', repeat: 'daily' });
  assert.equal(u.effectiveDeadline(exact, now).getTime(), now);
  assert.equal(u.effectiveDateIso(exact, now + 1), '2026-09-01');
});

test('每日 17:00 在到达后推进到次日，单次条目保持过期', () => {
  process.env.TZ = 'Asia/Shanghai';
  const daily = entry({ date: '2026-09-01', time: '17:00', repeat: 'daily' });
  const before = new Date('2026-09-01T09:30:00').getTime();
  assert.equal(u.effectiveDateIso(daily, before), '2026-09-01');
  assert.deepEqual(u.formatEntryText(daily, before, translate), {
    key: 'panel.left.hoursMinutes',
    h: 7,
    m: 30,
  });

  const exact = new Date('2026-09-01T17:00:00').getTime();
  assert.equal(u.effectiveDeadline(daily, exact).getTime(), exact);
  assert.equal(u.formatEntryText(daily, exact, translate).key, 'panel.soon');

  const after = exact + 1000;
  assert.equal(u.effectiveDateIso(daily, after), '2026-09-02');
  assert.deepEqual(u.formatEntryText(daily, after, translate), {
    key: 'panel.left.hoursMinutes',
    h: 23,
    m: 59,
  });

  const once = entry({ date: '2026-09-01', time: '17:00' });
  assert.equal(u.effectiveDateIso(once, after), '2026-09-01');
  assert.equal(u.isExpiredCountdown(once, after), true);
  assert.equal(u.formatEntryText(once, after, translate).key, 'panel.expiredOnly');
});

test('快捷面板使用短时间值与精简日期，方向信息不丢失', () => {
  process.env.TZ = 'Asia/Shanghai';
  const now = new Date('2026-09-01T09:30:00').getTime();
  const daily = entry({ date: '2026-09-01', time: '17:00', repeat: 'daily' });
  assert.equal(u.formatCompactEntryText(daily, now, compactTranslate), '7时30分');
  assert.equal(u.formatCompactEntryMeta(daily, now, compactTranslate), '每天 · 17:00');

  const once = entry({ date: '2026-09-12', time: '08:30' });
  assert.equal(u.formatCompactEntryMeta(once, now, compactTranslate), '9月12日 · 08:30');
  assert.equal(
    u.formatCompactEntryText(once, new Date('2026-09-12T08:30:01').getTime(), compactTranslate),
    '已到时',
  );

  const futureElapsed = entry({ entryType: 'elapsed', date: '2026-09-08' });
  assert.equal(u.formatCompactEntryText(futureElapsed, now, compactTranslate), '7天后');
});

test('周一至周五在周五到达后推进到下周一', () => {
  process.env.TZ = 'Asia/Shanghai';
  const workday = entry({ date: '2026-09-01', time: '17:00', repeat: 'workday' });
  const fridayAfter = new Date('2026-09-04T17:00:01').getTime();
  const next = u.effectiveDeadline(workday, fridayAfter);
  assert.equal(u.effectiveDateIso(workday, fridayAfter), '2026-09-07');
  assert.equal(next.getDay(), 1);
  assert.equal(u.sectionOf(workday, fridayAfter), 'soon');
});

test('面板按条目限额，排除归档与近屿重复，无进度区间不隐藏条目', () => {
  const now = new Date('2026-08-31T12:00:00').getTime();
  const items = Array.from({ length: 50 }, (_, i) =>
    entry({ id: `${i}`, pinned: i === 0, archived: i === 1 }),
  );
  const selected = u.panelSelection(items, now, true);
  const ids = [
    selected.featured?.id,
    ...selected.groups.flatMap((g) => g.items.map((e) => e.id)),
  ].filter(Boolean);
  assert.equal(ids.length, 6);
  assert.equal(new Set(ids).size, 6);
  assert.ok(!ids.includes('1'));
  const invalid = entry({ pinned: true, createdAt: 'broken' });
  assert.equal(u.featuredEntry([invalid], now), null);
  assert.equal(u.panelSelection([invalid], now, true).groups[0].items.length, 1);
  assert.equal(u.panelSelection([entry({ pinned: true })], now, true).groups.length, 0);
});

test('整月差对称，时刻正数日不误称过期，无效日期不产生数字', () => {
  const now = new Date('2026-09-01T12:00:00').getTime();
  assert.equal(u.entryUnitValue(entry({ date: '2026-08-31' }), 'month', now), 0);
  assert.equal(
    u.formatEntryText(
      entry({ entryType: 'elapsed', date: '2026-09-01', time: '11:00' }),
      now,
      translate,
    ).key,
    'panel.elapsedTime.hoursMinutes',
  );
  assert.equal(u.isValidDate('2026-02-30'), false);
  assert.equal(u.entryProgress(entry({ createdAt: 'broken' }), now), null);
});
