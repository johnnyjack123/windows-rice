<script lang="ts">
  interface Props {
    /** "#rrggbb" */
    value: string;
    label: string;
    oninput: (value: string) => void;
  }

  let { value, label, oninput }: Props = $props();
</script>

<label class="color-input">
  <span class="color-input__swatch" style:--swatch={value}></span>
  <span class="num">{value.toUpperCase()}</span>
  <input type="color" {value} aria-label={label} oninput={(e) => oninput(e.currentTarget.value)} />
</label>
