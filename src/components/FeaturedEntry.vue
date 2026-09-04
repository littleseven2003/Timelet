<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Entry } from '../types/entry';
import {
  entryProgress,
  formatCompactEntryMeta,
  formatCompactEntryText,
  formatEntryScheduleMeta,
  formatEntryText,
  isExpiredCountdown,
} from '../utils/entries';
const props = defineProps<{ entry: Entry; now: number; compact?: boolean }>();
defineEmits<{ edit: [entry: Entry] }>();
const { t } = useI18n();
const progress = computed(() => entryProgress(props.entry, props.now));
const dot = computed(() => {
  const angle = ((135 + (progress.value?.progress ?? 0) * 270) * Math.PI) / 180;
  return { x: 70 + 55 * Math.cos(angle), y: 65 + 55 * Math.sin(angle) };
});
const text = computed(() => formatEntryText(props.entry, props.now, t));
const compactText = computed(() => formatCompactEntryText(props.entry, props.now, t));
const compactMeta = computed(() => formatCompactEntryMeta(props.entry, props.now, t));
const fullMeta = computed(() => formatEntryScheduleMeta(props.entry, props.now, t));
const expired = computed(() => isExpiredCountdown(props.entry, props.now));
const arcValueParts = computed(() => {
  const value = compactText.value.trim();
  const spaced = value.match(/^(\S+)\s+(.+)$/);
  if (spaced) return { primary: spaced[1], secondary: spaced[2] };
  const compact = value.match(/^(.+?\D)(\d.+)$/u);
  return compact
    ? { primary: compact[1], secondary: compact[2] }
    : { primary: value, secondary: '' };
});
</script>

<template>
  <button
    class="feature"
    :class="{ compact, elapsed: entry.entryType === 'elapsed', expired }"
    type="button"
    :data-entry-id="entry.id"
    :aria-label="`${entry.name} — ${text}`"
    :title="compact ? `${entry.name} · ${compactMeta} · ${text}` : undefined"
    @click="$emit('edit', entry)"
  >
    <div class="feature__copy">
      <span class="feature__eyebrow">{{ t('config.featuredLabel') }}</span>
      <strong class="feature__name">{{ entry.name }}</strong>
      <span class="feature__date">
        {{ compact ? compactMeta : fullMeta }}
      </span>
    </div>
    <template v-if="compact">
      <strong class="feature__value">{{ compactText }}</strong>
      <span class="tide" :class="{ 'tide--static': !progress }" aria-hidden="true">
        <template v-if="progress">
          <span :style="{ width: `${progress.progress * 100}%` }" />
          <i :style="{ left: `${progress.progress * 100}%` }" />
        </template>
      </span>
    </template>
    <div v-else-if="progress" class="arc" aria-hidden="true">
      <svg viewBox="0 0 144 134" fill="none">
        <path class="arc__track" d="M31.1 103.9 A55 55 0 1 1 108.9 103.9" />
        <path
          class="arc__fill"
          d="M31.1 103.9 A55 55 0 1 1 108.9 103.9"
          pathLength="100"
          :stroke-dasharray="`${progress.progress * 100} 100`"
        />
        <circle :cx="dot.x" :cy="dot.y" r="3.2" fill="var(--ts-blue)" />
      </svg>
      <div class="arc__number" :class="{ 'arc__number--timed': entry.time }">
        <template v-if="entry.time"
          ><span>{{ arcValueParts.primary }}</span
          ><small v-if="arcValueParts.secondary">{{ arcValueParts.secondary }}</small></template
        ><template v-else
          >{{ progress.days }}<small>{{ t('config.unit.day') }}</small></template
        >
      </div>
      <span class="arc__start">{{ progress.start.slice(5).replace('-', '.') }}</span>
      <span class="arc__end">{{ progress.end.slice(5).replace('-', '.') }}</span>
    </div>
    <strong v-else class="feature__standalone-value">{{ compactText }}</strong>
    <span v-if="progress" class="sr-only">{{
      t('config.progressRange', { start: progress.start, end: progress.end })
    }}</span>
  </button>
</template>

<style scoped>
.feature {
  position: relative;
  width: 100%;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 144px;
  align-items: center;
  gap: 22px;
  min-height: 176px;
  padding: 20px 22px 18px 26px;
  border: 1px solid var(--ts-line);
  border-radius: 14px 14px 30px 14px;
  background: var(--ts-focus);
  color: var(--ts-text);
  text-align: left;
  overflow: hidden;
  cursor: pointer;
}
.feature::before {
  content: '';
  position: absolute;
  width: 64%;
  height: 65%;
  bottom: -30%;
  right: -6%;
  border-radius: 60% 40% 0 0;
  transform: rotate(-6deg);
  background: var(--ts-island);
  opacity: 0.7;
}
.feature__copy {
  position: relative;
  z-index: 1;
  min-width: 0;
  display: grid;
  gap: 7px;
}
.feature__eyebrow {
  color: var(--ts-text-2);
  font-size: 10px;
  letter-spacing: 0.12em;
}
.feature__name {
  font-size: 19px;
  font-weight: 550;
  line-height: 1.5;
  overflow-wrap: anywhere;
}
.feature__date {
  color: var(--ts-text-2);
  font-size: 12px;
  line-height: 1.55;
  overflow-wrap: anywhere;
}
.feature.elapsed .feature__standalone-value,
.feature.elapsed .feature__value {
  color: var(--ts-teal);
}
.feature.expired .feature__meaning,
.feature.expired .feature__standalone-value,
.feature.expired .feature__value {
  color: var(--ts-coral);
}
.arc {
  position: relative;
  z-index: 1;
  width: 144px;
  height: 134px;
  justify-self: end;
  font-variant-numeric: tabular-nums;
}
.arc svg {
  width: 144px;
  height: 134px;
}
.arc__track,
.arc__fill {
  stroke: var(--ts-line);
  stroke-width: 2.4;
  stroke-linecap: round;
}
.arc__fill {
  stroke: var(--ts-blue);
}
.arc__number {
  position: absolute;
  top: 31px;
  left: 0;
  width: 100%;
  text-align: center;
  font-size: 42px;
  line-height: 1;
  font-weight: 400;
  letter-spacing: -0.04em;
  color: var(--ts-blue);
}
.arc__number small {
  display: block;
  font-size: 11px;
  margin-top: 8px;
  letter-spacing: 0;
  color: var(--ts-text-2);
}
.arc__number--timed {
  top: 24px;
  left: 26px;
  width: 88px;
  height: 74px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 23px;
  line-height: 1;
  letter-spacing: -0.02em;
  white-space: nowrap;
}
.arc__number--timed > span {
  line-height: 1;
}
.arc__number--timed small {
  font-size: 12px;
  line-height: 1;
  margin-top: 0;
}
.arc__start,
.arc__end {
  position: absolute;
  bottom: 0;
  font-size: 10px;
  color: var(--ts-text-2);
}
.arc__start {
  left: 9px;
}
.arc__end {
  right: 7px;
}
.compact {
  display: flex;
  min-height: 0;
  align-items: center;
  padding: 12px 14px 18px;
  gap: 10px;
  border-radius: 10px 10px 20px 10px;
}
.compact .feature__copy {
  display: grid;
  gap: 3px;
  flex: 1;
  overflow: hidden;
}
.compact .feature__name {
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.compact .feature__date {
  font-size: 10px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.feature__value {
  position: relative;
  flex: none;
  white-space: nowrap;
  color: var(--ts-blue);
  font-size: 17px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}
.feature__standalone-value {
  position: relative;
  flex: none;
  max-width: 46%;
  color: var(--ts-blue);
  font-size: 28px;
  font-weight: 450;
  line-height: 1.3;
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.tide {
  position: absolute;
  left: 16px;
  right: 16px;
  bottom: 10px;
  height: 2px;
  background: var(--ts-line);
}
.tide span {
  display: block;
  height: 100%;
  background: var(--ts-blue);
}
.tide i {
  position: absolute;
  top: -2px;
  width: 6px;
  height: 6px;
  margin-left: -3px;
  background: var(--ts-blue);
  border-radius: 50%;
}
.tide--static {
  background: linear-gradient(90deg, var(--ts-line), var(--ts-blue), var(--ts-line));
  opacity: 0.5;
}
@media (prefers-contrast: more), (forced-colors: active) {
  .feature::before {
    display: none;
  }
}
@media (max-width: 740px) {
  .feature:not(.compact) {
    grid-template-columns: minmax(0, 1fr) 126px;
    gap: 14px;
    padding-inline: 20px 14px;
  }
  .feature:not(.compact) .arc {
    transform: scale(0.88);
    transform-origin: right center;
    margin-left: -18px;
  }
}
</style>
