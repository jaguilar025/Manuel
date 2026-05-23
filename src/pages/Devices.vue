<script setup>
import { onMounted } from 'vue';
import { useDevicesStore } from '../stores/devices';

const store = useDevicesStore();
onMounted(() => { store.refresh(); store.subscribe(); });
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-base font-semibold">Devices</h2>
      <button class="btn" @click="store.refresh()">Refresh</button>
    </div>
    <div class="overflow-hidden rounded-lg border border-white/5">
      <table class="w-full text-sm">
        <thead class="bg-bg-panel text-slate-400">
          <tr>
            <th class="text-left px-3 py-2">Name</th>
            <th class="text-left px-3 py-2 w-32">Vendor</th>
            <th class="text-left px-3 py-2 w-32">Product</th>
            <th class="text-left px-3 py-2 w-40">Path</th>
            <th class="text-left px-3 py-2 w-32">Status</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in store.list" :key="d.path"
              class="border-t border-white/5">
            <td class="px-3 py-2">{{ d.name }}</td>
            <td class="px-3 py-2 font-mono">
              {{ d.vendor_id ? '0x' + d.vendor_id.toString(16).padStart(4, '0') : '—' }}
            </td>
            <td class="px-3 py-2 font-mono">
              {{ d.product_id ? '0x' + d.product_id.toString(16).padStart(4, '0') : '—' }}
            </td>
            <td class="px-3 py-2 font-mono text-xs text-slate-400">{{ d.path }}</td>
            <td class="px-3 py-2">
              <span :class="d.connected ? 'text-emerald-400' : 'text-slate-500'">
                {{ d.connected ? 'Connected' : 'Disconnected' }}
              </span>
            </td>
          </tr>
          <tr v-if="!store.list.length">
            <td colspan="5" class="px-3 py-6 text-center text-slate-500">
              No devices detected. Make sure your user is in the <code>input</code> group.
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
