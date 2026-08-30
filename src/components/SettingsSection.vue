<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { getSettings, saveSettings, type AppSettings } from '../api/settings';

const { t } = useI18n();
const settings = ref<AppSettings>({ launchAtLogin: false, showExpired: true });
// 自启开关的真实状态以系统自启项为准
const launchEnabled = ref(false);
const launchError = ref(false);

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
    // 切换失败时回显系统真实状态并提示
    console.error('设置开机自启失败', err);
    launchError.value = true;
    try {
      launchEnabled.value = await isEnabled();
    } catch {
      /* 保持原状态 */
    }
  }
}

async function toggleShowExpired(enabled: boolean) {
  await persist({ ...settings.value, showExpired: enabled });
}
</script>

<template>
  <section class="settings">
    <h2 class="settings__title">{{ t('config.nav.settings') }}</h2>

    <div class="settings-card">
      <label class="settings-row">
        <div class="settings-row__text">
          <span class="settings-row__label">{{ t('settings.launchAtLogin') }}</span>
          <span class="settings-row__desc">{{ t('settings.launchAtLoginDesc') }}</span>
          <span v-if="launchError" class="settings-row__error">{{
            t('settings.launchError')
          }}</span>
        </div>
        <input
          type="checkbox"
          class="settings-switch"
          :checked="launchEnabled"
          @change="toggleLaunch(($event.target as HTMLInputElement).checked)"
        />
      </label>

      <div class="settings-card__divider" />

      <label class="settings-row">
        <div class="settings-row__text">
          <span class="settings-row__label">{{ t('settings.showExpired') }}</span>
          <span class="settings-row__desc">{{ t('settings.showExpiredDesc') }}</span>
        </div>
        <input
          type="checkbox"
          class="settings-switch"
          :checked="settings.showExpired"
          @change="toggleShowExpired(($event.target as HTMLInputElement).checked)"
        />
      </label>
    </div>
  </section>
</template>

<style scoped>
/* 与条目管理页的分区卡片保持同一套视觉变量 */
.settings {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.settings__title {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.settings-card {
  border: 1px solid var(--ts-line);
  border-radius: 10px;
  background-color: var(--ts-surface);
  padding: 4px 16px;
}

.settings-card__divider {
  height: 1px;
  background-color: var(--ts-line);
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 0;
  cursor: pointer;
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
  opacity: 0.55;
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
  background-color: rgba(0, 0, 0, 0.18);
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
  transition: transform 0.2s;
}

.settings-switch:checked {
  background-color: var(--ts-primary);
}

.settings-switch:checked::after {
  transform: translateX(16px);
}

@media (prefers-color-scheme: dark) {
  .settings-card {
    background-color: var(--ts-surface);
    border-color: var(--ts-line);
  }

  .settings-switch {
    background-color: rgba(255, 255, 255, 0.22);
  }

  .settings-switch:checked {
    background-color: var(--ts-primary);
  }
}
</style>
