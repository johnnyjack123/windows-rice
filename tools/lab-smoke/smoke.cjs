// Headless smoke test for the Island Lab: executes the real scripts in jsdom,
// drives every action and reports runtime errors and unknown icons.
// It does not check visuals — layout and appearance are reviewed by hand.
const path = require('path');
const fs = require('fs');
const vm = require('vm');
const { JSDOM } = require('jsdom');

const labDir = process.argv[2] || path.resolve(__dirname, '../../mocks/island');
const html = fs.readFileSync(path.join(labDir, 'index.html'), 'utf8')
  .replace(/<script[\s\S]*?<\/script>/g, '')
  .replace(/<link[^>]+>/g, '');

const dom = new JSDOM(html, { pretendToBeVisual: true, url: 'http://localhost/', runScripts: 'outside-only' });
const { window } = dom;
const problems = [];

window.addEventListener('error', (e) => problems.push(`window error: ${e.message}`));
window.console.error = (...a) => problems.push(`console.error: ${a.join(' ')}`);
window.console.warn = (...a) => problems.push(`console.warn: ${a.join(' ')}`);
process.on('unhandledRejection', (r) => problems.push(`unhandled rejection: ${r && r.stack || r}`));

// Minimal stubs for APIs jsdom does not implement.
window.document.fonts = { ready: Promise.resolve(), addEventListener() {} };
window.matchMedia = () => ({ matches: false, addEventListener() {} });
window.URL.createObjectURL = () => 'blob:stub';
window.URL.revokeObjectURL = () => {};
window.HTMLCanvasElement.prototype.getContext = function () {
  return {
    filter: 'none',
    drawImage() {},
    getImageData: (x, y, w, h) => {
      const data = new Uint8ClampedArray(w * h * 4);
      for (let i = 0; i < data.length; i += 4) {
        data[i] = 200; data[i + 1] = 110; data[i + 2] = 80; data[i + 3] = 255;
      }
      return { data };
    },
  };
};
window.HTMLCanvasElement.prototype.toBlob = function (cb) { cb(new window.Blob([])); };
window.Image = class {
  set src(v) { this._src = v; this.naturalWidth = 1920; this.naturalHeight = 1080; setTimeout(() => this.onload?.(), 0); }
  get src() { return this._src; }
};
window.Element.prototype.setPointerCapture = function () {};
window.Element.prototype.releasePointerCapture = function () {};
window.Element.prototype.hasPointerCapture = function () { return false; };

const ctx = dom.getInternalVMContext();
const order = ['motion', 'icons', 'art', 'data', 'themes', 'island', 'island-views', 'panel', 'lab', 'app'];
for (const name of order) {
  const file = path.join(labDir, 'js', `${name}.js`);
  try {
    vm.runInContext(fs.readFileSync(file, 'utf8'), ctx, { filename: file });
  } catch (e) {
    problems.push(`load ${name}.js: ${e.stack}`);
  }
}

const run = (label, code) => {
  try {
    vm.runInContext(code, ctx);
  } catch (e) {
    problems.push(`${label}: ${e.stack.split('\n').slice(0, 3).join(' | ')}`);
  }
};

run('init', 'App.init()');

const steps = [
  ['notification', 'App.pushNotification()'],
  ['volume osd', 'App.setVolume(0.3, true)'],
  ['brightness osd', 'App.setBrightness(0.9, true)'],
  ['pause', 'App.togglePlay()'],
  ['play', 'App.togglePlay()'],
  ['skip', 'App.skip(1); App.skip(-1)'],
  ['seek', 'App.seek(0.5)'],
  ['timer split', 'App.startTimer()'],
  ['event', 'App.triggerEvent()'],
  ['charging', 'App.toggleCharging(); App.toggleCharging()'],
  ['ticks', 'for (let i = 0; i < 5; i++) { App.tick(); }'],
  ['expand', 'App.island.expand("calendar")'],
  ['tabs', 'for (const t of TABS) { Panel.setTab(t.id); } Panel.updateSystem()'],
  ['tiles', 'for (const t of [...TILES.large, ...TILES.small]) { App.toggleSetting(t.id); App.toggleSetting(t.id); }'],
  ['airplane', 'App.toggleSetting("airplane"); App.toggleSetting("airplane")'],
  ['pages', 'Panel.openPage("wifi"); Panel.openPage("home"); Panel.openPage("bluetooth"); Panel.openPage("home")'],
  ['wifi connect', 'Panel.openPage("wifi"); Panel.connect("Telekom_FON")'],
  ['bt toggle', 'Panel.toggleDevice("buds"); Panel.pair()'],
  ['tasks', 'Panel.taskFilter = "private"; Panel.renderTaskLists(); Panel.renderTasks(); Panel.taskFilter = "all"; Panel.renderTaskLists(); Panel.renderTasks(); Panel.toggleTask(1)'],
  ['proc sort', 'Panel.procSort = "ram"; Panel.renderProcs()'],
  ['agenda days', 'for (let d = -2; d < 6; d++) { Panel.day = dayOffset(new Date(), d); Panel.renderWeek(); Panel.renderAgenda(); }'],
  ['clicks', `
    const root = document.querySelector('[data-view="panel"]');
    const click = (sel) => { const el = root.querySelector(sel); if (el) { el.dispatchEvent(new window.MouseEvent('click', { bubbles: true })); } };
    click('[data-tab="tasks"]'); click('[data-filter="rice"]'); click('.check'); click('[data-tab="notifications"]');
    click('[data-dismiss]'); click('[data-clear]'); click('[data-open="bluetooth"]'); click('[data-device="mx"]');
    click('[data-back]'); click('[data-toggle="dnd"]'); click('[data-action="next"]'); click('[data-settings]');
    click('[data-tab="system"]'); click('[data-sort="cpu"]'); click('[data-kill]');
  `],
  ['collapse', 'App.island.collapse()'],
  ['themes', 'for (const t of THEMES) { for (const s of ["dark", "light"]) { Lab.setTheme(t.id, false); Lab.setScheme(s); } }'],
  ['accent', 'Lab.setAccent("#ff0000"); Lab.setAccent(null)'],
  ['wallpapers', 'for (const w of Art.wallpapers) { Lab.setWall(w.id); } Lab.setWall(null, "blob:upload")'],
  ['tempo', 'Lab.setTempo(0.2); Lab.setTempo(1)'],
  ['reduced', 'Lab.setReduced(true); Lab.setReduced(false)'],
  ['backdrop', 'Lab.setBackdrop("light"); Lab.setBackdrop("dark"); Lab.setBackdrop("none")'],
  ['keyboard', `for (const k of ['1','2','3','4','5','n','v','b','m','.','e','t','c','d','s','r','h']) { document.body.dispatchEvent(new window.KeyboardEvent('keydown', { key: k, bubbles: true })); }`],
  ['hover', `const s = document.querySelector('.island'); s.dispatchEvent(new window.Event('pointerenter')); s.dispatchEvent(new window.Event('pointerleave'));`],
  ['linear()', 'if (!/^linear\\(0, /.test(Motion.toLinear(0.5, 0.25).easing)) throw new Error("bad easing")'],
];
for (const [label, code] of steps) {
  run(label, code);
}

setTimeout(() => {
  run('state', `
    const island = App.island;
    if (!island.current) { throw new Error('no current view'); }
    globalThis.__summary = { view: island.current, activities: [...island.activities.keys()], theme: ThemeEngine.theme, scheme: ThemeEngine.scheme };
  `);
  console.log('summary:', JSON.stringify(vm.runInContext('globalThis.__summary', ctx)));
  const unique = [...new Set(problems)];
  console.log(unique.length ? `PROBLEMS (${unique.length}):\n- ${unique.join('\n- ')}` : 'OK: no runtime errors, no unknown icons');
  window.close();
  process.exit(0);
}, 6500);
