<script lang="ts">
  import Icon from './Icon.svelte';
  import { i18n } from '$i18n/i18n.svelte';

  interface Props {
    /** Tile id, used for per-tile theme tints. */
    id: string;
    label: string;
    sub: string;
    icon: string;
    on: boolean;
    /** Has a drill-in page: the text zone opens it instead of toggling. */
    detail?: boolean;
    /** Feature not available on this machine. */
    unavailable?: boolean;
    /** Static state for the gallery. */
    state?: '' | 'hover' | 'pressed' | 'focus';
    ontoggle?: () => void;
    onopen?: () => void;
  }

  let {
    id, label, sub, icon, on, detail = false, unavailable = false, state = '', ontoggle, onopen,
  }: Props = $props();
</script>

<div class="tile" class:is-on={on} class:is-pressed={state === 'pressed'} class:is-unavailable={unavailable} data-tile={id}>
  <button
    class="tile__toggle has-state"
    aria-pressed={on}
    aria-label={i18n.m.common.toggle(label)}
    disabled={unavailable}
    onclick={() => ontoggle?.()}
  >
    <Icon name={icon} />
  </button>
  <button
    class="tile__main has-state"
    class:is-hover={state === 'hover'}
    class:is-focus={state === 'focus'}
    aria-label={detail ? i18n.m.common.details(label) : undefined}
    aria-pressed={detail ? undefined : on}
    disabled={unavailable}
    onclick={() => (detail ? onopen?.() : ontoggle?.())}
  >
    <span class="stack">
      <span class="tile__label">{label}</span>
      <span class="tile__sub">{sub}</span>
    </span>
    {#if detail}<Icon name="chevron_right" class="tile__chev ic--sm" />{/if}
  </button>
</div>
