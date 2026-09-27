// Mechanical extraction of upstream icon tables into context-keyed templates.
//
// Upstream icon parts and SIDC→icon mappings are JavaScript functions of a
// small rendering context (standard, frame, colours, …). Rather than
// translating that code, this tool evaluates it under enumerated contexts,
// replaces colours and dash arrays with symbolic slots, discovers which
// context variables every table entry depends on, and writes a table of
// variants per entry. A round-trip check then compares table lookups against
// fresh upstream evaluations for random contexts.
//
// Output: tools/codegen/out/tables.json (consumed by `cargo xtask emit`).

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { ms } from "../oracle/oracle.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.join(here, "out");

// ---------------------------------------------------------------------------
// Context variables and their domains. Order is significant: it is mirrored by
// the Rust `generated::Var` enum.
const SLOTS = ["fillColor", "frameColor", "iconColor", "iconFillColor", "black", "white"];
const AFF_KEYS = ["Civilian", "Friend", "Hostile", "Neutral", "Unknown", "Suspect"];
const GEOMS = Object.keys(ms._symbolGeometries).concat(["none"]);
const TOK = "\u0001";
const REF_KEY = Symbol.for("milsymbol.ref");
const colorToken = (slot, aff) => `${TOK}C:${slot}:${aff}${TOK}`;
const MONO_TOKEN = `${TOK}M${TOK}`;
const DASH_PENDING = `${TOK}D:pending${TOK}`;
const DASH_ANTICIPATED = `${TOK}D:anticipated${TOK}`;

export const VARS = [
  { name: "std2525", domain: [true, false] },
  { name: "frame", domain: [true, false] },
  { name: "numberSidc", domain: [true, false] },
  { name: "alternateMedal", domain: [false, true] },
  { name: "mono", domain: ["", MONO_TOKEN] },
  { name: "edition", domain: [undefined, "D", "E"] },
  { name: "notpresent", domain: ["", DASH_PENDING, DASH_ANTICIPATED] },
  { name: "affiliation", domain: ["Friend", "Hostile", "Neutral", "Unknown", "undefined", "none"] },
  { name: "geometry", domain: GEOMS },
].concat(SLOTS.map((s) => ({ name: "slot_" + s, domain: [true, false] })));

const VI = Object.fromEntries(VARS.map((v, i) => [v.name, i]));

function contextOf(cfg) {
  const val = (name) => VARS[VI[name]].domain[cfg[VI[name]]];
  const geomName = val("geometry");
  const baseGeometry =
    geomName === "none" ? { g: "", bbox: new ms.BBox() } : ms._symbolGeometries[geomName];
  const metadata = {
    frame: val("frame"),
    affiliation: val("affiliation"),
    baseGeometry,
    numberSIDC: val("numberSidc"),
    edition: val("edition"),
    notpresent: val("notpresent"),
  };
  const colors = { none: {} };
  for (const k of AFF_KEYS) colors.none[k] = false;
  for (const slot of SLOTS) {
    const truthy = val("slot_" + slot);
    colors[slot] = {};
    for (const k of AFF_KEYS) colors[slot][k] = truthy ? colorToken(slot, k) : false;
  }
  return {
    metadata,
    colors,
    std: val("std2525"),
    mono: val("mono"),
    altMedal: val("alternateMedal"),
    edition: val("edition"),
    aff: metadata.affiliation,
  };
}

// ---------------------------------------------------------------------------
// Template conversion of upstream values.

const PAINT_KEYS = new Set(["fill", "stroke"]);
const NUM_KEYS = new Set([
  "x", "y", "cx", "cy", "r", "fontsize", "strokewidth", "fillopacity",
  "degree", "factor", "non_scaling_stroke",
]);
const STR_KEYS = new Set([
  "d", "text", "fontfamily", "fontweight", "textanchor", "alignmentBaseline",
  "linecap", "clipPath", "strokedasharray",
]);
const BOOL_KEYS = new Set(["icon", "styleFill"]);
const TYPES = new Set(["path", "circle", "text", "translate", "rotate", "scale"]);

function paintTemplate(v, aff, where) {
  if (v === undefined) return undefined;
  if (v === false) return { k: "none" };
  if (typeof v !== "string") throw new Error(`paint ${where}: ${JSON.stringify(v)}`);
  if (v === MONO_TOKEN) return { k: "mono" };
  const m = /^\u0001C:([A-Za-z]+):([A-Za-z]+)\u0001$/.exec(v);
  if (m) return { k: "slot", slot: m[1], aff: m[2] === aff ? "self" : m[2] };
  if (v.includes(TOK)) throw new Error(`embedded token in paint ${where}: ${v}`);
  return { k: "lit", v };
}

function scalarTemplate(v, where) {
  if (typeof v === "number") return { n: v };
  if (typeof v === "string") {
    if (v.includes(TOK)) throw new Error(`embedded token in scalar ${where}: ${v}`);
    return { s: v };
  }
  if (typeof v === "boolean") return { b: v };
  throw new Error(`scalar ${where}: ${JSON.stringify(v)}`);
}

function strTemplate(key, v, where) {
  if (typeof v !== "string") throw new Error(`string ${where}.${key}: ${JSON.stringify(v)}`);
  if (key === "strokedasharray") {
    if (v === DASH_PENDING) return { dash: "pending" };
    if (v === DASH_ANTICIPATED) return { dash: "anticipated" };
  }
  if (v.includes(TOK)) throw new Error(`embedded token in ${where}.${key}: ${v}`);
  return { s: v };
}

export function toTemplate(v, aff, where = "", refs = false) {
  if (refs && v !== null && typeof v === "object" && v[REF_KEY] !== undefined) return { ref: v[REF_KEY] };
  if (v === undefined || v === null) return { missing: true };
  if (Array.isArray(v)) return { group: v.map((e, i) => toTemplate(e, aff, `${where}[${i}]`, refs)) };
  if (typeof v !== "object") return { scalar: scalarTemplate(v, where) };
  if (!TYPES.has(v.type)) throw new Error(`unknown node type at ${where}: ${v.type}`);
  const t = { type: v.type };
  for (const key of Object.keys(v).sort()) {
    const val = v[key];
    if (key === "type") continue;
    if (key === "draw") {
      t.draw = toTemplate(val, aff, `${where}.draw`, refs);
      continue;
    }
    if (val === undefined) continue; // present-but-undefined == absent downstream
    if (PAINT_KEYS.has(key)) t[key] = paintTemplate(val, aff, `${where}.${key}`);
    else if (NUM_KEYS.has(key)) t[key] = scalarTemplate(val, `${where}.${key}`);
    else if (STR_KEYS.has(key)) t[key] = strTemplate(key, val, where);
    else if (BOOL_KEYS.has(key)) {
      if (typeof val !== "boolean") throw new Error(`bool ${where}.${key}`);
      t[key] = val;
    } else throw new Error(`unknown key ${key} at ${where}`);
  }
  return t;
}

function bboxTemplate(v, where) {
  if (v === undefined) return undefined;
  const t = {};
  for (const k of Object.keys(v)) {
    if (!["x1", "x2", "y1", "y2"].includes(k)) throw new Error(`bbox key ${k} at ${where}`);
    if (typeof v[k] !== "number") throw new Error(`bbox value ${where}.${k}`);
    t[k] = v[k];
  }
  return t;
}

// ---------------------------------------------------------------------------
// Evaluation helpers.

let NUMBER_SETS = null;
let OVERRIDE_SETS = null; // mapping id ("N|15", "L") -> [part names mutated]

const freshParts = (c) => ms._getIconParts(c.metadata, c.colors, c.std, c.mono, c.altMedal);

function runMapping(id, parts, c) {
  if (id === "L") return ms._getIcons.letter(ms, parts, c.std);
  return ms._getIcons.number(ms, id.slice(2), parts, c.std, c.edition);
}

function partTemplates(parts, aff) {
  const m = new Map();
  for (const name of Object.keys(parts)) m.set(name, JSON.stringify(toTemplate(parts[name], aff, name)));
  return m;
}

// Parts and mapping-mutated part overrides under one context.
function evalParts(cfg) {
  const c = contextOf(cfg);
  const out = new Map();
  for (const [name, t] of partTemplates(freshParts(c), c.aff)) out.set(`P|${name}`, t);
  for (const [id, names] of OVERRIDE_SETS) {
    const parts = freshParts(c);
    runMapping(id, parts, c);
    for (const name of names) out.set(`O|${id}|${name}`, JSON.stringify(toTemplate(parts[name], c.aff, name)));
  }
  return out;
}

function mappingIds() {
  return NUMBER_SETS.map((ss) => `N|${ss}`).concat(["L"]);
}

// Mapping entries with part references, for one mapping id under one context.
// With `inline` the real part objects are used (for the round-trip check);
// otherwise every icn[...] read yields a reference token.
const REF = REF_KEY;
function evalMapping(id, cfg, inline = false) {
  const c = contextOf(cfg);
  let parts = freshParts(c);
  if (!inline) {
    const tokens = new Map();
    parts = new Proxy(parts, {
      get(target, name) {
        if (typeof name !== "string") return target[name];
        if (!tokens.has(name)) tokens.set(name, { [REF]: name });
        return tokens.get(name);
      },
    });
  }
  const r = runMapping(id, parts, c);
  const out = new Map();
  const kinds = id === "L" ? ["icons"] : ["icons", "m1", "m2"];
  for (const kind of kinds)
    for (const code of Object.keys(r[kind]))
      out.set(`${id}|${kind}|${code}`, JSON.stringify(toTemplate(r[kind][code], c.aff, code, !inline)));
  for (const code of Object.keys(r.bbox)) out.set(`${id}|bbox|${code}`, JSON.stringify({ bbox: bboxTemplate(r.bbox[code], code) }));
  return out;
}

// ---------------------------------------------------------------------------

function mulberry32(seed) {
  return function () {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const randomCfg = (rand) => VARS.map((v) => Math.floor(rand() * v.domain.length));

function discoverNumberSets() {
  const found = new Set();
  for (const cfg of [VARS.map(() => 0), VARS.map((v) => v.domain.length - 1)]) {
    const c = contextOf(cfg);
    for (let i = 0; i < 100; i++) {
      const ss = String(i).padStart(2, "0");
      for (const edition of [undefined, "D", "E"])
        for (const std of [true, false]) {
          const r = ms._getIcons.number(ms, ss, freshParts(c), std, edition);
          if (["icons", "m1", "m2", "bbox"].some((k) => Object.keys(r[k]).length)) found.add(ss);
        }
    }
  }
  return [...found].sort();
}

function discoverOverrides(rand) {
  const found = new Map();
  for (let i = 0; i < 40; i++) {
    const c = contextOf(randomCfg(rand));
    for (const id of mappingIds()) {
      const parts = freshParts(c);
      const before = partTemplates(parts, c.aff);
      runMapping(id, parts, c);
      const after = partTemplates(parts, c.aff);
      for (const [name, t] of after)
        if (before.get(name) !== t) {
          if (!found.has(id)) found.set(id, new Set());
          found.get(id).add(name);
        }
    }
  }
  return new Map([...found].map(([id, s]) => [id, [...s].sort()]));
}

// Build context-keyed tables for the keys produced by `evalFn` by perturbation.
function buildTables(evalFn, baselines, rand, log, label) {
  const deps = new Map();
  const note = (k, vi) => {
    if (!deps.has(k)) deps.set(k, new Set());
    if (vi !== undefined) deps.get(k).add(vi);
  };
  for (let b = 0; b < baselines; b++) {
    const base = randomCfg(rand);
    const eb = evalFn(base);
    for (const k of eb.keys()) note(k);
    for (let vi = 0; vi < VARS.length; vi++)
      for (let a = 0; a < VARS[vi].domain.length; a++) {
        if (a === base[vi]) continue;
        const cfg = base.slice();
        cfg[vi] = a;
        const ea = evalFn(cfg);
        for (const [k, v] of ea) if (eb.get(k) !== v) note(k, vi);
        for (const k of eb.keys()) if (!ea.has(k)) note(k, vi);
      }
    if (b % 40 === 0) log(`${label}: baseline ${b}/${baselines}, keys ${deps.size}`);
  }
  const byDeps = new Map();
  for (const [k, s] of deps) {
    const d = [...s].sort((a, b) => a - b);
    const sig = d.join(",");
    if (!byDeps.has(sig)) byDeps.set(sig, { deps: d, keys: [] });
    byDeps.get(sig).keys.push(k);
  }
  const tables = new Map();
  let runs = 0;
  for (const { deps: d, keys } of byDeps.values()) {
    let combos = [[]];
    for (const vi of d) {
      const next = [];
      for (const pre of combos) for (let a = 0; a < VARS[vi].domain.length; a++) next.push(pre.concat([a]));
      combos = next;
    }
    for (const k of keys) tables.set(k, { deps: d, rows: new Map() });
    for (const combo of combos) {
      const cfg = VARS.map(() => 0);
      d.forEach((vi, i) => (cfg[vi] = combo[i]));
      const e = evalFn(cfg);
      runs++;
      for (const k of keys) tables.get(k).rows.set(combo.join(","), e.has(k) ? e.get(k) : "absent");
    }
  }
  log(`${label}: ${tables.size} keys, ${byDeps.size} dependency sets, ${runs} enumeration runs`);
  return tables;
}

function lookup(tables, key, cfg) {
  const t = tables.get(key);
  if (!t) return "absent";
  const v = t.rows.get(t.deps.map((vi) => cfg[vi]).join(","));
  return v === undefined ? "absent" : v;
}

// Expand {ref} templates using the part tables (with per-mapping overrides).
function expand(tmplStr, id, cfg, tables) {
  const expandT = (t) => {
    if (t.ref !== undefined) {
      const o = lookup(tables, `O|${id}|${t.ref}`, cfg);
      const p = o !== "absent" ? o : lookup(tables, `P|${t.ref}`, cfg);
      return p === "absent" ? { missing: true } : JSON.parse(p);
    }
    if (t.group) return { group: t.group.map(expandT) };
    if (t.draw) return Object.assign({}, t, { draw: expandT(t.draw) });
    return t;
  };
  return JSON.stringify(expandT(JSON.parse(tmplStr)));
}

function main() {
  const t0 = Date.now();
  const log = (...a) => console.error(`[${((Date.now() - t0) / 1000).toFixed(1)}s]`, ...a);
  const baselines = Number(process.env.BASELINES || 160);
  const checks = Number(process.env.CHECKS || 300);
  const rand = mulberry32(0x5eed);

  NUMBER_SETS = discoverNumberSets();
  log("number symbol sets:", NUMBER_SETS.join(","));
  OVERRIDE_SETS = discoverOverrides(rand);
  log("mapping-mutated parts:", JSON.stringify([...OVERRIDE_SETS]));

  // Parts (and overrides) depend on the full context.
  const partTables = buildTables(evalParts, baselines, rand, log, "parts");

  // Mappings: evaluate with references; they must depend on std2525/edition only.
  const mapTables = new Map();
  for (const id of mappingIds()) {
    const t = buildTables((cfg) => evalMapping(id, cfg), Math.max(8, baselines / 10), rand, log, `map ${id}`);
    for (const [k, v] of t) {
      for (const vi of v.deps)
        if (!["std2525", "edition"].includes(VARS[vi].name)) throw new Error(`mapping ${k} depends on ${VARS[vi].name}`);
      mapTables.set(k, v);
    }
  }

  // Round trip: fully inlined upstream evaluation vs table reconstruction.
  const rand2 = mulberry32(0xc0ffee);
  let bad = 0;
  let compared = 0;
  for (let i = 0; i < checks; i++) {
    const cfg = randomCfg(rand2);
    const parts = evalParts(cfg);
    for (const [k, v] of parts) {
      compared++;
      if (lookup(partTables, k, cfg) !== v) {
        if (bad++ < 20) console.error("PART MISMATCH", k, JSON.stringify(cfg));
      }
    }
    const ids = mappingIds();
    const id = ids[i % ids.length];
    const real = evalMapping(id, cfg, true);
    const keys = new Set([...real.keys()].concat([...mapTables.keys()].filter((k) => k.startsWith(id + "|"))));
    for (const k of keys) {
      compared++;
      const t = lookup(mapTables, k, cfg);
      const got = t === "absent" ? "absent" : expand(t, id, cfg, partTables);
      const want = real.has(k) ? real.get(k) : "absent";
      if (got !== want) {
        if (bad++ < 20) console.error("MAPPING MISMATCH", k, JSON.stringify(cfg), "\n got ", got.slice(0, 300), "\n want", want.slice(0, 300));
      }
    }
  }
  if (bad) {
    console.error(`round-trip mismatches: ${bad} of ${compared}`);
    process.exit(1);
  }
  log(`round-trip check passed: ${compared} comparisons over ${checks} contexts`);

  const serial = (tables) =>
    [...tables.entries()]
      .sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0))
      .map(([key, t]) => ({ key, deps: t.deps, rows: [...t.rows.entries()] }));
  fs.mkdirSync(outDir, { recursive: true });
  const json = {
    upstream: JSON.parse(fs.readFileSync(path.join(here, "..", "oracle", "pin.json"), "utf8")),
    vars: VARS.map((v) => ({
      name: v.name,
      domain: v.domain.map((x) =>
        x === undefined ? null : x === MONO_TOKEN ? "$mono" : x === DASH_PENDING ? "$pending" : x === DASH_ANTICIPATED ? "$anticipated" : x
      ),
    })),
    numberSets: NUMBER_SETS,
    overrides: [...OVERRIDE_SETS],
    parts: serial(partTables),
    mappings: serial(mapTables),
  };
  fs.writeFileSync(path.join(outDir, "tables.json"), JSON.stringify(json));
  log(`wrote ${json.parts.length} part entries, ${json.mappings.length} mapping entries`);
}

main();
