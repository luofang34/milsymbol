#!/usr/bin/env node
// Extracts the SIDC code tables of MIL-STD-2525E w/Change 1 (Appendix A) from the PDF.
//
// usage: node tools/standards/mil2525e1.mjs <pdf> <outdir>
//
// The PDF is only ever read through poppler's `pdftotext -tsv` (word boxes).
// Table rows are rebuilt from word geometry: each code token is a row anchor and
// the name/remarks cell lines are assigned to anchors by a monotone minimum-cost
// partition. Anything that cannot be classified aborts the run.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const LEFT_MARGIN = 72;
const FOOTER_TOP = 715;
const HEADER_TOP = 45;
const LINE_TOL = 2.5;
const MAX_CELL_GAP = 13.2;
const MAX_GROUP_LINES = 8;
const MAX_ANCHOR_DEVIATION = 4;

class ExtractError extends Error {}
const fail = (msg) => {
  throw new ExtractError(msg);
};

function readWords(pdf) {
  const out = execFileSync("pdftotext", ["-tsv", pdf, "-"], {
    encoding: "utf8",
    maxBuffer: 1 << 30,
  });
  const pages = new Map();
  for (const line of out.split("\n").slice(1)) {
    if (line === "") continue;
    const f = line.split("\t");
    if (f.length < 12) fail(`malformed tsv row: ${line}`);
    const level = Number(f[0]);
    const page = Number(f[1]);
    if (level === 1) {
      pages.set(page, { page, words: [] });
    } else if (level === 5) {
      const text = f.slice(11).join("\t").replace(/ /g, " ").trim();
      if (text === "") continue;
      const l = Number(f[6]);
      const t = Number(f[7]);
      const w = Number(f[8]);
      pages.get(page).words.push({ l, t, r: l + w, cx: l + w / 2, s: text });
    }
  }
  return pages;
}

function groupLines(words, tol = LINE_TOL) {
  const sorted = [...words].sort((a, b) => a.t - b.t || a.l - b.l);
  const lines = [];
  for (const w of sorted) {
    const cur = lines[lines.length - 1];
    if (cur && w.t - cur.t <= tol) {
      cur.words.push(w);
    } else {
      lines.push({ t: w.t, words: [w] });
    }
  }
  for (const ln of lines) {
    ln.words.sort((a, b) => a.l - b.l);
    ln.text = ln.words.map((w) => w.s).join(" ");
  }
  return lines;
}

function cellsOf(line, gap = 12) {
  const cells = [];
  for (const w of line.words) {
    const c = cells[cells.length - 1];
    if (c && w.l - c.r <= gap) {
      c.s += " " + w.s;
      c.r = w.r;
    } else {
      cells.push({ s: w.s, l: w.l, r: w.r });
    }
  }
  for (const c of cells) c.cx = (c.l + c.r) / 2;
  return cells;
}

const joinLines = (lines) => lines.map((l) => l.text).join(" ").replace(/\s+/g, " ").trim();

// Monotone partition of cell `lines` (sorted by top) over row `anchors`
// (sorted by top): contiguous groups, one per anchor, minimising the distance
// between each group's vertical centre and its anchor.
function partition(lines, anchors, ctx) {
  const n = lines.length;
  const m = anchors.length;
  if (m === 0 && n === 0) return [];
  if (n < m) fail(`${ctx}: ${m} codes but only ${n} name lines`);
  if (m === 0) fail(`${ctx}: ${n} name lines without a code: ${lines[0].text}`);
  const INF = Infinity;
  const dp = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(INF));
  const back = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
  dp[0][0] = 0;
  for (let j = 1; j <= m; j++) {
    for (let i = j; i <= n; i++) {
      for (let k = 1; k <= MAX_GROUP_LINES && i - k >= j - 1; k++) {
        const a = i - k;
        if (dp[a][j - 1] === INF) continue;
        let ok = true;
        for (let q = a + 1; q < i; q++) {
          if (lines[q].t - lines[q - 1].t > MAX_CELL_GAP) ok = false;
        }
        if (!ok) continue;
        const centre = (lines[a].t + lines[i - 1].t) / 2;
        const cost = dp[a][j - 1] + Math.abs(centre - anchors[j - 1].t);
        if (cost < dp[i][j]) {
          dp[i][j] = cost;
          back[i][j] = a;
        }
      }
    }
  }
  if (dp[n][m] === INF) fail(`${ctx}: cannot partition ${n} lines over ${m} codes`);
  const groups = new Array(m);
  let i = n;
  for (let j = m; j >= 1; j--) {
    const a = back[i][j];
    groups[j - 1] = lines.slice(a, i);
    i = a;
  }
  groups.forEach((g, idx) => {
    const centre = (g[0].t + g[g.length - 1].t) / 2;
    const dev = Math.abs(centre - anchors[idx].t);
    if (dev > (g[g.length - 1].t - g[0].t) / 2 + MAX_ANCHOR_DEVIATION) {
      fail(`${ctx}: group "${joinLines(g)}" is ${dev.toFixed(1)}pt from code ${anchors[idx].s}`);
    }
  });
  return groups;
}

// Assign (possibly absent) cell lines to the nearest group band.
function attachSparse(lines, anchors, groups, ctx) {
  const result = anchors.map(() => []);
  for (const ln of lines) {
    let best = -1;
    let bestD = Infinity;
    let tie = false;
    anchors.forEach((a, idx) => {
      const g = groups[idx];
      const lo = Math.min(g[0].t, a.t) - 6;
      const hi = Math.max(g[g.length - 1].t, a.t) + 6;
      if (ln.t < lo || ln.t > hi) return;
      const d = Math.abs(ln.t - (g[0].t + g[g.length - 1].t) / 2);
      if (d < bestD - 0.01) {
        best = idx;
        bestD = d;
        tie = false;
      } else if (Math.abs(d - bestD) <= 0.01) {
        tie = true;
      }
    });
    if (best < 0 || tie) fail(`${ctx}: cannot place remarks line "${ln.text}"`);
    result[best].push(ln);
  }
  return result;
}

const ROMAN = /^A-[IVXLC]+\.?$/;

function classifyCaption(id, text) {
  const t = text.toLowerCase();
  if (id === "A-III" || id === "A-V") return { kind: "skip" };
  const fixed = {
    "A-I": "version",
    "A-II": "standard_identity",
    "A-IV": "symbol_set",
    "A-VI": "status",
    "A-VII": "hq_task_force_dummy",
    "A-VIII": "amplifying_descriptor",
    "A-LX": "frame_shape",
  };
  if (fixed[id]) return { kind: "header", field: fixed[id] };
  if (/sector 1 common modifier/.test(t)) return { kind: "common_m1" };
  if (/sector 2 common modifier/.test(t)) return { kind: "common_m2" };
  if (/sector 1 modifier/.test(t)) return { kind: "m1" };
  if (/sector 2 modifier/.test(t)) return { kind: "m2" };
  if (/entity\/entity type\/entity( subtype)?/.test(t)) return { kind: "entity" };
  if (/^table a-xxxviii\.\s+dismounted individual/.test(t)) return { kind: "entity" };
  return fail(`unclassifiable table caption ${id}: ${text}`);
}

// Splits a page into ordered events: headings, captions and body words.
function pageEvents(pg) {
  const words = pg.words.filter((w) => w.t > HEADER_TOP && w.t < FOOTER_TOP);
  const lines = groupLines(words);
  const events = [];
  for (let i = 0; i < lines.length; i++) {
    const ln = lines[i];
    const w0 = ln.words[0];
    const w1 = ln.words[1];
    if (w0.s === "TABLE" && w1 && ROMAN.test(w1.s)) {
      let text = ln.text;
      const next = lines[i + 1];
      if (
        /entity\/entity type\/entity$/.test(text) &&
        next &&
        next.t - ln.t < 20 &&
        /^subtype/.test(next.text)
      ) {
        text += " " + next.text;
        i++;
      }
      events.push({ type: "caption", t: ln.t, id: w1.s.replace(/\.$/, ""), text });
    } else if (/^A\.\d+(\.\d+)*$/.test(w0.s) && w0.l < 140) {
      events.push({ type: "heading", t: ln.t, first: w0.s, text: ln.text });
    }
  }
  return { words, lines, events };
}

function printedPage(pg) {
  const nums = pg.words.filter(
    (w) => w.t >= FOOTER_TOP && w.t < 745 && /^\d+$/.test(w.s),
  );
  if (nums.length !== 1) return null;
  return nums[0].s;
}

function entityColumns(headerLine, ctx) {
  const groups = [];
  for (const w of headerLine.words) {
    if (w.s === "Entity" || w.s === "Geometric" || w.s === "Value") {
      groups.push({ first: w.s, l: w.l, r: w.r, words: [w.s] });
    } else if (groups.length) {
      const g = groups[groups.length - 1];
      g.r = w.r;
      g.words.push(w.s);
    }
  }
  const names = groups.map((g) => g.words.join(" "));
  const hasGeom = names.includes("Geometric");
  const expect = hasGeom
    ? ["Entity", "Entity Type", "Entity Subtype", "Geometric", "Value"]
    : ["Entity", "Entity Type", "Entity Subtype", "Value"];
  if (names.join("|") !== expect.join("|")) {
    fail(`${ctx}: unexpected entity header ${names.join("|")}`);
  }
  const centres = groups.map((g) => (g.l + g.r) / 2);
  const bounds = [];
  let edge = LEFT_MARGIN;
  for (let i = 0; i < centres.length - 1; i++) {
    edge = 2 * centres[i] - edge;
    bounds.push(edge);
  }
  return { hasGeom, bounds, valueCx: centres[centres.length - 1] };
}

function levelOf(code) {
  if (code.slice(4) !== "00") return 2;
  if (code.slice(2, 4) !== "00") return 1;
  return 0;
}

function parseEntityBody(words, hdr, page, ctx, emit) {
  const { hasGeom, bounds, valueCx } = hdr;
  const cols = [[], [], [], []];
  const values = [];
  const geom = [];
  for (const w of words) {
    if (/^\d{6}$/.test(w.s) && Math.abs(w.cx - valueCx) < 45) {
      values.push(w);
      continue;
    }
    const idx = bounds.findIndex((b) => w.cx < b);
    if (idx < 0) fail(`${ctx}: word right of value column: ${w.s}`);
    if (idx === 3) {
      if (!hasGeom) fail(`${ctx}: unexpected word "${w.s}"`);
      geom.push(w);
    } else {
      if (hasGeom && idx === 2 && w.r > bounds[2] + 1) fail(`${ctx}: word straddles column: ${w.s}`);
      cols[idx].push(w);
    }
  }
  values.sort((a, b) => a.t - b.t);
  const colLines = cols.map((c) => groupLines(c));
  const byLevel = [[], [], []];
  for (const v of values) byLevel[levelOf(v.s)].push(v);
  const rows = new Map();
  for (let lvl = 0; lvl < 3; lvl++) {
    const groups = partition(colLines[lvl], byLevel[lvl], `${ctx} level ${lvl}`);
    byLevel[lvl].forEach((v, i) => rows.set(v, { name: joinLines(groups[i]) }));
  }
  // Remaining columns must be empty for rows of other levels: any line in the
  // wrong column for its row would have failed partitioning above.
  const geomLines = groupLines(geom);
  const geomOf = new Map();
  if (hasGeom) {
    const near = (ln) => {
      let best = null;
      let bd = Infinity;
      for (const v of values) {
        const d = Math.abs(v.t - ln.t);
        if (d < bd) {
          bd = d;
          best = v;
        }
      }
      if (!best || bd > 14) fail(`${ctx}: geometry "${ln.text}" has no code`);
      return best;
    };
    for (const ln of geomLines) {
      const v = near(ln);
      if (geomOf.has(v)) fail(`${ctx}: two geometry cells for ${v.s}`);
      geomOf.set(v, ln);
    }
  }
  for (const v of values) {
    const r = rows.get(v);
    if (r.name === "") fail(`${ctx}: empty name for ${v.s}`);
    emit({
      code: v.s,
      name: r.name,
      icon: !hasGeom || geomOf.has(v) ? "yes" : "no",
      remarks: "",
      page,
    });
  }
}

function parseModifierBody(lines, hdrLine, common, page, ctx, emit) {
  const hw = hdrLine.words;
  const catIdx = hw.findIndex((w) => w.s === "Category" || w.s === "Capability");
  const valIdx = hw.findIndex((w) => w.s === "Value");
  const remIdx = hw.findIndex((w) => w.s === "Remarks");
  if (catIdx < 1 || valIdx < catIdx || remIdx < valIdx) fail(`${ctx}: bad modifier header ${hdrLine.text}`);
  const nameC = (hw[0].l + hw[catIdx - 1].r) / 2;
  const nameRight = 2 * nameC - LEFT_MARGIN;
  const valCx = hw[valIdx].cx ?? (hw[valIdx].l + hw[valIdx].r) / 2;
  const codeRe = common ? /^\d{3}$/ : /^[0-9A-Z]{2}$/;
  const name = [];
  const remarks = [];
  const values = [];
  for (const ln of lines) {
    for (const w of ln.words) {
      if (w.cx < nameRight) name.push(w);
      else if (w.cx < valCx - 26) continue;
      else if (Math.abs(w.cx - valCx) <= 26 && codeRe.test(w.s)) values.push(w);
      else if (w.cx > valCx + 15) remarks.push(w);
      else fail(`${ctx}: unplaceable word "${w.s}" at x=${w.cx.toFixed(0)}`);
    }
  }
  values.sort((a, b) => a.t - b.t);
  const nameLines = groupLines(name);
  const groups = partition(nameLines, values, ctx);
  const remLines = groupLines(remarks);
  const rem = attachSparse(remLines, values, groups, ctx);
  values.forEach((v, i) => {
    const nm = joinLines(groups[i]);
    if (nm === "") fail(`${ctx}: empty name for ${v.s}`);
    emit({ code: v.s, name: nm, icon: "yes", remarks: joinLines(rem[i]), page });
  });
}

function parseHeaderTable(field, lines, page, ctx, st, emit) {
  for (const ln of lines) {
    const first = ln.words[0].s;
    if (first === "Note:") return true;
    if (first === "Value" || first === "Description") {
      if (field === "amplifying_descriptor") {
        const w1 = ln.words.find((w) => w.s === "1st");
        const w2 = ln.words.find((w) => w.s === "2nd" || w.s === "2d");
        if (w1 && w2) st.amp = { c1: w1.cx, c2: w2.cx };
      }
      continue;
    }
    const cells = cellsOf(ln);
    if (field === "version") {
      if (cells.length < 2 || !/^[0-9A-F]{2}(-[0-9A-F]{2})?$/.test(cells[0].s)) fail(`${ctx}: bad version row ${ln.text}`);
      emit({ field, code: cells[0].s, name: cells[1].s, page });
    } else if (field === "standard_identity") {
      if (cells.length === 1 && (cells[0].s === "Context" || cells[0].s === "Standard Identity")) {
        st.si = cells[0].s === "Context" ? "context" : "standard_identity";
        continue;
      }
      if (cells.length !== 2 || !/^\d$/.test(cells[1].s) || !st.si) fail(`${ctx}: bad row ${ln.text}`);
      emit({ field: st.si, code: cells[1].s, name: cells[0].s, page });
    } else if (field === "symbol_set") {
      if (cells.length !== 2 || !/^\d\d$/.test(cells[1].s)) fail(`${ctx}: bad row ${ln.text}`);
      emit({ field, code: cells[1].s, name: cells[0].s, page });
    } else if (field === "status" || field === "hq_task_force_dummy" || field === "frame_shape") {
      if (cells.length !== 2 || !/^[0-9A-Z]$/.test(cells[1].s)) fail(`${ctx}: bad row ${ln.text}`);
      emit({ field, code: cells[1].s, name: cells[0].s, page });
    } else if (field === "amplifying_descriptor") {
      if (!st.amp) fail(`${ctx}: amplifier columns unknown`);
      const mid = (st.amp.c1 + st.amp.c2) / 2;
      const codes = cells.slice(1);
      if (cells.length < 2 || codes.some((c) => !/^\d$/.test(c.s))) fail(`${ctx}: bad row ${ln.text}`);
      if (codes.length === 2 && codes[0].cx < mid && codes[1].cx >= mid) {
        emit({ field: "amplifying_descriptor", code: codes[0].s + codes[1].s, name: cells[0].s, page });
      } else if (codes.length === 1 && codes[0].cx < mid) {
        st.group = codes[0].s;
        emit({ field: "amplifying_descriptor_group", code: st.group, name: cells[0].s, page });
      } else if (codes.length === 1 && codes[0].cx >= mid && st.group !== undefined) {
        emit({ field: "amplifying_descriptor", code: st.group + codes[0].s, name: cells[0].s, page });
      } else {
        fail(`${ctx}: bad amplifier row ${ln.text}`);
      }
    }
  }
  return false;
}

function extract(pdf) {
  const pages = readWords(pdf);
  const nums = [...pages.keys()].sort((a, b) => a - b);
  const codes = [];
  const headers = [];
  let started = false;
  let curSet = null;
  let table = null;
  const hst = {};
  for (const pn of nums) {
    const pg = pages.get(pn);
    const head = groupLines(pg.words.filter((w) => w.t <= HEADER_TOP)).map((l) => l.text).join(" ");
    if (!started) {
      if (!/Appendix A$/.test(head)) continue;
      if (!pg.words.some((w) => w.s === "A-I.")) continue;
      started = true;
    }
    if (!/Appendix A$/.test(head)) {
      if (/Appendix [B-Z]$/.test(head)) break;
      if (head === "") continue;
      fail(`pdf page ${pn}: unexpected running header "${head}"`);
    }
    const printed = printedPage(pg);
    if (printed === null) fail(`pdf page ${pn}: no printed page number`);
    const { words, lines, events } = pageEvents(pg);
    events.sort((a, b) => a.t - b.t);
    // segment boundaries: [start, end) in top coordinate per event
    const segs = [{ start: -Infinity, ev: null }];
    for (const ev of events) segs.push({ start: ev.t, ev });
    for (let si = 0; si < segs.length; si++) {
      const seg = segs[si];
      const end = si + 1 < segs.length ? segs[si + 1].start : Infinity;
      if (seg.ev?.type === "heading") {
        table = null;
        const m = seg.ev.first === "A.5.4.1" ? "common" : null;
        if (m) curSet = "common";
        else if (/^A\.5\.4\.\d+$/.test(seg.ev.first)) {
          const sm = /\((\d\d)\)\.?$/.exec(seg.ev.text);
          if (!sm) fail(`pdf page ${pn}: set heading without code: ${seg.ev.text}`);
          curSet = sm[1];
        }
        continue;
      }
      if (seg.ev?.type === "caption") {
        const c = classifyCaption(seg.ev.id, seg.ev.text);
        table = { ...c, id: seg.ev.id, set: curSet, st: hst[seg.ev.id] ?? (hst[seg.ev.id] = {}), done: false };
        if (c.kind === "common_m1" || c.kind === "common_m2") table.set = "common";
        else if (["entity", "m1", "m2"].includes(c.kind) && !/^\d\d$/.test(curSet ?? "")) {
          fail(`pdf page ${pn}: ${seg.ev.id} outside a symbol set section`);
        }
      }
      if (!table || table.kind === "skip") continue;
      const segLines = lines.filter((l) => l.t >= (seg.ev ? seg.start + 1 : seg.start) && l.t < end);
      const ctx = `pdf page ${pn} (printed ${printed}) ${table.id}`;
      if (!seg.ev) {
        if (segLines.length) fail(`${ctx}: table body at page top without caption`);
        continue;
      }
      // drop caption lines themselves
      const body = segLines.filter(
        (l) => !(l.words[0].s === "TABLE" && ROMAN.test(l.words[1]?.s ?? "")) && !/^subtype\s*\.?$/.test(l.text),
      );
      const rec = (r) => {
        if (r.field) {
          headers.push({ ...r, table: table.id, printed });
        } else {
          codes.push({ symbol_set: table.set, kind: table.kind, ...r, page: printed, table: table.id });
        }
      };
      if (table.kind === "header") {
        if (table.done) continue;
        if (parseHeaderTable(table.field, body, printed, ctx, table.st, rec)) table.done = true;
      } else if (table.kind === "entity") {
        if (body.length === 0) continue;
        const noteIdx = body.findIndex((l) => l.words[0].s === "Note:");
        const hdrIdx = body.findIndex((l) => l.words[0].s === "Entity");
        if (hdrIdx < 0 || noteIdx < 0) fail(`${ctx}: entity header not found`);
        const hdr = entityColumns(body[hdrIdx], ctx);
        const bodyTop = body[noteIdx].t + 5;
        const ws = words.filter((w) => w.t >= bodyTop && w.t >= seg.start && w.t < end);
        parseEntityBody(ws, hdr, printed, ctx, (r) => rec(r));
      } else {
        if (body.length === 0) continue;
        const hdrIdx = body.findIndex((l) => l.words.some((w) => w.s === "Value") && l.words.some((w) => w.s === "Remarks"));
        if (hdrIdx > 0) fail(`${ctx}: modifier header not at table top`);
        if (hdrIdx === 0) table.st.hdr = body[0];
        if (!table.st.hdr) fail(`${ctx}: modifier header not found`);
        const common = table.kind.startsWith("common");
        parseModifierBody(hdrIdx === 0 ? body.slice(1) : body, table.st.hdr, common, printed, ctx, (r) => rec(r));
      }
    }
  }
  if (!started) fail("Appendix A not found");
  return { codes, headers };
}

function tsv(rows, cols) {
  const lines = [cols.join("\t")];
  for (const r of rows) {
    const vals = cols.map((c) => String(r[c]));
    for (const v of vals) if (/[\t\n\r]/.test(v)) fail(`control character in field: ${v}`);
    lines.push(vals.join("\t"));
  }
  return lines.join("\n") + "\n";
}

function main() {
  const [pdf, outdir] = process.argv.slice(2);
  if (!pdf || !outdir || process.argv.length !== 4) {
    process.stderr.write("usage: node tools/standards/mil2525e1.mjs <pdf> <outdir>\n");
    process.exit(2);
  }
  const { codes, headers } = extract(pdf);
  const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
  codes.sort(
    (a, b) => cmp(a.symbol_set, b.symbol_set) || cmp(a.kind, b.kind) || cmp(a.code, b.code),
  );
  mkdirSync(outdir, { recursive: true });
  writeFileSync(
    join(outdir, "codes.tsv"),
    tsv(codes, ["symbol_set", "kind", "code", "name", "icon", "remarks", "page", "table"]),
  );
  writeFileSync(
    join(outdir, "headers.tsv"),
    tsv(
      headers.map((h) => ({ ...h, page: h.printed })),
      ["field", "code", "name", "page", "table"],
    ),
  );
  const counts = new Map();
  for (const c of codes) {
    const k = `${c.symbol_set}\t${c.kind}`;
    counts.set(k, (counts.get(k) ?? 0) + 1);
  }
  for (const [k, n] of counts) process.stderr.write(`${k}\t${n}\n`);
}

try {
  main();
} catch (e) {
  if (e instanceof ExtractError) {
    process.stderr.write(`error: ${e.message}\n`);
    process.exit(1);
  }
  throw e;
}
