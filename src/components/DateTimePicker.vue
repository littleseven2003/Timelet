<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useClock } from '../composables/useClock';

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
    if (!Number.isFinite(year) || !Number.isFinite(month)) return;
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

const now = useClock();
const todayIso = computed(() => toIso(new Date(now.value)));

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
      isToday: toIso(date) === todayIso.value,
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
  const weekday = new Date(now.value).getDay();
  return (8 - weekday) % 7 || 7;
}

function applyQuick(offset: number) {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  emit('update:date', toIso(date));
}

const hour = computed(() => (props.time ?? '09:00').split(':')[0]!);
const minute = computed(() => (props.time ?? '09:00').split(':')[1]!);

function updateTime(part: 'hour' | 'minute', value: string) {
  const next = part === 'hour' ? `${value}:${minute.value}` : `${hour.value}:${value}`;
  emit('update:time', next);
}

// 步进调节：时 0-23、分 0-59 循环，支持逐分钟
function step(part: 'hour' | 'minute', delta: number) {
  const max = part === 'hour' ? 24 : 60;
  const raw = Number((props.time ?? '09:00').split(':')[part === 'hour' ? 0 : 1]);
  const current = Number.isNaN(raw) ? 0 : raw;
  const next = String((current + delta + max) % max).padStart(2, '0');
  updateTime(part, next);
}

// 滚轮调节：向上增、向下减
function onWheel(part: 'hour' | 'minute', event: WheelEvent) {
  step(part, event.deltaY < 0 ? 1 : -1);
}

// 键入提交：超出范围时收敛到合法值
function commit(part: 'hour' | 'minute', raw: string) {
  const max = part === 'hour' ? 23 : 59;
  const parsed = Math.floor(Number(raw));
  const value = String(Math.min(max, Math.max(0, Number.isNaN(parsed) ? 0 : parsed))).padStart(
    2,
    '0',
  );
  updateTime(part, value);
}

// 按住 ▲▼ 连发：400ms 延迟后以 110ms 间隔重复
let holdDelay: ReturnType<typeof setTimeout> | undefined;
let holdTimer: ReturnType<typeof setInterval> | undefined;

function startHold(stepFn: () => void) {
  stopHold();
  stepFn();
  holdDelay = setTimeout(() => {
    holdTimer = setInterval(stepFn, 110);
  }, 400);
  window.addEventListener('pointerup', stopHold, { once: true });
  window.addEventListener('pointercancel', stopHold, { once: true });
}

function stopHold() {
  clearTimeout(holdDelay);
  clearInterval(holdTimer);
}

onBeforeUnmount(() => {
  stopHold();
  window.removeEventListener('pointerup', stopHold);
  window.removeEventListener('pointercancel', stopHold);
});

// 与逐分钟输入保持一致，不把“现在”提前到上一个五分钟节点。
function setNow() {
  const now = new Date();
  const value = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
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
        :aria-label="cell.iso"
        :aria-pressed="cell.iso === date"
        @click="emit('update:date', cell.iso)"
      >
        {{ cell.day }}
      </button>
    </div>

    <div v-if="withTime" class="picker__time">
      <!-- 步进器：键入 / 滚轮 / 按住 ▲▼ 连发，三种路径都能逐分钟调节 -->
      <div class="tstep">
        <input
          class="tstep__value"
          type="text"
          inputmode="numeric"
          :value="hour"
          :aria-label="t('config.hourLabel')"
          @change="commit('hour', ($event.target as HTMLInputElement).value)"
          @wheel.prevent="onWheel('hour', $event)"
        />
        <div class="tstep__btns">
          <button
            class="tstep__btn"
            type="button"
            :aria-label="t('config.hourLabel') + ' +1'"
            @pointerdown="startHold(() => step('hour', 1))"
            @click="$event.detail === 0 && step('hour', 1)"
          >
            ▲
          </button>
          <button
            class="tstep__btn"
            type="button"
            :aria-label="t('config.hourLabel') + ' -1'"
            @pointerdown="startHold(() => step('hour', -1))"
            @click="$event.detail === 0 && step('hour', -1)"
          >
            ▼
          </button>
        </div>
      </div>

      <span class="picker__colon">:</span>

      <div class="tstep">
        <input
          class="tstep__value"
          type="text"
          inputmode="numeric"
          :value="minute"
          :aria-label="t('config.minuteLabel')"
          @change="commit('minute', ($event.target as HTMLInputElement).value)"
          @wheel.prevent="onWheel('minute', $event)"
        />
        <div class="tstep__btns">
          <button
            class="tstep__btn"
            type="button"
            :aria-label="t('config.minuteLabel') + ' +1'"
            @pointerdown="startHold(() => step('minute', 1))"
            @click="$event.detail === 0 && step('minute', 1)"
          >
            ▲
          </button>
          <button
            class="tstep__btn"
            type="button"
            :aria-label="t('config.minuteLabel') + ' -1'"
            @pointerdown="startHold(() => step('minute', -1))"
            @click="$event.detail === 0 && step('minute', -1)"
          >
            ▼
          </button>
        </div>
      </div>

      <button class="picker__now" type="button" @click="setNow">
        {{ t('config.timeNow') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.picker {
  border: 1px solid var(--ts-line);
  border-radius: 10px;
  background-color: var(--ts-surface);
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
  border: 1px solid var(--ts-line);
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
  background-color: rgba(42, 156, 219, 0.08);
  border-color: rgba(42, 156, 219, 0.45);
}

.picker__now {
  border: none;
  background: none;
  font-size: 12px;
  color: var(--ts-primary-text);
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
  white-space: nowrap;
}

.picker__now:hover {
  background-color: rgba(42, 156, 219, 0.08);
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
  color: var(--ts-text-2);
}

.picker__day--today {
  font-weight: 700;
}

.picker__day--selected {
  background-color: var(--ts-button);
  color: var(--ts-on-button);
}

.picker__day--selected:hover {
  filter: brightness(1.08);
}

.picker__time {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--ts-line);
}

.tstep {
  display: flex;
  align-items: stretch;
  border: 1px solid var(--ts-line);
  border-radius: 6px;
  background-color: var(--ts-surface);
  overflow: hidden;
}

.tstep__value {
  width: 34px;
  border: none;
  background: none;
  text-align: center;
  font-size: 13px;
  color: inherit;
  font-variant-numeric: tabular-nums;
  padding: 4px 0;
}

/* 隐藏数字输入的原生上下箭头 */
.tstep__value::-webkit-outer-spin-button,
.tstep__value::-webkit-inner-spin-button {
  appearance: none;
}

.tstep__btns {
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--ts-line);
}

.tstep__btn {
  border: none;
  background: none;
  font-size: 7px;
  line-height: 1;
  padding: 0 4px;
  cursor: pointer;
  color: inherit;
  opacity: 0.55;
  flex: 1;
}

.tstep__btn:hover {
  opacity: 1;
  background-color: rgba(42, 156, 219, 0.1);
}

.picker__colon {
  opacity: 0.5;
}
</style>
