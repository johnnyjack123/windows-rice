// Headless smoke test for the component gallery: renders every column and
// reports runtime errors and unknown icons. No visual checks.
const path = require('path');
const fs = require('fs');
const vm = require('vm');
const { JSDOM } = require('jsdom');

const dir = process.argv[2] || path.resolve(__dirname, '../../mocks/island');
const html = fs.readFileSync(path.join(dir, 'gallery.html'), 'utf8')
  .replace(/<script[\s\S]*?<\/script>/g, '')
  .replace(/<link[^>]+>/g, '');

const dom = new JSDOM(html, { pretendToBeVisual: true, url: 'http://localhost/', runScripts: 'outside-only' });
const problems = [];
dom.window.console.warn = (...a) => problems.push(`warn: ${a.join(' ')}`);
dom.window.console.error = (...a) => problems.push(`error: ${a.join(' ')}`);

const ctx = dom.getInternalVMContext();
try {
  for (const name of ['motion', 'icons', 'art', 'data', 'themes', 'panel', 'gallery']) {
    vm.runInContext(fs.readFileSync(path.join(dir, 'js', `${name}.js`), 'utf8'), ctx);
  }
  vm.runInContext('initGallery()', ctx);
} catch (e) {
  problems.push(e.stack);
}

console.log('columns:', dom.window.document.querySelectorAll('.g-col').length);
console.log(problems.length ? `PROBLEMS:\n${problems.join('\n')}` : 'OK');
