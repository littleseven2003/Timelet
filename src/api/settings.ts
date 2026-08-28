import { invoke } from '@tauri-apps/api/core';

export interface AppSettings {
  /** 开机自启（实际自启项由官方自启插件维护，此处为展示状态） */
  launchAtLogin: boolean;
  /** 面板是否显示已过期的倒计时条目 */
  showExpired: boolean;
}

export const getSettings = () => invoke<AppSettings>('settings_get');

export const saveSettings = (settings: AppSettings) => invoke<void>('settings_set', { settings });
