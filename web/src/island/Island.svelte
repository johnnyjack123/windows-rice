<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import { VIEW_CLASS } from './builtins.svelte';
  import { useIsland } from './context.svelte';
  import Panel from './panel/Panel.svelte';
  import ClockView from './views/ClockView.svelte';

  const { registry, controller, backend, engine } = useIsland();

  let shell: HTMLDivElement;
  let shadow: HTMLDivElement;
  let bg: HTMLDivElement;
  let clip: HTMLDivElement;
  let bubble: HTMLDivElement;
  let bubbleContent: HTMLButtonElement;
  let bridge: SVGPathElement;
  const views: Record<string, HTMLElement | undefined> = $state({});

  const definitions = registry.definitions;
  const bubbleDef = $derived(controller.bubbleId ? registry.def(controller.bubbleId) : undefined);
  const mainLabel = $derived.by(() => {
    const main = registry.main;
    return main ? registry.def(main.id)?.label() ?? i18n.m.island.label : i18n.m.island.label;
  });

  onMount(() => {
    controller.attach({ shell, shadow, bg, clip, bubble, bubbleContent, bridge, views });

    // Scrolling over the compact island changes the volume.
    const wheel = (e: WheelEvent): void => {
      if (controller.expanded || !backend.settings.value.island.scrollVolume) {
        return;
      }
      e.preventDefault();
      const audio = backend.audio.state;
      void backend.audio.setVolume((audio.muted ? 0 : audio.volume) - Math.sign(e.deltaY) * 0.04);
    };
    shell.addEventListener('wheel', wheel, { passive: false });

    // Any size change of the visible view (text, fonts, theme) re-targets the shape.
    const observer = new ResizeObserver((entries) => {
      if (entries.some((entry) => entry.target === views[controller.view])) {
        controller.refresh();
      }
    });
    for (const view of Object.values(views)) {
      if (view) {
        observer.observe(view);
      }
    }

    // Browser only: a click outside closes the panel (Tauri reports focus loss instead).
    const outside = (e: PointerEvent): void => {
      const target = e.target as Element | null;
      if (controller.expanded && target && !target.closest('.island, [data-lab]')) {
        controller.collapse();
      }
    };
    const escape = (e: KeyboardEvent): void => {
      if (e.key === 'Escape' && controller.expanded) {
        controller.collapse();
        shell.focus({ preventScroll: true });
      }
    };
    document.addEventListener('pointerdown', outside);
    document.addEventListener('keydown', escape);

    return () => {
      shell.removeEventListener('wheel', wheel);
      observer.disconnect();
      document.removeEventListener('pointerdown', outside);
      document.removeEventListener('keydown', escape);
      controller.detach();
    };
  });

  // Re-arbitrate whenever the owning or splitting activity or the mode changes.
  $effect(() => {
    void [registry.main?.id, registry.second?.id, controller.expanded, controller.previewing];
    untrack(() => controller.resolve());
  });

  // Physical bump when the owning activity updates in place.
  let lastBump = { id: '', n: 0 };
  $effect(() => {
    const main = registry.main;
    const id = main?.id ?? '';
    const n = main?.bumps ?? 0;
    untrack(() => {
      if (id && id === lastBump.id && n > lastBump.n) {
        controller.bump();
      }
      lastBump = { id, n };
    });
  });

  // Theme or editor changes: new springs, sizes and materials.
  $effect(() => {
    void engine.version;
    untrack(() => controller.onThemeChange());
  });

  function click(e: MouseEvent): void {
    if (controller.expanded || (e.target as Element).closest('button:not(.bubble-content), [role="slider"]')) {
      return;
    }
    const main = registry.main;
    controller.expand(main ? registry.def(main.id)?.section ?? null : null);
  }

  function key(e: KeyboardEvent): void {
    if (e.target === shell && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      controller.toggle();
    }
  }

  function sheen(e: PointerEvent): void {
    const rect = shell.getBoundingClientRect();
    bg.style.setProperty('--mx', `${((e.clientX - rect.left) / rect.width) * 100}%`);
  }
</script>

<div class="island-anchor">
  <div
    bind:this={shell}
    class="island"
    class:is-expanded={controller.expanded}
    tabindex="0"
    role="button"
    aria-label={controller.expanded ? i18n.m.island.controlCenter : mainLabel}
    onclick={click}
    onkeydown={key}
    onpointerenter={() => controller.pointerEnter()}
    onpointerleave={() => controller.pointerLeave()}
    onpointerdown={() => controller.press()}
    onpointerup={() => controller.release()}
    onpointercancel={() => controller.release()}
    onpointermove={sheen}
  >
    <div class="island__shadow" bind:this={shadow}></div>
    <svg class="island__bridge" aria-hidden="true"><path bind:this={bridge} /></svg>
    <div class="island__bubble" bind:this={bubble}>
      <div class="mat mat-rim"></div><div class="mat mat-core"></div><div class="mat mat-tint"></div><div class="mat mat-sheen"></div>
    </div>
    <div class="island__bg" bind:this={bg}>
      <div class="mat mat-rim"></div><div class="mat mat-core"></div><div class="mat mat-tint"></div><div class="mat mat-sheen"></div>
    </div>

    <div class="island__clip" bind:this={clip}>
      <div class="view view--clock" data-view="clock" data-compact inert bind:this={views.clock}><ClockView /></div>
      {#each definitions as def (def.id)}
        <div
          class="view view--{def.id} {VIEW_CLASS[def.id] ?? ''}"
          data-view={def.id}
          data-compact={def.compact ? '' : undefined}
          inert
          bind:this={views[def.id]}
        >
          <def.view />
        </div>
        {#if def.expandedView}
          <div class="view view--live view--{def.id}-expanded" data-view="{def.id}:expanded" inert bind:this={views[`${def.id}:expanded`]}>
            <def.expandedView />
          </div>
        {/if}
      {/each}
      <div class="view view--panel" data-view="panel" inert bind:this={views.panel}><Panel /></div>
    </div>

    <button
      class="bubble-content"
      bind:this={bubbleContent}
      tabindex="-1"
      aria-label={bubbleDef?.label() ?? ''}
      onclick={(e) => { e.stopPropagation(); controller.expand(bubbleDef?.section ?? null); }}
    >
      {#if bubbleDef?.bubble}<bubbleDef.bubble />{/if}
    </button>
  </div>
</div>
