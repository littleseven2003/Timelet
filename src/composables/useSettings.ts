import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getSettings, saveSettings, type AppSettings } from '../api/settings';

const settings = ref<AppSettings>({
  launchAtLogin: false,
  hideAppIcon: false,
  showExpired: true,
  theme: 'system',
  panelLimit: 6,
});
const loaded = ref(false);
const loadError = ref('');
const syncError = ref('');
const error = computed(() => loadError.value || syncError.value);
const busy = ref(false);
let users = 0;
let generation = 0;
let unlisten: UnlistenFn | undefined;
let listening: Promise<void> | undefined;

watch(
  () => settings.value.theme,
  (mode) => {
    if (!mode || mode === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = mode;
  },
  { immediate: true },
);

async function refresh() {
  const current = ++generation;
  try {
    const next = await getSettings();
    if (current === generation) {
      settings.value = next;
      loaded.value = true;
      loadError.value = '';
    }
  } catch (cause) {
    if (current === generation) loadError.value = String(cause);
  }
}
async function retry() {
  if (!unlisten && !listening) {
    listening = listen('settings-changed', () => {
      void refresh();
    })
      .then((stop) => {
        syncError.value = '';
        if (users) unlisten = stop;
        else stop();
      })
      .catch((cause) => {
        syncError.value = String(cause);
      })
      .finally(() => {
        listening = undefined;
      });
  }
  await listening;
  await refresh();
}
async function persist(patch: Partial<AppSettings>) {
  if (busy.value || !loaded.value || error.value) throw new Error('请先成功读取设置后再保存');
  busy.value = true;
  const next = { ...settings.value, ...patch };
  try {
    await saveSettings(next);
    ++generation;
    settings.value = next;
  } finally {
    busy.value = false;
  }
}
export function useSettings() {
  onMounted(() => {
    if (users++ === 0) {
      void retry();
      window.addEventListener('focus', retry);
    }
  });
  onBeforeUnmount(() => {
    if (--users === 0) {
      unlisten?.();
      unlisten = undefined;
      window.removeEventListener('focus', retry);
    }
  });
  return { settings, loaded, error, busy, persist, retry };
}
