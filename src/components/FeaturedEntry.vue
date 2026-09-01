<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Entry } from '../types/entry';
import {
  effectiveDateIso,
  effectiveTime,
  entryProgress,
  formatCompactEntryMeta,
  formatCompactEntryText,
  formatEntryText,
} from '../utils/entries';
const props = defineProps<{ entry: Entry; now: number; compact?: boolean }>();
defineEmits<{ edit: [entry: Entry] }>();
const { t } = useI18n();
const progress = computed(() => entryProgress(props.entry, props.now));
const dot = computed(() => {
  const angle = ((135 + (progress.value?.progress ?? 0) * 270) * Math.PI) / 180;
  return { x: 60 + 52 * Math.cos(angle), y: 60 + 52 * Math.sin(angle) };
});
const text = computed(() => formatEntryText(props.entry, props.now, t));
const compactText = computed(() => formatCompactEntryText(props.entry, props.now, t));
const compactMeta = computed(() => formatCompactEntryMeta(props.entry, props.now, t));
</script>

<template>
  <button
    class="feature"
    :class="{ compact }"
    type="button"
    :data-entry-id="entry.id"
    :aria-label="compact ? `${entry.name} — ${text}` : undefined"
    :title="compact ? `${entry.name} · ${compactMeta} · ${text}` : undefined"
    @click="$emit('edit', entry)"
  >
    <div class="feature__copy">
      <span class="feature__eyebrow">{{ t('config.featuredLabel') }}</span>
      <strong class="feature__name">{{ entry.name }}</strong>
      <span class="feature__date">
        <template v-if="compact">{{ compactMeta }}</template>
        <template v-else
          ><template v-if="entry.repeat && entry.entryType === 'countdown'"
            >{{ t(`config.repeat.${entry.repeat}`) }} · {{ t('config.nextOccurrence') }} </template
          >{{ effectiveDateIso(entry, now)
          }}<template v-if="entry.time"> · {{ effectiveTime(entry, now) }}</template></template
        >
      </span>
      <span v-if="!compact" class="feature__meaning">{{ text }}</span>
    </div>
    <template v-if="compact">
      <strong class="feature__value">{{ compactText }}</strong>
      <span v-if="progress" class="tide" aria-hidden="true">
        <span :style="{ width: `${progress.progress * 100}%` }" />
        <i :style="{ left: `${progress.progress * 100}%` }" />
      </span>
    </template>
    <div v-else-if="progress" class="arc" aria-hidden="true">
      <svg viewBox="0 0 120 118" fill="none">
        <path class="arc__track" d="M23.23 96.77 A52 52 0 1 1 96.77 96.77" />
        <path
          class="arc__fill"
          d="M23.23 96.77 A52 52 0 1 1 96.77 96.77"
          pathLength="100"
          :stroke-dasharray="`${progress.progress * 100} 100`"
        />
        <circle :cx="dot.x" :cy="dot.y" r="3.2" fill="var(--ts-blue)" />
      </svg>
      <div class="arc__number">
        {{ progress.days }}<small>{{ t('config.unit.day') }}</small>
      </div>
      <span class="arc__start">{{ progress.start.slice(5).replace('-', '.') }}</span>
      <span class="arc__end">{{ entry.date.slice(5).replace('-', '.') }}</span>
    </div>
    <span v-if="progress" class="sr-only">{{
      t('config.progressRange', { start: progress.start, end: entry.date })
    }}</span>
  </button>
</template>

<style scoped>
.feature {
  position: relative;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 20px 26px;
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
.feature__date,
.feature__meaning {
  color: var(--ts-text-2);
  font-size: 12px;
}
.feature__meaning {
  color: var(--ts-blue);
}
.arc {
  position: relative;
  width: 120px;
  height: 125px;
  flex: 0 0 120px;
  font-variant-numeric: tabular-nums;
}
.arc svg {
  width: 120px;
  height: 118px;
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
  top: 29px;
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
.arc__start,
.arc__end {
  position: absolute;
  bottom: 0;
  font-size: 10px;
  color: var(--ts-text-2);
}
.arc__start {
  left: 6px;
}
.arc__end {
  right: 6px;
}
.compact {
  display: flex;
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
@media (prefers-contrast: more), (forced-colors: active) {
  .feature::before {
    display: none;
  }
}
</style>
