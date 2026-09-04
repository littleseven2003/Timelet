import { onMounted, onBeforeUnmount, readonly, ref } from 'vue';

const now = ref(Date.now());
let users = 0;
let timer: ReturnType<typeof setInterval> | undefined;
const refresh = () => {
  now.value = Date.now();
};

export function useClock() {
  onMounted(() => {
    if (users++ === 0) {
      refresh();
      timer = setInterval(refresh, 1000);
      window.addEventListener('focus', refresh);
      document.addEventListener('visibilitychange', refresh);
    }
  });
  onBeforeUnmount(() => {
    if (--users === 0) {
      clearInterval(timer);
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', refresh);
    }
  });
  return readonly(now);
}
