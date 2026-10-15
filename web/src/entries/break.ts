import '$design/styles';
import { mount } from 'svelte';
import BreakApp from '$break/BreakApp.svelte';
import { boot } from './boot';

const { backend, engine } = await boot(['settings', 'system', 'themes', 'media']);
mount(BreakApp, { target: document.getElementById('app')!, props: { backend, engine } });
