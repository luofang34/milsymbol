//! Every row of the pinned code tables is classified and exercised.

use milsymbol::catalog::standards::{self, CodeKind, Scope};
use milsymbol::options::SymbolOptions;
use milsymbol::sidc::SidcCheckError;
use milsymbol::{Renderer, Standard};
use std::collections::BTreeMap;
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

fn check_codes(std: &str, version: &str, renderer: &Renderer) -> TestResult {
    let text = read(&format!("{std}/codes.tsv"))?;
    let scope_text = read("scope.tsv")?;
    let scopes = rows(&scope_text)?;
    for row in rows(&text)? {
        let (set, kind, code) = (
            cell(&row, "symbol_set")?,
            cell(&row, "kind")?,
            cell(&row, "code")?,
        );
        let scope = scopes
            .iter()
            .find(|r| {
                r.get("document") == Some(&std)
                    && r.get("symbol_set") == Some(&set)
                    && r.get("kind") == Some(&kind)
                    && r.get("code") == Some(&code)
            })
            .ok_or("unclassified code table row")?;
        let (catalog_kind, catalog_code) = match kind {
            "entity" => (CodeKind::Entity, code.to_owned()),
            "m1" => (CodeKind::Modifier1, format!("0{code}")),
            "m2" => (CodeKind::Modifier2, format!("0{code}")),
            "common_m1" => (CodeKind::Modifier1, code.to_owned()),
            "common_m2" => (CodeKind::Modifier2, code.to_owned()),
            _ => return Err(format!("unhandled code kind {kind}").into()),
        };
        let entry = standards::entry(version.parse()?, set, catalog_kind, &catalog_code)
            .ok_or_else(|| format!("missing catalog entry {std} {set} {kind} {code}"))?;
        let sidc = code_sidc(version, set, kind, code).ok_or("invalid fixture code")?;
        if cell(scope, "scope")? == "multipoint" {
            assert_eq!(entry.scope, Scope::Multipoint);
            assert!(
                !cell(scope, "drawing_page")?.is_empty(),
                "multipoint needs a source"
            );
            assert!(
                matches!(
                    renderer.check_sidc(&sidc),
                    Err(SidcCheckError::Unsupported { .. })
                ),
                "{sidc}"
            );
            continue;
        }
        let checked = renderer.check_sidc(&sidc);
        assert!(checked.is_ok(), "{std} {set} {kind} {code}: {checked:?}");
        if entry.scope == Scope::Point {
            let mut options = SymbolOptions::default();
            if let Some(field) = entry.input_field() {
                options.set_text(field, "123");
            }
            let symbol = renderer.symbol(sidc.as_str()).options(options).render()?;
            let blank = code_sidc(
                version,
                if set == "common" { "10" } else { set },
                "entity",
                "000000",
            )
            .ok_or("invalid blank code")?;
            let empty = renderer.symbol(blank.as_str()).render()?;
            assert_eq!(symbol.drawing().invalid_paths, 0, "{sidc}");
            assert_ne!(
                symbol.instructions(),
                empty.instructions(),
                "empty point graphic: {std} {set} {kind} {code}"
            );
        }
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

fn check_headers(std: &str, version: &str, renderer: &Renderer) -> TestResult {
    let text = read(&format!("{std}/headers.tsv"))?;
    for row in rows(&text)? {
        let (field, code) = (cell(&row, "field")?, cell(&row, "code")?);
        if let Some(sidc) = header_sidc(version, field, code) {
            let result = renderer.check_sidc(&sidc);
            assert!(result.is_ok(), "{std} {field} {code}: {result:?}");
        } else if field == "version" {
            let (first, last) = code.split_once('-').ok_or("version range")?;
            for value in u8::from_str_radix(first, 16)?..=u8::from_str_radix(last, 16)? {
                let version = format!("{value:02X}");
                let sidc = Fields::new(&version).sidc();
                assert!(
                    matches!(
                        renderer.check_sidc(&sidc),
                        Err(SidcCheckError::Malformed(_))
                    ),
                    "reserved version {version}"
                );
            }
        } else {
            assert_eq!(
                field, "amplifying_descriptor_group",
                "unhandled header {field}"
            );
            assert!(
                rows(&text)?
                    .iter()
                    .any(|r| r.get("field") == Some(&"amplifying_descriptor")
                        && r.get("code").is_some_and(|c| c.starts_with(code))),
                "empty amplifier group {code}"
            );
        }
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
#[test]
fn frame_dimensions_follow_display_standard_for_every_numeric_edition() -> TestResult {
    let text = read("frames.tsv")?;
    for row in rows(&text)? {
        let std = cell(&row, "standard")?;
        let standard = STANDARDS
            .iter()
            .find(|s| s.0 == std)
            .ok_or("unknown standard")?
            .2;
        let renderer = Renderer::builder().standard(standard).build();
        let (dimension, group) = (cell(&row, "dimension")?, cell(&row, "identity_group")?);
        for version in ["10", "11", "12", "13", "14", "15", "16"] {
            let sidc = frame_sidc(version, dimension, group).ok_or("unknown frame")?;
            let bbox = renderer.symbol(sidc.as_str()).render()?.bounding_box();
            let (w, h) = (bbox.width() / 100.0, bbox.height() / 100.0);
            let want_w: f64 = cell(&row, "width_l")?.parse()?;
            let want_h: f64 = cell(&row, "height_l")?.parse()?;
            assert!(
                (w - want_w).abs() <= FRAME_TOLERANCE && (h - want_h).abs() <= FRAME_TOLERANCE,
                "{std} version {version} {dimension} {group}: {w}x{h}, expected {want_w}x{want_h}"
            );
        }
    }
    Ok(())
}

#[test]
fn every_in_scope_code_and_header_renders() -> TestResult {
    for (std, version, standard) in STANDARDS {
        let renderer = Renderer::builder().standard(standard).build();
        check_headers(std, version, &renderer)?;
        check_codes(std, version, &renderer)?;
    }
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
