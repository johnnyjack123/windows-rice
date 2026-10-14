<script lang="ts">
  import { onMount } from 'svelte';
  import { provideBackend, type Backend } from '$backend/index';
  import { provideEngine, type ThemeEngine } from '$design/engine.svelte';
  import { connectSources } from './builtins.svelte';
  import { provideIsland, type IslandContext } from './context.svelte';
  import { createIslandContext } from './create.svelte';
  import Island from './Island.svelte';
  import { connectWindow, type IslandWindowState } from './window.svelte';

  interface Props {
    backend: Backend;
    engine: ThemeEngine;
    /** Gives dev tools (lab) access to the island internals. */
    onready?: (ctx: IslandContext, window: IslandWindowState) => void;
  }

  const props: Props = $props();
  // Backend and engine are fixed for the lifetime of the window, so reading
  // their initial value is intended.
  // svelte-ignore state_referenced_locally
  const { backend, engine } = props;
  const { ctx, windowState } = createIslandContext(backend, engine);

  provideBackend(backend);
  provideEngine(engine);
  provideIsland(ctx);

  onMount(() => {
    const stopSources = connectSources(ctx);
    const stopWindow = connectWindow(ctx, windowState);
    props.onready?.(ctx, windowState);
    return () => {
      stopSources();
      stopWindow();
    };
  });
</script>

<Island />
