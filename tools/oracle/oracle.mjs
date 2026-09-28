// Oracle runner for the pinned upstream milsymbol.js.
//
// Dev-only tooling: renders cases through the JavaScript implementation and
// emits canonical records the Rust port is compared against.
//
// Determinism policy: upstream keeps a process-global icon/label cache and
// `ms._scale(…, true)` mutates cached icon parts in place, so upstream output
// can depend on which symbols were rendered earlier. The oracle clears both
// caches before every case, which defines the compatibility baseline as
// "each symbol rendered by a freshly initialised milsymbol".

import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));
const upstreamIndex = path.join(here, "upstream", "index.js");

// Silence upstream's "Override of: …" console.warn noise during import.
const warn = console.warn;
console.warn = () => {};
const { default: ms } = await import(upstreamIndex);
console.warn = warn;

export { ms };

const DEFAULT_DASH = ["4,4", "8,12", "8,8"];

// Canonical JSON: sorted keys, undefined dropped (as JSON.stringify), arrays
// keep order and map undefined/functions to null, non-finite numbers to null.
export function canonical(value) {
  return JSON.stringify(sortKeys(value));
}

function sortKeys(v) {
  if (Array.isArray(v)) return v.map((e) => (typeof e === "function" ? null : sortKeys(e)));
  if (v && typeof v === "object") {
    const out = Object.create(null);
    for (const k of Object.keys(v).sort()) {
      const e = v[k];
      if (typeof e === "function" || typeof e === "undefined") continue;
      out[k] = sortKeys(e);
    }
    return out;
  }
  return v;
}

function applyConfig(cfg) {
  if (!cfg) return;
  if (cfg.standard !== undefined) ms.setStandard(cfg.standard);
  if (cfg.dashArrays) ms.setDashArrays(...cfg.dashArrays);
  if (cfg.hqStaffLength !== undefined) ms.setHqStaffLength(cfg.hqStaffLength);
}

function resetConfig(cfg) {
  if (!cfg) return;
  ms.setStandard("2525");
  ms.setDashArrays(...DEFAULT_DASH);
  ms.setHqStaffLength(100);
}

// FNV-1a 64-bit over UTF-8 bytes, lowercase hex. Mirrored in the Rust tests.
export function fnv64(str) {
  const bytes = Buffer.from(str, "utf8");
  let h = 0xcbf29ce484222325n;
  const p = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (const b of bytes) {
    h ^= BigInt(b);
    h = (h * p) & mask;
  }
  return h.toString(16).padStart(16, "0");
}

function metadataView(md) {
  const out = {};
  for (const k of Object.keys(md)) {
    if (k === "baseGeometry") continue;
    out[k] = md[k];
  }
  const g = md.baseGeometry || {};
  out.baseGeometry = {
    g: g.g,
    bbox: g.bbox ? { x1: g.bbox.x1, y1: g.bbox.y1, x2: g.bbox.x2, y2: g.bbox.y2 } : undefined,
  };
  return out;
}

// Render one case. `c` is { sidc, options?, cfg? }.
export function renderCase(c) {
  ms._iconCache = {};
  ms._labelCache = {};
  applyConfig(c.cfg);
  const rec = {};
  try {
    const opts = Object.assign({}, c.options || {});
    const s = new ms.Symbol(c.sidc, opts);
    rec.svg = s.asSVG();
    rec.sem = {
      instructions: s.drawInstructions,
      metadata: metadataView(s.metadata),
      colors: s.colors,
      bbox: { x1: s.bbox.x1, y1: s.bbox.y1, x2: s.bbox.x2, y2: s.bbox.y2 },
      size: s.getSize(),
      anchor: s.getAnchor(),
      octagonAnchor: s.getOctagonAnchor(),
      valid: s.isValid(),
      validExtended: s.isValid(true),
      options: s.getOptions(),
    };
  } catch (e) {
    rec.error = String(e && e.message ? e.message : e);
  } finally {
    resetConfig(c.cfg);
  }
  return rec;
}

// Compact record for checked-in fixtures.
export function fixtureLine(c) {
  const r = renderCase(c);
  const line = { sidc: c.sidc };
  if (c.options) line.options = c.options;
  if (c.cfg) line.cfg = c.cfg;
  if (r.error !== undefined) {
    line.error = r.error;
    return line;
  }
  line.svg = fnv64(r.svg);
  line.sem = fnv64(canonical(r.sem));
  line.valid = r.sem.valid;
  return line;
}
