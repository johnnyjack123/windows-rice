/**
 * Switches between two stacked panels (pages or panes) with a direction-
 * aware slide. The incoming panel is first placed on the correct side
 * without a transition, so it never slides in from the wrong direction.
 *
 * @param container - Parent that carries `--dir`.
 * @param from - Currently visible panel (may be null on first run).
 * @param to - Panel to show.
 * @param dir - 1 = forward, -1 = backward, 0 = no travel.
 */
export function swapPanels(container: HTMLElement, from: HTMLElement | null, to: HTMLElement, dir: number): void {
  if (from === to) {
    to.classList.add('is-current');
    to.inert = false;
    return;
  }
  container.style.setProperty('--dir', String(dir));
  for (const el of container.children) {
    if (el !== from && el instanceof HTMLElement) {
      el.classList.remove('is-leaving');
      if (el !== to) {
        el.inert = true;
      }
    }
  }
  to.style.transition = 'none';
  to.classList.remove('is-current');
  void to.offsetWidth;
  to.style.transition = '';
  if (from) {
    from.classList.remove('is-current');
    from.classList.add('is-leaving');
    from.inert = true;
  }
  to.classList.add('is-current');
  to.inert = false;
}
