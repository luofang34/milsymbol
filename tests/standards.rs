//! Conformance with the pinned standards (`tools/standards/pin.json`):
//! MIL-STD-2525E Change 1 (SIDC version `15`) and APP-6(E) Version 2 (`16`).
//!
//! Every row of the tables extracted by `tools/standards` gets a status, and
//! the statuses must equal `tests/data/standards/conformance.tsv`. A fix or a
//! regression is therefore a reviewed change to that file: copy the file this
//! test writes after checking the difference it prints.

use milsymbol::sidc::SidcCheckError;
use milsymbol::{Renderer, Standard};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Fixture directory and SIDC version of each pinned standard.
const STANDARDS: [(&str, &str, Standard); 2] = [
    ("mil2525e1", "15", Standard::Mil2525),
    ("app6e2", "16", Standard::App6),
];

/// Half the rounding step of the frame size tables, which give `L` to two
/// decimals.
const FRAME_TOLERANCE: f64 = 0.005;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/standards")
}

fn read(relative: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = data_dir().join(relative);
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()).into())
}

/// Rows of a tab-separated table with a header line, keyed by column name.
fn rows(text: &str) -> Result<Vec<BTreeMap<&str, &str>>, Box<dyn std::error::Error>> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or("empty table")?.split('\t').collect();
    lines
        .filter(|l| !l.is_empty())
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            if cells.len() == header.len() {
                Ok(header.iter().copied().zip(cells).collect())
            } else {
                Err(format!("{} cells, expected {}: {line}", cells.len(), header.len()).into())
            }
        })
        .collect()
}

fn cell<'a>(row: &BTreeMap<&str, &'a str>, column: &str) -> Result<&'a str, String> {
    row.get(column)
        .copied()
        .ok_or_else(|| format!("no column {column}"))
}

/// What the renderer makes of a SIDC, as one word.
fn verdict(renderer: &Renderer, sidc: &str) -> &'static str {
    match renderer.check_sidc(sidc) {
        Ok(_) => "ok",
        Err(SidcCheckError::Malformed(_)) => "malformed",
        Err(SidcCheckError::Unsupported { .. }) => "unrecognised",
        Err(_) => "render-error",
    }
}

/// A 30-digit SIDC: version, context `0`, identity friend, the given set,
/// status, HQ/TF/dummy and amplifier, then entity, sector modifiers, the
/// common modifier digits and the frame shape (position 23).
struct Fields<'a> {
    version: &'a str,
    context: &'a str,
    identity: &'a str,
    symbol_set: &'a str,
    status: &'a str,
    hq_tf_dummy: &'a str,
    amplifier: &'a str,
    entity: &'a str,
    m1: &'a str,
    m2: &'a str,
    common: &'a str,
    frame_shape: &'a str,
}

impl<'a> Fields<'a> {
    fn new(version: &'a str) -> Self {
        Fields {
            version,
            context: "0",
            identity: "3",
            symbol_set: "10",
            status: "0",
            hq_tf_dummy: "0",
            amplifier: "00",
            entity: "000000",
            m1: "00",
            m2: "00",
            common: "00",
            frame_shape: "0",
        }
    }

    fn sidc(&self) -> String {
        let f = self;
        format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}0000000",
            f.version,
            f.context,
            f.identity,
            f.symbol_set,
            f.status,
            f.hq_tf_dummy,
            f.amplifier,
            f.entity,
            f.m1,
            f.m2,
            f.common,
            f.frame_shape
        )
    }
}

/// The SIDC that exercises one row of `codes.tsv`. Common modifiers are
/// tried in the land unit set; their first digit goes to position 21 or 22.
fn code_sidc(version: &str, set: &str, kind: &str, code: &str) -> Option<String> {
    let mut f = Fields::new(version);
    let common;
    match kind {
        "entity" => (f.symbol_set, f.entity) = (set, code),
        "m1" => (f.symbol_set, f.m1) = (set, code),
        "m2" => (f.symbol_set, f.m2) = (set, code),
        "common_m1" | "common_m2" => {
            let first = code.get(..1)?;
            let rest = code.get(1..)?;
            common = if kind == "common_m1" {
                f.m1 = rest;
                format!("{first}0")
            } else {
                f.m2 = rest;
                format!("0{first}")
            };
            f.common = &common;
        }
        _ => return None,
    }
    let sidc = f.sidc();
    (sidc.len() == 30).then_some(sidc)
}

fn check_codes(out: &mut String, std: &str, version: &str, renderer: &Renderer) -> TestResult {
    let text = read(&format!("{std}/codes.tsv"))?;
    for row in rows(&text)? {
        let (set, kind, code) = (
            cell(&row, "symbol_set")?,
            cell(&row, "kind")?,
            cell(&row, "code")?,
        );
        // Until the version is recognised, the status of the same code under
        // the last APP-6(E) edition upstream knows (`14`) shows whether the
        // existing tables have an entry for it at all.
        let status = match (
            code_sidc(version, set, kind, code),
            code_sidc("14", set, kind, code),
        ) {
            (Some(sidc), Some(old)) => match verdict(renderer, &sidc) {
                "ok" => String::from("ok"),
                now => format!("{now}; as 14: {}", verdict(renderer, &old)),
            },
            _ => String::from("not-a-code"),
        };
        writeln!(out, "{std}\tcode\t{set} {kind} {code}\t{status}")?;
    }
    Ok(())
}

/// The SIDC that exercises one header value, or `None` for fields that do
/// not have one place in the code (e.g. a version range like `17-3F`).
fn header_sidc(version: &str, field: &str, code: &str) -> Option<String> {
    let mut f = Fields::new(version);
    match field {
        "version" => f.version = code,
        "context" => f.context = code,
        "standard_identity" => f.identity = code,
        "symbol_set" => f.symbol_set = code,
        "status" => f.status = code,
        "hq_tf_dummy" | "hq_task_force_dummy" => f.hq_tf_dummy = code,
        "amplifier" | "amplifying_descriptor" => f.amplifier = code,
        "frame_shape" => f.frame_shape = code,
        _ => return None,
    }
    let sidc = f.sidc();
    (sidc.len() == 30).then_some(sidc)
}

fn check_headers(out: &mut String, std: &str, version: &str, renderer: &Renderer) -> TestResult {
    let text = read(&format!("{std}/headers.tsv"))?;
    for row in rows(&text)? {
        let (field, code) = (cell(&row, "field")?, cell(&row, "code")?);
        let status = match header_sidc(version, field, code) {
            Some(sidc) => verdict(renderer, &sidc),
            None => "not-checked",
        };
        writeln!(out, "{std}\theader\t{field} {code}\t{status}")?;
    }
    Ok(())
}

fn frame_sidc(version: &str, dimension: &str, group: &str) -> Option<String> {
    let set = match dimension {
        "air" => "01",
        "space" => "05",
        "land_unit" => "10",
        "land_equipment" => "15",
        "dismounted" => "27",
        "subsurface" => "35",
        "activity" => "40",
        "cyberspace" => "60",
        _ => return None,
    };
    let identity = match group {
        "unknown" => "1",
        "friend" => "3",
        "neutral" => "4",
        "hostile" => "6",
        _ => return None,
    };
    let mut f = Fields::new(version);
    (f.symbol_set, f.identity) = (set, identity);
    Some(f.sidc())
}

/// Frame proportions, measured on the frame-only symbol in the octagon's
/// units (`L` = 100).
fn check_frames(out: &mut String, renderers: &BTreeMap<&str, (&str, Renderer)>) -> TestResult {
    let text = read("frames.tsv")?;
    for row in rows(&text)? {
        let std = cell(&row, "standard")?;
        let (dimension, group) = (cell(&row, "dimension")?, cell(&row, "identity_group")?);
        let (version, renderer) = renderers.get(std).ok_or("unknown standard")?;
        let sidc = frame_sidc(version, dimension, group).ok_or("unknown frame")?;
        let bbox = renderer.symbol(sidc.as_str()).render()?.bounding_box();
        let (w, h) = ((bbox.x2 - bbox.x1) / 100.0, (bbox.y2 - bbox.y1) / 100.0);
        let want_w: f64 = cell(&row, "width_l")?.parse()?;
        let want_h: f64 = cell(&row, "height_l")?.parse()?;
        let status =
            if (w - want_w).abs() <= FRAME_TOLERANCE && (h - want_h).abs() <= FRAME_TOLERANCE {
                String::from("ok")
            } else {
                format!("measured {w:.3}x{h:.3}")
            };
        writeln!(
            out,
            "{std}\tframe\t{dimension} {group} {want_w}x{want_h}\t{status}"
        )?;
    }
    Ok(())
}

/// The lines of `expected` and `actual` that differ, as a short report.
fn difference(expected: &str, actual: &str) -> String {
    let old: std::collections::BTreeSet<&str> = expected.lines().collect();
    let new: std::collections::BTreeSet<&str> = actual.lines().collect();
    let mut report = String::new();
    for line in old.difference(&new).take(20) {
        report.push_str("\n- ");
        report.push_str(line);
    }
    for line in new.difference(&old).take(20) {
        report.push_str("\n+ ");
        report.push_str(line);
    }
    report
}

#[test]
fn statuses_match_the_reviewed_conformance_table() -> TestResult {
    let mut renderers = BTreeMap::new();
    let mut out = String::from("standard\tsection\titem\tstatus\n");
    for (std, version, standard) in STANDARDS {
        let renderer = Renderer::builder().standard(standard).build();
        check_headers(&mut out, std, version, &renderer)?;
        check_codes(&mut out, std, version, &renderer)?;
        renderers.insert(std, (version, renderer));
    }
    check_frames(&mut out, &renderers)?;

    let mut totals: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for line in out.lines().skip(1) {
        let mut cells = line.split('\t');
        let std = cells.next().unwrap_or_default();
        let status = cells.nth(2).unwrap_or_default();
        *totals
            .entry((std, if status == "ok" { "ok" } else { "open" }))
            .or_default() += 1;
    }

    let actual_path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("conformance.tsv");
    std::fs::write(&actual_path, &out)?;
    let expected = read("conformance.tsv").unwrap_or_default();
    assert!(
        expected == out,
        "conformance changed ({totals:?}); review and copy {} to {}:{}",
        actual_path.display(),
        data_dir().join("conformance.tsv").display(),
        difference(&expected, &out)
    );
    Ok(())
}

#[test]
fn every_built_sidc_has_thirty_digits() {
    assert_eq!(Fields::new("16").sidc(), "160310000000000000000000000000");
    assert_eq!(
        code_sidc("16", "common", "common_m2", "134").as_deref(),
        Some("160310000000000000340100000000")
    );
    assert_eq!(
        header_sidc("15", "frame_shape", "A").as_deref(),
        Some("1503100000000000000000A0000000")
    );
}
