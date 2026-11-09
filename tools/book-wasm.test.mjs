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
assert.equal(first.rust_needs_serde, false);
assert.ok(!first.rust.includes('serde_json'), first.rust);
assert.throws(() => render({ sidc: 'invalid' }), /Invalid SIDC/);

for (const version of [15, 16]) {
  const catalog = JSON.parse(wasm.catalog(version));
  assert.ok(catalog.rows.length > 1500);
  assert.ok(catalog.rows.every(row => row.scope !== 'Multipoint'));
  assert.equal(catalog.sets.some(set => set.code === '64'), version === 16);
  assert.ok(catalog.sets.every(set => catalog.rows.some(row => row.set === set.code && row.kind === 'entity')));
  for (const { name, ...request } of catalog.presets) assert.equal(render(request).valid, true, name);
}
for (const [version, standard, mode] of [[16, 'App6', 'Standards'], [15, 'Mil2525', 'Standards'], [10, 'Mil2525', 'Milsymbol304']]) {
  const seen = new Set();
  for (let seed = 0; seed < 20; seed++) {
    const sidc = wasm.random_sidc(version, standard, mode, seed);
    const result = render({ sidc, standard, mode });
    assert.equal(result.valid, true, `${sidc}: ${result.issues}`);
    assert.equal(result.fields.version, String(version));
    seen.add(sidc);
  }
  assert.ok(seen.size > 15, [...seen].join(' '));
}
assert.throws(() => wasm.random_sidc(16, 'App6', 'Milsymbol304', 1), /No valid symbol/);
assert.throws(() => wasm.random_sidc(16, 'Nato', 'Standards', 1), /Invalid request JSON/);
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
console.log('Book WASM: native catalog, presets, random symbols, controls, live inputs, escaping and configuration round-trip passed.');
