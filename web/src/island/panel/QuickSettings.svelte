<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Slider from '$ui/Slider.svelte';
  import SmallTile from '$ui/SmallTile.svelte';
  import Tile from '$ui/Tile.svelte';
  import { useIsland } from '../context.svelte';
  import MediaCard from './MediaCard.svelte';
  import type { TileDef } from './tiles';

  let { tiles }: { tiles: Record<string, TileDef> } = $props();
  const { backend, panel } = useIsland();

  const config = $derived(backend.settings.value.panel);
  const large = $derived(config.tilesLarge.map((id) => tiles[id]).filter((t): t is TileDef => Boolean(t)));
  const small = $derived(config.tilesSmall.map((id) => tiles[id]).filter((t): t is TileDef => Boolean(t)));
  const volume = $derived(backend.audio.state.muted ? 0 : backend.audio.state.volume);
  const brightness = $derived(backend.display.state.brightness);
</script>

{#if large.length}
  <div class="tiles" data-stagger style:--i={1}>
    {#each large as tile (tile.id)}
      <Tile
        id={tile.id}
        label={tile.label()}
        sub={tile.sub()}
        icon={tile.icon()}
        on={tile.on()}
        unavailable={!tile.available()}
        detail={Boolean(tile.page)}
        ontoggle={tile.toggle}
        onopen={() => (panel.page = tile.page ?? 'home')}
      />
    {/each}
  </div>
{/if}

{#if small.length}
  <div class="tiles tiles--small" data-stagger style:--i={2}>
    {#each small as tile (tile.id)}
      <SmallTile id={tile.id} label={tile.label()} icon={tile.icon()} on={tile.on()} unavailable={!tile.available()} ontoggle={tile.toggle} />
    {/each}
  </div>
{/if}

<div class="sliders" data-stagger style:--i={3}>
  <Slider
    value={volume}
    icon={volume === 0 ? 'volume_off' : volume < 0.5 ? 'volume_down' : 'volume_up'}
    label={i18n.m.sliders.volume}
    onchange={(v) => void backend.audio.setVolume(v)}
  />
  {#if brightness !== null}
    <Slider
      value={brightness}
      icon={brightness < 0.34 ? 'brightness_low' : brightness < 0.67 ? 'brightness_medium' : 'brightness_high'}
      label={i18n.m.sliders.brightness}
      onchange={(v) => void backend.display.setBrightness(v)}
    />
  {/if}
</div>

<MediaCard />
