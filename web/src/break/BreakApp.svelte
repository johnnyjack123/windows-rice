<script lang="ts">
  import { onMount } from 'svelte';
  import type { BreakOverlay } from '$backend/contract';
  import { clock } from '$backend/clock.svelte';
  import { provideBackend, type Backend } from '$backend/index';
  import { provideEngine, type ThemeEngine } from '$design/engine.svelte';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { timeToken } from '$motion/time';

  interface Props {
    backend: Backend;
    engine: ThemeEngine;
  }

  const props: Props = $props();
  // Backend and engine are fixed for the lifetime of the window.
  // svelte-ignore state_referenced_locally
  const { backend, engine } = props;
  provideBackend(backend);
  provideEngine(engine);

  /** `break.html?preview`, `?preview=long` or `?preview=over` shows a demo break without a running pomodoro. */
  function previewOverlay(): BreakOverlay | null {
    const mode = new URLSearchParams(location.search).get('preview');
    if (mode === null) {
      return null;
    }
    const duration = (mode === 'long' ? 15 : 5) * 60_000;
    return {
      endsAt: mode === 'over' ? Date.now() : Date.now() + duration - 1000,
      remaining: duration,
      duration,
      kind: mode === 'long' ? 'long' : 'short',
      round: mode === 'long' ? 4 : 2,
      rounds: 4,
    };
  }

  let overlay = $state<BreakOverlay | null>(previewOverlay());
  let entered = $state(false);
  let leaving = $state(false);
  let holding = $state(false);
  let inhale = $state(true);
  const b = $derived(i18n.m.breakScreen);
  /** One tip per break, in whatever language is active. */
  const tipIndex = Math.floor(Math.random() * 64);

  const remaining = $derived(overlay ? (overlay.endsAt === null ? overlay.remaining : Math.max(0, overlay.endsAt - clock.now)) : 0);
  const over = $derived(overlay !== null && overlay.endsAt !== null && remaining <= 0);
  const paused = $derived(overlay?.endsAt === null);
  const progress = $derived(overlay && overlay.duration ? remaining / overlay.duration : 0);

  let holdTimer: ReturnType<typeof setTimeout> | undefined;

  function finish(): void {
    holding = false;
    leaving = true;
    void backend.transport.invoke('break_finish');
  }

  function holdStart(): void {
    if (holding || over) {
      return;
    }
    holding = true;
    holdTimer = setTimeout(finish, timeToken(document.documentElement, '--break-hold', 1600));
  }

  function holdEnd(): void {
    holding = false;
    clearTimeout(holdTimer);
  }

  function holdKey(e: KeyboardEvent): void {
    if ((e.key === ' ' || e.key === 'Enter') && !e.repeat) {
      e.preventDefault();
      if (e.type === 'keydown') {
        holdStart();
      } else {
        holdEnd();
      }
    }
  }

  onMount(() => {
    clock.start();
    if (!overlay) {
      void backend.transport.invoke('break_state').then((o) => {
        overlay = o;
      });
    }
    const off = backend.transport.listen('break://state', (o) => {
      if (o) {
        overlay = o;
        leaving = false;
      } else {
        leaving = true;
      }
    });
    // Breathing guide: the caption follows the CSS animation's halves ("8s" → 4 s each).
    const breath = setInterval(() => (inhale = !inhale), timeToken(document.documentElement, '--break-breath', 8000) / 2);
    requestAnimationFrame(() => (entered = true));
    return () => {
      off();
      clearInterval(breath);
      clearTimeout(holdTimer);
    };
  });
</script>

<main
  class="break"
  class:is-in={entered && overlay !== null}
  class:is-leaving={leaving}
  class:is-over={over}
  class:is-paused={paused}
  class:is-long={overlay?.kind === 'long'}
  aria-live="polite"
>
  <div class="break__backdrop" aria-hidden="true">
    <span class="break__orb break__orb--a"></span>
    <span class="break__orb break__orb--b"></span>
    <span class="break__orb break__orb--c"></span>
  </div>

  {#if overlay}
    <header class="break__top" style:--i={0}>
      <span class="break__clock num">{fmt.time(clock.date)}</span>
    </header>

    <section class="break__center">
      <div class="break__breath" aria-hidden="true"></div>
      <svg class="break__ring" viewBox="0 0 100 100" aria-hidden="true">
        <circle class="break__track" cx="50" cy="50" r="48" pathLength="1" />
        <circle class="break__progress" cx="50" cy="50" r="48" pathLength="1" style:--p={progress} />
      </svg>
      <div class="break__content">
        <div class="break__kind" style:--i={1}>{over ? b.over : paused ? b.paused : b.kinds[overlay.kind]}</div>
        {#if over}
          <div class="break__time break__time--done" style:--i={2}><Icon name="check" /></div>
          <div class="break__sub" style:--i={3}>{b.overHint}</div>
        {:else}
          <div class="break__time num" role="timer" style:--i={2}>{fmt.duration(Math.ceil(remaining / 1000))}</div>
          <div class="break__sub break__breath-label" style:--i={3}>{inhale ? b.breatheIn : b.breatheOut}</div>
        {/if}
        <div class="break__ascii num" aria-hidden="true">
          [{'#'.repeat(Math.round((1 - progress) * 16))}{'·'.repeat(16 - Math.round((1 - progress) * 16))}]
        </div>
        <span class="break__dots" style:--i={4} role="img" aria-label={i18n.m.pomodoro.session(Math.max(1, overlay.round), overlay.rounds)}>
          {#each { length: overlay.rounds } as _, i (i)}
            <i class:is-done={i < overlay.round}></i>
          {/each}
        </span>
      </div>
    </section>

    <p class="break__tip" style:--i={5}>{b.tips[tipIndex % b.tips.length]}</p>

    <footer class="break__actions" style:--i={6}>
      {#if over}
        <button class="btn btn--filled break__back has-state" onclick={finish}>{b.back}<Icon name="arrow_forward" /></button>
      {:else}
        <button
          class="break__hold has-state"
          class:is-holding={holding}
          aria-label={b.holdLabel}
          onpointerdown={holdStart}
          onpointerup={holdEnd}
          onpointerleave={holdEnd}
          onpointercancel={holdEnd}
          onkeydown={holdKey}
          onkeyup={holdKey}
        >
          <span class="break__hold-fill" aria-hidden="true"></span>
          <Icon name="fast_forward" />{b.hold}
        </button>
      {/if}
    </footer>
  {/if}
</main>
