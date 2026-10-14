<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    /** Static focus state for the gallery. */
    focus?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let { checked, label, disabled = false, focus = false, onchange }: Props = $props();
</script>

<button
  class="switch"
  class:is-focus={focus}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange?.(!checked)}
></button>
