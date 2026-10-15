<script lang="ts">
  import { useBackend } from '$backend/index';
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import Switch from '$ui/Switch.svelte';

  const backend = useBackend();
  const p = $derived(i18n.m.settings.panel);
  const panel = $derived(backend.settings.value.panel);

  const ALL_TILES = ['wifi', 'bluetooth', 'dnd', 'night', 'airplane', 'saver', 'mic', 'hotspot'] as const;
  const sectionLabel = (id: string): string => (i18n.m.sections as Record<string, string>)[id] ?? id;
  const tileLabel = (id: string): string => (i18n.m.tiles as Record<string, unknown>)[id] as string ?? id;

  function move<T>(list: T[], index: number, delta: number): T[] {
    const next = [...list];
    const target = index + delta;
    if (target < 0 || target >= next.length) {
      return next;
    }
    [next[index], next[target]] = [next[target]!, next[index]!];
    return next;
  }

  const unused = $derived(ALL_TILES.filter((id) => !panel.tilesLarge.includes(id) && !panel.tilesSmall.includes(id)));

  function setTiles(key: 'tilesLarge' | 'tilesSmall', list: string[]): void {
    void backend.settings.patch({ panel: { [key]: list } });
  }

  function toggleSection(id: string, visible: boolean): void {
    const hidden = visible ? panel.hiddenSections.filter((x) => x !== id) : [...panel.hiddenSections, id];
    void backend.settings.patch({ panel: { hiddenSections: hidden } });
  }
</script>

{#snippet tileList(key: 'tilesLarge' | 'tilesSmall', title: string, hint?: string)}
  <section class="settings-card">
    <div class="settings-card__title">{title}</div>
    {#if hint}<div class="settings-card__hint">{hint}</div>{/if}
    {#each panel[key] as id, i (id)}
      <div class="order-row">
        <Icon name="dashboard" class="ic--sm" />
        <span class="setting__label">{tileLabel(id)}</span>
        <span class="order-row__actions">
          <button class="icon-btn has-state" aria-label={p.moveUp(tileLabel(id))} disabled={i === 0} onclick={() => setTiles(key, move(panel[key], i, -1))}><Icon name="arrow_upward" /></button>
          <button class="icon-btn has-state" aria-label={p.moveDown(tileLabel(id))} disabled={i === panel[key].length - 1} onclick={() => setTiles(key, move(panel[key], i, 1))}><Icon name="arrow_downward" /></button>
          <button class="icon-btn has-state" aria-label={p.remove(tileLabel(id))} onclick={() => setTiles(key, panel[key].filter((x) => x !== id))}><Icon name="close" /></button>
        </span>
      </div>
    {/each}
    {#if unused.length}
      <div class="order-row">
        <Icon name="add" class="ic--sm" />
        <select
          class="select"
          aria-label={p.addTile}
          value=""
          onchange={(e) => {
            const id = e.currentTarget.value;
            if (id) {
              setTiles(key, [...panel[key], id]);
            }
            e.currentTarget.value = '';
          }}
        >
          <option value="">{p.addTile}</option>
          {#each unused as id (id)}<option value={id}>{tileLabel(id)}</option>{/each}
        </select>
        <span></span>
      </div>
    {/if}
  </section>
{/snippet}

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.panel}</h1>

  <section class="settings-card">
    <div class="settings-card__title">{p.sections}</div>
    <div class="settings-card__hint">{p.sectionsHint}</div>
    {#each panel.sections as id, i (id)}
      <div class="order-row" class:is-hidden={panel.hiddenSections.includes(id)}>
        <Switch checked={!panel.hiddenSections.includes(id)} label={p.visible(sectionLabel(id))} onchange={(v) => toggleSection(id, v)} />
        <span class="setting__label">{sectionLabel(id)}</span>
        <span class="order-row__actions">
          <button class="icon-btn has-state" aria-label={p.moveUp(sectionLabel(id))} disabled={i === 0} onclick={() => void backend.settings.patch({ panel: { sections: move(panel.sections, i, -1) } })}><Icon name="arrow_upward" /></button>
          <button class="icon-btn has-state" aria-label={p.moveDown(sectionLabel(id))} disabled={i === panel.sections.length - 1} onclick={() => void backend.settings.patch({ panel: { sections: move(panel.sections, i, 1) } })}><Icon name="arrow_downward" /></button>
        </span>
      </div>
    {/each}
  </section>

  {@render tileList('tilesLarge', p.tilesLarge, p.tilesHint)}
  {@render tileList('tilesSmall', p.tilesSmall)}
</div>
