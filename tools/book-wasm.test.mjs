// Exercise the actual WASM artifact that the book ships, including its JS boundary.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

const root = resolve('target/book/wasm');
const wasm = await import(pathToFileURL(`${root}/milsymbol_book.js`));
await wasm.default({ module_or_path: await readFile(`${root}/milsymbol_book_bg.wasm`) });
const render = request => JSON.parse(wasm.render(JSON.stringify(request)));
const first = render({});
assert.equal(first.valid, true);
assert.match(first.svg, /^<svg\b/);
assert.equal(render(first.config).svg, first.svg);
assert.throws(() => render({ sidc: 'invalid' }), /Invalid SIDC/);

for (const version of [15, 16]) {
  const catalog = JSON.parse(wasm.catalog(version));
  assert.ok(catalog.rows.length > 1500);
  assert.ok(catalog.rows.every(row => row.scope !== 'Multipoint'));
  assert.equal(catalog.sets.some(set => set.code === '64'), version === 16);
  assert.ok(catalog.sets.every(set => catalog.rows.some(row => row.set === set.code && row.kind === 'entity')));
  for (const { name, ...request } of catalog.presets) assert.equal(render(request).valid, true, name);
}
const changed = render({ fields: { ...first.fields, version: '16', symbol_set: '64',
  entity: '130100', amplifier: '00', modifier1: '000', modifier2: '000' }, standard: 'App6' });
assert.equal(changed.valid, true);
assert.equal(changed.source.name, 'Firewall');
assert.equal(changed.fields.symbol_set, '64');
assert.notEqual(changed.svg, first.svg);

const weather = value => render({ sidc: '15034500001101020000', options: { text: { altitudeDepth: value } } });
assert.match(weather('450').svg, />450<\/text>/);
assert.notEqual(weather('320').svg, weather('450').svg);
const escaped = render({ options: { text: { uniqueDesignation: '<script>alert(1)</script>' } } });
assert.ok(!escaped.svg.includes('<script>'));
assert.ok(escaped.svg.includes('&lt;script&gt;'));
assert.equal(render(escaped.config).svg, escaped.svg);
console.log('Book WASM: native catalog, presets, controls, live inputs, escaping and configuration round-trip passed.');
