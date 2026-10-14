<script lang="ts">
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../../../context.svelte';
  import DurationPicker from './DurationPicker.svelte';

  const { timers, backend } = useIsland();
  const t = $derived(i18n.m.timers);
  const presets = $derived(backend.settings.value.timers.presets);

  let seconds = $state(5 * 60);
  let label = $state('');
  let composing = $state(false);
  const showComposer = $derived(composing || timers.active.length === 0);

  function start(): void {
    if (seconds <= 0) {
      return;
    }
    timers.start(seconds * 1000, label.trim());
    label = '';
    composing = false;
  }
</script>

{#if timers.active.length}
  <div class="timer-list">
    {#each timers.active as timer (timer.id)}
      {@const remaining = timers.remainingOf(timer)}
      <div class="timer-card" class:is-paused={timer.state === 'paused'}>
        <span class="timer-ring" style:--p={timer.duration ? remaining / timer.duration : 0}>
          <Icon name="hourglass_top" class="ic--sm" />
        </span>
        <span class="stack">
          <span class="timer-card__time num">{fmt.duration(Math.ceil(remaining / 1000))}</span>
          <span class="timer-card__label">{timer.label || t.defaultName} · {fmt.duration(timer.duration / 1000)}</span>
        </span>
        <span class="timer-card__actions">
          <button class="icon-btn has-state" aria-label={t.addMinute} title={t.addMinute} onclick={() => timers.addMinute(timer.id)}>
            <Icon name="more_time" />
          </button>
          {#if timer.state === 'running'}
            <button class="icon-btn has-state" aria-label={t.pause} title={t.pause} onclick={() => timers.pause(timer.id)}><Icon name="pause" /></button>
          {:else}
            <button class="icon-btn has-state" aria-label={t.resume} title={t.resume} onclick={() => timers.resume(timer.id)}><Icon name="play_arrow" /></button>
          {/if}
          <button class="icon-btn has-state" aria-label={t.cancel} title={t.cancel} onclick={() => timers.cancel(timer.id)}><Icon name="close" /></button>
        </span>
      </div>
    {/each}
  </div>
{/if}

{#if showComposer}
  <div class="composer">
    <DurationPicker value={seconds} onchange={(v) => (seconds = v)} />
    <div class="preset-row" role="group" aria-label={t.presets}>
      {#each presets as minutes (minutes)}
        <button class="chip has-state" class:is-selected={seconds === minutes * 60} onclick={() => (seconds = minutes * 60)}>{t.minutesShort(minutes)}</button>
      {/each}
    </div>
    <label class="field">
      <Icon name="edit" class="ic--sm" />
      <input bind:value={label} placeholder={t.name} aria-label={t.name} onkeydown={(e) => e.key === 'Enter' && start()} />
    </label>
    <button class="btn btn--filled btn--block has-state" disabled={seconds <= 0} onclick={start}><Icon name="play_arrow" />{t.start}</button>
  </div>
{:else}
  <div class="composer">
    <button class="btn btn--tonal btn--block has-state" onclick={() => (composing = true)}><Icon name="add" />{t.newTimer}</button>
  </div>
{/if}
