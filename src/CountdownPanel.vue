<script setup lang="ts">
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { useEntries } from './composables/useEntries';
import { useSettings } from './composables/useSettings';
import { useClock } from './composables/useClock';
import { panelSelection } from './utils/entries';
import type { Entry } from './types/entry';
import FeaturedEntry from './components/FeaturedEntry.vue';
import EntryRow from './components/EntryRow.vue';
import IslandAtmosphere from './components/IslandAtmosphere.vue';
import InterfaceSymbol from './components/InterfaceSymbol.vue';

const { t, locale } = useI18n();
const { entries, loaded, loading, error, busy, reload, upsert } = useEntries();
const { settings, error: settingsError, retry: retrySettings } = useSettings();
const now = useClock();
const expanded = ref<string | null>(null);
const actionError = ref('');
const selection = computed(() =>
  panelSelection(entries.value, now.value, settings.value.showExpired, settings.value.panelLimit),
);
const empty = computed(() => !selection.value.featured && selection.value.groups.length === 0);
const today = computed(() =>
  new Date(now.value).toLocaleDateString(locale.value, {
    month: 'long',
    day: 'numeric',
    weekday: 'short',
  }),
);
async function act(action: () => Promise<unknown>) {
  actionError.value = '';
  try {
    await action();
  } catch (cause) {
    actionError.value = String(cause);
  }
}
const openMain = () => act(() => invoke('open_main_window'));
const openCreate = () => act(() => invoke('open_main_create'));
const openEdit = (entry: Entry) => act(() => invoke('open_entry_editor', { id: entry.id }));
const pin = (entry: Entry) =>
  act(() => upsert({ ...entry, pinned: !entry.pinned, updatedAt: new Date().toISOString() }));
</script>

<template>
  <aside
    class="panel"
    @contextmenu.self.prevent="act(() => invoke('show_panel_menu', { entryId: null }))"
  >
    <IslandAtmosphere :strength="0.7" />
    <header class="panel-header">
      <div>
        <strong>{{ t('panel.brand') }}</strong
        ><span>{{ today }}</span>
      </div>
      <button
        class="add-button"
        type="button"
        :aria-label="t('panel.addEntry')"
        :disabled="busy || !!error"
        @click="openCreate"
      >
        <InterfaceSymbol name="plus" />
      </button>
    </header>
    <div class="panel-scroll">
      <div v-if="error" class="error-notice" role="alert">
        <strong>{{ t('common.loadError') }}</strong>
        <p>{{ error }}</p>
        <button class="btn" type="button" :disabled="loading" @click="reload">
          {{ t('common.retry') }}
        </button>
      </div>
      <div v-if="settingsError" class="error-notice" role="alert">
        {{ settingsError
        }}<button class="btn" type="button" @click="retrySettings">{{ t('common.retry') }}</button>
      </div>
      <div v-if="actionError" class="error-notice" role="alert">
        {{ actionError
        }}<button class="btn" type="button" @click="actionError = ''">
          {{ t('common.dismiss') }}
        </button>
      </div>
      <p v-if="!loaded && !error" class="empty" role="status">{{ t('common.loading') }}</p>
      <div v-else-if="loaded && empty && !error" class="empty">
        <InterfaceSymbol name="now" />
        <p>
          {{
            t(entries.some((entry) => !entry.archived) ? 'panel.filteredEmpty' : 'panel.emptyTitle')
          }}
        </p>
        <button class="btn" type="button" @click="openCreate">
          {{ t(entries.some((entry) => !entry.archived) ? 'panel.addEntry' : 'panel.createFirst') }}
        </button>
      </div>
      <FeaturedEntry
        v-if="selection.featured"
        :entry="selection.featured"
        :now="now"
        compact
        @edit="openEdit"
      />
      <section v-for="group in selection.groups" :key="group.key" class="panel-group">
        <h2>{{ t(`panel.sections.${group.key}`) }}</h2>
        <ul>
          <EntryRow
            v-for="entry in group.items"
            :key="entry.id"
            :entry="entry"
            :now="now"
            :expanded="expanded === entry.id"
            compact
            :disabled="busy || !!error"
            @expand="expanded = expanded === entry.id ? null : entry.id"
            @edit="openEdit(entry)"
            @pin="pin(entry)"
          />
        </ul>
      </section>
    </div>
    <footer class="panel-footer">
      <button type="button" @click="openMain">
        <InterfaceSymbol name="window" /><span>{{ t('panel.viewAll') }}</span
        ><InterfaceSymbol name="arrow" />
      </button>
    </footer>
  </aside>
</template>

<style scoped>
.panel {
  position: relative;
  height: 100dvh;
  min-height: 260px;
  background: var(--ts-surface);
  border: 1px solid var(--ts-line);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  isolation: isolate;
  color: var(--ts-text);
}
.panel-header {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 19px 19px 15px;
  flex-shrink: 0;
}
.panel-header > div {
  display: flex;
  gap: 10px;
  align-items: baseline;
  flex-wrap: wrap;
}
.panel-header strong {
  font-size: 16px;
  font-weight: 550;
  letter-spacing: 0.1em;
}
.panel-header span {
  font-size: 10px;
  color: var(--ts-text-2);
}
.add-button {
  display: grid;
  place-items: center;
  border: 1px solid var(--ts-line);
  background: var(--ts-surface);
  color: var(--ts-blue);
  border-radius: 7px;
  min-width: 30px;
  height: 30px;
  cursor: pointer;
}
.panel-scroll {
  position: relative;
  flex: 1;
  overflow-y: auto;
  padding: 0 14px 10px;
  scrollbar-width: thin;
  min-height: 0;
}
.panel-group {
  margin-top: 17px;
}
.panel-group h2 {
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.05em;
  margin: 0 3px 3px;
  color: var(--ts-text-2);
}
.panel-group ul {
  padding: 0;
  margin: 0;
}
.empty {
  padding: 48px 10px;
  text-align: center;
  font-size: 12px;
  color: var(--ts-text-2);
  line-height: 1.8;
}
.empty svg {
  width: 46px;
  height: 46px;
  color: var(--ts-blue);
  opacity: 0.6;
}
.panel-footer {
  position: relative;
  border-top: 1px solid var(--ts-line);
  flex-shrink: 0;
}
.panel-footer button {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 14px 18px;
  color: var(--ts-text-2);
  background: transparent;
  border: 0;
  font-size: 11px;
  cursor: pointer;
}
.panel-footer button span {
  flex: 1;
  text-align: left;
}
.panel-footer button:hover {
  background: var(--ts-hover);
}
.panel-footer button svg:last-child {
  width: 13px;
}
</style>
