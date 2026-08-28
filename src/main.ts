import { createApp } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import App from './App.vue';
import CountdownPanel from './CountdownPanel.vue';
import ConfigWindow from './ConfigWindow.vue';
import { i18n } from './i18n';
import './style.css';

// 按 URL 参数或窗口标签区分视图：面板、配置窗口与主视图
function resolveRoot() {
  const param = new URLSearchParams(window.location.search).get('window');
  if (param === 'panel') return CountdownPanel;
  if (param === 'config') return ConfigWindow;

  try {
    const label = getCurrentWindow().label;
    if (label === 'panel') return CountdownPanel;
    if (label === 'config') return ConfigWindow;
  } catch {
    // 纯浏览器环境无窗口内部对象，走默认视图
  }
  return App;
}

createApp(resolveRoot()).use(i18n).mount('#app');
