import '$design/styles';
import { mount } from 'svelte';
import IslandApp from '$island/IslandApp.svelte';
import { boot, ISLAND_STORES } from './boot';

const { backend, engine } = await boot(ISLAND_STORES);
mount(IslandApp, { target: document.getElementById('app')!, props: { backend, engine } });
