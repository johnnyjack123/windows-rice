<script lang="ts">
  import { ICON_SET } from '$design/icons';

  interface Props {
    /** Material Symbols ligature from design/icons.json. */
    name: string;
    /** Extra classes, e.g. "ic--sm spin". */
    class?: string;
  }

  let { name, class: cls = '' }: Props = $props();

  $effect(() => {
    if (import.meta.env.DEV && !ICON_SET.has(name)) {
      console.warn(`[icons] "${name}" is not in design/icons.json and will not render`);
    }
  });
</script>

<span class="ic {cls}" aria-hidden="true">{name}</span>
