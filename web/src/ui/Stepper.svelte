<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    /** Accessible name, e.g. "Focus". */
    label: string;
    /** Visible value, e.g. "25 min". */
    text: string;
    onchange: (value: number) => void;
  }

  let { value, min, max, step = 1, label, text, onchange }: Props = $props();

  function set(next: number): void {
    const v = Math.min(max, Math.max(min, Math.round(next / step) * step));
    if (v !== value) {
      onchange(v);
    }
  }

  function key(e: KeyboardEvent): void {
    const moves: Record<string, number> = {
      ArrowUp: value + step,
      ArrowRight: value + step,
      ArrowDown: value - step,
      ArrowLeft: value - step,
      PageUp: value + step * 5,
      PageDown: value - step * 5,
      Home: min,
      End: max,
    };
    const next = moves[e.key];
    if (next !== undefined) {
      e.preventDefault();
      set(next);
    }
  }
</script>

<div class="stepper">
  <button class="icon-btn has-state" aria-label={i18n.m.timers.decrease(label)} disabled={value <= min} onclick={() => set(value - step)}>
    <Icon name="remove" />
  </button>
  <span
    class="stepper__value num"
    role="spinbutton"
    tabindex="0"
    aria-label={label}
    aria-valuenow={value}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuetext={text}
    onkeydown={key}
  >{text}</span>
  <button class="icon-btn has-state" aria-label={i18n.m.timers.increase(label)} disabled={value >= max} onclick={() => set(value + step)}>
    <Icon name="add" />
  </button>
</div>
