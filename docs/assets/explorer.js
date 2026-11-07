import init, { render, catalog } from '../wasm/milsymbol_book.js';

const root = document.getElementById('symbol-lab-app');
const byId = name => document.getElementById(`lab-${name}`);
const status = byId('status');
const cache = new Map();
let state;
let draft;
let currentCatalog;
let populated = '';
const fieldElement = name => name === 'status' ? byId('status-code') : byId(name);
const fields = () => Object.fromEntries([...root.querySelectorAll('[data-field]')].map(el => [el.dataset.field, el.value]));

function setStatus(message, error = false) {
  status.textContent = message;
  status.dataset.error = String(error);
}
function choices(select, rows, value, fallback = 'Unlisted code') {
  const unique = [...new Map(rows.map(row => [row.code, row])).values()];
  if (!unique.some(row => row.code === value)) unique.unshift({ code: value, name: fallback });
  select.replaceChildren(...unique.map(row => {
    const option = document.createElement('option');
    option.value = row.code;
    option.textContent = `${row.code} · ${row.name}`;
    option.selected = row.code === value;
    return option;
  }));
}
function getCatalog(version) {
  if (!cache.has(version)) cache.set(version, JSON.parse(catalog(Number(version))));
  return cache.get(version);
}
function populateEntity(value) {
  const set = byId('symbol_set').value;
  const query = byId('search').value.trim().toLowerCase();
  const entries = currentCatalog.rows.filter(row => row.set === set && row.kind === 'entity' &&
    (!query || `${row.code} ${row.name}`.toLowerCase().includes(query)));
  const current = currentCatalog.rows.find(row => row.set === set && row.kind === 'entity' && row.code === value);
  choices(byId('entity'), [{ code: '000000', name: 'Unspecified' }, ...entries], value, current?.name || 'Unlisted code');
}
function populate(f) {
  if (!f) return;
  currentCatalog = getCatalog(f.version);
  const key = `${f.version}/${f.symbol_set}`;
  if (key !== populated) {
    byId('search').value = '';
    choices(byId('symbol_set'), currentCatalog.sets, f.symbol_set);
    const headerMap = { context: ['context'], identity: ['standard_identity'], status: ['status'],
      hq: ['hq_tf_dummy', 'hq_task_force_dummy'], amplifier: ['amplifier', 'amplifying_descriptor'] };
    for (const [field, names] of Object.entries(headerMap)) {
      choices(fieldElement(field), currentCatalog.headers.filter(row => names.includes(row.field)), f[field]);
    }
    for (const field of ['modifier1', 'modifier2']) {
      choices(byId(field), [{ code: '000', name: 'None' }, ...currentCatalog.rows.filter(row =>
        row.kind === field && (row.set === f.symbol_set || row.set === 'common'))], f[field]);
    }
    populated = key;
  }
  populateEntity(f.entity);
  for (const [field, value] of Object.entries(f)) {
    const el = fieldElement(field);
    if (!el) continue;
    if (el.tagName === 'SELECT' && ![...el.options].some(option => option.value === value)) {
      const option = new Option(`${value} · Unlisted code`, value);
      el.add(option);
    }
    el.value = value;
  }
}
function graphicInput() {
  return state?.source?.input || null;
}
function readOptions() {
  const options = structuredClone(state?.options || {});
  options.style ||= {};
  options.text ||= {};
  options.style.size = Number(byId('size').value || state?.options.style.size || 100);
  for (const field of ['fill', 'frame', 'icon']) options.style[field] = byId(field).checked;
  options.direction = byId('direction').value === '' ? null : Number(byId('direction').value);
  options.text.uniqueDesignation = byId('label').value;
  if (graphicInput()) options.text[graphicInput()] = byId('graphic-value').value;
  return options;
}
function request(extra = {}) {
  return { sidc: draft?.sidc ?? state.sidc, fields: draft?.fields ?? null, standard: byId('standard').value, mode: byId('mode').value,
    options: readOptions(), ...extra };
}
function svgElement(tag, attributes) {
  const element = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, String(value));
  return element;
}
function showPreview() {
  const svg = new DOMParser().parseFromString(state.svg, 'image/svg+xml').documentElement;
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', state.source?.name || `Symbol ${state.sidc}`);
  if (byId('guides').checked) {
    const [x, y, width, height] = svg.getAttribute('viewBox').split(/[ ,]+/).map(Number);
    const scale = width / state.size.width;
    svg.setAttribute('viewBox', `${x - 16} ${y - 16} ${width + 32} ${height + 32}`);
    svg.setAttribute('class', 'lab-outer');
    const box = state.bbox;
    svg.append(svgElement('rect', { x: box.x1, y: box.y1, width: box.x2 - box.x1, height: box.y2 - box.y1,
      fill: 'none', stroke: '#64748b', 'stroke-width': 1, 'stroke-dasharray': '4 3', 'vector-effect': 'non-scaling-stroke' }));
    const ax = x + state.anchor.x * scale, ay = y + state.anchor.y * scale;
    svg.append(svgElement('path', { d: `M${ax - 7},${ay}h14 M${ax},${ay - 7}v14`, stroke: '#dc2626',
      'stroke-width': 1.5, 'vector-effect': 'non-scaling-stroke' }));
    svg.append(svgElement('circle', { cx: x + state.octagon_anchor.x * scale, cy: y + state.octagon_anchor.y * scale,
      r: 4, fill: 'none', stroke: '#2563eb', 'stroke-width': 1.5, 'vector-effect': 'non-scaling-stroke' }));
  }
  byId('guide-legend').hidden = !byId('guides').checked;
  byId('preview').replaceChildren(svg);
}
function present(result) {
  state = result;
  root.querySelectorAll('[data-copy]').forEach(button => { button.disabled = false; });
  byId('sidc').value = state.sidc;
  byId('standard').value = state.standard;
  byId('mode').value = state.mode;
  byId('numeric').disabled = !state.fields;
  byId('version').disabled = !state.fields;
  populate(state.fields);
  byId('size').value = state.options.style.size;
  byId('direction').value = state.options.direction ?? '';
  byId('label').value = state.options.text.uniqueDesignation || '';
  for (const field of ['fill', 'frame', 'icon']) byId(field).checked = state.options.style[field];
  const input = graphicInput();
  byId('graphic-label').firstChild.textContent = input ? `Graphic input · ${input} ` : 'Graphic input · not used ';
  byId('graphic-value').disabled = !input;
  byId('graphic-value').value = input ? state.options.text[input] || '' : '';
  byId('name').textContent = state.source?.name || (state.fields ? 'Symbol preview' : 'Legacy letter symbol');
  byId('validity').textContent = state.valid ? 'Valid SIDC' : 'Unsupported content';
  byId('validity').dataset.valid = String(state.valid);
  const md = state.metadata;
  const readable = value => value.replace(/([a-z])([A-Z])/g, '$1 $2');
  byId('explanation').textContent = state.valid
    ? (['45', '46', '47'].includes(state.fields?.symbol_set)
      ? 'Weather symbol · standard identity does not apply'
      : [md.context || 'Unspecified context', md.affiliation || 'Unspecified identity', md.dimension || 'Unspecified dimension'].map(readable).join(' · '))
    : state.issues.join('; ');
  if (state.source?.scope === 'Hierarchy') byId('explanation').textContent += ' · Hierarchy heading: no associated icon.';
  const source = state.source?.drawing || state.source?.code;
  byId('citation').textContent = source ? `${source.document === 'app6e2' ? 'APP-6(E) Version 2' : 'MIL-STD-2525E Change 1'} · table ${source.table} · printed page ${source.page}` : 'Code meanings from the upstream compatibility tables.';
  const number = n => Number(n.toFixed(2)).toString();
  const point = p => `${number(p.x)}, ${number(p.y)}`;
  const metrics = { 'Image size (px)': `${number(state.size.width)} × ${number(state.size.height)}`, 'Map anchor (px)': point(state.anchor),
    'Geometry bounds': `${number(state.bbox.x1)}, ${number(state.bbox.y1)} → ${number(state.bbox.x2)}, ${number(state.bbox.y2)}` };
  byId('metrics').replaceChildren(...Object.entries(metrics).flatMap(([key, value]) => {
    const dt = document.createElement('dt'), dd = document.createElement('dd'); dt.textContent = key; dd.textContent = value; return [dt, dd];
  }));
  byId('rust').textContent = state.rust;
  byId('rust-note').hidden = !state.rust_needs_serde;
  byId('metadata').textContent = JSON.stringify(state.metadata, null, 2);
  showPreview();
  setStatus('Rendered locally with the Rust library.');
}
function run(payload) {
  draft = payload;
  try { present(JSON.parse(render(JSON.stringify(payload)))); }
  catch (error) {
    byId('preview').replaceChildren();
    root.querySelectorAll('[data-copy]').forEach(button => { button.disabled = true; });
    for (const id of ['name', 'citation', 'explanation', 'rust', 'metadata', 'metrics']) byId(id).replaceChildren();
    byId('validity').textContent = 'Invalid input';
    byId('validity').dataset.valid = 'false';
    setStatus(String(error), true);
  }
}
function updateField(event) {
  const field = event.target.dataset.field;
  if (!field || !state) return;
  const f = { ...state.fields, ...fields() };
  if (field === 'symbol_set') { f.entity = '000000'; f.modifier1 = '000'; f.modifier2 = '000'; f.amplifier = '00'; }
  if (field === 'version' || field === 'symbol_set') populate(f);
  run(request({ fields: f }));
}
function applySidc() { run(request({ sidc: byId('sidc').value.trim(), fields: null })); }
function addPresets() {
  byId('presets').replaceChildren(...currentCatalog.presets.map(preset => {
    const button = document.createElement('button'); button.type = 'button'; button.textContent = preset.name;
    button.addEventListener('click', () => { populated = ''; const { name, ...payload } = preset; run(payload); });
    return button;
  }));
}
async function copy(kind) {
  if (!state) return;
  const value = kind === 'config' ? JSON.stringify(state.config, null, 2) : state[kind];
  try { await navigator.clipboard.writeText(value); setStatus(`Copied ${kind === 'rust' ? 'Rust call' : kind}.`); }
  catch { setStatus('Clipboard access is unavailable. Select and copy the displayed value.', true); }
}

try {
  await init();
  run({});
  addPresets();
  root.setAttribute('aria-busy', 'false');
  byId('controls').addEventListener('submit', event => event.preventDefault());
  byId('controls').addEventListener('change', updateField);
  byId('apply').addEventListener('click', applySidc);
  byId('sidc').addEventListener('keydown', event => { if (event.key === 'Enter') { event.preventDefault(); applySidc(); } });
  byId('search').addEventListener('input', () => populateEntity(byId('entity').value));
  for (const field of ['standard', 'mode']) byId(field).addEventListener('change', () => run(request()));
  for (const field of ['size', 'direction', 'label', 'graphic-value', 'fill', 'frame', 'icon']) {
    byId(field).addEventListener('input', () => run(request()));
  }
  byId('guides').addEventListener('change', showPreview);
  root.querySelectorAll('[data-copy]').forEach(button => button.addEventListener('click', () => copy(button.dataset.copy)));
} catch (error) {
  root.setAttribute('aria-busy', 'false');
  setStatus(`The WASM renderer could not load: ${error}. Serve this book over HTTP or open the published GitHub Pages site.`, true);
}
