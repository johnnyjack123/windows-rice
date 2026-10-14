<script lang="ts">
  import Art from '$ui/Art.svelte';
  import { useIsland } from '../context.svelte';

  const { backend } = useIsland();
</script>

<Art src={backend.media.current?.artUrl ?? null} class="art--bubble" />
