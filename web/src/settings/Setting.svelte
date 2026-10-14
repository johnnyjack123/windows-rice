<script lang="ts">
  import type { Snippet } from 'svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';

  interface Props {
    label: string;
    hint?: string;
    /** Differs from the default; shows a dot and the reset button. */
    modified?: boolean;
    /** Wide control column for sliders. */
    range?: boolean;
    onreset?: () => void;
    children: Snippet;
  }

  let { label, hint, modified = false, range = false, onreset, children }: Props = $props();
</script>

<div class="setting" class:setting--range={range} class:is-modified={modified}>
  <div>
    <div class="setting__label">{label}</div>
    {#if hint}<div class="setting__hint">{hint}</div>{/if}
  </div>
  <div class="setting__control">
    {@render children()}
    {#if onreset}
      <button class="icon-btn has-state setting__reset" aria-label={i18n.m.settings.editor.resetControl(label)} title={i18n.m.settings.editor.resetControl(label)} onclick={onreset}>
        <Icon name="restart_alt" />
      </button>
    {/if}
  </div>
</div>
