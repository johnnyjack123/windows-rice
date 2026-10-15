import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath, URL } from 'node:url';

const path = (p: string): string => fileURLToPath(new URL(p, import.meta.url));

/**
 * One Vite app, one HTML page per window. Tauri loads `island.html`,
 * `settings.html` and `break.html`; `lab.html` and `gallery.html` are
 * browser-only dev tools.
 */
export default defineConfig({
  root: 'web',
  publicDir: false,
  plugins: [svelte({ configFile: path('./svelte.config.js') })],
  resolve: {
    // Component tests need Svelte's browser build (mount is client-only).
    conditions: process.env.VITEST ? ['browser'] : undefined,
    alias: {
      $design: path('./web/src/design'),
      $motion: path('./web/src/motion'),
      $i18n: path('./web/src/i18n'),
      $ui: path('./web/src/ui'),
      $backend: path('./web/src/backend'),
      $island: path('./web/src/island'),
      $settings: path('./web/src/settings'),
      $break: path('./web/src/break'),
      $lab: path('./web/src/lab'),
    },
  },
  clearScreen: false,
  // Tauri expects a fixed port; see docs/BACKEND.md.
  server: { port: 1420, strictPort: true, host: '127.0.0.1' },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    // WebView2 is evergreen Chromium.
    target: 'chrome120',
    rollupOptions: {
      input: {
        island: path('./web/island.html'),
        settings: path('./web/settings.html'),
        break: path('./web/break.html'),
        lab: path('./web/lab.html'),
        gallery: path('./web/gallery.html'),
      },
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    setupFiles: ['src/test-setup.ts'],
  },
});
