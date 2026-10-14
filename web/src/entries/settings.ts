import '$design/styles';
import { mount } from 'svelte';
import SettingsApp from '$settings/SettingsApp.svelte';
import { boot } from './boot';

const { backend, engine } = await boot(['settings', 'system', 'themes', 'media', 'calendar']);
mount(SettingsApp, { target: document.getElementById('app')!, props: { backend, engine } });
