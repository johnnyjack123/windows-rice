<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { ui } = useIsland();
  const value = $derived(ui.osd.muted ? 0 : ui.osd.value);

  const icon = $derived.by(() => {
    if (ui.osd.kind === 'brightness') {
      return value < 0.34 ? 'brightness_low' : value < 0.67 ? 'brightness_medium' : 'brightness_high';
    }
    if (value === 0) {
      return 'volume_off';
    }
    return value < 0.34 ? 'volume_mute' : value < 0.67 ? 'volume_down' : 'volume_up';
  });
</script>

<span class="osd-icon"><Icon name={icon} /></span>
<div
  class="osd-bar"
  role="meter"
  aria-label={ui.osd.kind === 'volume' ? i18n.m.osd.volume : i18n.m.osd.brightness}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={Math.round(value * 100)}
>
  <div class="osd__fill" style:--v={value}></div>
</div>
<span class="osd-value num">{Math.round(value * 100)}</span>
