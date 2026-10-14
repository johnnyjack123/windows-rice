<script lang="ts">
  import { useEngine } from '$design/engine.svelte';
  import { sparkPath, type SparkStyle } from './spark';

  interface Props {
    values: number[];
    max: number;
  }

  let { values, max }: Props = $props();
  const engine = useEngine();
  let el: SVGSVGElement;
  let style = $state<SparkStyle>('smooth');

  // The curve style is a theme token; re-read it whenever tokens change.
  $effect(() => {
    void engine?.version;
    const value = getComputedStyle(el).getPropertyValue('--spark-style').trim();
    style = value === 'step' || value === 'linear' ? value : 'smooth';
  });

  const paths = $derived(sparkPath(values, max, style));
</script>

<svg bind:this={el} class="spark" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true">
  <path class="area" d={paths.area} />
  <path class="line" d={paths.line} />
</svg>
