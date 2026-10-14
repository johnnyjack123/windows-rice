<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import Art from '$ui/Art.svelte';
  import Equalizer from '$ui/Equalizer.svelte';
  import Icon from '$ui/Icon.svelte';
  import Progress from '$ui/Progress.svelte';
  import { useIsland } from '../context.svelte';

  const { backend } = useIsland();
  const media = backend.media;
  const session = $derived(media.current);
  const m = $derived(i18n.m.media);
</script>

<div class="live-head">
  <Art src={session?.artUrl ?? null} class="live-art" />
  <div class="truncate">
    <div class="live-title truncate">{session?.title ?? ''}</div>
    <div class="live-sub truncate">{session?.artist ?? ''}</div>
  </div>
  <Equalizer playing={session?.playing ?? false} class="eq--lg" />
</div>
{#if session}
  <Progress
    position={media.position(clock.now)}
    duration={session.duration}
    label={m.seek}
    onseek={session.canSeek ? (p) => void media.seek(p) : undefined}
  />
{/if}
<div class="live-controls">
  <button class="icon-btn has-state" aria-label={m.previous} disabled={!session?.canPrevious} onclick={() => void media.control('previous')}>
    <Icon name="skip_previous" />
  </button>
  <button class="icon-btn has-state is-primary" aria-label={m.playPause} onclick={() => void media.control('toggle')}>
    <Icon name={session?.playing ? 'pause' : 'play_arrow'} />
  </button>
  <button class="icon-btn has-state" aria-label={m.next} disabled={!session?.canNext} onclick={() => void media.control('next')}>
    <Icon name="skip_next" />
  </button>
</div>
