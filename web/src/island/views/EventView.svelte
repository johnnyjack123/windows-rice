<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import { fmt } from '$i18n/i18n.svelte';
  import { useIsland } from '../context.svelte';

  const { ui, backend } = useIsland();
  const event = $derived(ui.event);
  const minutes = $derived(event ? (Date.parse(event.start) - clock.now) / 60000 : 0);
  // The ring fills up during the reminder lead time.
  const lead = $derived(backend.settings.value.calendar.reminderLead || 10);
</script>

{#if event}
  <span class="ev-ring" style:--c={backend.calendar.colorOf(event.calendarId)} style:--p={Math.min(1, Math.max(0.04, 1 - minutes / lead))}></span>
  <span class="ev-title">{event.title}</span>
  <span class="ev-when">{fmt.until(new Date(event.start), new Date(clock.now))}</span>
{/if}
