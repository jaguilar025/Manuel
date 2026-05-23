import { defineStore } from 'pinia';
import { api } from '../services/tauri';

const defaults = {
  start_on_boot: false,
  start_minimized: false,
  run_in_tray: true,
  enable_notifications: true,
};

export const useSettingsStore = defineStore('settings', {
  state: () => ({ values: { ...defaults } }),
  actions: {
    async load() {
      const config = await api.loadConfig();
      this.values = { ...defaults, ...(config.settings || {}) };
      // Sync autostart with reality
      this.values.start_on_boot = await api.isAutostartEnabled();
    },
    async set(key, value) {
      this.values[key] = value;
      if (key === 'start_on_boot') {
        await api.setAutostart(value);
      }
      const { useMappingsStore } = await import('./mappings');
      await api.saveConfig({
        mappings: useMappingsStore().items,
        settings: this.values,
      });
    },
  },
});
