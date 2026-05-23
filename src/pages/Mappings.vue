<script setup>
import { ref } from 'vue';
import { useMappingsStore } from '../stores/mappings';
import MappingRow from '../components/MappingRow.vue';
import InputRecorder from '../components/InputRecorder.vue';

const store = useMappingsStore();
const recorderTarget = ref(null);

function openRecorder(id) {
  recorderTarget.value = id;
}

function onRecorded(captured) {
  if (recorderTarget.value) {
    store.update(recorderTarget.value, { input: captured });
  }
  recorderTarget.value = null;
}
</script>

<template>
  <div class="space-y-4 max-w-5xl mx-auto">
    <div class="flex items-center justify-between">
      <h2 class="text-base font-semibold">Mappings</h2>
      <button class="btn-primary" @click="store.add()">+ New mapping</button>
    </div>

    <div class="space-y-3">
      <MappingRow
        v-for="m in store.items"
        :key="m.id"
        :mapping="m"
        @record="openRecorder"
      />
      <div
        v-if="!store.items.length"
        class="rounded-lg border border-dashed border-white/10 px-4 py-12 text-center text-slate-500"
      >
        No mappings yet. Click "New mapping" to add one.
      </div>
    </div>

    <InputRecorder
      v-if="recorderTarget"
      @captured="onRecorded"
      @cancel="recorderTarget = null"
    />
  </div>
</template>
