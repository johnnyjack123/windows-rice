<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import { useIsland } from '../context.svelte';
  import Mascot from '../mascot/Mascot.svelte';

  const { ui } = useIsland();
  const a = $derived(i18n.m.agents);
  const status = $derived(ui.agentStatus);
</script>

{#if status}
  <!-- A new report replays the mascot's reaction. -->
  {#key status.time}
    <Mascot mood={status.state === 'done' ? 'done' : 'waiting'} class="mascot--compact" />
  {/key}
  <span class="agent-status">
    <span class="agent-status__title">{status.state === 'done' ? a.done(a.names[status.agent]) : a.waiting(a.names[status.agent])}</span>
    <span class="agent-status__project">{status.project}</span>
  </span>
{/if}
