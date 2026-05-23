import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export const api = {
  // ---- Config / mappings ----
  loadConfig: () => invoke('load_config'),
  saveConfig: (config) => invoke('save_config', { config }),

  // ---- Devices ----
  listDevices: () => invoke('list_devices'),

  // ---- Input recorder ----
  startRecording: () => invoke('start_recording'),
  stopRecording: () => invoke('stop_recording'),

  // ---- Engine ----
  reloadEngine: () => invoke('reload_engine'),
  testOutput: (output) => invoke('test_output', { output }),

  // ---- Autostart ----
  setAutostart: (enabled) => invoke('set_autostart', { enabled }),
  isAutostartEnabled: () => invoke('is_autostart_enabled'),

  // ---- Window ----
  hideWindow: () => invoke('hide_window'),
  showWindow: () => invoke('show_window'),

  // ---- Open config in file manager / editor ----
  openConfig: (reveal = false) => invoke('open_config', { reveal }),

  // ---- Event listeners ----
  onRecorded: (cb) => listen('input-recorded', (e) => cb(e.payload)),
  onDeviceChange: (cb) => listen('devices-changed', (e) => cb(e.payload)),
};
