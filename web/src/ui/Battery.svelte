<script lang="ts">
  interface Props {
    /** 0…1 */
    level: number;
    charging?: boolean;
  }

  let { level, charging = false }: Props = $props();
</script>

<span
  class="battery"
  class:is-charging={charging}
  class:is-low={!charging && level < 0.15}
  style:--level={Math.max(0, Math.min(1, level))}
  aria-hidden="true"
><i></i></span>
