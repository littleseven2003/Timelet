<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { DisplayUnit, Entry } from '../types/entry';
import { effectiveTime, effectiveDateIso, formatEntryText, isValidDate } from '../utils/entries';
import DateTimePicker from './DateTimePicker.vue';
import EntryTypeSymbol from './EntryTypeSymbol.vue';
import SegmentedControl from './SegmentedControl.vue';
import ToggleSwitch from './ToggleSwitch.vue';

const draft = defineModel<Entry>({ required: true });
const props = defineProps<{
  isNew: boolean;
  now: number;
  busy: boolean;
  blocked?: boolean;
  error?: string;
}>();
const emit = defineEmits<{ save: [entry: Entry]; cancel: [] }>();
const { t } = useI18n();
const nameInput = ref<HTMLInputElement>();
const dateButton = ref<HTMLButtonElement>();
const form = ref<HTMLFormElement>();
const calendarOpen = ref(!draft.value.date);
const validation = ref('');
const preview = computed(() => formatEntryText(draft.value, props.now, t));
const types = computed(() => [
  { value: 'countdown', label: t('config.typeCountdown') },
  { value: 'elapsed', label: t('config.typeElapsed') },
]);
const units = computed(() =>
  ['day', 'week', 'month', 'year'].map((value) => ({ value, label: t(`config.unit.${value}`) })),
);
const repeats = computed(() =>
  ['none', 'daily', 'workday'].map((value) => ({ value, label: t(`config.repeat.${value}`) })),
);
const dateLabel = computed(() => {
  if (draft.value.entryType === 'elapsed') return t('config.startDate');
  return t(draft.value.time && draft.value.repeat ? 'config.firstDate' : 'config.targetDate');
});
const canSave = computed(() => !!draft.value.name.trim() && isValidDate(draft.value.date));

function changeType(type: string) {
  draft.value.entryType = type as Entry['entryType'];
  if (type === 'elapsed') draft.value.repeat = undefined;
}
function toggleTime(enabled: boolean) {
  draft.value.time = enabled ? '09:00' : undefined;
  if (!enabled) draft.value.repeat = undefined;
  if (enabled) calendarOpen.value = true;
}
function chooseDate(date: string) {
  draft.value.date = date;
  if (!draft.value.time) {
    calendarOpen.value = false;
    void nextTick(() => dateButton.value?.focus());
  }
}
function submit() {
  if (props.busy || props.blocked) return;
  if (!canSave.value) {
    validation.value = t('config.saveHint');
    return;
  }
  if (draft.value.time && !/^([01]\d|2[0-3]):[0-5]\d$/.test(draft.value.time)) {
    validation.value = t('config.invalidTime');
    return;
  }
  validation.value = '';
  emit('save', {
    ...draft.value,
    name: draft.value.name.trim(),
    repeat:
      draft.value.entryType === 'countdown' && draft.value.time ? draft.value.repeat : undefined,
    updatedAt: new Date().toISOString(),
  });
}
onMounted(() => nameInput.value?.focus());
defineExpose({ submit: () => form.value?.requestSubmit() });
</script>

<template>
  <form ref="form" class="editor" @submit.prevent="submit">
    <header>
      <span class="eyebrow">TIMELET</span>
      <h1>{{ t(isNew ? 'config.addEntry' : 'config.editEntry') }}</h1>
    </header>
    <fieldset :disabled="busy">
      <div class="field">
        <span id="type-label">{{ t('config.fieldType') }}</span
        ><SegmentedControl
          :model-value="draft.entryType"
          :options="types"
          aria-labelledby="type-label"
          @update:model-value="changeType"
          ><template #option="{ option }"
            ><EntryTypeSymbol :type="option.value as Entry['entryType']" />{{
              option.label
            }}</template
          ></SegmentedControl
        >
      </div>
      <div class="field">
        <label for="entry-name">{{ t('config.fieldName') }}</label
        ><input
          id="entry-name"
          ref="nameInput"
          v-model="draft.name"
          class="text-input title-input"
          type="text"
          required
          maxlength="30"
          :placeholder="t('config.namePlaceholder')"
        />
      </div>
      <div class="field">
        <span id="date-label">{{ dateLabel }}</span>
        <button
          ref="dateButton"
          type="button"
          class="date-button"
          aria-labelledby="date-label chosen-date"
          :aria-expanded="calendarOpen"
          aria-controls="entry-calendar"
          @click="calendarOpen = !calendarOpen"
        >
          <span id="chosen-date"
            >{{ draft.date || t('config.chooseDate')
            }}<template v-if="draft.time"> · {{ draft.time }}</template></span
          ><span aria-hidden="true">{{ calendarOpen ? '−' : '+' }}</span>
        </button>
        <div v-if="calendarOpen" id="entry-calendar">
          <DateTimePicker
            :date="draft.date"
            :time="draft.time ?? null"
            :with-time="!!draft.time"
            :past="draft.entryType === 'elapsed'"
            @update:date="chooseDate"
            @update:time="draft.time = $event"
          />
        </div>
        <label class="inline-field"
          ><span>{{ t('config.includeTime') }}</span
          ><ToggleSwitch :model-value="!!draft.time" @update:model-value="toggleTime"
        /></label>
      </div>
      <div v-if="draft.time && draft.entryType === 'countdown'" class="field frequency-field">
        <span id="repeat-label">{{ t('config.fieldRepeat') }}</span
        ><SegmentedControl
          :model-value="draft.repeat ?? 'none'"
          :options="repeats"
          aria-labelledby="repeat-label"
          @update:model-value="
            draft.repeat = $event === 'none' ? undefined : ($event as Entry['repeat'])
          "
        /><small>{{ t('config.repeatHint') }}</small>
      </div>
      <div class="semantic-preview" aria-live="polite">
        <EntryTypeSymbol :type="draft.entryType" />
        <div>
          <strong>{{ draft.name || t('config.previewName') }}</strong
          ><span
            >{{ preview || t('config.previewDays')
            }}<template v-if="isValidDate(draft.date)">
              <template v-if="draft.repeat && draft.entryType === 'countdown'">
                · {{ t(`config.repeat.${draft.repeat}`) }} ·
                {{ t('config.nextOccurrence') }} </template
              ><template v-else> · </template> {{ effectiveDateIso(draft, now)
              }}<template v-if="draft.time"> {{ effectiveTime(draft, now) }}</template></template
            ></span
          >
        </div>
      </div>
      <details class="more">
        <summary>{{ t('config.showMore') }}</summary>
        <div v-if="!draft.time" class="field">
          <span id="unit-label">{{ t('config.fieldUnit') }}</span
          ><SegmentedControl
            :model-value="draft.displayUnit ?? 'day'"
            :options="units"
            aria-labelledby="unit-label"
            @update:model-value="draft.displayUnit = $event as DisplayUnit"
          />
        </div>
        <label class="inline-field"
          ><span>{{ t('config.fieldPinned') }}</span
          ><ToggleSwitch v-model="draft.pinned"
        /></label>
        <div class="field">
          <label for="entry-note">{{ t('config.fieldNote') }}</label
          ><textarea
            id="entry-note"
            v-model="draft.note"
            class="text-input"
            rows="3"
            maxlength="500"
            :placeholder="t('config.notePlaceholder')"
          />
        </div>
      </details>
    </fieldset>
    <p v-if="error || validation" class="error-notice" role="alert">{{ error || validation }}</p>
    <footer class="editor-actions">
      <small>{{ t('config.saveHint') }}</small
      ><button class="btn" type="button" :disabled="busy" @click="emit('cancel')">
        {{ t('config.cancel') }}</button
      ><button class="btn btn--primary" type="submit" :disabled="busy || blocked || !canSave">
        {{ busy ? t('common.saving') : t(isNew ? 'config.createEntry' : 'config.saveChanges') }}
      </button>
    </footer>
  </form>
</template>

<style scoped>
.editor {
  max-width: 540px;
  margin: 0 auto;
}
header {
  margin-bottom: 24px;
}
h1 {
  font-size: 24px;
  font-weight: 500;
  margin: 8px 0 0;
}
.eyebrow {
  font-size: 10px;
  letter-spacing: 0.18em;
  color: var(--ts-text-2);
}
fieldset {
  border: 0;
  padding: 0;
  margin: 0;
  min-width: 0;
}
.field {
  display: grid;
  gap: 9px;
  margin-bottom: 20px;
  font-size: 12px;
}
.field > span,
.field > label {
  color: var(--ts-text-2);
}
.title-input {
  font-size: 18px;
  padding: 12px;
}
.inline-field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  gap: 12px;
  padding: 7px 0;
  color: var(--ts-text-2);
}
.date-button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  text-align: left;
  padding: 12px;
  border: 1px solid var(--ts-line);
  background: var(--ts-surface);
  border-radius: 8px;
  color: var(--ts-text);
  cursor: pointer;
}
.semantic-preview {
  display: flex;
  gap: 14px;
  align-items: center;
  padding: 16px;
  background: var(--ts-focus);
  border-radius: 10px 10px 20px 10px;
  color: var(--ts-blue);
}
.semantic-preview > div {
  display: grid;
  gap: 7px;
  min-width: 0;
}
.semantic-preview strong {
  font-size: 13px;
  color: var(--ts-text);
  overflow-wrap: anywhere;
  font-weight: 500;
}
.semantic-preview span {
  font-size: 11px;
  line-height: 1.7;
}
.more {
  margin-top: 18px;
  border-bottom: 1px solid var(--ts-line);
}
summary {
  padding: 10px 0 18px;
  cursor: pointer;
  font-size: 12px;
  color: var(--ts-text-2);
}
.more .field {
  margin-top: 16px;
}
.frequency-field small,
.more small {
  color: var(--ts-text-2);
  line-height: 1.6;
}
.editor-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 9px;
  padding: 20px 0 6px;
}
.editor-actions small {
  flex: 1;
  font-size: 10px;
  min-width: 110px;
  color: var(--ts-text-2);
}
</style>
