<script setup>
import { useSettingsStore } from '../stores/settings';
import { api } from '../services/tauri';
const store = useSettingsStore();

const opts = [
  { key: 'start_on_boot',       label: 'Start on system boot' },
  { key: 'start_minimized',     label: 'Start minimized' },
  { key: 'run_in_tray',         label: 'Run in tray' },
  { key: 'enable_notifications',label: 'Enable notifications' },
];
</script>

<template>
  <div class="max-w-xl space-y-4">
    <h2 class="text-base font-semibold">Settings</h2>
    <div class="bg-bg-panel rounded-lg border border-white/5 divide-y divide-white/5">
      <label v-for="o in opts" :key="o.key"
             class="flex items-center justify-between px-4 py-3 cursor-pointer">
        <span class="text-sm">{{ o.label }}</span>
        <input
          type="checkbox"
          class="accent-accent w-4 h-4"
          :checked="store.values[o.key]"
          @change="store.set(o.key, $event.target.checked)"
        />
      </label>
    </div>
    <p class="text-xs text-slate-500">
      Config is stored at
      <button
        type="button"
        class="text-accent hover:underline font-mono"
        @click="api.openConfig(false)"
        title="Open the config.json file"
      >~/.config/manuel/config.json</button>
      —
      <button
        type="button"
        class="text-accent hover:underline"
        @click="api.openConfig(true)"
        title="Open the folder containing config.json"
      >open folder</button>
    </p>
  </div>
</template>
