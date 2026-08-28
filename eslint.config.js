import js from '@eslint/js';
import pluginVue from 'eslint-plugin-vue';
import tseslint from 'typescript-eslint';
import eslintConfigPrettier from 'eslint-config-prettier';

export default tseslint.config(
  { ignores: ['dist/', 'src-tauri/target/', 'src-tauri/gen/'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  eslintConfigPrettier,
  {
    // .vue 文件的 script 使用 TS 语法，需为 vue 解析器指定 TS 子解析器
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    // 模板中对组合式变量的使用无法被该规则识别，.vue 文件关闭
    files: ['**/*.vue'],
    rules: {
      'no-useless-assignment': 'off',
    },
  },
);
