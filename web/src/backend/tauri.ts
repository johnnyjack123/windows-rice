import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Transport } from './transport';

/**
 * Whether the page runs inside a Tauri webview.
 *
 * @returns True inside the desktop app, false in a normal browser.
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Transport backed by Tauri IPC. Commands with input receive it as the single
 * parameter `args` (see contract.ts and docs/BACKEND.md).
 *
 * @returns The transport.
 */
export function createTauriTransport(): Transport {
  return {
    kind: 'tauri',
    invoke(command, ...args) {
      return invoke(command, args.length ? { args: args[0] } : {});
    },
    listen(event, handler) {
      let unlisten: UnlistenFn | null = null;
      let cancelled = false;
      void listen(event, (e) => handler(e.payload as never)).then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      });
      return () => {
        cancelled = true;
        unlisten?.();
      };
    },
  };
}
