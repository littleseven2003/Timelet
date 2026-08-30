<script setup lang="ts">
// 统一分段控件：类型、单位、重复等互斥选项共用
defineProps<{
  options: { value: string; label: string }[];
  modelValue: string;
}>();

defineEmits<{ (e: 'update:modelValue', value: string): void }>();
</script>

<template>
  <div class="seg" role="tablist">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      class="seg__item"
      :class="{ 'seg__item--active': option.value === modelValue }"
      :aria-pressed="option.value === modelValue"
      @click="$emit('update:modelValue', option.value)"
    >
      {{ option.label }}
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: flex;
  gap: 2px;
  padding: 2px;
  border-radius: 8px;
  background-color: rgba(23, 35, 45, 0.06);
}

.seg__item {
  flex: 1;
  border: none;
  background: none;
  padding: 6px 0;
  font-size: 13px;
  border-radius: 6px;
  cursor: pointer;
  color: inherit;
  white-space: nowrap;
  transition:
    background-color 0.15s ease-out,
    box-shadow 0.15s ease-out;
}

.seg__item--active {
  background-color: var(--ts-surface);
  font-weight: 500;
  box-shadow: 0 1px 3px rgba(23, 35, 45, 0.14);
}

@media (prefers-color-scheme: dark) {
  .seg {
    background-color: rgba(255, 255, 255, 0.08);
  }

  .seg__item--active {
    background-color: #2c3d49;
  }
}
</style>
