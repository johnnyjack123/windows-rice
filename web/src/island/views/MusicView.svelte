<script lang="ts">
  import Art from '$ui/Art.svelte';
  import Equalizer from '$ui/Equalizer.svelte';
  import { useIsland } from '../context.svelte';

  const { backend } = useIsland();
  const session = $derived(backend.media.current);
</script>

<Art src={session?.artUrl ?? null} class="art--island" />
<span class="island-track">
  <span class="island-track__title">{session?.title ?? ''}</span>
  <span class="island-track__artist">{session?.artist ?? ''}</span>
</span>
<Equalizer playing={session?.playing ?? false} />
