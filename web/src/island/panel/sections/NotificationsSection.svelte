<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import type { AppNotification } from '$backend/contract';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import { motionSettings } from '$motion/spring';
  import AppIcon from '$ui/AppIcon.svelte';
  import Banner from '$ui/Banner.svelte';
  import Empty from '$ui/Empty.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../../context.svelte';

  const { backend, controller } = useIsland();
  const store = backend.notifications;
  const n = $derived(i18n.m.notifications);
  /** Pointer travel (px) below which a press counts as a tap, not a swipe. */
  const TAP_SLOP = 4;

  /** Ids currently animating out (still rendered until the collapse ends). */
  let removing = $state<string[]>([]);
  let leaving = $state<AppNotification[]>([]);
  const firstIds = new Set(store.items.map((x) => x.id));
  const list = $derived([...store.items, ...leaving.filter((l) => !store.items.some((x) => x.id === l.id))]
    .sort((a, b) => b.time.localeCompare(a.time)));

  function dismiss(item: AppNotification, direction = 1): void {
    if (removing.includes(item.id)) {
      return;
    }
    const card = document.querySelector<HTMLElement>(`[data-nid="${CSS.escape(item.id)}"] .notif`);
    if (card) {
      card.style.translate = `${direction * 110}% 0`;
      card.style.opacity = '0';
    }
    leaving = [...leaving, item];
    removing = [...removing, item.id];
    void store.dismiss(item.id);
    setTimeout(() => {
      leaving = leaving.filter((l) => l.id !== item.id);
      removing = removing.filter((id) => id !== item.id);
    }, 420 / motionSettings.timeScale);
  }

  /** Opens the sending app and closes the panel so the app is in view; stays put if Windows cannot start it. */
  function open(item: AppNotification): void {
    if (removing.includes(item.id)) {
      return;
    }
    store.open(item.id).then(
      () => controller.collapse(),
      () => {},
    );
  }

  function clearAll(): void {
    store.items.forEach((item, i) => setTimeout(() => dismiss(item), (i * 45) / motionSettings.timeScale));
  }

  // Swipe to dismiss.
  let drag: { card: HTMLElement; item: AppNotification; x: number; dx: number; id: number } | null = null;

  function down(e: PointerEvent, item: AppNotification): void {
    if ((e.target as HTMLElement).closest('button')) {
      return;
    }
    const card = e.currentTarget as HTMLElement;
    card.setPointerCapture(e.pointerId);
    drag = { card, item, x: e.clientX, dx: 0, id: e.pointerId };
  }

  function move(e: PointerEvent): void {
    if (!drag || e.pointerId !== drag.id) {
      return;
    }
    drag.dx = e.clientX - drag.x;
    if (Math.abs(drag.dx) > TAP_SLOP) {
      drag.card.classList.add('is-dragging');
    }
    drag.card.style.translate = `${drag.dx}px 0`;
    drag.card.style.opacity = String(1 - Math.min(0.7, Math.abs(drag.dx) / 320));
  }

  /** Ends a press: a long swipe dismisses, a tap opens the app (a cancelled press does neither). */
  function up(e: PointerEvent): void {
    if (!drag) {
      return;
    }
    const { card, item, dx } = drag;
    card.classList.remove('is-dragging');
    if (Math.abs(dx) > 90) {
      dismiss(item, Math.sign(dx));
    } else {
      card.style.translate = '';
      card.style.opacity = '';
      if (e.type === 'pointerup' && Math.abs(dx) <= TAP_SLOP) {
        open(item);
      }
    }
    drag = null;
  }

</script>

{#if store.permission === 'denied' || store.permission === 'unknown'}
  <Banner icon="notifications" title={n.permissionTitle} body={n.permissionBody}>
    <button class="btn btn--filled has-state" onclick={() => void store.requestAccess()}>{n.permissionAction}</button>
  </Banner>
{:else if store.permission === 'unsupported'}
  <Banner tone="warning" icon="info" title={n.permissionTitle} body={n.unsupported} />
{/if}

{#if list.length}
  <div class="pane-toolbar">
    <span class="pane-title">{n.count(store.items.length)}</span>
    <button class="btn btn--text has-state" onclick={clearAll}>{n.clearAll}</button>
  </div>
  <div class="pane__scroll scroll">
    <ul class="notif-list">
      {#each list as item (item.id)}
        <li class="notif-wrap" class:is-removing={removing.includes(item.id)} class:is-new={!firstIds.has(item.id)} data-nid={item.id}>
          <div class="notif-clip">
            <!-- Tapping and swiping are pointer shortcuts; keyboard users open via the title and dismiss via the close button. -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <article
              class="notif"
              aria-label={`${item.appName}: ${item.title}`}
              onpointerdown={(e) => down(e, item)}
              onpointermove={move}
              onpointerup={up}
              onpointercancel={up}
            >
              <AppIcon name={item.appName} src={item.iconUrl} />
              <div class="notif__main">
                <div class="notif__meta">
                  <span class="truncate">{item.appName}</span>
                  <span>{fmt.ago(new Date(item.time), new Date(clock.now))}</span>
                </div>
                <button type="button" class="notif__title" onclick={() => open(item)}>{item.title}</button>
                <div class="notif__body">{item.body}</div>
              </div>
              <button class="icon-btn has-state notif__close" aria-label={n.dismiss} onclick={() => dismiss(item)}>
                <Icon name="close" />
              </button>
            </article>
          </div>
        </li>
      {/each}
    </ul>
  </div>
{:else}
  <Empty icon="done_all" title={n.emptyTitle} body={n.emptyBody} />
{/if}
