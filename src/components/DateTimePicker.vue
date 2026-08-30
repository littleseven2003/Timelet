<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

// 自研日期（可选时刻）选择器，替代原生控件；不引入第三方依赖
const props = defineProps<{
  /** ISO 日期（YYYY-MM-DD） */
  date: string;
  /** 时刻（HH:mm），withTime 为 false 时忽略 */
  time: string | null;
  /** 是否启用时刻选择 */
  withTime: boolean;
  /** 快捷选项面向过去（正计时起始日）还是未来（倒计时目标日） */
  past?: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:date', value: string): void;
  (e: 'update:time', value: string): void;
}>();

const { t } = useI18n();
const weekdayLabels = ['一', '二', '三', '四', '五', '六', '日'];

const viewYear = ref(new Date().getFullYear());
const viewMonth = ref(new Date().getMonth()); // 0 基

// 外部日期变化（如编辑回填）时同步日历视图
watch(
  () => props.date,
  (value) => {
    if (!value) return;
    const [year, month] = value.split('-').map(Number);
    viewYear.value = year!;
    viewMonth.value = month! - 1;
  },
  { immediate: true },
);

interface DayCell {
  iso: string;
  day: number;
  inMonth: boolean;
  isToday: boolean;
}

function toIso(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day}`;
}

const todayIso = toIso(new Date());

const cells = computed<DayCell[]>(() => {
  const first = new Date(viewYear.value, viewMonth.value, 1);
  // 周一为每行首列
  const offset = (first.getDay() + 6) % 7;
  const start = new Date(viewYear.value, viewMonth.value, 1 - offset);
  return Array.from({ length: 42 }, (_, i) => {
    const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
    return {
      iso: toIso(date),
      day: date.getDate(),
      inMonth: date.getMonth() === viewMonth.value,
      isToday: toIso(date) === todayIso,
    };
  });
});

const monthLabel = computed(() => `${viewYear.value} 年 ${viewMonth.value + 1} 月`);

function shiftMonth(delta: number) {
  const next = new Date(viewYear.value, viewMonth.value + delta, 1);
  viewYear.value = next.getFullYear();
  viewMonth.value = next.getMonth();
}

// 快捷日期：按未来/过去两套语义提供常用项
const quickOptions = computed(() => {
  if (props.past) {
    return [
      { label: t('config.quickToday'), offset: 0 },
      { label: t('config.quickYesterday'), offset: -1 },
      { label: t('config.quick7Ago'), offset: -7 },
    ];
  }
  return [
    { label: t('config.quickToday'), offset: 0 },
    { label: t('config.quickTomorrow'), offset: 1 },
    { label: t('config.quickNextMonday'), offset: nextMondayOffset() },
  ];
});

function nextMondayOffset(): number {
  const weekday = new Date().getDay();
  return (8 - weekday) % 7 || 7;
}

function applyQuick(offset: number) {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  emit('update:date', toIso(date));
}

const hourOptions = Array.from({ length: 24 }, (_, i) => String(i).padStart(2, '0'));
// 分钟按 5 分钟步进，减少长列表滚动
const minuteOptions = Array.from({ length: 12 }, (_, i) => String(i * 5).padStart(2, '0'));

const hour = computed(() => (props.time ?? '09:00').split(':')[0]!);
const minute = computed(() => (props.time ?? '09:00').split(':')[1]!);

function updateTime(part: 'hour' | 'minute', value: string) {
  const next = part === 'hour' ? `${value}:${minute.value}` : `${hour.value}:${value}`;
  emit('update:time', next);
}

// 时刻设为当前时间（取整到 5 分钟）
function setNow() {
  const now = new Date();
  const value = `${String(now.getHours()).padStart(2, '0')}:${String(
    Math.floor(now.getMinutes() / 5) * 5,
  ).padStart(2, '0')}`;
  emit('update:time', value);
}
</script>

<template>
  <div class="picker">
    <div class="picker__quick">
      <button
        v-for="option in quickOptions"
        :key="option.label"
        type="button"
        class="picker__chip"
        @click="applyQuick(option.offset)"
      >
        {{ option.label }}
      </button>
    </div>

    <div class="picker__header">
      <button class="picker__nav" type="button" aria-label="上一月" @click="shiftMonth(-1)">
        ‹
      </button>
      <span class="picker__month">{{ monthLabel }}</span>
      <button class="picker__nav" type="button" aria-label="下一月" @click="shiftMonth(1)">
        ›
      </button>
    </div>

    <div class="picker__grid">
      <span v-for="label in weekdayLabels" :key="label" class="picker__weekday">
        {{ label }}
      </span>
      <button
        v-for="cell in cells"
        :key="cell.iso"
        type="button"
        class="picker__day"
        :class="{
          'picker__day--dim': !cell.inMonth,
          'picker__day--today': cell.isToday,
          'picker__day--selected': cell.iso === date,
        }"
        @click="emit('update:date', cell.iso)"
      >
        {{ cell.day }}
      </button>
    </div>

    <div v-if="withTime" class="picker__time">
      <select
        class="picker__select"
        :value="hour"
        :aria-label="t('config.hourLabel')"
        @change="updateTime('hour', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="option in hourOptions" :key="option" :value="option">{{ option }} 时</option>
      </select>
      <span class="picker__colon">:</span>
      <select
        class="picker__select"
        :value="minute"
        :aria-label="t('config.minuteLabel')"
        @change="updateTime('minute', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="option in minuteOptions" :key="option" :value="option">
          {{ option }} 分
        </option>
      </select>
      <button class="picker__now" type="button" @click="setNow">
        {{ t('config.timeNow') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.picker {
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 10px;
  background-color: #fff;
  padding: 12px;
  width: 100%;
}

.picker__quick {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}

.picker__chip {
  flex: 1;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: none;
  border-radius: 6px;
  padding: 5px 0;
  font-size: 12px;
  cursor: pointer;
  color: inherit;
  transition:
    background-color 0.15s ease-out,
    border-color 0.15s ease-out;
}

.picker__chip:hover {
  background-color: rgba(0, 145, 255, 0.08);
  border-color: rgba(0, 145, 255, 0.4);
}

.picker__now {
  border: none;
  background: none;
  font-size: 12px;
  color: #0067c0;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
  white-space: nowrap;
}

.picker__now:hover {
  background-color: rgba(0, 145, 255, 0.08);
}

.picker__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.picker__month {
  font-size: 13px;
  font-weight: 600;
}

.picker__nav {
  border: none;
  background: none;
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  color: inherit;
  padding: 2px 8px;
  border-radius: 6px;
}

.picker__nav:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

.picker__grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
}

.picker__weekday {
  font-size: 11px;
  text-align: center;
  opacity: 0.5;
  padding: 2px 0;
}

.picker__day {
  border: none;
  background: none;
  font-size: 12px;
  padding: 6px 0;
  border-radius: 6px;
  cursor: pointer;
  color: inherit;
  font-variant-numeric: tabular-nums;
}

.picker__day:hover {
  background-color: rgba(0, 0, 0, 0.06);
}

.picker__day--dim {
  opacity: 0.3;
}

.picker__day--today {
  font-weight: 700;
}

.picker__day--selected {
  background-color: #0067c0;
  color: #fff;
}

.picker__day--selected:hover {
  background-color: #0072d4;
}

.picker__time {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
}

.picker__select {
  flex: 1;
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 6px;
  padding: 4px 6px;
  font-size: 12px;
  background-color: #fff;
  color: inherit;
}

.picker__colon {
  opacity: 0.5;
}

@media (prefers-color-scheme: dark) {
  .picker {
    background-color: #333;
    border-color: rgba(255, 255, 255, 0.12);
  }

  .picker__chip {
    border-color: rgba(255, 255, 255, 0.14);
  }

  .picker__chip:hover {
    background-color: rgba(0, 145, 255, 0.16);
    border-color: rgba(108, 184, 255, 0.5);
  }

  .picker__nav:hover {
    background-color: rgba(255, 255, 255, 0.06);
  }

  .picker__day:hover {
    background-color: rgba(255, 255, 255, 0.06);
  }

  .picker__day--selected {
    background-color: #0067c0;
  }

  .picker__time {
    border-top-color: rgba(255, 255, 255, 0.1);
  }

  .picker__select {
    background-color: #2b2b2b;
    border-color: rgba(255, 255, 255, 0.12);
  }

  .picker__now {
    color: #6cb8ff;
  }

  .picker__now:hover {
    background-color: rgba(0, 145, 255, 0.16);
  }
}
</style>
