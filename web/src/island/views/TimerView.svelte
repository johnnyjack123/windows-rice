<script lang="ts">
  import { fmt } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { timers } = useIsland();
  const soonest = $derived(timers.active[0]);
  const more = $derived(timers.active.length - 1);
</script>

<Icon name="hourglass_top" />
{#if soonest}
  <span class="timer-time num">{fmt.duration(Math.ceil(timers.remainingOf(soonest) / 1000))}</span>
{/if}
{#if more > 0}<span class="island-count num">+{more}</span>{/if}
