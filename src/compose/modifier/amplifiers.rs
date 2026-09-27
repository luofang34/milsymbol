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

/// Shape of an echelon indicator.
enum Echelon {
    TeamCrew,
    Dots(&'static [f64]),
    Bars(&'static [&'static str]),
    Crosses(&'static [(&'static str, &'static str)]),
    Command,
}

/// Top of the echelon bounding box.
enum Top {
    Tall,
    Dot,
    Cross,
    Wide(f64, f64),
}

fn echelon_spec(name: &str) -> Option<(Echelon, Top)> {
    use Echelon::*;
    Some(match name {
        "Team/Crew" => (TeamCrew, Top::Tall),
        "Squad" => (Dots(&[100.0]), Top::Dot),
        "Section" => (Dots(&[115.0, 85.0]), Top::Dot),
        "Platoon/detachment" => (Dots(&[100.0, 70.0, 130.0]), Top::Dot),
        "Company/battery/troop" => (Bars(&["100"]), Top::Tall),
        "Battalion/squadron" => (Bars(&["90", "110"]), Top::Tall),
        "Regiment/group" => (Bars(&["100", "120", "80"]), Top::Tall),
        "Brigade" => (Crosses(&[("", "87.5")]), Top::Cross),
        "Division" => (
            Crosses(&[("", "70"), ("   ", "105")]),
            Top::Wide(70.0, 130.0),
        ),
        "Corps/MEF" => (
            Crosses(&[("", "52.5"), ("    ", "87.5"), ("    ", "122.5")]),
            Top::Wide(52.5, 147.5),
        ),
        "Army" => (
            Crosses(&[("", "35"), ("   ", "70"), ("   ", "105"), ("    ", "140")]),
            Top::Wide(35.0, 165.0),
        ),
        "Army Group/front" => (
            Crosses(&[
                ("", "17.5"),
                ("    ", "52.5"),
                ("    ", "87.5"),
                ("    ", "122.5"),
                ("       ", "157.5"),
            ]),
            Top::Wide(17.5, 182.5),
        ),
        "Region/Theater" => (
            Crosses(&[
                ("", "0"),
                ("   ", "35"),
                ("   ", "70"),
                ("   ", "105"),
                ("    ", "140"),
                ("     ", "175"),
            ]),
            Top::Wide(0.0, 200.0),
        ),
        "Command" => (Command, Top::Wide(70.0, 130.0)),
        _ => return None,
    })
}

pub(super) fn echelon(s: &SymbolState<'_>, acc: &mut Acc, bbox: &BBox) -> Result<(), RenderError> {
    let Some((shape, top)) = echelon_spec(s.metadata.echelon.as_deref().unwrap_or("")) else {
        return Ok(());
    };
    let pad = if s.metadata.installation { 15.0 } else { 0.0 };
    let y1 = bbox.y1;
    let geom: Vec<Node> = match shape {
        Echelon::TeamCrew => alloc::vec![
            circle(100.0, y1 - 20.0, 15.0, None),
            path(alloc::format!("M80,{}L120,{}", n(y1 - 10.0), n(y1 - 30.0)))
        ],
        Echelon::Dots(xs) => xs
            .iter()
            .map(|&x| circle(x, y1 - 20.0, 7.5, Some(&acc.color)))
            .collect(),
        Echelon::Bars(xs) => xs
            .iter()
            .map(|x| path(alloc::format!("M{x},{}L{x},{}", n(y1 - 10.0), n(y1 - 35.0))))
            .collect(),
        Echelon::Crosses(xs) => alloc::vec![path(crosses(y1 - 10.0, xs))],
        Echelon::Command => {
            let y = n(y1 - 22.5);
            alloc::vec![path(alloc::format!(
                "M70,{y} l25,0 m-12.5,12.5 l0,-25   M105,{y} l25,0 m-12.5,12.5 l0,-25"
            ))]
        }
    };
    let cross_top = y1 - 15.0 - 25.0 - pad;
    let gb = match top {
        Top::Tall => PartialBBox {
            y1: Some(y1 - 40.0 - pad),
            ..PartialBBox::default()
        },
        Top::Dot => PartialBBox {
            y1: Some(y1 - 20.0 - 7.5 - pad),
            ..PartialBBox::default()
        },
        Top::Cross => PartialBBox {
            y1: Some(cross_top),
            ..PartialBBox::default()
        },
        Top::Wide(x1, x2) => PartialBBox {
            y1: Some(cross_top),
            x1: Some(x1),
            x2: Some(x2),
            y2: None,
        },
    };
    acc.push_group(s, 0.0, -pad, geom)?;
    acc.gbbox.merge(gb);
    Ok(())
}

const AMPHIBIOUS: &str = "M 65,10 c 0,-10 10,-10 10,0 0,10 10,10 10,0\t0,-10 10,-10 10,0 0,10 10,10 10,0\t0,-10 10,-10 10,0 0,10 10,10 10,0\t0,-10 10,-10 10,0";

/// One element of a mobility indicator.
#[derive(Clone, Copy)]
enum Mob {
    /// Unfilled path.
    P(&'static str),
    /// Path filled with the modifier colour.
    F(&'static str),
    /// Circle of radius 8.
    C(f64, f64),
}

/// Mobility indicators: elements, bbox growth below the frame, and x extent.
type MobilitySpec = (&'static [Mob], f64, Option<f64>, Option<f64>);

/// Mobility indicators by name.
use Mob::{C, F, P};

const MOBILITIES: [(&str, MobilitySpec); 13] = [
    (
        "Wheeled limited cross country",
        (
            &[P("M 53,1 l 94,0"), C(58.0, 8.0), C(142.0, 8.0)],
            8.0 * 2.0,
            None,
            None,
        ),
    ),
    (
        "Wheeled cross country",
        (
            &[
                P("M 53,1 l 94,0"),
                C(58.0, 8.0),
                C(142.0, 8.0),
                C(100.0, 8.0),
            ],
            8.0 * 2.0,
            None,
            None,
        ),
    ),
    (
        "Tracked",
        (
            &[P(
                "M 53,1 l 100,0 c15,0 15,15 0,15 l -100,0 c-15,0 -15,-15 0,-15",
            )],
            18.0,
            Some(42.0),
            Some(168.0),
        ),
    ),
    (
        "Wheeled and tracked combination",
        (
            &[
                C(58.0, 8.0),
                P("M 83,1 l 70,0 c15,0 15,15 0,15 l -70,0 c-15,0 -15,-15 0,-15"),
            ],
            8.0 * 2.0,
            None,
            Some(168.0),
        ),
    ),
    (
        "Towed",
        (
            &[P("M 63,1 l 74,0"), C(58.0, 3.0), C(142.0, 3.0)],
            10.0,
            None,
            None,
        ),
    ),
    (
        "Rail",
        (
            &[
                P("M 53,1 l 96,0"),
                C(58.0, 8.0),
                C(73.0, 8.0),
                C(127.0, 8.0),
                C(142.0, 8.0),
            ],
            8.0 * 2.0,
            None,
            None,
        ),
    ),
    (
        "Over snow (prime mover)",
        (&[P("M 50,-9 l10,10 90,0")], 9.0, None, None),
    ),
    (
        "Sled",
        (
            &[P(
                "M 145,-12  c15,0 15,15 0,15 l -90,0 c-15,0 -15,-15 0,-15",
            )],
            15.0,
            Some(42.0),
            Some(168.0),
        ),
    ),
    (
        "Pack animals",
        (
            &[P("M 80,20 l 10,-20 10,20 10,-20 10,20")],
            20.0,
            None,
            None,
        ),
    ),
    (
        "Barge",
        (
            &[P("M 50,1 l 100,0 c0,10 -100,10 -100,0")],
            10.0,
            None,
            None,
        ),
    ),
    ("Amphibious", (&[P(AMPHIBIOUS)], 20.0, None, None)),
    (
        "Short towed array",
        (
            &[F(
                "M 50,5 l 100,0 M50,0 l10,0 0,10 -10,0 z M150,0 l-10,0 0,10 10,0 z M100,0 l5,5 -5,5 -5,-5 z",
            )],
            10.0,
            None,
            None,
        ),
    ),
    (
        "Long towed Array",
        (
            &[F(
                "M 50,5 l 100,0 M50,0 l10,0 0,10 -10,0 z M150,0 l-10,0 0,10 10,0 z M105,0 l-10,0 0,10 10,0 z M75,0 l5,5 -5,5 -5,-5 z  M125,0 l5,5 -5,5 -5,-5 z",
            )],
            10.0,
            None,
            None,
        ),
    ),
];

fn mobility_spec(name: &str) -> Option<MobilitySpec> {
    MOBILITIES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, spec)| *spec)
}

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
    let Some((elems, dy, x1, x2)) = mobility_spec(mobility) else {
        return Ok(());
    };
    let geom = elems
        .iter()
        .map(|e| match *e {
            Mob::P(d) => path(String::from(d)),
            Mob::C(cx, cy) => circle(cx, cy, 8.0, None),
            Mob::F(d) => {
                let mut node = path(String::from(d));
                if let Some(st) = node.style_mut() {
                    st.fill = acc.color.clone();
                }
                node
            }
        })
        .collect();
    let y2 = bbox.y2;
    acc.push_group(s, 0.0, y2, geom)?;
    acc.gbbox.merge(PartialBBox {
        y2: Some(y2 + dy),
        x1,
        x2,
        y1: None,
    });
    Ok(())
}
