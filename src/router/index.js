import { createRouter, createWebHashHistory } from 'vue-router';
import Mappings from '../pages/Mappings.vue';
import Devices from '../pages/Devices.vue';
import Settings from '../pages/Settings.vue';

const routes = [
  { path: '/', redirect: '/mappings' },
  { path: '/mappings', name: 'mappings', component: Mappings },
  { path: '/devices', name: 'devices', component: Devices },
  { path: '/settings', name: 'settings', component: Settings },
];

export default createRouter({
  history: createWebHashHistory(),
  routes,
});
