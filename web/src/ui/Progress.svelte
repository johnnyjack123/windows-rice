<script lang="ts">
  import { fmt } from '$i18n/i18n.svelte';

  interface Props {
    /** Seconds played. */
    position: number;
    /** Total seconds. */
    duration: number;
    label: string;
    /** Disable the 1 s smoothing (after seeks or track changes). */
    jump?: boolean;
    onseek?: (position: number) => void;
  }

  let { position, duration, label, jump = false, onseek }: Props = $props();
  let track: HTMLDivElement;

  const fraction = $derived(duration > 0 ? Math.min(1, position / duration) : 0);

  function seek(e: MouseEvent): void {
    if (!onseek || duration <= 0) {
      return;
    }
    const rect = track.getBoundingClientRect();
    onseek(Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width)) * duration);
  }

  function key(e: KeyboardEvent): void {
    if (!onseek) {
      return;
    }
    if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
      e.preventDefault();
      onseek(Math.min(duration, Math.max(0, position + (e.key === 'ArrowRight' ? 5 : -5))));
    }
  }
</script>

<div class="progress" class:is-jump={jump} style:--p={fraction}>
  <span class="num">{fmt.duration(position)}</span>
  <div
    bind:this={track}
    class="progress__track"
    role="slider"
    tabindex={onseek ? 0 : -1}
    aria-label={label}
    aria-valuemin={0}
    aria-valuemax={Math.round(duration)}
    aria-valuenow={Math.round(position)}
    aria-valuetext={fmt.duration(position)}
    onclick={seek}
    onkeydown={key}
  >
    <div class="progress__fill"></div>
  </div>
  <span class="num">-{fmt.duration(duration - position)}</span>
</div>
