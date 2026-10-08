#!/usr/bin/env node
// Extracts the SIDC code tables (Annex A) of NATO APP-6(E) Version 2 from the
// published PDF using poppler's `pdftotext -tsv` word boxes.
//
// Usage: node tools/standards/app6e2.mjs <pdf> <outdir>
//
// Writes <outdir>/codes.tsv and <outdir>/headers.tsv. Any construct that cannot
// be classified aborts with a non-zero exit status instead of being dropped.

import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

class ExtractError extends Error {}
const fail = (msg) => {
  throw new ExtractError(msg);
};

const LINE_TOL = 2.5;
const MAX_ANCHOR_DEV = 7;
const COL_PENALTY = 1e7;

// Entity rows whose name sits in a column other than the one implied by the
// code (verified against the PDF text); keyed by `table/code/printed page`.
const KNOWN_COLUMN_ANOMALIES = new Set(['A-32/151204/787']);

// Table ids that carry code data. Kind is derived from the caption text.
const ENTITY_RE = /Entity\/Entity/;
const SIMPLE_TABLES = {
  1: 'version',
  2: 'context',
  4: 'symbol_set',
  6: 'status',
  7: 'hq_tf_dummy',
  8: 'amplifier',
};
const FIRST_CODE_TABLE = 11;
const LAST_CODE_TABLE = 54;

// Cyberspace sector modifier tables carry no set heading of their own (they
// follow the "(25)" heading of Table A-50). Chapter 9 of the same document
// (Tables 9-10 and 9-11, "Symbol Set Code: 60") assigns them to set 60.
const SET_OVERRIDE = { 51: '60', 52: '60' };

const NOTE_VOCAB = new Set(
  'note: the actual code is shown in bold numbers. remaining numbers are used to show placement within six digits.'.split(' '),
);

// Entity rows whose Remarks mark them as hierarchy-only.
const NO_ICON_RE = /no associated (symbol|icon)|no icon|hierarchical (purposes|explanation)/i;

// Prose rows of Table A-50 (the "Actions" row has no code); matched as whole
// visual lines.
const NON_CODE_LINES = [
  /^Cyberspace actions use Mission Task and$/,
  /^Actions Control Measure SIDCs\.$/,
  /^See respective tables for Symbol Set and SIDCs\.$/,
];

function readWords(pdf) {
  const out = execFileSync('pdftotext', ['-tsv', pdf, '-'], {
    maxBuffer: 1 << 30,
    encoding: 'utf8',
  });
  const pages = new Map();
  const lines = out.split('\n');
  for (let i = 1; i < lines.length; i++) {
    const f = lines[i].split('\t');
    if (f.length < 12 || f[0] !== '5') continue;
    const text = f.slice(11).join('\t').replace(/ /g, ' ').trim();
    if (text === '') continue;
    const page = Number(f[1]);
    const x = Number(f[6]);
    const y = Number(f[7]);
    const w = Number(f[8]);
    const h = Number(f[9]);
    if (![page, x, y, w, h].every(Number.isFinite)) fail(`bad tsv line ${i}`);
    if (!pages.has(page)) pages.set(page, []);
    pages.get(page).push({ page, x, y, w, h, r: x + w, b: y + h, yc: y + h / 2, t: text });
  }
  return pages;
}

function clusterLines(words, tol = LINE_TOL) {
  const sorted = [...words].sort((a, b) => a.yc - b.yc || a.x - b.x);
  const lines = [];
  for (const w of sorted) {
    const last = lines[lines.length - 1];
    if (last && Math.abs(w.yc - last.ws[last.ws.length - 1].yc) <= tol) last.ws.push(w);
    else lines.push({ ws: [w] });
  }
  for (const l of lines) {
    l.ws.sort((a, b) => a.x - b.x);
    l.text = l.ws.map((w) => w.t).join(' ');
    l.x0 = l.ws[0].x;
    l.top = Math.min(...l.ws.map((w) => w.y));
    l.bottom = Math.max(...l.ws.map((w) => w.b));
    l.yc = (l.top + l.bottom) / 2;
  }
  return lines;
}

function pageFooter(words) {
  const eds = words.filter((w) => w.t === 'Edition').sort((a, b) => a.y - b.y);
  const ed = eds[eds.length - 1];
  if (!ed) return null;
  const num = words
    .filter((w) => Math.abs(w.yc - ed.yc) < 3 && w.r <= ed.x + 1 && /^\d{3}$/.test(w.t))
    .sort((a, b) => b.x - a.x)[0];
  if (!num) fail(`page ${ed.page}: footer without page number`);
  return { top: ed.y - 1, printed: Number(num.t) };
}

function findCaptions(words) {
  const caps = [];
  for (const l of clusterLines(words)) {
    const [a, b] = l.ws;
    if (!b || a.t !== 'Table') continue;
    const m = /^A-(\d+):$/.exec(b.t);
    if (m) caps.push({ id: Number(m[1]), top: l.top, bottom: l.bottom });
  }
  return caps;
}

function findHeaders(words) {
  const heads = [];
  for (const l of clusterLines(words)) {
    const texts = l.ws.map((w) => w.t);
    const hasCode = texts.includes('Code') || texts.includes('Digit');
    const hasLabel = texts.some((t) =>
      ['Description', 'Entity', 'First', 'Second'].includes(t),
    );
    if (hasCode && hasLabel) heads.push({ line: l, top: l.top, bottom: l.bottom });
  }
  return heads;
}

// Associates every header line with its table: the highest-numbered caption
// between the previous header and this one (closing captions of the previous
// table and hidden duplicate captions have lower numbers).
function segmentsOf(words) {
  const footer = pageFooter(words);
  if (!footer) return { footer: null, segments: [], caps: [] };
  const caps = findCaptions(words).sort((a, b) => a.top - b.top);
  const heads = findHeaders(words).sort((a, b) => a.top - b.top);
  const segments = [];
  let prevTop = -Infinity;
  for (const h of heads) {
    const above = caps.filter((c) => c.top < h.top && c.top > prevTop);
    prevTop = h.top;
    if (above.length === 0) continue;
    const cap = above.reduce((a, b) => (b.id > a.id ? b : a));
    const isSimple = cap.id in SIMPLE_TABLES;
    if (!isSimple && (cap.id < FIRST_CODE_TABLE || cap.id > LAST_CODE_TABLE)) continue;
    const below = caps.filter((c) => c.top > h.top).map((c) => c.top);
    const bound = Math.min(footer.top, ...below);
    const captionText = words
      .filter((w) => w.yc > cap.top - 1 && w.b < h.top)
      .sort((a, b) => a.yc - b.yc || a.x - b.x)
      .map((w) => w.t)
      .join(' ');
    segments.push({ id: cap.id, capTop: cap.top, head: h, bound, captionText });
  }
  return { footer, segments, caps };
}

function entityOrModKind(seg) {
  const t = seg.captionText;
  if (ENTITY_RE.test(t)) return 'entity';
  if (/Common Modifier Sector 1/.test(t)) return 'common_m1';
  if (/Common Modifier Sector 2/.test(t)) return 'common_m2';
  if (/Sector 1 Modifier/.test(t)) return 'm1';
  if (/Sector 2 Modifier/.test(t)) return 'm2';
  return fail(`table A-${seg.id}: cannot classify caption "${t}"`);
}

function blockOffset(block, anchor, topAligned) {
  if (topAligned) return block[0].yc - anchor.yc;
  return (block[0].top + block[block.length - 1].bottom) / 2 - anchor.yc;
}

// Minimal-cost contiguous assignment of text lines to anchored rows. Row
// content is vertically centred on its anchor in the source layout.
function assignLines(lines, anchors, { allowEmpty, ctx, topAligned = false, maxPerBlock = Infinity, maxDev = MAX_ANCHOR_DEV }) {
  const n = lines.length;
  const m = anchors.length;
  const INF = Infinity;
  const cost = (j, k, i) => {
    if (k === i) return allowEmpty ? 0 : INF;
    if (i - k > maxPerBlock) return INF;
    const col = lines[k].col;
    for (let q = k; q < i; q++) if (lines[q].col !== col) return INF;
    const colPenalty = anchors[j].col !== undefined && col !== anchors[j].col ? COL_PENALTY : 0;
    const d = blockOffset(lines.slice(k, i), anchors[j], topAligned);
    return d * d + colPenalty;
  };
  const f = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(INF));
  const from = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(-1));
  f[0][0] = 0;
  for (let j = 1; j <= m; j++) {
    for (let i = 0; i <= n; i++) {
      for (let k = 0; k <= i; k++) {
        if (f[j - 1][k] === INF) continue;
        const c = f[j - 1][k] + cost(j - 1, k, i);
        if (c < f[j][i]) {
          f[j][i] = c;
          from[j][i] = k;
        }
      }
    }
  }
  if (f[m][n] === INF) {
    if (process.env.APP6_DEBUG) {
      process.stderr.write(lines.map((l) => `L ${l.col} ${l.top.toFixed(1)} ${l.text}\n`).join(''));
      process.stderr.write(anchors.map((a) => `A ${a.level} ${a.yc.toFixed(1)} ${a.text}\n`).join(''));
    }
    fail(`${ctx}: cannot assign ${n} text lines to ${m} rows`);
  }
  const blocks = new Array(m);
  let i = n;
  for (let j = m; j >= 1; j--) {
    const k = from[j][i];
    blocks[j - 1] = lines.slice(k, i);
    i = k;
  }
  blocks.forEach((b, j) => {
    if (b.length === 0) return;
    const d = blockOffset(b, anchors[j], topAligned);
    if (process.env.APP6_DEBUG && Math.abs(d) > 6) {
      process.stderr.write(`DEV ${ctx} ${anchors[j].text} ${d.toFixed(1)} ${b.map((l) => l.text).join(' | ')}\n`);
    }
    if (Math.abs(d) > maxDev) {
      fail(`${ctx}: row ${anchors[j].text} text off-centre by ${d.toFixed(1)}pt: ${b.map((l) => l.text).join(' | ')}`);
    }
  });
  return blocks;
}

// Groups consecutive lines of one column into paragraphs: wrapped lines of one
// cell are a fixed 1.25 line-heights apart, while neighbouring rows are
// separated by additional cell padding.
function paragraphs(lines) {
  const out = [];
  for (const l of lines) {
    const last = out[out.length - 1];
    const prev = last && last.lines[last.lines.length - 1];
    const h = l.bottom - l.top;
    if (prev && prev.col === l.col && l.top - prev.top <= 1.25 * h + 0.4) last.lines.push(l);
    else out.push({ lines: [l] });
  }
  return out.map((p) => ({
    col: p.lines[0].col,
    colX: p.lines[0].colX,
    top: p.lines[0].top,
    bottom: p.lines[p.lines.length - 1].bottom,
    yc: (p.lines[0].top + p.lines[p.lines.length - 1].bottom) / 2,
    text: joinText(p.lines.map((l) => l.text)),
  }));
}

const clean = (s) => s.replace(/\s+/g, ' ').trim();

// Joins wrapped lines with a single space; a hard hyphen at the wrap point
// ("Multi-" / "Beam") is rejoined without a space.
const joinText = (parts) =>
  clean(parts.reduce((acc, p) => (acc === '' ? p : /\S-$/.test(acc) ? acc + p : `${acc} ${p}`), ''));

function levelOfCode(code) {
  if (code.slice(2) === '0000') return 0;
  if (code.slice(4) === '00') return 1;
  return 2;
}

function parseCodeSegment(seg, words, footer, set, rows) {
  const id = seg.id;
  const kind = entityOrModKind(seg);
  const ctx = `table A-${id} page ${footer.printed}`;
  const hl = seg.head.line;
  const code = hl.ws.find((w) => w.t === 'Code');
  const remarksHdr = hl.ws.find((w) => w.t.startsWith('Remarks'));
  if (!code || !remarksHdr) fail(`${ctx}: header lacks Code/Remarks`);
  let colBounds = [-Infinity];
  if (kind === 'entity') {
    const ents = hl.ws.filter((w) => w.t === 'Entity');
    if (ents.length !== 3) fail(`${ctx}: expected three Entity header columns`);
    // Header text is centred in its cell while names are left-aligned, so
    // columns are split halfway between the header centres.
    const typeW = hl.ws.find((w) => w.t === 'Type');
    const subW = hl.ws.find((w) => w.t === 'Subtype');
    if (!typeW || !subW) fail(`${ctx}: header lacks Type/Subtype`);
    const centres = [ents[0].x + ents[0].w / 2, (ents[1].x + typeW.r) / 2, (ents[2].x + subW.r) / 2];
    colBounds = [-Infinity, (centres[0] + centres[1]) / 2, (centres[1] + centres[2]) / 2];
  } else {
  }

  const inSeg = words.filter((w) => w.yc > seg.head.bottom && w.b <= seg.bound);
  const noteW = inSeg.find((w) => w.t === 'Note:' && w.yc - seg.head.bottom < 12);
  const codeLeft = Math.min(code.x - 5, noteW ? noteW.x - 3 : Infinity);
  const codeRe = kind === 'entity' ? /^[0-9]{6}$/ : kind.startsWith('common') ? /^[0-9A-F]{3}$/ : /^[0-9A-Z]{2}$/;
  const codeWords = inSeg
    .filter((w) => w.x >= code.x - 5 && w.x < remarksHdr.x - 3 && codeRe.test(w.t))
    .sort((a, b) => a.yc - b.yc);
  const boundary = codeWords.length ? Math.max(...codeWords.map((w) => w.r)) + 3 : remarksHdr.x - 3;

  // Header note box: only for entity tables, sits under the Code/Remarks headers.
  let cutoff = seg.head.bottom;
  const right = inSeg.filter((w) => w.x >= codeLeft && w.yc > seg.head.bottom);
  for (const l of clusterLines(right)) {
    if (l.ws.every((w) => NOTE_VOCAB.has(w.t.toLowerCase()))) cutoff = Math.max(cutoff, l.bottom);
    else break;
  }
  const proseWords = new Set(
    clusterLines(inSeg)
      .filter((l) => NON_CODE_LINES.some((re) => re.test(l.text)))
      .flatMap((l) => l.ws),
  );
  const body = inSeg.filter((w) => w.y > cutoff - 0.5 && !proseWords.has(w));

  const nameWords = body.filter((w) => w.x < codeLeft);
  const remarkWords = body.filter((w) => w.x >= boundary);
  const strays = body.filter(
    (w) => w.x >= codeLeft && w.x < boundary && !codeWords.includes(w),
  );
  if (strays.length) fail(`${ctx}: unclassified words in code column: ${strays.map((w) => w.t).join(' ')}`);

  let nameLines = clusterLines(nameWords);
  for (const l of nameLines) {
    l.col = 0;
  }
  if (kind === 'entity') {
    // Names are left-aligned in their column; columns are the distinct x0 clusters.
    const xs = [...new Set(nameLines.map((l) => l.x0))].sort((a, b) => a - b);
    const centres = [];
    for (const x of xs) {
      if (centres.length && x - centres[centres.length - 1] <= 3) continue;
      centres.push(x);
    }
    if (centres.length > 3) fail(`${ctx}: more than three name columns`);
    for (const l of nameLines) {
      l.col = centres.reduce((bi, c, i) => (Math.abs(c - l.x0) < Math.abs(centres[bi] - l.x0) ? i : bi), 0);
      l.colX = centres[l.col];
    }
  }
  const remarkLines = paragraphs(clusterLines(remarkWords).map((l) => ({ ...l, col: 0 })));
  const anchors = codeWords.map((w) => ({
    yc: w.yc,
    text: w.t,
    level: kind === 'entity' ? levelOfCode(w.t) : undefined,
  }));
  if (kind === 'entity') {
    const levels = [...new Set(anchors.map((a) => a.level))].sort((a, b) => a - b);
    const ncols = Math.max(...nameLines.map((l) => l.col)) + 1;
    if (levels.length !== ncols) {
      fail(`${ctx}: ${ncols} name columns but ${levels.length} code levels on the page`);
    }
    for (const a of anchors) a.col = levels.indexOf(a.level);
  }
  if (anchors.length === 0) fail(`${ctx}: no codes found`);
  const nameBlocks = assignLines(kind === 'entity' ? paragraphs(nameLines) : nameLines, anchors, {
    allowEmpty: false,
    maxDev: 400,
    ctx: `${ctx} names`,
  });
  const remarkAnchors = anchors.map((a) => ({ ...a, level: undefined, col: undefined }));
  const remarkBlocks = assignLines(remarkLines, remarkAnchors, {
    allowEmpty: true,
    maxPerBlock: 1,
    maxDev: 60,
    ctx: `${ctx} remarks`,
  });
  codeWords.forEach((w, j) => {
    const name = joinText(nameBlocks[j].map((l) => l.text));
    const remarks = joinText(remarkBlocks[j].map((l) => l.text));
    if (kind === 'entity' && nameBlocks[j][0].col !== anchors[j].col) {
      const key = `A-${id}/${w.t}/${footer.printed}`;
      if (!KNOWN_COLUMN_ANOMALIES.has(key)) fail(`${ctx}: code ${w.t} "${name}" is in an unexpected name column`);
    }
    rows.push({
      set,
      kind,
      code: w.t,
      name,
      icon: kind === 'entity' && NO_ICON_RE.test(remarks) ? 'no' : 'yes',
      remarks,
      page: footer.printed,
      table: `A-${id}`,
    });
  });
}

function parseSimpleSegment(seg, words, footer, state, hdrRows) {
  const id = seg.id;
  const field = SIMPLE_TABLES[id];
  const ctx = `table A-${id} page ${footer.printed}`;
  const hl = seg.head.line.ws;
  // Digit columns from "1st Digit"/"2nd Digit"/"Code" headers.
  const digitCols = [];
  for (let i = 0; i < hl.length; i++) {
    if (hl[i].t === 'Digit') {
      const prev = hl[i - 1];
      if (!prev || !/^(1st|2nd)$/.test(prev.t)) fail(`${ctx}: odd Digit header`);
      digitCols.push({ name: prev.t === '1st' ? 'd1' : 'd2', c: (prev.x + hl[i].r) / 2 });
    } else if (hl[i].t === 'Code') {
      digitCols.push({ name: 'code', c: (hl[i].x + hl[i].r) / 2 });
    }
  }
  if (digitCols.length === 0) fail(`${ctx}: no digit columns`);
  const prom = hl.find((w) => w.t === 'Promulgated');
  const desc = hl.find((w) => w.t === 'Description');
  const descCenter = desc ? (desc.x + desc.r) / 2 : 0;
  const promCut = prom ? (descCenter + (prom.x + prom.r) / 2) / 2 : Infinity;

  let body = words.filter(
    (w) => w.yc > seg.head.bottom && w.b <= seg.bound && w.h >= 8.5,
  );
  const noteLine = clusterLines(body).find((l) => /^Note\b/.test(l.text));
  if (noteLine) body = body.filter((w) => w.yc < noteLine.top);

  const anchorRe = field === 'symbol_set' ? /^\d\d$/ : /^\d$/;
  const anchorWords = body.filter((w) => {
    if (!anchorRe.test(w.t)) return false;
    if (!(id === 1 || id === 2 || id === 8)) return true;
    const c = (w.x + w.r) / 2;
    return digitCols.some((d) => Math.abs(d.c - c) < 14);
  });
  const anchorSet = new Set(anchorWords);
  const rowsA = [];
  for (const l of clusterLines(anchorWords)) {
    const r = { yc: l.yc, text: l.text };
    for (const w of l.ws) {
      const c = (w.x + w.r) / 2;
      const col = digitCols.reduce((a, b) => (Math.abs(b.c - c) < Math.abs(a.c - c) ? b : a));
      if (r[col.name] !== undefined) fail(`${ctx}: two ${col.name} cells in one row`);
      r[col.name] = w.t;
    }
    rowsA.push(r);
  }
  const descWords = body.filter((w) => !anchorSet.has(w) && (w.x + w.r) / 2 < promCut);
  let descLines = clusterLines(descWords).map((l) => ({ ...l, col: 0 }));
  // Sub-heading rows of Table A-2 carry no code.
  const subRows = [];
  if (id === 2) {
    descLines = descLines.filter((l) => {
      if (l.text === 'Context' || l.text === 'Standard Identity') {
        subRows.push(l);
        return false;
      }
      return true;
    });
  }
  const rows = rowsA;
  const blocks = assignLines(descLines, rows, { allowEmpty: false, ctx, topAligned: id === 8 });
  const nameOf = (j) => joinText(blocks[j].map((l) => l.text)).replace(/\*+$/, '');
  rows.forEach((r, j) => {
    const name = nameOf(j);
    if (id === 1) {
      if (r.d1 === undefined || r.d2 === undefined) fail(`${ctx}: incomplete version row`);
      hdrRows.push({ field, code: r.d1 + r.d2, name, page: footer.printed, table: `A-${id}` });
    } else if (id === 2) {
      let f;
      let code;
      if (r.d1 !== undefined && r.d2 === undefined) {
        f = 'context';
        code = r.d1;
      } else if (r.d2 !== undefined && r.d1 === undefined) {
        f = 'standard_identity';
        code = r.d2;
      } else fail(`${ctx}: ambiguous row ${r.text}`);
      hdrRows.push({ field: f, code, name, page: footer.printed, table: `A-${id}` });
    } else if (id === 8) {
      if (r.d1 !== undefined && r.d2 !== undefined) {
        if (r.d1 + r.d2 !== '00') fail(`${ctx}: unexpected two-cell row ${r.text}`);
        hdrRows.push({ field, code: '00', name, page: footer.printed, table: `A-${id}` });
      } else if (r.d1 !== undefined) {
        state.group = r.d1;
      } else {
        if (state.group === undefined) fail(`${ctx}: leaf row before group`);
        hdrRows.push({ field, code: state.group + r.d2, name, page: footer.printed, table: `A-${id}` });
      }
    } else {
      if (r.code === undefined) fail(`${ctx}: row without code ${r.text}`);
      hdrRows.push({ field, code: r.code, name, page: footer.printed, table: `A-${id}` });
    }
  });
}

function headingsOf(words, segments, footer) {
  const out = [];
  for (const l of clusterLines(words)) {
    const m = /^(\S.*) \((\d\d)\)$/.exec(l.text);
    if (!m || l.x0 > 100 || l.top >= footer.top) continue;
    if (segments.some((s) => l.top >= s.head.bottom && l.bottom <= s.bound)) continue;
    if (l.ws.some((w) => w.h < 8.5)) continue;
    out.push({ y: l.top, set: m[2], text: l.text });
  }
  return out;
}

function main() {
  const [pdf, outdir] = process.argv.slice(2);
  if (!pdf || !outdir || process.argv.length !== 4) {
    fail('usage: node tools/standards/app6e2.mjs <pdf> <outdir>');
  }
  const pages = readWords(pdf);
  const pageNums = [...pages.keys()].sort((a, b) => a - b);

  // Annex A starts at the first page whose caption is "Table A-1:" outside the
  // table of contents (which uses dot leaders).
  const rows = [];
  const hdrRows = [];
  const state = {};
  const seen = new Map();
  let started = false;
  let currentSet = null;
  let lastId = 0;
  for (const p of pageNums) {
    const words = pages.get(p);
    if (!started) {
      const hasLeader = words.some((w) => /\.{4,}/.test(w.t));
      const cap = findCaptions(words).find((c) => c.id === 1);
      if (!cap || hasLeader) continue;
      started = true;
    }
    const { footer, segments } = segmentsOf(words);
    if (!footer) continue;
    const heads = headingsOf(words, segments, footer);
    const evs = [
      ...heads.map((h) => ({ y: h.y, heading: h })),
      ...segments.map((s) => ({ y: s.head.top, seg: s })),
    ].sort((a, b) => a.y - b.y);
    for (const e of evs) {
      if (e.heading) {
        currentSet = e.heading.set;
        continue;
      }
      const seg = e.seg;
      if (!(seg.id in SIMPLE_TABLES)) {
        if (seg.id < lastId) fail(`page ${footer.printed}: table A-${seg.id} after A-${lastId}`);
        if (seg.id > lastId + 1 && seg.id !== FIRST_CODE_TABLE) {
          fail(`page ${footer.printed}: table A-${seg.id} follows A-${lastId} (gap)`);
        }
        lastId = seg.id;
      }
      seen.set(seg.id, (seen.get(seg.id) ?? 0) + 1);
      if (seg.id in SIMPLE_TABLES) {
        parseSimpleSegment(seg, words, footer, state, hdrRows);
      } else {
        const kind = entityOrModKind(seg);
        let set = kind.startsWith('common') ? 'common' : currentSet;
        if (seg.id in SET_OVERRIDE) set = SET_OVERRIDE[seg.id];
        if (!set) fail(`table A-${seg.id}: no symbol-set heading`);
        parseCodeSegment(seg, words, footer, set, rows);
      }
    }
  }
  for (const id of [...Object.keys(SIMPLE_TABLES).map(Number), ...range(FIRST_CODE_TABLE, LAST_CODE_TABLE)]) {
    if (!seen.has(id)) fail(`table A-${id} never found`);
  }

  // Table A-7 is also rendered as a two-row fragment on the page of Table A-2;
  // identical repeats are collapsed, keeping the last (full-table) occurrence.
  // Table A-1 legitimately lists code 10 twice (two different standards).
  const byKey = new Map();
  const kept = [];
  for (const r of hdrRows) {
    if (r.field === 'version') {
      kept.push(r);
      continue;
    }
    const k = `${r.field}|${r.code}`;
    const prev = byKey.get(k);
    if (prev && prev.name !== r.name) fail(`headers: conflicting duplicate ${k}: "${prev.name}" vs "${r.name}"`);
    if (prev) kept.splice(kept.indexOf(prev), 1);
    byKey.set(k, r);
    kept.push(r);
  }
  hdrRows.length = 0;
  hdrRows.push(...kept);

  const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
  rows.sort((a, b) => cmp(a.set, b.set) || cmp(a.kind, b.kind) || cmp(a.code, b.code));
  const FIELD_ORDER = ['version', 'context', 'standard_identity', 'symbol_set', 'status', 'hq_tf_dummy', 'amplifier'];
  hdrRows.sort((a, b) => FIELD_ORDER.indexOf(a.field) - FIELD_ORDER.indexOf(b.field) || cmp(a.code, b.code));

  const tsv = (cols, data) =>
    [cols.join('\t'), ...data.map((r) => cols.map((c) => String(r[c])).join('\t'))].join('\n') + '\n';
  for (const r of [...rows, ...hdrRows]) {
    for (const v of Object.values(r)) {
      if (/[\t\n\r]/.test(String(v))) fail(`control character in value ${JSON.stringify(v)}`);
    }
  }
  mkdirSync(outdir, { recursive: true });
  const codeCols = ['symbol_set', 'kind', 'code', 'name', 'icon', 'remarks', 'page', 'table'];
  writeFileSync(join(outdir, 'codes.tsv'), tsv(codeCols, rows.map((r) => ({ ...r, symbol_set: r.set }))));
  writeFileSync(join(outdir, 'headers.tsv'), tsv(['field', 'code', 'name', 'page', 'table'], hdrRows));
}

function range(a, b) {
  const r = [];
  for (let i = a; i <= b; i++) r.push(i);
  return r;
}

try {
  main();
} catch (e) {
  process.stderr.write(`app6e2: ${e instanceof ExtractError ? e.message : e.stack}\n`);
  process.exit(1);
}
