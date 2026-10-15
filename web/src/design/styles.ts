/**
 * Global stylesheet order — the only place it is defined:
 * tokens → fonts → base → components → features → themes.
 * Themes come last so their scoped overrides win ties with component CSS;
 * user themes and editor overrides are appended at runtime after these.
 */
import './tokens.css';
import '@fontsource-variable/roboto-flex/full.css';
import '@fontsource-variable/jetbrains-mono/index.css';
import '@fontsource-variable/instrument-sans/index.css';
import '@fontsource/instrument-serif/400.css';
import '@fontsource/instrument-serif/400-italic.css';
import './fonts/material-symbols.css';
import './base.css';
import '$ui/ui.css';
import '$island/island.css';
import '$island/panel.css';
import '$settings/settings.css';
import '$break/break.css';
import './themes/obsidian.css';
import './themes/monet.css';
import './themes/glass.css';
import './themes/terminal.css';
import './themes/paper.css';
