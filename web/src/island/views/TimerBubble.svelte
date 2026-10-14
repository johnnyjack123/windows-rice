<script lang="ts">
  import { useIsland } from '../context.svelte';

  const { timers } = useIsland();
  const soonest = $derived(timers.active[0]);
  const minutes = $derived(soonest ? Math.ceil(timers.remainingOf(soonest) / 60000) : 0);
</script>

<span class="bubble-text num">{minutes}′</span>
