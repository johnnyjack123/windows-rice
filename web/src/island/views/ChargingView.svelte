<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Battery from '$ui/Battery.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { backend } = useIsland();
  const level = $derived(backend.power.state.battery ?? 0);
</script>

<span class="ch-label"><Icon name="bolt" /> {i18n.m.island.charging}</span>
<span class="ch-right">
  <span class="num">{i18n.m.format.percent(level * 100)}</span>
  <Battery {level} charging />
</span>
