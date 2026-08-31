<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useSettings } from '../composables/useSettings';
import type { AppSettings, ThemeMode } from '../api/settings';
import SegmentedControl from './SegmentedControl.vue';
import ToggleSwitch from './ToggleSwitch.vue';
const { t } = useI18n();
const { settings, loaded, busy, error, persist, retry } = useSettings();
const actionError = ref('');
const message = ref('');
const launchEnabled = ref(false);
const launchKnown = ref(false);
const launchBusy = ref(false);
const disabled = computed(() => !loaded.value || busy.value || !!error.value || launchBusy.value);
const themes = computed(() =>
  ['system', 'light', 'dark'].map((value) => ({ value, label: t(`settings.themes.${value}`) })),
);
const limits = computed(() =>
  [5, 6, 7, 8].map((value) => ({ value: String(value), label: String(value) })),
);
async function readLaunch() {
  try {
    launchEnabled.value = await isEnabled();
    launchKnown.value = true;
  } catch (cause) {
    actionError.value = `${t('settings.launchError')}：${String(cause)}`;
  }
}
onMounted(readLaunch);
async function save(patch: Partial<AppSettings>) {
  actionError.value = '';
  message.value = '';
  try {
    await persist(patch);
    message.value = t('common.saved');
  } catch (cause) {
    actionError.value = String(cause);
  }
}
async function toggleLaunch(enabled: boolean) {
  if (disabled.value || !launchKnown.value) return;
  launchBusy.value = true;
  actionError.value = '';
  message.value = '';
  const previous = launchEnabled.value;
  try {
    await (enabled ? enable() : disable());
    try {
      await persist({ launchAtLogin: enabled });
    } catch (cause) {
      try {
        await (previous ? enable() : disable());
      } catch {
        actionError.value = t('settings.launchRollbackError');
      }
      throw cause;
    }
    message.value = t('common.saved');
  } catch (cause) {
    actionError.value = `${actionError.value} ${t('settings.launchError')}：${String(cause)}`;
  } finally {
    await readLaunch();
    launchBusy.value = false;
  }
}
</script>

<template>
  <section class="settings">
    <div v-if="error" class="error-notice" role="alert">
      {{ error }}<button class="btn" type="button" @click="retry">{{ t('common.retry') }}</button>
    </div>
    <div v-if="actionError" class="error-notice" role="alert">
      {{ actionError
      }}<button v-if="!launchKnown" class="btn" type="button" @click="readLaunch">
        {{ t('common.retry') }}
      </button>
    </div>
    <p v-if="message" class="saved" role="status">{{ message }}</p>
    <fieldset :disabled="disabled">
      <legend>{{ t('settings.groupGeneral') }}</legend>
      <label class="settings-row"
        ><span
          ><strong>{{ t('settings.launchAtLogin') }}</strong
          ><small>{{ t('settings.launchAtLoginDesc') }}</small></span
        ><ToggleSwitch
          :model-value="launchEnabled"
          :disabled="!launchKnown"
          @update:model-value="toggleLaunch"
      /></label>
      <label class="settings-row"
        ><span
          ><strong>{{ t('settings.showExpired') }}</strong
          ><small>{{ t('settings.showExpiredDesc') }}</small></span
        ><ToggleSwitch
          :model-value="settings.showExpired"
          @update:model-value="save({ showExpired: $event })"
      /></label>
      <div class="settings-row">
        <span
          ><strong id="panel-limit-label">{{ t('settings.panelLimit') }}</strong
          ><small>{{ t('settings.panelLimitDesc') }}</small></span
        ><SegmentedControl
          :model-value="String(settings.panelLimit ?? 6)"
          :options="limits"
          aria-labelledby="panel-limit-label"
          @update:model-value="save({ panelLimit: Number($event) })"
        />
      </div>
    </fieldset>
    <fieldset :disabled="disabled">
      <legend>{{ t('settings.groupAppearance') }}</legend>
      <div class="settings-row">
        <span
          ><strong id="theme-label">{{ t('settings.theme') }}</strong
          ><small>{{ t('settings.themeDesc') }}</small></span
        ><SegmentedControl
          :model-value="settings.theme ?? 'system'"
          :options="themes"
          aria-labelledby="theme-label"
          @update:model-value="save({ theme: $event as ThemeMode })"
        />
      </div>
    </fieldset>
  </section>
</template>

<style scoped>
.settings {
  display: grid;
  gap: 22px;
}
fieldset {
  border: 0;
  padding: 0;
  min-width: 0;
  margin: 0;
}
legend {
  color: var(--ts-text-2);
  font-size: 11px;
  margin-bottom: 4px;
  padding: 0;
}
.settings-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 18px 0;
  border-bottom: 1px solid var(--ts-line);
}
.settings-row > span {
  display: grid;
  gap: 6px;
  flex: 1;
  min-width: 170px;
}
.settings-row strong {
  font-size: 13px;
  font-weight: 500;
}
.settings-row small {
  color: var(--ts-text-2);
  font-size: 11px;
  line-height: 1.7;
}
.saved {
  color: var(--ts-teal);
  font-size: 12px;
  margin: 0;
}
</style>
