<script lang="ts" generics="T extends string">
  /** One option. */
  interface Option {
    value: T;
    label: string;
    disabled?: boolean;
  }

  interface Props {
    options: Option[];
    value: T;
    label: string;
    block?: boolean;
    onchange: (value: T) => void;
  }

  let { options, value, label, block = false, onchange }: Props = $props();
</script>

<div class="seg" class:seg--block={block} role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <button
      class="seg__btn"
      class:is-active={option.value === value}
      role="radio"
      aria-checked={option.value === value}
      disabled={option.disabled}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>
