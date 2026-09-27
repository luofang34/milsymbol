// Extracts the small upstream data tables: label overrides, base frame
// geometries, default colour modes and character widths.
//
// Output: tools/codegen/out/misc.json (consumed by `cargo xtask emit`).

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { ms } from "../oracle/oracle.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(here, "out");

function labels(type) {
  const cache = {};
  for (const fn of ms._labelOverrides[type] || []) fn.call({}, cache);
  const out = [];
  for (const sidc of Object.keys(cache).sort()) {
    const fields = [];
    for (const field of Object.keys(cache[sidc])) {
      const v = cache[sidc][field];
      const list = Array.isArray(v) ? v : [v];
      fields.push({ field, isArray: Array.isArray(v), labels: list });
    }
    out.push({ sidc, fields });
  }
  return out;
}

function geometries() {
  const out = [];
  for (const name of Object.keys(ms._symbolGeometries)) {
    const g = ms._symbolGeometries[name];
    out.push({ name, g: g.g, bbox: { x1: g.bbox.x1, y1: g.bbox.y1, x2: g.bbox.x2, y2: g.bbox.y2 } });
  }
  return out;
}

function colorModes() {
  const out = [];
  for (const name of Object.keys(ms._colorModes)) out.push({ name, mode: ms._colorModes[name] });
  return out;
}

// Character widths live in a module-private table; recover them through the
// public text layout by measuring single characters.
async function charWidths() {
  const mod = await import(path.join(here, "..", "oracle", "upstream", "src", "symbolfunctions", "string-width.js"));
  const strWidth = mod.default;
  const src = fs.readFileSync(
    path.join(here, "..", "oracle", "upstream", "src", "symbolfunctions", "string-width.js"),
    "utf8"
  );
  const body = src.slice(src.indexOf("{") + 1, src.indexOf("};"));
  const table = new Function(`return {${body}};`)();
  const out = [];
  for (const ch of Object.keys(table)) {
    if (ch.length !== 1) throw new Error(`multi-unit key ${JSON.stringify(ch)}`);
    if (strWidth(ch, 30, 0) !== table[ch]) throw new Error(`width mismatch for ${ch}`);
    out.push([ch.charCodeAt(0), table[ch]]);
  }
  out.sort((a, b) => a[0] - b[0]);
  return out;
}

const json = {
  labels: { letter: labels("letter"), number: labels("number") },
  geometries: geometries(),
  colorModes: colorModes(),
  charWidths: await charWidths(),
};
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, "misc.json"), JSON.stringify(json, null, 1));
console.error(
  `labels: ${json.labels.letter.length} letter, ${json.labels.number.length} number; ` +
    `${json.geometries.length} geometries; ${json.colorModes.length} colour modes; ${json.charWidths.length} widths`
);
