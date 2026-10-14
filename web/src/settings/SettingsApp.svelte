<script lang="ts">
  import { provideBackend, type Backend } from '$backend/index';
  import { provideEngine, type ThemeEngine } from '$design/engine.svelte';
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import AboutPage from './pages/AboutPage.svelte';
  import AppearancePage from './pages/AppearancePage.svelte';
  import CalendarPage from './pages/CalendarPage.svelte';
  import IslandPage from './pages/IslandPage.svelte';
  import LanguagePage from './pages/LanguagePage.svelte';
  import PanelPage from './pages/PanelPage.svelte';
  import TimersPage from './pages/TimersPage.svelte';

  interface Props {
    backend: Backend;
    engine: ThemeEngine;
  }

  const props: Props = $props();
  // Backend and engine are fixed for the lifetime of the window.
  // svelte-ignore state_referenced_locally
  provideBackend(props.backend);
  // svelte-ignore state_referenced_locally
  provideEngine(props.engine);

  type PageId = 'appearance' | 'island' | 'panel' | 'calendar' | 'timers' | 'language' | 'about';
  const PAGES: Array<{ id: PageId; icon: string }> = [
    { id: 'appearance', icon: 'palette' },
    { id: 'island', icon: 'pill' },
    { id: 'panel', icon: 'dashboard' },
    { id: 'calendar', icon: 'calendar_month' },
    { id: 'timers', icon: 'alarm' },
    { id: 'language', icon: 'translate' },
    { id: 'about', icon: 'info' },
  ];

  const fromHash = (): PageId => {
    const id = location.hash.slice(1);
    return PAGES.some((p) => p.id === id) ? (id as PageId) : 'appearance';
  };
  let page = $state<PageId>(fromHash());

  $effect(() => {
    const onHash = (): void => {
      page = fromHash();
    };
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  });

  $effect(() => {
    document.title = i18n.m.settings.title;
  });

  let message = $state('');
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  function toast(text: string): void {
    message = text;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (message = ''), 2400);
  }
</script>

<div class="settings">
  <nav class="settings-nav" aria-label={i18n.m.settings.title}>
    <div class="settings-nav__title">{i18n.m.settings.title}</div>
    {#each PAGES as p (p.id)}
      <a
        class="settings-nav__item has-state"
        href="#{p.id}"
        aria-current={page === p.id ? 'page' : undefined}
      >
        <Icon name={p.icon} />{i18n.m.settings.nav[p.id]}
      </a>
    {/each}
  </nav>
  <main class="settings-main">
    {#key page}
      {#if page === 'appearance'}
        <AppearancePage {toast} />
      {:else if page === 'island'}
        <IslandPage />
      {:else if page === 'panel'}
        <PanelPage />
      {:else if page === 'calendar'}
        <CalendarPage />
      {:else if page === 'timers'}
        <TimersPage />
      {:else if page === 'language'}
        <LanguagePage />
      {:else}
        <AboutPage />
      {/if}
    {/key}
  </main>
</div>

<div class="settings-toast" role="status" aria-live="polite" class:is-visible={Boolean(message)}>{message}</div>
