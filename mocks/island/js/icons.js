/* Icon registry. Material Symbols are loaded as a variable font subset that
   contains exactly these glyphs, in all three styles a theme may pick. */

/** Every icon the UI may use. Using an unlisted name renders as plain text. */
const ICON_NAMES = [
  'add', 'airplanemode_active', 'arrow_back', 'arrow_downward', 'arrow_upward',
  'bluetooth', 'bluetooth_disabled', 'bluetooth_searching', 'bolt',
  'brightness_high', 'brightness_low', 'brightness_medium', 'calendar_month',
  'chat', 'check', 'chevron_right', 'close', 'commit', 'do_not_disturb_on',
  'done_all', 'earbuds', 'edit', 'energy_savings_leaf', 'event', 'forum',
  'headphones', 'keyboard', 'lock', 'mail', 'mic', 'mic_off', 'monitoring',
  'mouse', 'network_wifi_1_bar', 'network_wifi_2_bar', 'network_wifi_3_bar',
  'nightlight', 'notifications', 'notifications_off', 'open_in_new', 'pause',
  'play_arrow', 'power_settings_new', 'progress_activity', 'restart_alt',
  'settings', 'signal_wifi_4_bar', 'skip_next', 'skip_previous', 'speaker',
  'sports_esports', 'sync', 'task_alt', 'timer', 'volume_down', 'volume_mute',
  'volume_off', 'volume_up', 'wifi', 'wifi_off', 'wifi_tethering',
];

const ICON_SET = new Set(ICON_NAMES);

/**
 * Injects the Material Symbols stylesheet for all icon styles used by the
 * themes, subset to {@link ICON_NAMES}. Google Fonts requires the names to
 * be sorted alphabetically.
 */
function loadIconFonts() {
  const axes = 'opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200';
  const families = ['Rounded', 'Sharp', 'Outlined']
    .map((style) => `family=Material+Symbols+${style}:${axes}`)
    .join('&');
  const names = [...ICON_SET].sort().join(',');
  const link = document.createElement('link');
  link.rel = 'stylesheet';
  link.href = `https://fonts.googleapis.com/css2?${families}&icon_names=${names}&display=block`;
  document.head.append(link);
}

/**
 * Renders an icon as markup.
 *
 * @param {string} name - Material Symbols ligature name from {@link ICON_NAMES}.
 * @param {string} [cls=''] - Extra classes.
 * @returns {string} HTML string of an aria-hidden icon.
 */
function ic(name, cls = '') {
  if (!ICON_SET.has(name)) {
    console.warn(`[icons] "${name}" is not in ICON_NAMES and will not render`);
  }
  return `<span class="ic ${cls}" aria-hidden="true">${name}</span>`;
}
