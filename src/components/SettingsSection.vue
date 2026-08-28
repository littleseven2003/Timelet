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

    <label class="settings__row">
      <div class="settings__text">
        <span class="settings__label">{{ t('settings.launchAtLogin') }}</span>
        <span class="settings__desc">{{ t('settings.launchAtLoginDesc') }}</span>
        <span v-if="launchError" class="settings__error">{{ t('settings.launchError') }}</span>
      </div>
      <input
        type="checkbox"
        class="settings__switch"
        :checked="launchEnabled"
        @change="toggleLaunch(($event.target as HTMLInputElement).checked)"
      />
    </label>

    <label class="settings__row">
      <div class="settings__text">
        <span class="settings__label">{{ t('settings.showExpired') }}</span>
        <span class="settings__desc">{{ t('settings.showExpiredDesc') }}</span>
      </div>
      <input
        type="checkbox"
        class="settings__switch"
        :checked="settings.showExpired"
        @change="toggleShowExpired(($event.target as HTMLInputElement).checked)"
      />
    </label>
  </section>
</template>

<style scoped>
.settings {
  max-width: 480px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.settings__title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 4px;
}

.settings__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 14px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  background-color: #fff;
  cursor: pointer;
}

.settings__text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings__label {
  font-size: 14px;
  font-weight: 500;
}

.settings__desc {
  font-size: 12px;
  opacity: 0.55;
}

.settings__error {
  font-size: 12px;
  color: #d33;
}

/* 开关样式 */
.settings__switch {
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

.settings__switch::after {
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

.settings__switch:checked {
  background-color: #0067c0;
}

.settings__switch:checked::after {
  transform: translateX(16px);
}

@media (prefers-color-scheme: dark) {
  .settings__row {
    background-color: #333;
    border-color: rgba(255, 255, 255, 0.12);
  }

  .settings__switch {
    background-color: rgba(255, 255, 255, 0.22);
  }

  .settings__switch:checked {
    background-color: #0067c0;
  }
}
</style>
