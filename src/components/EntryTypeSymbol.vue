<script setup lang="ts">
import type { EntryType } from '../types/entry';

// 类型符号（设计语言 6.3）：同一几何母体（开放圆弧 + 端点），以时间方向区分——
// 倒数日：弧线向目标点收束（端点为实心点）；正数日：弧线自起点向外展开（起点为实心点）
defineProps<{ type: EntryType }>();
</script>

<template>
  <svg
    class="type-symbol"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <!-- 倒数日：近乎闭合的弧，缺口处置一点，表示时间收束向目标 -->
    <template v-if="type === 'countdown'">
      <path d="M12.2 4.3a5.4 5.4 0 1 0 .9 5.6" />
      <circle cx="8" cy="2.4" r="1.5" fill="currentColor" stroke="none" />
    </template>
    <!-- 正数日：自左侧起点展开的大弧，表示时间向外延伸 -->
    <template v-else>
      <path d="M4.2 12.4a5.7 5.7 0 0 1 0-8.8 5.7 5.7 0 0 1 8.3 1.8" />
      <circle cx="3.1" cy="13.3" r="1.5" fill="currentColor" stroke="none" />
    </template>
  </svg>
</template>

<style scoped>
.type-symbol {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  opacity: 0.65;
}
</style>
