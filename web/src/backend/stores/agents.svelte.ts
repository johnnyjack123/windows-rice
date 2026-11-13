import type { AgentDecision, AgentId, AgentIntegration, AgentRequest, AgentStatus } from '../contract';
import { Emitter, type Transport } from '../transport';

/** Coding agents: permission requests waiting in the island, status reports and the hook setup. */
export class AgentsStore {
  /** Waiting requests, oldest first. */
  pending = $state<AgentRequest[]>([]);
  /** Agents rice can be hooked into (settings window). */
  integrations = $state<AgentIntegration[]>([]);
  /** Fires for every new request (the island shows it). */
  readonly requested = new Emitter<AgentRequest>();
  /** Fires when an agent finished or waits for input. */
  readonly status = new Emitter<AgentStatus>();

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads waiting requests and subscribes. */
  async init(): Promise<void> {
    this.pending = await this.t.invoke('agents_pending');
    this.t.listen('agents://request', (request) => {
      this.pending = [...this.pending.filter((r) => r.id !== request.id), request];
      this.requested.emit(request);
    });
    this.t.listen('agents://resolved', ({ id }) => {
      this.pending = this.pending.filter((r) => r.id !== id);
    });
    this.t.listen('agents://status', (status) => this.status.emit(status));
  }

  /**
   * Answers a request. It leaves the list at once; the agent may have stopped
   * waiting meanwhile, which is fine.
   *
   * @param id - Request to answer.
   * @param decision - Allow, deny, or leave it to the agent's own prompt.
   */
  async respond(id: string, decision: AgentDecision): Promise<void> {
    this.pending = this.pending.filter((r) => r.id !== id);
    try {
      await this.t.invoke('agents_respond', { id, decision });
    } catch {
      // Already answered in the terminal or timed out: nothing left to do.
    }
  }

  /** Reads which agents are installed and hooked up. */
  async loadIntegrations(): Promise<void> {
    this.integrations = await this.t.invoke('agents_list');
  }

  /**
   * Hooks rice into an agent or removes the hook again.
   *
   * @param id - Agent.
   * @param connected - Target state.
   */
  async setConnected(id: AgentId, connected: boolean): Promise<void> {
    const next = connected ? await this.t.invoke('agents_connect', { id }) : await this.t.invoke('agents_disconnect', { id });
    this.integrations = this.integrations.map((i) => (i.id === id ? next : i));
  }
}
