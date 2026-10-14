<script lang="ts">
  import Icon from './Icon.svelte';

  /** One tab. */
  interface TabItem {
    id: string;
    label: string;
    icon: string;
    badge?: number;
  }

  interface Props {
    items: TabItem[];
    active: string;
    label: string;
    onselect: (id: string) => void;
  }

  let { items, active, label, onselect }: Props = $props();
  let el: HTMLDivElement;

  function key(e: KeyboardEvent): void {
    if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') {
      return;
    }
    e.preventDefault();
    const index = items.findIndex((t) => t.id === active);
    const next = items[(index + (e.key === 'ArrowRight' ? 1 : items.length - 1)) % items.length];
    if (next) {
      onselect(next.id);
      queueMicrotask(() => el.querySelector<HTMLElement>(`[data-tab="${next.id}"]`)?.focus());
    }
  }
</script>

<div class="tabs" role="tablist" aria-label={label} tabindex="-1" bind:this={el} onkeydown={key}>
  {#each items as tab (tab.id)}
    <button
      class="tab has-state"
      class:is-active={tab.id === active}
      role="tab"
      data-tab={tab.id}
      aria-selected={tab.id === active}
      tabindex={tab.id === active ? 0 : -1}
      title={tab.label}
      onclick={() => onselect(tab.id)}
    >
      <Icon name={tab.icon} />
      <span class="tab__label">{tab.label}</span>
      {#if tab.badge}<span class="tab__badge num">{tab.badge}</span>{/if}
    </button>
  {/each}
</div>
