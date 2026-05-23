import { defineStore } from 'pinia';
import { api } from '../services/tauri';
import { useSettingsStore } from './settings';

function newMapping() {
  return {
    id: crypto.randomUUID(),
    enabled: true,
    description: '',
    input: { kind: 'KeyCombo', value: '' }, // KeyCombo | HidButton
    output: { kind: 'Text', value: '' },    // Text | Key | Combo | Macro | Shell
  };
}

export const useMappingsStore = defineStore('mappings', {
  state: () => ({
    items: [],
    loaded: false,
  }),
  actions: {
    async load() {
      const config = await api.loadConfig();
      this.items = config.mappings || [];
      this.loaded = true;
    },
    async persist() {
      const settings = useSettingsStore();
      await api.saveConfig({
        mappings: this.items,
        settings: settings.values,
      });
      await api.reloadEngine();
    },
    add() {
      this.items.push(newMapping());
      this.persist();
    },
    remove(id) {
      this.items = this.items.filter((m) => m.id !== id);
      this.persist();
    },
    update(id, patch) {
      const m = this.items.find((x) => x.id === id);
      if (!m) return;
      Object.assign(m, patch);
      this.persist();
    },
  },
});
