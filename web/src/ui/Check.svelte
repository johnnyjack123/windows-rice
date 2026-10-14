<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    onchange?: (checked: boolean) => void;
  }

  let { checked, label, onchange }: Props = $props();
</script>

<button class="check" role="checkbox" aria-checked={checked} aria-label={label} onclick={() => onchange?.(!checked)}>
  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.2 4.2L19 7.5" /></svg>
</button>
