<script lang="ts">
  interface Props {
    /** Cover URL; null shows the neutral placeholder surface. */
    src: string | null;
    class?: string;
  }

  let { src, class: cls = '' }: Props = $props();

  let el: HTMLSpanElement;
  let previous: string | null = null;

  // Replays the swap animation whenever the cover changes.
  $effect(() => {
    const current = src;
    if (previous !== null && current !== previous) {
      el.classList.remove('is-swapping');
      void el.offsetWidth;
      el.classList.add('is-swapping');
    }
    previous = current;
  });
</script>

<span bind:this={el} class="art {cls}" style:--art={src ? `url("${src}")` : 'none'} aria-hidden="true"></span>
