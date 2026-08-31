<script setup lang="ts">
withDefaults(defineProps<{ strength?: number }>(), { strength: 1 });
</script>

<template>
  <div class="atmosphere" :style="{ opacity: strength }" aria-hidden="true">
    <svg viewBox="0 0 800 310" preserveAspectRatio="xMaxYMax slice" focusable="false">
      <path class="far" d="M240 310c72-30 115-60 193-65 91-6 98-70 188-74 78-4 126 29 179 8v131Z" />
      <path class="near" d="M370 310c62-19 108-34 160-31 68 4 80-43 147-48 58-4 92 16 123 6v73Z" />
      <path class="water" d="M221 285c83-17 119-59 210-63 91-4 104-71 186-75 88-5 126 30 183 8" />
      <path class="water" d="M455 298c57-11 103-10 140-34 63-41 129-17 205-40" opacity=".65" />
      <path class="water" d="M45 289c39-5 68-3 94-9m-56 22c40-3 62-5 89-12" opacity=".55" />
    </svg>
  </div>
</template>

<style scoped>
.atmosphere {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
  z-index: 0;
  background:
    radial-gradient(ellipse at 96% 5%, var(--ts-air), transparent 57%),
    radial-gradient(ellipse at 74% 110%, var(--ts-horizon), transparent 56%);
}
svg {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 100%;
  min-width: 500px;
  height: auto;
}
.far {
  fill: var(--ts-isle-far);
  opacity: 0.38;
}
.near {
  fill: var(--ts-isle-near);
  opacity: 0.23;
}
.water {
  stroke: var(--ts-current);
  fill: none;
  stroke-width: 1.4;
  opacity: 0.27;
}
@media (prefers-contrast: more), (forced-colors: active) {
  .atmosphere {
    display: none;
  }
}
</style>
