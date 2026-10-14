<script lang="ts">
  interface Props {
    /** App or process name; its first letter is the fallback glyph. */
    name: string;
    /** Icon image (data URL / asset URL) if the backend has one. */
    src?: string | null;
    class?: string;
  }

  let { name, src = null, class: cls = '' }: Props = $props();

  // Stable hue per name, normalised by the theme like other data colours.
  const color = $derived.by(() => {
    let hash = 0;
    for (const ch of name) {
      hash = (hash * 31 + ch.charCodeAt(0)) | 0;
    }
    return `hsl(${Math.abs(hash) % 360} 60% 50%)`;
  });
</script>

<span class="app-icon {cls}" class:has-image={Boolean(src)} style:--app-color={color} aria-hidden="true">
  {#if src}
    <img {src} alt="" draggable="false" />
  {:else}
    {name.trim().charAt(0).toUpperCase()}
  {/if}
</span>
