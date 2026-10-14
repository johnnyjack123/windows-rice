<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';

  interface Props {
    /** Total seconds. */
    value: number;
    onchange: (seconds: number) => void;
  }

  let { value, onchange }: Props = $props();
  const t = $derived(i18n.m.timers);

  const units = $derived([
    { key: 'h', label: t.hours, size: 3600, max: 23, part: Math.floor(value / 3600) },
    { key: 'm', label: t.minutes, size: 60, max: 59, part: Math.floor((value % 3600) / 60) },
    { key: 's', label: t.seconds, size: 1, max: 59, part: value % 60 },
  ]);

  function step(size: number, max: number, part: number, delta: number): void {
    const next = (part + delta + max + 1) % (max + 1);
    onchange(Math.max(0, value + (next - part) * size));
  }

  function key(e: KeyboardEvent, size: number, max: number, part: number): void {
    const map: Record<string, number> = { ArrowUp: 1, ArrowDown: -1, PageUp: 5, PageDown: -5 };
    if (e.key in map) {
      e.preventDefault();
      step(size, max, part, map[e.key]!);
    }
  }

  /** Non-passive wheel handler so the pane does not scroll while adjusting. */
  function wheel(node: HTMLElement, unit: { size: number; max: number }) {
    const handler = (e: WheelEvent): void => {
      e.preventDefault();
      const part = Math.floor((value % (unit.size * (unit.max + 1))) / unit.size);
      step(unit.size, unit.max, part, e.deltaY < 0 ? 1 : -1);
    };
    node.addEventListener('wheel', handler, { passive: false });
    return { destroy: () => node.removeEventListener('wheel', handler) };
  }

  // Vertical drag adjusts the value like a physical wheel.
  let drag: { y: number; part: number; size: number; max: number } | null = null;

  function down(e: PointerEvent, size: number, max: number, part: number): void {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { y: e.clientY, part, size, max };
  }

  function move(e: PointerEvent): void {
    if (!drag) {
      return;
    }
    const delta = Math.round((drag.y - e.clientY) / 14);
    const next = Math.min(drag.max, Math.max(0, drag.part + delta));
    const current = Math.floor((value % (drag.size * (drag.max + 1))) / drag.size);
    if (next !== current) {
      onchange(Math.max(0, value + (next - current) * drag.size));
    }
  }
</script>

<div class="duration-picker">
  {#each units as unit, i (unit.key)}
    {#if i > 0}<span class="duration-sep" aria-hidden="true">:</span>{/if}
    <div class="duration-unit">
      <div
        class="duration-unit__value num has-state"
        role="spinbutton"
        tabindex="0"
        aria-label={unit.label}
        aria-valuemin={0}
        aria-valuemax={unit.max}
        aria-valuenow={unit.part}
        use:wheel={{ size: unit.size, max: unit.max }}
        onkeydown={(e) => key(e, unit.size, unit.max, unit.part)}
        onpointerdown={(e) => down(e, unit.size, unit.max, unit.part)}
        onpointermove={move}
        onpointerup={() => (drag = null)}
        onpointercancel={() => (drag = null)}
      >
        {String(unit.part).padStart(2, '0')}
      </div>
      <span class="duration-unit__caption">{unit.label}</span>
    </div>
  {/each}
</div>
