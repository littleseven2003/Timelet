<script setup lang="ts">
// 统一分段控件：类型、单位、重复等互斥选项共用
defineProps<{
  options: { value: string; label: string }[];
  modelValue: string;
  disabled?: boolean;
}>();

defineEmits<{ (e: 'update:modelValue', value: string): void }>();
</script>

<template>
  <div class="seg" role="group">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      :disabled="disabled"
      class="seg__item"
      :class="{ 'seg__item--active': option.value === modelValue }"
      :aria-pressed="option.value === modelValue"
      @click="$emit('update:modelValue', option.value)"
    >
      <slot name="option" :option="option">{{ option.label }}</slot>
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: flex;
  gap: 2px;
  padding: 2px;
  border-radius: 8px;
  background-color: var(--ts-rail);
}

.seg__item {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  flex: 1;
  border: none;
  background: none;
  padding: 8px 10px;
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
</style>
