<script lang="ts">
  import { useIsland } from '../context.svelte';

  const { timers } = useIsland();
</script>

<span class="bubble-text num">{Math.floor(timers.stopwatchElapsed / 60000)}′</span>
