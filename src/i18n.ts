import { createI18n } from 'vue-i18n';
import zhCN from './locales/zh-CN';

// 首版仅中文；en-US 语言包已预留，目录结构与注册方式保留双语扩展能力
export const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
  },
});
