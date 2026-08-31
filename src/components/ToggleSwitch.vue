<script setup lang="ts">
// 统一开关控件：包含时刻、置顶、设置页共用
defineProps<{ modelValue: boolean }>();

defineEmits<{ (e: 'update:modelValue', value: boolean): void }>();
</script>

<template>
  <input
    type="checkbox"
    class="toggle"
    :checked="modelValue"
    @change="$emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
  />
</template>

<style scoped>
.toggle {
  appearance: none;
  width: 38px;
  height: 22px;
  border-radius: 11px;
  background-color: var(--ts-line);
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  transition: background-color 0.2s;
}

.toggle::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background-color: #fff;
  transition: transform 0.2s ease-out;
}

.toggle:checked {
  background-color: var(--ts-primary);
}

.toggle:checked::after {
  transform: translateX(16px);
}
</style>
