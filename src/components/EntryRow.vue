<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Entry } from '../types/entry';
import {
  effectiveTime,
  effectiveDateIso,
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
}>();
defineEmits<{
  expand: [];
  edit: [];
  pin: [];
  archive: [];
  restore: [];
  duplicate: [];
  remove: [];
}>();
const { t } = useI18n();
const text = computed(() => formatEntryText(props.entry, props.now, t));
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
      :title="compact ? entry.name : undefined"
      @click="$emit('expand')"
      @contextmenu.prevent="$emit('expand')"
    >
      <EntryTypeSymbol :type="entry.entryType" :class="['entry-row__symbol', entry.entryType]" />
      <span class="entry-row__body">
        <span class="entry-row__title"
          >{{ entry.name
          }}<span v-if="entry.pinned" class="pin" :aria-label="t('config.pinnedTag')">·</span></span
        >
        <span class="entry-row__meta"
          >{{ t(entry.entryType === 'countdown' ? 'config.typeCountdown' : 'config.typeElapsed') }}
          <template v-if="entry.repeat && entry.entryType === 'countdown'">
            · {{ t(`config.repeat.${entry.repeat}`) }} · {{ t('config.nextOccurrence') }} </template
          ><template v-else> · </template> {{ effectiveDateIso(entry, now)
          }}<template v-if="entry.time"> {{ effectiveTime(entry, now) }}</template></span
        >
      </span>
      <span class="entry-row__value" :class="{ expired, elapsed: entry.entryType === 'elapsed' }">{{
        text
      }}</span>
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
  padding: 13px 3px;
  gap: 9px;
}
.compact .entry-row__title {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.compact .entry-row__value {
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
