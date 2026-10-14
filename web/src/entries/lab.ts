import '$design/styles';
import '$lab/lab.css';
import { mount } from 'svelte';
import LabApp from '$lab/LabApp.svelte';
import { boot, ISLAND_STORES } from './boot';

const { backend, engine } = await boot(ISLAND_STORES);
mount(LabApp, { target: document.getElementById('app')!, props: { backend, engine } });
