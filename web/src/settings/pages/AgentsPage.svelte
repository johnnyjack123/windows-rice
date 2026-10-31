<script lang="ts">
  import { onMount } from 'svelte';
  import type { AgentIntegration } from '$backend/contract';
  import { useBackend } from '$backend/index';
  import { errorCode } from '$backend/transport';
  import { i18n } from '$i18n/i18n.svelte';
  import Banner from '$ui/Banner.svelte';
  import Switch from '$ui/Switch.svelte';
  import Setting from '../Setting.svelte';

  const backend = useBackend();
  const s = $derived(i18n.m.settings.agents);
  const names = $derived(i18n.m.agents.names);
  const agents = $derived(backend.agents.integrations);
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    void backend.agents.loadIntegrations().catch(() => (error = s.errorGeneric));
  });

  async function toggle(agent: AgentIntegration, connected: boolean): Promise<void> {
    busy = true;
    error = null;
    try {
      await backend.agents.setConnected(agent.id, connected);
    } catch (err) {
      error = errorCode(err) === 'invalid_args' ? s.errorInvalid : s.errorGeneric;
    } finally {
      busy = false;
    }
  }
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.agents}</h1>
  <section class="settings-card">
    <div class="settings-card__title">{s.title}</div>
    <div class="settings-card__hint">{s.intro}</div>
    {#each agents as agent (agent.id)}
      <Setting label={names[agent.id]} hint={agent.available ? s.claudeHint : s.notFound}>
        <Switch
          checked={agent.connected}
          label={names[agent.id]}
          disabled={busy || !agent.available}
          onchange={(v) => void toggle(agent, v)}
        />
      </Setting>
    {/each}
    {#if error}<Banner tone="error" icon="error" title={error} />{/if}
    <div class="settings-card__hint">{s.later}</div>
  </section>

  <section class="settings-card">
    <div class="settings-card__title">{s.safety}</div>
    <div class="settings-card__hint">{s.safetyRules}</div>
  </section>
</div>
