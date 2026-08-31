<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
import { useI18n } from 'vue-i18n';
defineProps<{ title: string; confirm: string; busy?: boolean; danger?: boolean }>();
const emit = defineEmits<{ confirm: []; cancel: [] }>();
const { t } = useI18n();
const dialog = ref<HTMLDialogElement>();
const previous = document.activeElement as HTMLElement | null;
onMounted(() => dialog.value?.showModal());
onBeforeUnmount(() => {
  dialog.value?.close();
  previous?.focus();
});
</script>

<template>
  <dialog
    ref="dialog"
    class="confirm-dialog"
    aria-labelledby="confirm-title"
    aria-describedby="confirm-text"
    @cancel.prevent="!busy && emit('cancel')"
  >
    <h2 id="confirm-title">{{ title }}</h2>
    <div id="confirm-text"><slot /></div>
    <div class="dialog-actions">
      <button type="button" class="btn" autofocus :disabled="busy" @click="$emit('cancel')">
        {{ t('config.cancel') }}
      </button>
      <button
        type="button"
        class="btn"
        :class="danger ? 'btn--danger' : 'btn--primary'"
        :disabled="busy"
        @click="$emit('confirm')"
      >
        {{ busy ? t('common.saving') : confirm }}
      </button>
    </div>
  </dialog>
</template>

<style scoped>
.confirm-dialog {
  width: min(410px, calc(100vw - 40px));
  border: 1px solid var(--ts-line);
  border-radius: 14px;
  color: var(--ts-text);
  background: var(--ts-surface);
  padding: 24px;
  box-shadow: 0 18px 60px var(--ts-shadow);
}
.confirm-dialog::backdrop {
  background: rgb(9 24 34 / 38%);
}
h2 {
  font-size: 17px;
  margin: 0 0 14px;
}
#confirm-text {
  font-size: 13px;
  line-height: 1.7;
  overflow-wrap: anywhere;
}
.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 24px;
}
</style>
