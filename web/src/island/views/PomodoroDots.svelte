<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';

  interface Props {
    /** Focus sessions completed in this cycle. */
    round: number;
    rounds: number;
    /** A focus session is under way (its dot shows as current). */
    focus: boolean;
  }

  let { round, rounds, focus }: Props = $props();
  const current = $derived(Math.min(rounds, focus ? round + 1 : round));
</script>

<span class="pomo-dots" role="img" aria-label={i18n.m.pomodoro.session(Math.max(1, current), rounds)}>
  {#each { length: rounds } as _, i (i)}
    <i class:is-done={i < round} class:is-current={focus && i === round}></i>
  {/each}
</span>
