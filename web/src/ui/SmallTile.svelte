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
  }

  let { id, label, icon, on, unavailable = false, state = '', ontoggle }: Props = $props();
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
>
  <Icon name={icon} />
</button>
