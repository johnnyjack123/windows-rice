import type { ArgsOf, BackendError, CommandName, EventName, Events, ResultOf } from './contract';

/** Arguments tuple of a command: empty when the command takes no input. */
export type InvokeArgs<C extends CommandName> = ArgsOf<C> extends void ? [] : [ArgsOf<C>];

/**
 * How the frontend talks to the backend. Two implementations exist: Tauri
 * (real Windows backend) and a simulated backend for the browser.
 */
export interface Transport {
  /** Which implementation is active. */
  readonly kind: 'tauri' | 'mock';

  /**
   * Calls a backend command.
   *
   * @param command - Command name from the contract.
   * @param args - The command's single argument object, if it takes one.
   * @returns The command result.
   * @throws {BackendError} When the backend rejects the call.
   */
  invoke<C extends CommandName>(command: C, ...args: InvokeArgs<C>): Promise<ResultOf<C>>;

  /**
   * Subscribes to a backend event.
   *
   * @param event - Event name from the contract.
   * @param handler - Receives the payload.
   * @returns Unsubscribe function (safe to call at any time).
   */
  listen<E extends EventName>(event: E, handler: (payload: Events[E]) => void): () => void;
}

/**
 * Narrows unknown rejection values to the contract's error shape.
 *
 * @param error - Value a command rejected with.
 * @returns Whether it is a {@link BackendError}.
 */
export function isBackendError(error: unknown): error is BackendError {
  return typeof error === 'object' && error !== null && 'code' in error && 'message' in error;
}

/**
 * Extracts the error code of a rejected command.
 *
 * @param error - Value a command rejected with.
 * @returns The code, or "internal" for unexpected errors.
 */
export function errorCode(error: unknown): BackendError['code'] {
  return isBackendError(error) ? error.code : 'internal';
}

/** Minimal typed event emitter for discrete frontend events. */
export class Emitter<T> {
  private readonly handlers = new Set<(value: T) => void>();

  /**
   * @param handler - Called on every emit.
   * @returns Unsubscribe function.
   */
  on(handler: (value: T) => void): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  /** @param value - Value passed to every handler. */
  emit(value: T): void {
    for (const handler of this.handlers) {
      handler(value);
    }
  }
}
