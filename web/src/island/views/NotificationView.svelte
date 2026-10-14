<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import AppIcon from '$ui/AppIcon.svelte';
  import { useIsland } from '../context.svelte';

  const { ui } = useIsland();
  const n = $derived(ui.notification);
</script>

{#if n}
  <div class="nv">
    <AppIcon name={n.appName} src={n.iconUrl} />
    <div class="truncate">
      <div class="nv-meta"><span class="truncate">{n.appName}</span><span>{i18n.m.common.justNow}</span></div>
      <div class="nv-title truncate">{n.title}</div>
      <div class="nv-body truncate">{n.body}</div>
    </div>
  </div>
{/if}
