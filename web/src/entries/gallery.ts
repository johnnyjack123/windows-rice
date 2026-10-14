import '$design/styles';
import '$lab/lab.css';
import '../gallery/gallery.css';
import { mount } from 'svelte';
import GalleryApp from '../gallery/GalleryApp.svelte';
import { boot } from './boot';

const { backend, engine } = await boot(['settings', 'system', 'themes', 'media']);
mount(GalleryApp, { target: document.getElementById('app')!, props: { backend, engine } });
