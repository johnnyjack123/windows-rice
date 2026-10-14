<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    icon: string;
    title: string;
    body?: string;
    children?: Snippet;
  }

  let { icon, title, body, children }: Props = $props();
</script>

<div class="empty">
  <Icon name={icon} />
  <div class="empty__title">{title}</div>
  {#if body}<div>{body}</div>{/if}
  {@render children?.()}
</div>
