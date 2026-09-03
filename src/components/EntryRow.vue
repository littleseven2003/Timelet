<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Entry } from '../types/entry';
import {
  formatCompactEntryMeta,
  formatCompactEntryText,
  formatEntryScheduleMeta,
  formatEntryText,
  isExpiredCountdown,
} from '../utils/entries';
import EntryTypeSymbol from './EntryTypeSymbol.vue';
const props = defineProps<{
  entry: Entry;
  now: number;
  expanded?: boolean;
  compact?: boolean;
  disabled?: boolean;
  nearIsle?: boolean;
}>();
defineEmits<{
  expand: [];
  edit: [];
  pin: [];
  nearIsle: [];
  archive: [];
  restore: [];
  duplicate: [];
  remove: [];
}>();
const { t } = useI18n();
const text = computed(() => formatEntryText(props.entry, props.now, t));
const compactText = computed(() => formatCompactEntryText(props.entry, props.now, t));
const compactMeta = computed(() => formatCompactEntryMeta(props.entry, props.now, t));
const fullMeta = computed(
  () =>
    `${t(
      props.entry.entryType === 'countdown' ? 'config.typeCountdown' : 'config.typeElapsed',
    )} · ${formatEntryScheduleMeta(props.entry, props.now, t)}`,
);
const expired = computed(() => isExpiredCountdown(props.entry, props.now));
</script>

<template>
  <li class="entry-row" :class="{ compact, expanded }">
    <button
      class="entry-row__summary"
      type="button"
      :data-entry-id="entry.id"
      :aria-expanded="!!expanded"
      :aria-controls="`detail-${entry.id}`"
      :aria-label="compact ? `${entry.name} — ${text}` : undefined"
      :title="compact ? `${entry.name} · ${compactMeta} · ${text}` : undefined"
      @click="$emit('expand')"
      @contextmenu.prevent="$emit('expand')"
    >
      <EntryTypeSymbol :type="entry.entryType" :class="['entry-row__symbol', entry.entryType]" />
      <span class="entry-row__body">
        <span class="entry-row__title"
          >{{ entry.name
          }}<span v-if="entry.pinned" class="pin" :aria-label="t('config.pinnedTag')">·</span
          ><span v-if="nearIsle" class="near-isle-tag">{{ t('config.nearIsleTag') }}</span></span
        >
        <span class="entry-row__meta">
          {{ compact ? compactMeta : fullMeta }}
        </span>
      </span>
      <span class="entry-row__value" :class="{ expired, elapsed: entry.entryType === 'elapsed' }">
        {{ compact ? compactText : text }}
      </span>
    </button>
    <div v-if="expanded" :id="`detail-${entry.id}`" class="entry-row__detail">
      <p v-if="entry.note" class="entry-row__note">{{ entry.note }}</p>
      <div class="entry-row__actions">
        <template v-if="!entry.archived">
          <button type="button" :disabled="disabled" @click="$emit('edit')">
            {{ t('config.edit') }}
          </button>
          <button type="button" :disabled="disabled" @click="$emit('pin')">
            {{ t(entry.pinned ? 'config.unpin' : 'config.pinIt') }}
          </button>
          <button type="button" :disabled="disabled" @click="$emit('nearIsle')">
            {{ t(nearIsle ? 'config.removeNearIsle' : 'config.setNearIsle') }}
          </button>
        </template>
        <button v-else type="button" :disabled="disabled" @click="$emit('restore')">
          {{ t('config.restore') }}
        </button>
        <template v-if="!compact">
          <button type="button" :disabled="disabled" @click="$emit('duplicate')">
            {{ t('config.duplicate') }}
          </button>
          <button
            v-if="!entry.archived"
            type="button"
            :disabled="disabled"
            @click="$emit('archive')"
          >
            {{ t('config.archiveAction') }}
          </button>
          <button type="button" class="danger-text" :disabled="disabled" @click="$emit('remove')">
            {{ t('config.delete') }}
          </button>
        </template>
      </div>
    </div>
  </li>
</template>

<style scoped>
.entry-row {
  list-style: none;
  border-bottom: 1px solid var(--ts-line);
}
.entry-row:last-child {
  border-bottom: 0;
}
.entry-row__summary {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 13px;
  border: 0;
  background: transparent;
  color: var(--ts-text);
  text-align: left;
  padding: 16px 7px;
  cursor: pointer;
  border-radius: 8px;
}
.entry-row__summary:hover,
.expanded .entry-row__summary {
  background: var(--ts-hover);
}
.entry-row__symbol {
  color: var(--ts-blue);
  flex-shrink: 0;
}
.entry-row__body {
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 5px;
}
.entry-row__title {
  font-size: 14px;
  overflow-wrap: anywhere;
  line-height: 1.5;
}
.pin {
  color: var(--ts-blue);
  margin-left: 6px;
  font-size: 17px;
}
.near-isle-tag {
  margin-left: 7px;
  color: var(--ts-blue);
  font-size: 10px;
  font-weight: 500;
}
.entry-row__meta {
  font-size: 10px;
  line-height: 1.6;
  color: var(--ts-text-2);
}
.entry-row__value {
  font-size: 19px;
  color: var(--ts-blue);
  font-weight: 450;
  text-align: right;
  font-variant-numeric: tabular-nums;
  max-width: 44%;
  line-height: 1.4;
}
.elapsed {
  color: var(--ts-teal);
}
.expired {
  color: var(--ts-coral);
}
.entry-row__detail {
  padding: 2px 9px 13px 41px;
}
.entry-row__note {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-size: 12px;
  line-height: 1.7;
  margin: 4px 0 8px;
  color: var(--ts-text-2);
}
.entry-row__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  margin-top: 6px;
}
.entry-row__actions button {
  border: none;
  background: none;
  font-size: 12px;
  padding: 6px 0;
  color: var(--ts-blue);
  cursor: pointer;
}
.entry-row__actions .danger-text {
  color: var(--ts-coral);
  margin-left: auto;
}
.compact .entry-row__summary {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  padding: 9px 3px;
  gap: 9px;
}
.compact .entry-row__symbol {
  grid-column: 1;
  grid-row: 1;
}
.compact .entry-row__body {
  grid-column: 2;
  grid-row: 1;
  display: grid;
  gap: 1px;
}
.compact .entry-row__title {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.compact .entry-row__value {
  grid-column: 3;
  grid-row: 1;
  align-self: center;
  font-size: 16px;
  flex: none;
  max-width: none;
  white-space: nowrap;
}
.compact .entry-row__meta {
  font-size: 10px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
@media (max-width: 420px) {
  .entry-row__value {
    font-size: 16px;
  }
}
</style>
