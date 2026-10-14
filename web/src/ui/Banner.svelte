<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    tone?: 'info' | 'warning' | 'error';
    icon: string;
    title: string;
    body?: string;
    /** Action buttons. */
    children?: Snippet;
  }

  let { tone = 'info', icon, title, body, children }: Props = $props();
</script>

<div class="banner" class:banner--warning={tone === 'warning'} class:banner--error={tone === 'error'} role={tone === 'error' ? 'alert' : 'status'}>
  <Icon name={icon} />
  <div class="banner__title">{title}</div>
  {#if body}<div class="banner__body">{body}</div>{/if}
  {#if children}<div class="banner__actions">{@render children()}</div>{/if}
</div>
