<script lang="ts">
  import { BUNDLED_FONTS, PILL, type ControlDef, type EditorValues } from '$design/editor';
  import { i18n } from '$i18n/i18n.svelte';
  import ColorInput from '$ui/ColorInput.svelte';
  import Range from '$ui/Range.svelte';
  import Switch from '$ui/Switch.svelte';
  import Setting from './Setting.svelte';

  interface Props {
    def: ControlDef;
    value: EditorValues[keyof EditorValues];
    fallback: EditorValues[keyof EditorValues];
    modified: boolean;
    /** Installed font families for font pickers. */
    systemFonts: string[];
    /** Label override (e.g. glass tint instead of background). */
    label?: string;
    onchange: (value: EditorValues[keyof EditorValues]) => void;
    onreset: () => void;
  }

  let { def, value, fallback, modified, systemFonts, label, onchange, onreset }: Props = $props();
  const e = $derived(i18n.m.settings.editor);
  const text = $derived(label ?? (e[def.key as keyof typeof e] as string));

  function display(v: number): string {
    if ((def.key === 'islandRadius' || def.key === 'tileRadius') && v >= PILL) {
      return e.pill;
    }
    return def.format ? def.format(v) : String(v);
  }

  // Radius sliders show "pill" at their maximum.
  const numeric = $derived.by(() => {
    const v = Number(value);
    return v >= PILL ? def.max ?? v : v;
  });

  function rangeInput(v: number): void {
    const isPill = (def.key === 'islandRadius' || def.key === 'tileRadius') && v >= (def.max ?? Infinity);
    onchange(isPill ? PILL : v);
  }

  const fallbackText = $derived(
    typeof fallback === 'number' ? display(fallback) : typeof fallback === 'boolean' ? (fallback ? i18n.m.common.on : i18n.m.common.off) : String(fallback),
  );
</script>

<Setting label={text} hint={modified ? e.themeValue(def.key === 'iconStyle' ? e.iconStyles[fallback as 'Rounded'] : fallbackText) : undefined} {modified} range={def.kind === 'range'} {onreset}>
  {#if def.kind === 'color'}
    <ColorInput value={String(value)} label={text} oninput={(v) => onchange(v)} />
  {:else if def.kind === 'range'}
    <Range value={numeric} min={def.min ?? 0} max={def.max ?? 1} step={def.step ?? 0.01} label={text} text={display(Number(value))} oninput={rangeInput} />
    <span class="setting__value num">{display(Number(value))}</span>
  {:else if def.kind === 'toggle'}
    <Switch checked={Boolean(value)} label={text} onchange={(v) => onchange(v)} />
  {:else if def.kind === 'font'}
    <select class="select" aria-label={text} value={String(value)} onchange={(ev) => onchange(ev.currentTarget.value)}>
      <optgroup label={e.bundledFonts}>
        {#each BUNDLED_FONTS as font (font.id)}<option value={font.id}>{font.id}</option>{/each}
      </optgroup>
      {#if systemFonts.length}
        <optgroup label={e.systemFonts}>
          {#each systemFonts.filter((f) => !BUNDLED_FONTS.some((b) => b.id === f)) as font (font)}<option value={font}>{font}</option>{/each}
        </optgroup>
      {/if}
      {#if !BUNDLED_FONTS.some((b) => b.id === value) && !systemFonts.includes(String(value))}
        <option value={String(value)}>{String(value)}</option>
      {/if}
    </select>
  {:else if def.kind === 'select'}
    <select class="select" aria-label={text} value={String(value)} onchange={(ev) => onchange(ev.currentTarget.value)}>
      {#each def.options ?? [] as option (option)}
        <option value={option}>{def.key === 'iconStyle' ? e.iconStyles[option as 'Rounded'] : option}</option>
      {/each}
    </select>
  {/if}
</Setting>
