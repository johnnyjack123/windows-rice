<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { useEngine } from '$design/engine.svelte';
  import type { MascotMood } from './motion';
  import { attachMascot, type MascotHandle } from './renderer';

  interface Props {
    mood?: MascotMood;
    /** Never animates (gallery); otherwise it moves while its island view is shown. */
    still?: boolean;
    class?: string;
  }

  const { mood = 'idle', still = false, class: className = '' }: Props = $props();
  const engine = useEngine();
  let canvas: HTMLCanvasElement;
  let handle = $state<MascotHandle | null>(null);

  onMount(() => {
    const mascot = attachMascot(canvas, still);
    handle = mascot;
    // Only the view on show animates (DESIGN §11.4: hidden views stand still).
    const view = canvas.closest('.view');
    const sync = (): void => mascot.setActive(view?.classList.contains('is-active') ?? false);
    const classes = new MutationObserver(sync);
    if (view) {
      classes.observe(view, { attributes: true, attributeFilter: ['class'] });
    }
    sync();
    const size = new ResizeObserver(() => mascot.refresh());
    size.observe(canvas);
    return () => {
      classes.disconnect();
      size.disconnect();
      mascot.dispose();
      handle = null;
    };
  });

  $effect(() => {
    handle?.setMood(mood);
  });

  // Theme, scheme or motion setting changed: new colours, maybe no motion.
  $effect(() => {
    void engine.version;
    untrack(() => handle?.refresh());
  });
</script>

<canvas bind:this={canvas} class="mascot {className}" aria-hidden="true"></canvas>
