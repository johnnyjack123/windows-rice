<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    id: string;
    label: string;
    icon: string;
    on: boolean;
    unavailable?: boolean;
    /** Static state for the gallery. */
    state?: '' | 'hover' | 'pressed';
    ontoggle?: () => void;
    /** Right-click (or the context-menu key) opens the matching system settings. */
    onsettings?: () => void;
  }

  let { id, label, icon, on, unavailable = false, state = '', ontoggle, onsettings }: Props = $props();

  function contextmenu(e: MouseEvent): void {
    if (onsettings) {
      e.preventDefault();
      onsettings();
    }
  }
</script>

<button
  class="tile tile--small has-state"
  class:is-on={on}
  class:is-hover={state === 'hover'}
  class:is-pressed={state === 'pressed'}
  data-tile={id}
  aria-pressed={on}
  aria-label={label}
  title={label}
  disabled={unavailable}
  onclick={() => ontoggle?.()}
  oncontextmenu={contextmenu}
>
  <Icon name={icon} />
</button>
