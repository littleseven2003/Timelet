<script setup lang="ts">
import type { EntryType } from '../types/entry';

// 类型图标（设计文档 6.3 定稿路径）：24×24 网格、1.75 笔画、圆角端点
// 倒数日：带顶部刻度与指针的开放计时环；正数日：带外向箭头的开放时弧与时间指针
defineProps<{ type: EntryType }>();
</script>

<template>
  <svg
    class="type-symbol"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <template v-if="type === 'countdown'">
      <path d="M9 2.75h6M12 2.75v2.5" />
      <path d="M16.75 6.85a7.6 7.6 0 1 1-8.15-1.02" />
      <path d="M12 8.5v4.35l2.8 1.65" />
    </template>
    <template v-else>
      <path d="M12.1 4.4a7.8 7.8 0 1 0 7.5 10.1" />
      <path d="M12 8.25V12l-2.6 2.1M15.25 8.75l5-5M15.75 3.75h4.5v4.5" />
    </template>
  </svg>
</template>

<style scoped>
.type-symbol {
  width: 21px;
  height: 21px;
  flex-shrink: 0;
}
</style>
