<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    /** 0…1 */
    value: number;
    icon: string;
    label: string;
    disabled?: boolean;
    onchange: (value: number) => void;
  }

  let { value, icon, label, disabled = false, onchange }: Props = $props();

  let el: HTMLDivElement;
  let dragging = $state(false);

  const clamp = (v: number): number => Math.min(1, Math.max(0, v));

  function valueAt(clientX: number): number {
    const rect = el.getBoundingClientRect();
    const h = rect.height;
    return clamp((clientX - rect.left - h / 2) / Math.max(1, rect.width - h));
  }

  function down(e: PointerEvent): void {
    if (disabled) {
      return;
    }
    el.setPointerCapture(e.pointerId);
    dragging = true;
    onchange(valueAt(e.clientX));
  }

  function move(e: PointerEvent): void {
    if (dragging && el.hasPointerCapture(e.pointerId)) {
      onchange(valueAt(e.clientX));
    }
  }

  function up(e: PointerEvent): void {
    if (el.hasPointerCapture(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
    dragging = false;
  }

  function key(e: KeyboardEvent): void {
    const steps: Record<string, number> = { ArrowRight: 0.05, ArrowUp: 0.05, ArrowLeft: -0.05, ArrowDown: -0.05, PageUp: 0.2, PageDown: -0.2 };
    if (disabled) {
      return;
    }
    if (e.key in steps) {
      e.preventDefault();
      onchange(clamp(value + steps[e.key]!));
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      onchange(e.key === 'Home' ? 0 : 1);
    }
  }

  // Wheel needs a non-passive listener to stop page scrolling.
  $effect(() => {
    const wheel = (e: WheelEvent): void => {
      if (disabled) {
        return;
      }
      e.preventDefault();
      onchange(clamp(value - Math.sign(e.deltaY) * 0.05));
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => el.removeEventListener('wheel', wheel);
  });
</script>

<div
  bind:this={el}
  class="slider"
  class:is-dragging={dragging}
  class:is-disabled={disabled}
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={Math.round(value * 100)}
  aria-disabled={disabled}
  style:--value={value}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
>
  <div class="slider__track"><div class="slider__fill"></div></div>
  <span class="slider__icon"><Icon name={icon} /></span>
  <span class="slider__handle"></span>
</div>
