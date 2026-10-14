<script lang="ts">
  import { fmt } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { timers } = useIsland();
</script>

<Icon name="timer" />
<span class="timer-time num">{fmt.stopwatch(timers.stopwatchElapsed).slice(0, -3)}</span>
