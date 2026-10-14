<script lang="ts">
  interface Props {
    value: number;
    min: number;
    max: number;
    step: number;
    label: string;
    /** Human readable value for screen readers and the value label. */
    text?: string;
    oninput: (value: number) => void;
  }

  let { value, min, max, step, label, text, oninput }: Props = $props();

  const fill = $derived(`${((value - min) / (max - min)) * 100}%`);
</script>

<input
  class="range"
  type="range"
  {min}
  {max}
  {step}
  {value}
  aria-label={label}
  aria-valuetext={text}
  style:--fill={fill}
  oninput={(e) => oninput(Number(e.currentTarget.value))}
/>
