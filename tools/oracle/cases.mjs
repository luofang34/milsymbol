// Generates the differential corpus case lists.
//
// Usage: node cases.mjs <suite> > cases.jsonl
// Suites: base, modifiers, options, config, invalid, fuzz, all-letter
//
// Cases are {sidc, options?, cfg?}. Entity codes are discovered from the
// upstream mapping functions themselves, independently of tools/codegen.

import { ms } from "./oracle.mjs";

function mulberry32(seed) {
  return function () {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const rand = mulberry32(20260927);
const pick = (a) => a[Math.floor(rand() * a.length)];
const out = (c) => process.stdout.write(JSON.stringify(c) + "\n");

// Numeric symbol sets with at least one main icon under any standard or
// edition, discovered from upstream rather than listed by hand.
const NUMBER_SETS = (() => {
  const s = new ms.Symbol("10031000001211000000");
  const found = [];
  for (let i = 0; i < 100; i++) {
    const ss = String(i).padStart(2, "0");
    let any = false;
    for (const std of [true, false])
      for (const ed of ["D", "E"]) {
        const parts = ms._getIconParts(s.metadata, s.colors, std, "", false);
        if (Object.keys(ms._getIcons.number(ms, ss, parts, std, ed).icons).length) any = true;
      }
    if (any) found.push(ss);
  }
  return found;
})();

function numberTables(ss, std = true, edition = "D") {
  const s = new ms.Symbol("10031000001211000000");
  const parts = ms._getIconParts(s.metadata, s.colors, std, "", false);
  return ms._getIcons.number(ms, ss, parts, std, edition);
}

function numberEntities(ss) {
  const keys = new Set();
  for (const std of [true, false]) for (const ed of ["D", "E"]) for (const k of Object.keys(numberTables(ss, std, ed).icons)) keys.add(k);
  const candidates = new Set(["000000", ...keys]);
  for (const k of keys) for (const hq of ["95", "96", "97", "98", "99"]) candidates.add(k.slice(0, 4) + hq);
  return [...candidates].sort();
}

function letterPatterns() {
  const s = new ms.Symbol("SFGPUCI-----");
  const keys = new Set();
  for (const std of [true, false]) {
    const parts = ms._getIconParts(s.metadata, s.colors, std, "", false);
    for (const k of Object.keys(ms._getIcons.letter(ms, parts, std).icons)) keys.add(k);
  }
  return [...keys].sort();
}

const valid = (sidc, options) => new ms.Symbol(sidc, Object.assign({}, options || {})).isValid();

// 109,216-symbol-style base space: valid numeric entities × 7 standard
// identities, and letter patterns × 6 affiliations (+ echelons).
function base() {
  const seen = new Set();
  const perSet = new Map(NUMBER_SETS.map((ss) => [ss, 0]));
  for (const ss of NUMBER_SETS) {
    for (const entity of numberEntities(ss)) {
      if (!valid("1003" + ss + "0000" + entity + "0000")) continue;
      for (const aff of ["0", "1", "2", "3", "4", "5", "6"]) {
        const sidc = "100" + aff + ss + "0000" + entity + "0000";
        if (!seen.has(sidc) && valid(sidc)) {
          seen.add(sidc);
          perSet.set(ss, perSet.get(ss) + 1);
          out({ sidc });
        }
      }
    }
  }
  // Frame-only SIDCs (entity 000000) are valid for every symbol set, with
  // or without icons (e.g. 45, atmospheric METOC).
  for (let i = 0; i < 100; i++) {
    const ss = String(i).padStart(2, "0");
    for (const aff of ["0", "1", "2", "3", "4", "5", "6"]) {
      const sidc = "100" + aff + ss + "0000" + "000000" + "0000";
      if (!seen.has(sidc) && valid(sidc)) {
        seen.add(sidc);
        out({ sidc });
      }
    }
  }
  const uncovered = [...perSet].filter(([, n]) => n === 0).map(([ss]) => ss);
  if (uncovered.length) throw new Error(`symbol sets without base cases: ${uncovered.join(",")}`);
  console.error(`base: numeric symbol sets ${NUMBER_SETS.join(",")}`);
  const echelons = ["A", "B", "C", "D", "E", "F", "G", "H"];
  for (const pattern of letterPatterns()) {
    for (const aff of ["F", "H", "N", "U", "A", "S"]) {
      const sidc = pattern.length >= 2 ? pattern[0] + aff + pattern.slice(2) : pattern;
      if (!seen.has(sidc) && valid(sidc)) {
        seen.add(sidc);
        out({ sidc });
      }
      if (sidc.length >= 10) {
        for (const e of echelons) {
          const withEch = sidc.substring(0, 10) + "-" + e;
          if (!seen.has(withEch) && valid(withEch)) {
            seen.add(withEch);
            out({ sidc: withEch });
          }
        }
      }
    }
  }
}

// Systematic SIDC-field coverage on representative entities.
function modifiers() {
  const reps = {};
  for (const ss of NUMBER_SETS) {
    const ents = numberEntities(ss).filter((e) => valid("1003" + ss + "0000" + e + "0000"));
    reps[ss] = [ents[1] || ents[0], ents[Math.floor(ents.length / 2)], ents[ents.length - 1]].filter(Boolean);
  }
  const digits = "0123456789".split("");
  for (const ss of NUMBER_SETS) {
    const t = numberTables(ss, true, "E");
    const m1 = Object.keys(t.m1);
    const m2 = Object.keys(t.m2);
    for (const e of reps[ss]) {
      for (const version of ["10", "11", "12", "13", "14", "15"])
        for (const ctx of ["0", "1", "2", "3"])
          for (const si of ["0", "1", "2", "3", "4", "5", "6", "7"]) out({ sidc: version + ctx + si + ss + "0000" + e + "0000" });
      for (const si of ["1", "3", "5", "6"])
        for (const st of digits)
          for (const hq of digits) out({ sidc: "130" + si + ss + st + hq + "00" + e + "0000" });
      for (let em = 0; em < 100; em++)
        for (const si of ["3", "6", "4", "1"]) out({ sidc: "100" + si + ss + "00" + String(em).padStart(2, "0") + e + "0000" });
      for (const si of ["3", "6"]) {
        for (const k of m1) out({ sidc: "130" + si + ss + "0000" + e + k.padStart(2, "0").slice(-2) + "00" + (k.length === 3 ? k[0] : "") });
        for (const k of m2) out({ sidc: "130" + si + ss + "0000" + e + "00" + k.padStart(2, "0").slice(-2) + (k.length === 3 ? "0" + k[0] : "") });
        for (const a of m1.slice(0, 6)) for (const b of m2.slice(0, 6)) out({ sidc: "130" + si + ss + "0000" + e + a.slice(-2) + b.slice(-2) });
        for (const fs of [...digits, "A", "B"]) out({ sidc: "130" + si + ss + "0000" + e + "0000" + "00" + fs });
      }
    }
  }
  // Letter SIDC fields.
  const letterReps = ["SFGPUCI-----", "SFAPMF------", "SFSPCLCV----", "SFUPSN------", "SFPPS-------", "SFGPEVAT----", "SFGPIRN-----", "OFVPMA------", "EFOPA-------", "IFGPSCC-----", "GFGPGPP-----", "WAS-PC----P----"];
  const letters = "-ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");
  for (const rep of letterReps) {
    for (const aff of letters) for (const status of ["P", "A", "C", "D", "X", "F", "-"]) out({ sidc: rep[0] + aff + rep[2] + status + rep.slice(4) });
    for (const dim of letters) out({ sidc: rep[0] + "F" + dim + rep.slice(3) });
    for (const m11 of letters) for (const m12 of letters) out({ sidc: rep.slice(0, 10) + m11 + m12 });
  }
}

const TEXT_FIELDS = {
  quantity: "200", reinforcedReduced: "(+)", staffComments: "For reinforcement", additionalInformation: "Add info",
  evaluationRating: "A1", combatEffectiveness: "Green", signatureEquipment: "!", higherFormation: "1/66",
  hostile: "ENY", iffSif: "M1:4577", sigint: "E", uniqueDesignation: "Alpha", type: "MACHINE GUN",
  dtg: "30140000ZSEP97", altitudeDepth: "FL150", location: "32UNB0000", speed: "200 KPH",
  specialHeadquarters: "SHQ", country: "SE", platformType: "ELNOT", equipmentTeardownTime: "15",
  commonIdentifier: "Hawk", auxiliaryEquipmentIndicator: "Aux", headquartersElement: "TOC",
  installationComposition: "Comp", guardedUnit: "BB", specialDesignator: "SD",
};

const OPTION_SIDCS = [
  "10031000001211000000", "10061000161211000000", "10041000001211000000", "10011000001211000000",
  "10030100001101000000", "10060100001101000000", "10033000001201000000", "10033500001101000000",
  "10031500001101000000", "10032000001101000000", "10032700001101000000", "10034000001101000000",
  "10036000001101000000", "10030500001101000000", "10032500001101000000", "10032500000401000000",
  "13031000001211000000", "SFGPUCI-----", "SHGPUCI-----", "SNAPMF------", "SUSPCL------", "SFGPEWM-----",
  "GFGPGPP-----", "GFGPGPRI----", "SFUPSN------", "SFGAUCI---A-", "SFGCUCI---D-",
];

function options() {
  const colorModes = ["Light", "Medium", "Dark"];
  const perAff = { Friend: "rgb(1,2,3)", Hostile: "red", Neutral: "#00ff00", Unknown: "yellow", Civilian: "purple", Suspect: "orange" };
  const singles = [
    { size: 30 }, { size: 5 }, { size: 250 }, { strokeWidth: 1 }, { strokeWidth: 8 }, { outlineWidth: 3 },
    { outlineWidth: 3, outlineColor: "blue" }, { outlineWidth: 2, outlineColor: perAff }, { padding: 10 }, { square: true },
    { fill: false }, { frame: false }, { icon: false }, { frame: false, icon: false }, { fill: false, frame: false },
    { monoColor: "black" }, { monoColor: "rgb(255,0,0)", outlineWidth: 2 }, { fillOpacity: 0.5 }, { fillColor: "pink" },
    { colorMode: "Dark" }, { colorMode: "Medium" }, { colorMode: perAff }, { frameColor: perAff }, { iconColor: perAff },
    { frameColor: "red" }, { civilianColor: false }, { alternateMedal: true }, { standard: "APP6" }, { standard: "2525" },
    { hqStaffLength: 50 }, { infoFields: false, uniqueDesignation: "A" }, { infoSize: 60, uniqueDesignation: "A" },
    { infoColor: "red", uniqueDesignation: "A" }, { infoColor: perAff, uniqueDesignation: "A" },
    { infoBackground: "white", uniqueDesignation: "A", dtg: "1", staffComments: "x" },
    { infoBackground: perAff, uniqueDesignation: "A", higherFormation: "B" },
    { infoOutlineWidth: 2, uniqueDesignation: "A" }, { infoOutlineWidth: 2, infoOutlineColor: "", uniqueDesignation: "A" },
    { outlineWidth: 2, uniqueDesignation: "A" }, { fontfamily: "Courier", uniqueDesignation: "A" },
    { fontfamily: "Evil<script>", uniqueDesignation: "A" }, { simpleStatusModifier: true }, { styleFill: true },
    { direction: 45 }, { direction: 0 }, { direction: 270, speedLeader: 50 }, { direction: 135, speedLeader: 80, size: 50 },
    { direction: 90, outlineWidth: 2 }, { direction: 200, speedLeader: 30, outlineWidth: 2 }, { direction: 33.3 },
    { engagementBar: "2:10", engagementType: "TARGET" }, { engagementBar: "LONG ENGAGEMENT", engagementType: "non-target" },
    { engagementBar: "X", engagementType: "expired", outlineWidth: 2 }, { engagementBar: "E", monoColor: "blue" },
    { stack: 2 }, { stack: 3, outlineWidth: 2 }, { stack: 1, uniqueDesignation: "A", staffComments: "B" },
    { country_flag: "SE", uniqueDesignation: "A", higherFormation: "B", staffComments: "C" },
    { signature: "!", higherFormation: "B" }, { full_frame_flag: true, country_flag: "US", higherFormation: "B" },
    { uniqueDesignation: "<script>alert(1)</script>" }, { staffComments: "a & b \" ' < >" },
    { uniqueDesignation: "null value" }, { specialHeadquarters: "HQ" }, { specialHeadquarters: "ABCD" },
    { quantity: "12" }, { headquartersElement: "TOC" }, { dtg1: "D1", uniqueDesignation1: "U1", additionalInformation1: "A1" },
    { targetNumber: "T1", uniqueDesignation: "U" }, { type: "ÅÄÖ Ж 😀" },
    { fillColor: "\u0085red\u0085" }, { fillColor: "\u3000pink\ufeff" },
  ];
  for (const sidc of OPTION_SIDCS) {
    out({ sidc, options: TEXT_FIELDS });
    for (const [k, v] of Object.entries(TEXT_FIELDS)) out({ sidc, options: { [k]: v } });
    for (const o of singles) out({ sidc, options: o });
    for (const mode of colorModes) out({ sidc, options: Object.assign({ colorMode: mode }, TEXT_FIELDS) });
  }
  // Random combinations.
  for (let i = 0; i < 4000; i++) {
    const o = {};
    const n = 1 + Math.floor(rand() * 5);
    for (let j = 0; j < n; j++) Object.assign(o, pick(singles));
    for (let j = 0; j < Math.floor(rand() * 6); j++) {
      const k = pick(Object.keys(TEXT_FIELDS));
      o[k] = TEXT_FIELDS[k];
    }
    out({ sidc: pick(OPTION_SIDCS), options: o });
  }
}

function config() {
  for (const sidc of OPTION_SIDCS) {
    out({ sidc, cfg: { standard: "APP6" } });
    out({ sidc, cfg: { dashArrays: ["2,2", "3,3", "5,5"] } });
    out({ sidc, cfg: { hqStaffLength: 30 } });
    out({ sidc, options: { hqStaffLength: 70 }, cfg: { hqStaffLength: 30 } });
  }
  for (const sidc of ["10031002001211000000", "10061022001211000000", "SFGPUCI---A-", "SFGAUCI---B-", "SFGPUCI---D-"]) {
    for (const hq of [0, 20, 150]) out({ sidc, cfg: { hqStaffLength: hq }, options: { direction: 30 } });
  }
}

function invalid() {
  const samples = ["", "1", "10", "1003", "10031", "1003100000", "100310000012", "10031000001211", "1003100000121100000",
    "10031000001211000000000", "100310000012110000000000000", "99999999999999999999", "10991000001211000000",
    "10031099001211000000", "10031000991211000000", "S", "SF", "SFG", "SFGP", "SFGPU", "sfgpuci", "SFG*UCI*****",
    "S F G P U C I", "XXXXXXXXXXXX", "SZGPUCI-----", "SFZPUCI-----", "SFXPUCI-----", "SDZP--------", "SJZP--------",
    "10031000001211009999", "10031000001211000009", "1003100000121100000A", "abc", "undefined", "null", "NaN",
    "-1", "1e3", "0x10", " 10031000001211000000 ", "10031000001211000000\t", "１００３", "SFGP😀-----"];
  for (const sidc of samples) out({ sidc });
  out({ sidc: "10031000001211000000", options: { colorMode: "NoSuchMode" } });
}

function fuzz() {
  const digits = "0123456789";
  const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ-*";
  for (let i = 0; i < 20000; i++) {
    let sidc = "";
    if (rand() < 0.6) {
      const len = pick([20, 20, 20, 20, 30, 10, 21, 22, 23, 15]);
      for (let j = 0; j < len; j++) sidc += digits[Math.floor(rand() * 10)];
      if (rand() < 0.7) sidc = pick(["10", "11", "13", "14"]) + sidc.slice(2);
      if (rand() < 0.7) sidc = sidc.slice(0, 4) + pick(NUMBER_SETS) + sidc.slice(6);
    } else {
      const len = pick([10, 12, 15, 12, 12, 4]);
      for (let j = 0; j < len; j++) sidc += letters[Math.floor(rand() * letters.length)];
      if (rand() < 0.7) sidc = pick(["S", "G", "W", "I", "O", "E"]) + sidc.slice(1);
    }
    out(rand() < 0.2 ? { sidc, options: { uniqueDesignation: "A", outlineWidth: 2, direction: 10 } } : { sidc });
  }
}

// Direction of movement and speed leaders over the full circle in 0.1°
// steps: these coordinates go through Math.sin/Math.cos.
function direction() {
  const sidcs = ["10031000001211000000", "10030100001101000000", "10031002001211000000", "10033000001201000000", "SFGPUCI-----"];
  for (const sidc of sidcs)
    for (let i = 0; i < 3600; i++) {
      out({ sidc, options: { direction: i / 10 } });
      out({ sidc, options: { direction: i / 10, speedLeader: 50 } });
    }
}

const suites = { base, modifiers, options, config, invalid, fuzz, direction };
const suite = process.argv[2];
if (!suites[suite]) {
  console.error(`unknown suite ${suite}; one of ${Object.keys(suites).join(", ")}`);
  process.exit(2);
}
suites[suite]();
