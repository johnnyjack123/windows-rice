<script lang="ts">
  import type { AgentDecision } from '$backend/contract';
  import { i18n } from '$i18n/i18n.svelte';
  import { motionSettings } from '$motion/spring';
  import { timeToken } from '$motion/time';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';
  import Mascot from '../mascot/Mascot.svelte';
  import { ONE_SHOT, type MascotMood } from '../mascot/motion';

  const { backend, ui } = useIsland();
  const a = $derived(i18n.m.agents);
  const farewell = $derived(ui.agentFarewell);
  const shown = $derived(farewell?.request ?? backend.agents.pending[0] ?? null);
  const more = $derived(backend.agents.pending.filter((r) => r.id !== shown?.id).length);

  /** Hops for attention first, then waits patiently. */
  let patient = $state(false);
  /** Clicks count only after a moment, so nobody agrees by accident. */
  let armed = $state(false);
  let farewellTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const id = shown?.id;
    patient = false;
    armed = false;
    if (!id) {
      return;
    }
    const root = document.documentElement;
    const calm = setTimeout(() => (patient = true), timeToken(root, '--agent-patience', 5200) / motionSettings.timeScale);
    const arm = setTimeout(() => (armed = true), timeToken(root, '--agent-arm-delay', 500));
    return () => {
      clearTimeout(calm);
      clearTimeout(arm);
    };
  });

  const mood = $derived.by((): MascotMood => {
    if (farewell) {
      return farewell.decision === 'allow' ? 'yes' : 'no';
    }
    return patient ? 'waiting' : 'ask';
  });

  function decide(decision: AgentDecision): void {
    const request = shown;
    if (!request || !armed || farewell) {
      return;
    }
    // The agent gets the answer at once; the island only lingers for the reaction.
    void backend.agents.respond(request.id, decision);
    if (decision === 'terminal' || motionSettings.reduced) {
      return;
    }
    ui.agentFarewell = { request, decision };
    clearTimeout(farewellTimer);
    const duration = ONE_SHOT[decision === 'allow' ? 'yes' : 'no'] ?? 0;
    farewellTimer = setTimeout(() => (ui.agentFarewell = null), (duration * 1000) / motionSettings.timeScale);
  }
</script>

{#if shown}
  <div class="agent-head">
    <Mascot {mood} class="mascot--live" />
    <div class="agent-text">
      <div class="nv-meta">
        <span class="truncate">{a.names[shown.agent]} · {shown.project}</span>
        {#if more > 0}<span>{a.more(more)}</span>{/if}
      </div>
      <!-- The agent's own description reads best; the command below shows what really runs. -->
      <div class="agent-title" title={shown.summary ?? undefined}>{shown.summary ?? a.asks[shown.kind]}</div>
    </div>
  </div>
  <div class="agent-detail" title={shown.detail}>{shown.detail}</div>
  <div class="live-actions agent-actions" class:is-decided={farewell !== null}>
    <button class="btn btn--tonal has-state" title={a.terminalHint} disabled={farewell !== null} onclick={() => decide('terminal')}>
      <Icon name="terminal" />{a.terminal}
    </button>
    <button class="btn btn--tonal has-state" class:is-chosen={farewell?.decision === 'deny'} disabled={farewell !== null} onclick={() => decide('deny')}>
      <Icon name="block" />{a.deny}
    </button>
    <button class="btn btn--filled has-state" class:is-chosen={farewell?.decision === 'allow'} disabled={farewell !== null} onclick={() => decide('allow')}>
      <Icon name="check" />{a.allow}
    </button>
  </div>
{/if}
