<script lang="ts">
  import { untrack } from 'svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import Tabs from '$ui/Tabs.svelte';
  import { swapPanels } from '$ui/swap';
  import { useIsland } from '../context.svelte';
  import BluetoothPage from './BluetoothPage.svelte';
  import PanelHeader from './PanelHeader.svelte';
  import QuickSettings from './QuickSettings.svelte';
  import { builtinSections } from './sections';
  import { builtinTiles } from './tiles';
  import WifiPage from './WifiPage.svelte';

  const { backend, panel, timers } = useIsland();
  const tiles = builtinTiles(backend);
  const sections = builtinSections(backend, timers);

  const config = $derived(backend.settings.value.panel);
  const visible = $derived(
    config.sections.filter((id) => sections[id] && !config.hiddenSections.includes(id)).map((id) => sections[id]!),
  );
  const activeSection = $derived(visible.some((s) => s.id === panel.section) ? panel.section : (visible[0]?.id ?? ''));

  let stack: HTMLDivElement;
  let panes: HTMLDivElement;
  let shownPage: string | null = null;
  let shownSection: string | null = null;

  $effect(() => {
    const page = panel.page;
    untrack(() => {
      const from = shownPage ? stack.querySelector<HTMLElement>(`[data-page="${shownPage}"]`) : null;
      const to = stack.querySelector<HTMLElement>(`[data-page="${page}"]`);
      if (to) {
        swapPanels(stack, from, to, shownPage === null ? 0 : page === 'home' ? -1 : 1);
        shownPage = page;
      }
    });
  });

  $effect(() => {
    const id = activeSection;
    const order = visible.map((s) => s.id);
    untrack(() => {
      const from = shownSection ? panes.querySelector<HTMLElement>(`[data-pane="${shownSection}"]`) : null;
      const to = panes.querySelector<HTMLElement>(`[data-pane="${id}"]`);
      if (to) {
        const dir = shownSection === null ? 0 : order.indexOf(id) > order.indexOf(shownSection) ? 1 : -1;
        swapPanels(panes, from, to, dir);
        shownSection = id;
      }
    });
  });

  /** The tab is remembered when the panel closes (see create.svelte.ts), not on every click. */
  function select(id: string): void {
    panel.section = id;
  }
</script>

<header class="p-header" data-stagger style:--i={0}>
  <PanelHeader />
</header>

<div class="p-body">
  <div class="p-left" bind:this={stack}>
    <div class="page" data-page="home" inert><QuickSettings {tiles} /></div>
    <div class="page page--detail" data-page="wifi" inert><WifiPage /></div>
    <div class="page page--detail" data-page="bluetooth" inert><BluetoothPage /></div>
  </div>

  <section class="p-right" data-stagger style:--i={2}>
    <Tabs
      label={i18n.m.sections.label}
      active={activeSection}
      items={visible.map((s) => ({ id: s.id, label: s.label(), icon: s.icon, badge: s.badge?.() }))}
      onselect={select}
    />
    <div class="panes" bind:this={panes}>
      {#each visible as section (section.id)}
        <div class="pane" data-pane={section.id} role="tabpanel" aria-label={section.label()} inert>
          <section.component />
        </div>
      {/each}
    </div>
  </section>
</div>
