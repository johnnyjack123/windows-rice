<script lang="ts">
  import { onMount } from 'svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import { parseSpring, SpringSystem, SpringValue } from '$motion/spring';
  import Equalizer from '$ui/Equalizer.svelte';
  import Icon from '$ui/Icon.svelte';

  let el: HTMLDivElement;
  let wide = $state(false);
  const system = new SpringSystem(() => {
    el.style.width = `${w.value}px`;
    el.style.height = `${h.value}px`;
  });
  const w = system.add(new SpringValue(140, [0.5, 0.25]));
  const h = system.add(new SpringValue(34, [0.5, 0.25]));

  function size(expanded: boolean): { width: number; height: number } {
    const cs = getComputedStyle(el);
    const height = parseFloat(cs.getPropertyValue('--island-height')) || 34;
    return expanded ? { width: 260, height: height * 2.3 } : { width: 140, height };
  }

  /** Morphs between compact and live size with the theme's current spring. */
  function play(): void {
    const spring = parseSpring(getComputedStyle(document.documentElement).getPropertyValue('--spring-island'));
    w.setSpring(spring);
    h.setSpring(spring);
    wide = !wide;
    const target = size(wide);
    w.to(target.width);
    h.to(target.height);
    system.kick();
  }

  onMount(() => {
    const start = size(false);
    w.jump(start.width);
    h.jump(start.height);
    el.style.width = `${start.width}px`;
    el.style.height = `${start.height}px`;
    return () => system.stop();
  });
</script>

<div class="preview-island" bind:this={el}>
  <span class="preview-island__content">
    <span class="art art--island" style:background="var(--media-color)"></span>
    <span class="num">14:32</span>
    <Equalizer playing />
  </span>
</div>
<button class="btn btn--tonal has-state" onclick={play}><Icon name="play_arrow" />{i18n.m.settings.appearance.playMotion}</button>
