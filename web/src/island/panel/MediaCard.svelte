<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import Art from '$ui/Art.svelte';
  import Icon from '$ui/Icon.svelte';
  import Progress from '$ui/Progress.svelte';
  import { useIsland } from '../context.svelte';

  const { backend } = useIsland();
  const media = backend.media;
  const session = $derived(media.current);
  const device = $derived(backend.audio.state.device);
  const m = $derived(i18n.m.media);
</script>

{#if session}
  <section class="media" data-stagger style:--i={4} aria-label={m.region}>
    <Art src={session.artUrl} class="media__backdrop" />
    <div class="media__row">
      <Art src={session.artUrl} class="media__art" />
      <div class="truncate">
        <div class="media__title truncate">{session.title}</div>
        <div class="media__artist truncate">{session.artist}</div>
      </div>
      <div class="media__controls">
        <button class="icon-btn has-state" aria-label={m.previous} disabled={!session.canPrevious} onclick={() => void media.control('previous')}>
          <Icon name="skip_previous" />
        </button>
        <button class="icon-btn has-state media__play" aria-label={m.playPause} onclick={() => void media.control('toggle')}>
          <Icon name={session.playing ? 'pause' : 'play_arrow'} />
        </button>
        <button class="icon-btn has-state" aria-label={m.next} disabled={!session.canNext} onclick={() => void media.control('next')}>
          <Icon name="skip_next" />
        </button>
      </div>
    </div>
    <Progress
      position={media.position(clock.now)}
      duration={session.duration}
      label={m.seek}
      onseek={session.canSeek ? (p) => void media.seek(p) : undefined}
    />
    <div class="media__foot">
      <Icon name={device?.kind === 'bluetooth' || device?.kind === 'headphones' ? 'headphones' : 'speaker'} />
      <span class="truncate">{device?.name ?? ''}</span>
      <span aria-hidden="true">·</span>
      <span class="truncate">{session.appName}</span>
    </div>
  </section>
{:else}
  <section class="media is-empty" data-stagger style:--i={4}>
    <Icon name="music_note" class="ic--lg" />
    <div class="media__title">{m.nothingPlaying}</div>
    <div class="media__artist">{m.nothingPlayingHint}</div>
  </section>
{/if}
