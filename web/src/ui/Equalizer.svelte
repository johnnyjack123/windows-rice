<script lang="ts">
  interface Props {
    playing: boolean;
    class?: string;
  }

  let { playing, class: cls = '' }: Props = $props();
</script>

<span class="eq {cls}" data-playing={playing} aria-hidden="true"><i></i><i></i><i></i><i></i><i></i></span>
