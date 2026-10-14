<script lang="ts">
  import { useIsland } from '../context.svelte';

  const { backend, ui } = useIsland();
</script>

{#if ui.event}
  <span class="ev-ring" style:--c={backend.calendar.colorOf(ui.event.calendarId)} style:--p={0.7}></span>
{/if}
