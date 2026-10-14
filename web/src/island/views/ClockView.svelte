<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import { fmt } from '$i18n/i18n.svelte';
  import { useIsland } from '../context.svelte';

  const { backend } = useIsland();
  const now = $derived(new Date(clock.now));
</script>

<span class="island-clock num">
  {fmt.time(now)}<span class="island-clock__sec">:{String(now.getSeconds()).padStart(2, '0')}</span>
</span>
{#if backend.notifications.items.length}<span class="island-unread"></span>{/if}
