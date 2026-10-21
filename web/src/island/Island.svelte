<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import { VIEW_CLASS } from './builtins.svelte';
  import { useIsland } from './context.svelte';
  import { LiquidGlass, readGlassParams } from './glass/liquid-glass';
  import Panel from './panel/Panel.svelte';
  import ClockView from './views/ClockView.svelte';

  const { registry, controller, backend, engine } = useIsland();

  let shell: HTMLDivElement;
  let shadow: HTMLDivElement;
  let bg: HTMLDivElement;
  let clip: HTMLDivElement;
  let bubble: HTMLDivElement;
  let bubbleContent: HTMLDivElement;
  let bridge: SVGPathElement;
  let glassCanvas: HTMLCanvasElement;
  let glass: LiquidGlass | null = null;
  const views: Record<string, HTMLElement | undefined> = $state({});
  const bubbleViews: { compact?: HTMLElement; expanded?: HTMLElement } = $state({});

  // Any size change of a visible view (text, fonts, theme) re-targets the shape.
  const observer = new ResizeObserver((entries) => {
    const watched = [views[controller.view], bubbleViews.expanded];
    if (entries.some((entry) => watched.includes(entry.target as HTMLElement))) {
      controller.refresh();
    }
  });

  const definitions = registry.definitions;
  const bubbleDef = $derived(controller.bubbleId ? registry.def(controller.bubbleId) : undefined);
  const mainLabel = $derived.by(() => {
    const main = registry.main;
    return main ? registry.def(main.id)?.label() ?? i18n.m.island.label : i18n.m.island.label;
  });

  /** Glass themes get the WebGL material when available, otherwise the CSS layers. */
  function applyGlass(): void {
    const root = document.documentElement;
    const isGlass = getComputedStyle(root).getPropertyValue('--island-material').trim() === 'glass';
    const on = glass?.setEnabled(isGlass) ?? false;
    if (on) {
      glass?.setParams(readGlassParams(root));
    }
    root.dataset.glass = on ? 'webgl' : 'css';
  }

  onMount(() => {
    glass = new LiquidGlass(glassCanvas, () => {
      applyGlass();
      controller.onThemeChange();
    });
    controller.attach({ shell, shadow, bg, clip, bubble, bubbleContent, bridge, views, bubbleViews, glass });

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
      glass?.dispose();
      glass = null;
    };
  });

  $effect(() => {
    const layers = engine.glassLayers;
    untrack(() => void glass?.setLayers(layers));
  });

  $effect(() => {
    const placement = engine.glass;
    untrack(() => glass?.setPlacement(placement));
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

  // The bubble's preview mounts with the bubble's activity.
  $effect(() => {
    const view = bubbleViews.expanded;
    if (view) {
      observer.observe(view);
      return () => observer.unobserve(view);
    }
  });

  // Theme or editor changes: new springs, sizes and materials.
  $effect(() => {
    void engine.version;
    untrack(() => {
      applyGlass();
      controller.onThemeChange();
    });
  });

  function click(e: MouseEvent): void {
    if (controller.expanded || (e.target as Element).closest('button, [role="slider"], .bubble-content')) {
      return;
    }
    const main = registry.main;
    const def = main ? registry.def(main.id) : undefined;
    if (def?.open) {
      void def.open().catch(() => controller.expand(def.section ?? null));
      return;
    }
    controller.expand(def?.section ?? null);
  }

  function key(e: KeyboardEvent): void {
    if (e.target === shell && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      controller.toggle();
    }
  }

  function openBubble(e: MouseEvent): void {
    e.stopPropagation();
    // Quick actions inside the bubble's preview act on their own.
    if (!(e.target as Element).closest('button, [role="slider"]')) {
      controller.expand(bubbleDef?.section ?? null);
    }
  }

  function bubbleKey(e: KeyboardEvent): void {
    if (e.target === bubbleContent && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      e.stopPropagation();
      controller.expand(bubbleDef?.section ?? null);
    }
  }

  // Mice report up to 1000 moves per second; the sheen and the glass light follow once per frame.
  let pointer: { x: number; y: number } | null = null;
  let pointerFrame = 0;

  function sheen(e: PointerEvent): void {
    pointer = { x: e.clientX, y: e.clientY };
    if (!pointerFrame) {
      pointerFrame = requestAnimationFrame(() => {
        pointerFrame = 0;
        if (pointer) {
          const rect = shell.getBoundingClientRect();
          bg.style.setProperty('--mx', `${((pointer.x - rect.left) / rect.width) * 100}%`);
          glass?.setPointer(pointer.x, pointer.y);
        }
      });
    }
  }

  function sheenEnd(): void {
    pointer = null;
    glass?.setPointer(null);
  }
</script>

<div class="island-anchor">
  <canvas class="island-glass" bind:this={glassCanvas} aria-hidden="true"></canvas>
  <div
    bind:this={shell}
    class="island"
    class:is-expanded={controller.expanded}
    tabindex="0"
    role="button"
    aria-label={controller.expanded ? i18n.m.island.controlCenter : mainLabel}
    onclick={click}
    onkeydown={key}
    onpointerup={() => controller.release()}
    onpointercancel={() => controller.release()}
    onpointermove={sheen}
    onpointerleave={sheenEnd}
  >
    <div class="island__shadow" bind:this={shadow}></div>
    <svg class="island__bridge" aria-hidden="true"><path bind:this={bridge} /></svg>
    <div class="island__bubble" bind:this={bubble}>
      <div class="mat mat-rim"></div><div class="mat mat-core"></div><div class="mat mat-tint"></div><div class="mat mat-sheen"></div>
    </div>
    <div class="island__bg" bind:this={bg}>
      <div class="mat mat-rim"></div><div class="mat mat-core"></div><div class="mat mat-tint"></div><div class="mat mat-sheen"></div>
    </div>

    <div
      class="island__clip"
      bind:this={clip}
      onpointerenter={() => controller.pointerEnter('main')}
      onpointerleave={() => controller.pointerLeave('main')}
      onpointerdown={() => controller.press('main')}
      role="presentation"
    >
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

    <div
      class="bubble-content"
      bind:this={bubbleContent}
      role="button"
      tabindex="-1"
      aria-label={bubbleDef?.label() ?? ''}
      onclick={openBubble}
      onkeydown={bubbleKey}
      onpointerenter={() => controller.pointerEnter('bubble')}
      onpointerleave={() => controller.pointerLeave('bubble')}
      onpointerdown={() => controller.press('bubble')}
    >
      <div class="view view--bubble" inert bind:this={bubbleViews.compact}>
        {#if bubbleDef?.bubble}<bubbleDef.bubble />{/if}
      </div>
      {#if bubbleDef?.expandedView}
        <div class="view view--live view--{bubbleDef.id}-expanded" inert bind:this={bubbleViews.expanded}>
          <bubbleDef.expandedView />
        </div>
      {/if}
    </div>
  </div>
</div>
