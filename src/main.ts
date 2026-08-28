import { createApp } from 'vue';
import App from './App.vue';
import CountdownPanel from './CountdownPanel.vue';
import { i18n } from './i18n';

// 按 URL 查询参数区分窗口视图：面板窗口挂 Panel，其余（后续配置窗口）挂 App
const params = new URLSearchParams(window.location.search);
const root = params.get('window') === 'panel' ? CountdownPanel : App;

createApp(root).use(i18n).mount('#app');
