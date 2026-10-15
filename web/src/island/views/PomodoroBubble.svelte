<script lang="ts">
  import { useIsland } from '../context.svelte';

  const { pomodoro } = useIsland();
  const minutes = $derived(Math.ceil(pomodoro.remaining / 60000));
</script>

<span class="bubble-text num pomo-bubble" class:is-break={pomodoro.onBreak}>{minutes}′</span>
