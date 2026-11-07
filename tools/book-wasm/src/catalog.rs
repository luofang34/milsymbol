//! Browser choices derived from the Rust catalog and pinned header fixtures.

use milsymbol::catalog::standards::{self, CodeKind, Scope, Source};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MIL_HEADERS: &str = include_str!("../../../tests/data/standards/mil2525e1/headers.tsv");
const APP_HEADERS: &str = include_str!("../../../tests/data/standards/app6e2/headers.tsv");

pub(crate) fn source(source: Source) -> Value {
    json!({"document":source.document,"page":source.page,"table":source.table})
}

pub(crate) fn build(version: u8) -> Value {
    let modern = matches!(version, 15 | 16);
    let mut rows: Vec<Value> = if modern {
        standards::entries(version).filter(|e|e.scope!=Scope::Multipoint)
        .map(|e|json!({"set":e.symbol_set,"kind":match e.kind {CodeKind::Entity=>"entity",CodeKind::Modifier1=>"modifier1",CodeKind::Modifier2=>"modifier2",_=>"unknown"},
            "code":e.code,"name":e.name,"scope":format!("{:?}",e.scope),"source":source(e.source),"input":e.input_field()})).collect()
    } else {
        legacy()
    };
    let cyber_modifiers: Vec<Value> = rows
        .iter()
        .filter(|r| {
            version == 16
                && r.get("set").and_then(Value::as_str) == Some("60")
                && r.get("kind").and_then(Value::as_str) != Some("entity")
        })
        .map(|row| {
            let mut row = row.clone();
            if let Some(object) = row.as_object_mut() {
                object.insert("set".into(), json!("64"));
            }
            row
        })
        .collect();
    rows.extend(cyber_modifiers);
    let sets: BTreeSet<&str> = rows
        .iter()
        .filter(|r| r.get("kind").and_then(Value::as_str) == Some("entity"))
        .filter_map(|r| r.get("set")?.as_str())
        .filter(|s| *s != "common")
        .collect();
    let headers = if version == 16 {
        APP_HEADERS
    } else {
        MIL_HEADERS
    };
    let set_names: Vec<Value> = sets
        .into_iter()
        .map(|code| {
            let name = headers
                .lines()
                .skip(1)
                .find_map(|line| {
                    let mut cells = line.split('\t');
                    (cells.next() == Some("symbol_set") && cells.next() == Some(code))
                        .then(|| cells.next())
                        .flatten()
                })
                .unwrap_or(code);
            json!({"code":code,"name":name})
        })
        .collect();
    let header_fields = header_choices(version, headers);
    json!({"sets":set_names,"rows":rows,"headers":header_fields,
    "presets":[
        {"name":"Infantry platoon","sidc":"15031000141211000000","standard":"Mil2525"},
        {"name":"Restricted target","sidc":"15331000001211000000","standard":"Mil2525"},
        {"name":"No-strike exercise","sidc":"16631000001211000000","standard":"App6"},
        {"name":"Cyber firewall","sidc":"16036400001301000000","standard":"App6"},
        {"name":"Tropopause height","sidc":"15034500001101020000","standard":"Mil2525","options":{"text":{"altitudeDepth":"320"}}},
        {"name":"Combined combat","sidc":"16031000001221000000","standard":"App6"},
        {"name":"Harbour entrance O","sidc":"16032500002129020000","standard":"App6"}
    ]})
}

fn header_choices(version: u8, headers: &str) -> Vec<Value> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cells: Vec<_> = line.split('\t').collect();
            let field = *cells.first()?;
            if !matches!(
                field,
                "context"
                    | "standard_identity"
                    | "status"
                    | "hq_tf_dummy"
                    | "hq_task_force_dummy"
                    | "amplifier"
                    | "amplifying_descriptor"
            ) {
                return None;
            }
            let code = *cells.get(1)?;
            if code.contains('-') || cells.get(2)?.to_lowercase().contains("reserved") {
                return None;
            }
            let (start, end) = match field {
                "context" => (2, 3),
                "standard_identity" => (3, 4),
                "status" => (6, 7),
                "hq_tf_dummy" | "hq_task_force_dummy" => (7, 8),
                _ => (8, 10),
            };
            if code.len() != end - start {
                return None;
            }
            let mut sidc = format!("{version:02}031000000000000000");
            sidc.replace_range(start..end, code);
            if milsymbol::sidc::Sidc::parse(&sidc).is_err() {
                return None;
            }
            Some(json!({"field":field,"code":code,"name":cells.get(2)?}))
        })
        .collect()
}

fn legacy() -> Vec<Value> {
    let mut rows = Vec::new();
    for set in milsymbol::catalog::number_symbol_sets() {
        for entity in milsymbol::catalog::number_entities(set) {
            rows.push(
                json!({"set":set,"kind":"entity","code":entity,"name":entity,"scope":"Point"}),
            );
        }
        for (kind, codes) in [
            (
                "modifier1",
                milsymbol::catalog::number_modifier1(set).collect::<Vec<_>>(),
            ),
            (
                "modifier2",
                milsymbol::catalog::number_modifier2(set).collect::<Vec<_>>(),
            ),
        ] {
            for code in codes {
                rows.push(json!({"set":set,"kind":kind,"code":if code.len()==2 {format!("0{code}")}else{code.into()},"name":code}));
            }
        }
    }
    rows
}
