<script setup>
import { onMounted } from 'vue';
import { RouterLink, RouterView } from 'vue-router';
import { useMappingsStore } from './stores/mappings';
import { useSettingsStore } from './stores/settings';
import { useDevicesStore } from './stores/devices';
import logo from './assets/logo.png';

const mappings = useMappingsStore();
const settings = useSettingsStore();
const devices = useDevicesStore();

onMounted(async () => {
  await Promise.all([
    mappings.load(),
    settings.load(),
    devices.refresh(),
  ]);
});
</script>

<template>
  <div class="flex flex-col h-full">
    <header class="flex items-center gap-2 px-4 py-3 border-b border-white/5">
      <img :src="logo" alt="" class="w-7 h-7 rounded shrink-0" />
      <div class="text-lg font-semibold tracking-tight">Manuel</div>
      <nav class="ml-6 flex gap-1">
        <RouterLink to="/mappings" class="tab-link"
                    active-class="tab-link-active">Mappings</RouterLink>
        <RouterLink to="/devices" class="tab-link"
                    active-class="tab-link-active">Devices</RouterLink>
        <RouterLink to="/settings" class="tab-link"
                    active-class="tab-link-active">Settings</RouterLink>
      </nav>
    </header>
    <main class="flex-1 overflow-auto p-4">
      <RouterView />
    </main>
    <footer class="px-4 py-2 border-t border-white/5 text-[11px] text-slate-500 text-right">
      by jaguilar025 | 2026
    </footer>
  </div>
</template>
