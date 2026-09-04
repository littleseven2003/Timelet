<script setup lang="ts">
defineProps<{
  name: 'now' | 'archive' | 'settings' | 'about' | 'window' | 'search' | 'arrow' | 'plus' | 'pin';
}>();
const paths = {
  now: 'M3 15c3-2 5-2 9 0s6 2 9 0M4 19c3-2 5-2 8 0s5 2 8 0M7 11a5 5 0 0 1 10 0',
  archive: 'M4 4h16v4H4zM5 8v12h14V8M10 12h4',
  settings: 'M4 6h16M4 12h16M4 18h16M8 3v6M16 9v6M10 15v6',
  about: 'M12 16v-5M12 7h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z',
  window:
    'M4 4.5h16a1.5 1.5 0 0 1 1.5 1.5v12a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 18V6A1.5 1.5 0 0 1 4 4.5ZM2.5 8.5h19M8 8.5v11',
  search: 'M16 16l5 5M18 10.5a7.5 7.5 0 1 1-15 0 7.5 7.5 0 0 1 15 0Z',
  arrow: 'm9 5 7 7-7 7',
  plus: 'M12 5v14M5 12h14',
  pin: 'm8 4 8 1-1 6 4 4-6 1-5 5 1-7-4-4 5-1z',
};
</script>
<template>
  <svg
    viewBox="0 0 24 24"
    width="18"
    height="18"
    fill="none"
    stroke="currentColor"
    stroke-width="1.65"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <path :d="paths[name]" />
  </svg>
</template>
