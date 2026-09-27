//! Echelon and mobility indicators.

use super::{Acc, SymbolState};
use crate::bbox::{BBox, PartialBBox};
use crate::error::RenderError;
use crate::ir::{CircleNode, Node, Paint, PathData, PathNode, Style};
use crate::js::number_to_string as n;
use alloc::string::String;
use alloc::vec::Vec;

fn path(d: String) -> Node {
    Node::Path(PathNode {
        d: PathData::new(d),
        style: Style::default(),
    })
}

fn circle(cx: f64, cy: f64, r: f64, fill: Option<&Option<Paint>>) -> Node {
    let mut c = CircleNode {
        cx: cx.into(),
        cy: cy.into(),
        r: r.into(),
        style: Style::default(),
    };
    if let Some(f) = fill {
        c.style.fill = f.clone();
    }
    Node::Circle(c)
}

/// Rows of crosses (`x`) used by brigade and higher echelons, each preceded by
/// upstream's exact separator.
fn crosses(y: f64, xs: &[(&str, &str)]) -> String {
    let mut d = String::new();
    for (sep, x) in xs {
        d.push_str(sep);
        d.push('M');
        d.push_str(x);
        d.push(',');
        d.push_str(&n(y));
        d.push_str(" l25,-25 m0,25 l-25,-25");
    }
    d
}

pub(super) fn echelon(s: &SymbolState<'_>, acc: &mut Acc, bbox: &BBox) -> Result<(), RenderError> {
    let pad = if s.metadata.installation { 15.0 } else { 0.0 };
    let y1 = bbox.y1;
    let color = Some(&acc.color);
    let dots = |xs: &[f64]| -> Vec<Node> {
        xs.iter()
            .map(|&x| circle(x, y1 - 20.0, 7.5, color))
            .collect()
    };
    let bars = |xs: &[&str]| -> Vec<Node> {
        xs.iter()
            .map(|x| path(alloc::format!("M{x},{}L{x},{}", n(y1 - 10.0), n(y1 - 35.0))))
            .collect()
    };
    let tall = y1 - 40.0 - pad;
    let dot_top = y1 - 20.0 - 7.5 - pad;
    let cross_top = y1 - 15.0 - 25.0 - pad;
    let wide = |x1: f64, x2: f64| PartialBBox {
        y1: Some(cross_top),
        x1: Some(x1),
        x2: Some(x2),
        y2: None,
    };
    let top = |y| PartialBBox {
        y1: Some(y),
        ..PartialBBox::default()
    };
    let c10 = y1 - 10.0;
    let (geom, gb): (Vec<Node>, PartialBBox) = match s.metadata.echelon.as_deref().unwrap_or("") {
        "Team/Crew" => (
            alloc::vec![
                circle(100.0, y1 - 20.0, 15.0, None),
                path(alloc::format!("M80,{}L120,{}", n(y1 - 10.0), n(y1 - 30.0)))
            ],
            top(tall),
        ),
        "Squad" => (dots(&[100.0]), top(dot_top)),
        "Section" => (dots(&[115.0, 85.0]), top(dot_top)),
        "Platoon/detachment" => (dots(&[100.0, 70.0, 130.0]), top(dot_top)),
        "Company/battery/troop" => (bars(&["100"]), top(tall)),
        "Battalion/squadron" => (bars(&["90", "110"]), top(tall)),
        "Regiment/group" => (bars(&["100", "120", "80"]), top(tall)),
        "Brigade" => (
            alloc::vec![path(crosses(c10, &[("", "87.5")]))],
            top(cross_top),
        ),
        "Division" => (
            alloc::vec![path(crosses(c10, &[("", "70"), ("   ", "105")]))],
            wide(70.0, 130.0),
        ),
        "Corps/MEF" => (
            alloc::vec![path(crosses(
                c10,
                &[("", "52.5"), ("    ", "87.5"), ("    ", "122.5")]
            ))],
            wide(52.5, 147.5),
        ),
        "Army" => (
            alloc::vec![path(crosses(
                c10,
                &[("", "35"), ("   ", "70"), ("   ", "105"), ("    ", "140")]
            ))],
            wide(35.0, 165.0),
        ),
        "Army Group/front" => (
            alloc::vec![path(crosses(
                c10,
                &[
                    ("", "17.5"),
                    ("    ", "52.5"),
                    ("    ", "87.5"),
                    ("    ", "122.5"),
                    ("       ", "157.5")
                ]
            ))],
            wide(17.5, 182.5),
        ),
        "Region/Theater" => (
            alloc::vec![path(crosses(
                c10,
                &[
                    ("", "0"),
                    ("   ", "35"),
                    ("   ", "70"),
                    ("   ", "105"),
                    ("    ", "140"),
                    ("     ", "175")
                ]
            ))],
            wide(0.0, 200.0),
        ),
        "Command" => {
            let y = n(y1 - 22.5);
            let d = alloc::format!(
                "M70,{y} l25,0 m-12.5,12.5 l0,-25   M105,{y} l25,0 m-12.5,12.5 l0,-25"
            );
            (alloc::vec![path(d)], wide(70.0, 130.0))
        }
        _ => return Ok(()),
    };
    acc.push_group(s, 0.0, -pad, geom)?;
    acc.gbbox.merge(gb);
    Ok(())
}

const AMPHIBIOUS: &str = "M 65,10 c 0,-10 10,-10 10,0 0,10 10,10 10,0\t0,-10 10,-10 10,0 0,10 10,10 10,0\t0,-10 10,-10 10,0 0,10 10,10 10,0\t0,-10 10,-10 10,0";

pub(super) fn mobility(
    s: &SymbolState<'_>,
    acc: &mut Acc,
    bbox: &mut BBox,
) -> Result<(), RenderError> {
    let md = s.metadata;
    let mobility = md.mobility.as_deref().unwrap_or("");
    if md.affiliation.as_deref() == Some("Neutral") {
        match mobility {
            "Towed" | "Short towed array" | "Long towed Array" => bbox.y2 += 8.0,
            "Over snow (prime mover)" | "Sled" => bbox.y2 += 18.0,
            "Barge" => bbox.y2 += 5.0,
            _ => {}
        }
    }
    let y2 = bbox.y2;
    let p = |d: &str| path(String::from(d));
    let c = |cx, cy| circle(cx, cy, 8.0, None);
    let filled = |d: &str| {
        let mut node = path(String::from(d));
        if let Some(st) = node.style_mut() {
            st.fill = acc.color.clone();
        }
        node
    };
    let b = |dy: f64, x1: Option<f64>, x2: Option<f64>| PartialBBox {
        y2: Some(y2 + dy),
        x1,
        x2,
        y1: None,
    };
    let (geom, gb): (Vec<Node>, PartialBBox) = match mobility {
        "Wheeled limited cross country" => (
            alloc::vec![p("M 53,1 l 94,0"), c(58.0, 8.0), c(142.0, 8.0)],
            b(8.0 * 2.0, None, None),
        ),
        "Wheeled cross country" => (
            alloc::vec![
                p("M 53,1 l 94,0"),
                c(58.0, 8.0),
                c(142.0, 8.0),
                c(100.0, 8.0)
            ],
            b(8.0 * 2.0, None, None),
        ),
        "Tracked" => (
            alloc::vec![p(
                "M 53,1 l 100,0 c15,0 15,15 0,15 l -100,0 c-15,0 -15,-15 0,-15"
            )],
            b(18.0, Some(42.0), Some(168.0)),
        ),
        "Wheeled and tracked combination" => (
            alloc::vec![
                c(58.0, 8.0),
                p("M 83,1 l 70,0 c15,0 15,15 0,15 l -70,0 c-15,0 -15,-15 0,-15")
            ],
            b(8.0 * 2.0, None, Some(168.0)),
        ),
        "Towed" => (
            alloc::vec![p("M 63,1 l 74,0"), c(58.0, 3.0), c(142.0, 3.0)],
            b(10.0, None, None),
        ),
        "Rail" => (
            alloc::vec![
                p("M 53,1 l 96,0"),
                c(58.0, 8.0),
                c(73.0, 8.0),
                c(127.0, 8.0),
                c(142.0, 8.0)
            ],
            b(8.0 * 2.0, None, None),
        ),
        "Over snow (prime mover)" => (alloc::vec![p("M 50,-9 l10,10 90,0")], b(9.0, None, None)),
        "Sled" => (
            alloc::vec![p(
                "M 145,-12  c15,0 15,15 0,15 l -90,0 c-15,0 -15,-15 0,-15"
            )],
            b(15.0, Some(42.0), Some(168.0)),
        ),
        "Pack animals" => (
            alloc::vec![p("M 80,20 l 10,-20 10,20 10,-20 10,20")],
            b(20.0, None, None),
        ),
        "Barge" => (
            alloc::vec![p("M 50,1 l 100,0 c0,10 -100,10 -100,0")],
            b(10.0, None, None),
        ),
        "Amphibious" => (alloc::vec![p(AMPHIBIOUS)], b(20.0, None, None)),
        "Short towed array" => (
            alloc::vec![filled(
                "M 50,5 l 100,0 M50,0 l10,0 0,10 -10,0 z M150,0 l-10,0 0,10 10,0 z M100,0 l5,5 -5,5 -5,-5 z"
            )],
            b(10.0, None, None),
        ),
        "Long towed Array" => (
            alloc::vec![filled(
                "M 50,5 l 100,0 M50,0 l10,0 0,10 -10,0 z M150,0 l-10,0 0,10 10,0 z M105,0 l-10,0 0,10 10,0 z M75,0 l5,5 -5,5 -5,-5 z  M125,0 l5,5 -5,5 -5,-5 z"
            )],
            b(10.0, None, None),
        ),
        _ => return Ok(()),
    };
    acc.push_group(s, 0.0, y2, geom)?;
    acc.gbbox.merge(gb);
    Ok(())
}
