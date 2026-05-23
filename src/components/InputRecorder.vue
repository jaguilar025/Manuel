<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { api } from '../services/tauri';

const emit = defineEmits(['captured', 'cancel']);
const status = ref('Listening… press a key, combo, or HID button.');
let unlisten = null;

onMounted(async () => {
  unlisten = await api.onRecorded((payload) => {
    status.value = 'Captured.';
    emit('captured', payload);
  });
  await api.startRecording();
});

onUnmounted(async () => {
  if (unlisten) unlisten();
  await api.stopRecording();
});
</script>

<template>
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
    <div class="bg-bg-panel border border-white/10 rounded-xl p-6 w-[28rem] space-y-4">
      <div class="text-lg font-semibold">Detect input</div>
      <div class="text-sm text-slate-400">{{ status }}</div>
      <div class="flex justify-end gap-2">
        <button class="btn" @click="emit('cancel')">Cancel</button>
      </div>
    </div>
  </div>
</template>
