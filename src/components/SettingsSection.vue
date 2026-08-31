<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import {
  getSettings,
  saveSettings,
  type AppSettings,
  type ThemeMode,
} from '../api/settings';
import SegmentedControl from './SegmentedControl.vue';

const { t } = useI18n();
const settings = ref<AppSettings>({ launchAtLogin: false, showExpired: true, theme: 'system' });
const launchEnabled = ref(false);
const launchError = ref(false);

const themeOptions = [
  { value: 'system', label: '跟随系统' },
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' },
];

onMounted(async () => {
  settings.value = await getSettings();
  try {
    launchEnabled.value = await isEnabled();
  } catch {
    launchEnabled.value = settings.value.launchAtLogin;
  }
});

async function persist(next: AppSettings) {
  settings.value = next;
  await saveSettings(next);
}

async function toggleLaunch(enabled: boolean) {
  try {
    if (enabled) {
      await enable();
    } else {
      await disable();
    }
    launchEnabled.value = enabled;
    launchError.value = false;
    await persist({ ...settings.value, launchAtLogin: enabled });
  } catch (err) {
    console.error('设置开机自启失败', err);
    launchError.value = true;
    try {
      launchEnabled.value = await isEnabled();
    } catch {
      /* 保持原状态 */
    }
  }
}

async function setTheme(theme: string) {
  await persist({ ...settings.value, theme: theme as ThemeMode });
}
</script>

<template>
  <section class="settings">
    <div class="settings-group">
      <h3 class="settings-group__title">{{ t('settings.groupGeneral') }}</h3>
      <label class="settings-row">
        <div class="settings-row__text">
          <span class="settings-row__label">{{ t('settings.launchAtLogin') }}</span>
          <span class="settings-row__desc">{{ t('settings.launchAtLoginDesc') }}</span>
          <span v-if="launchError" class="settings-row__error">{{ t('settings.launchError') }}</span>
        </div>
        <input
          type="checkbox"
          class="settings-switch"
          :checked="launchEnabled"
          @change="toggleLaunch(($event.target as HTMLInputElement).checked)"
        />
      </label>
      <label class="settings-row">
        <div class="settings-row__text">
          <span class="settings-row__label">{{ t('settings.showExpired') }}</span>
          <span class="settings-row__desc">{{ t('settings.showExpiredDesc') }}</span>
        </div>
        <input
          type="checkbox"
          class="settings-switch"
          :checked="settings.showExpired"
          @change="persist({ ...settings, showExpired: ($event.target as HTMLInputElement).checked })"
        />
      </label>
    </div>

    <div class="settings-group">
      <h3 class="settings-group__title">{{ t('settings.groupAppearance') }}</h3>
      <div class="settings-row">
        <div class="settings-row__text">
          <span class="settings-row__label">{{ t('settings.theme') }}</span>
          <span class="settings-row__desc">{{ t('settings.themeDesc') }}</span>
        </div>
        <SegmentedControl
          :model-value="settings.theme ?? 'system'"
          :options="themeOptions"
          @update:model-value="setTheme($event)"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.settings-group {
  border: 1px solid var(--ts-line);
  border-radius: 10px;
  background-color: var(--ts-surface);
  padding: 12px 16px 6px;
}

.settings-group__title {
  font-size: 12px;
  font-weight: 600;
  color: var(--ts-text-2);
  margin: 0 0 4px;
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 0;
  cursor: pointer;
}

.settings-row + .settings-row {
  border-top: 1px solid var(--ts-line);
}

.settings-row__text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-row__label {
  font-size: 14px;
  font-weight: 500;
}

.settings-row__desc {
  font-size: 12px;
  color: var(--ts-text-2);
}

.settings-row__error {
  font-size: 12px;
  color: var(--ts-coral);
}

.settings-switch {
  appearance: none;
  width: 38px;
  height: 22px;
  border-radius: 11px;
  background-color: var(--ts-line);
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  transition: background-color 0.2s;
}

.settings-switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background-color: #fff;
  transition: transform 0.2s ease-out;
}

.settings-switch:checked {
  background-color: var(--ts-button);
}

.settings-switch:checked::after {
  transform: translateX(16px);
}
</style>
