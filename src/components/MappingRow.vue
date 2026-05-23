<script setup>
import { computed } from 'vue';
import { useMappingsStore } from '../stores/mappings';

const props = defineProps({ mapping: { type: Object, required: true } });
const emit = defineEmits(['record']);
const store = useMappingsStore();

const m = computed(() => props.mapping);

function patch(p)        { store.update(m.value.id, p); }
function patchInput(p)   { patch({ input:  { ...m.value.input,  ...p } }); }
function patchOutput(p)  { patch({ output: { ...m.value.output, ...p } }); }

const triggerLabel = computed(() => {
  const v = m.value.input.value;
  if (!v) return '';
  if (typeof v === 'string') return v;
  if (v.device_name) {
    const vid = v.vendor_id ? `0x${v.vendor_id.toString(16).padStart(4, '0')}` : '?';
    const pid = v.product_id ? `0x${v.product_id.toString(16).padStart(4, '0')}` : '?';
    return `${v.device_name} (${vid}:${pid}) — code ${v.code}`;
  }
  return JSON.stringify(v);
});
</script>

<template>
  <div class="rounded-lg border border-white/5 bg-bg-panel/50 px-4 py-3 space-y-3">
    <!-- Row 1: When -->
    <div class="flex items-center gap-3">
      <div class="text-xs text-slate-500 w-10 shrink-0">When</div>

      <select
        class="input w-40 shrink-0"
        :value="m.input.kind"
        @change="patchInput({ kind: $event.target.value, value: '' })"
      >
        <option value="KeyCombo">Keyboard combo</option>
        <option value="HidButton">HID button</option>
      </select>

      <input
        class="input flex-1 font-mono text-xs"
        :value="triggerLabel"
        readonly
        placeholder="(not set — click Detect)"
      />

      <button class="btn shrink-0" @click="emit('record', m.id)">Detect</button>
      <button class="btn shrink-0" @click="store.remove(m.id)" title="Delete mapping">✕</button>
    </div>

    <!-- Row 2: Do -->
    <div class="flex items-start gap-3">
      <div class="text-xs text-slate-500 w-10 shrink-0 pt-2">Do</div>

      <select
        class="input w-40 shrink-0"
        :value="m.output.kind"
        @change="patchOutput({ kind: $event.target.value, value: '' })"
      >
        <option value="Text">Type text</option>
        <option value="Key">Press key</option>
        <option value="Combo">Press combo</option>
        <option value="Macro">Run macro</option>
        <option value="Shell">Shell command</option>
      </select>

      <textarea
        v-if="m.output.kind === 'Macro'"
        rows="5"
        class="input flex-1 font-mono text-xs"
        :value="typeof m.output.value === 'string'
                  ? m.output.value
                  : JSON.stringify(m.output.value, null, 2)"
        @change="patchOutput({ value: $event.target.value })"
        placeholder='[
  {"op":"type","text":"Hola"},
  {"op":"delay","ms":100},
  {"op":"press","key":"Enter"}
]'
      />
      <input
        v-else
        class="input flex-1"
        :value="m.output.value"
        @change="patchOutput({ value: $event.target.value })"
        :placeholder="m.output.kind === 'Text'   ? 'e.g. ñ' :
                      m.output.kind === 'Key'    ? 'e.g. Enter, F13' :
                      m.output.kind === 'Combo'  ? 'e.g. Ctrl+Shift+P' :
                      'e.g. notify-send hi'"
      />

      <button
        type="button"
        @click="patch({ enabled: !m.enabled })"
        :class="[
          'shrink-0 w-[38px] h-[34px] rounded-md border transition',
          m.enabled
            ? 'bg-emerald-500 hover:bg-emerald-400 border-emerald-400'
            : 'bg-bg-panel hover:bg-bg-soft border-white/5'
        ]"
        :title="m.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'"
      ></button>
    </div>
  </div>
</template>
