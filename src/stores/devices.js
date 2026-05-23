import { defineStore } from 'pinia';
import { api } from '../services/tauri';

export const useDevicesStore = defineStore('devices', {
  state: () => ({ list: [] }),
  actions: {
    async refresh() {
      this.list = await api.listDevices();
    },
    subscribe() {
      api.onDeviceChange((payload) => {
        this.list = payload;
      });
    },
  },
});
