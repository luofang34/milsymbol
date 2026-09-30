//! Symbol composition: the ordered pipeline of symbol parts.
//!
//! Each part (upstream `ms._symbolParts` entry) inspects the symbol state and
//! returns instructions drawn behind (`pre`) and in front of (`post`) what
//! earlier parts produced, plus a bounding box to merge.

use crate::bbox::{BBox, PartialBBox};
use crate::color::ColorSet;
use crate::config::RendererConfig;
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, Str, Style};
use crate::metadata::Metadata;
use crate::options::{ColorChoice, SymbolOptions};
use crate::registry::Registry;
use alloc::borrow::Cow;
use alloc::vec::Vec;

mod affiliation;
mod base_geometry;
mod direction;
mod engagement;
mod icon;
mod modifier;
mod pipeline;
mod stack;
mod status;
mod textfields;

pub(crate) use pipeline::{Composition, compose};

/// Read-only view of a symbol while its parts are drawn (upstream `this`).
pub struct SymbolState<'a> {
    pub(crate) sidc: &'a str,
    pub(crate) options: &'a SymbolOptions,
    pub(crate) metadata: &'a Metadata,
    pub(crate) colors: &'a ColorSet,
    pub(crate) bbox: BBox,
    pub(crate) config: &'a RendererConfig,
    pub(crate) registry: &'a Registry,
}

impl core::fmt::Debug for SymbolState<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SymbolState")
            .field("sidc", &self.sidc)
            .field("options", &self.options)
            .field("bbox", &self.bbox)
            .finish_non_exhaustive()
    }
}

impl<'a> SymbolState<'a> {
    /// The normalized SIDC.
    pub fn sidc(&self) -> &'a str {
        self.sidc
    }

    /// Options and style.
    pub fn options(&self) -> &'a SymbolOptions {
        self.options
    }

    /// Typed metadata.
    pub fn metadata(&self) -> crate::domain::Metadata {
        crate::domain::Metadata::from_internal(self.metadata)
    }

    /// A borrowed, allocation-free view of milsymbol.js metadata.
    pub fn js_metadata(&self) -> crate::compat::JsMetadata<'_> {
        self.metadata.into()
    }

    /// Resolved colours.
    pub fn colors(&self) -> &'a ColorSet {
        self.colors
    }

    /// Bounding box accumulated from the parts drawn so far.
    pub fn bbox(&self) -> BBox {
        self.bbox
    }

    /// Renderer configuration.
    pub fn config(&self) -> &'a RendererConfig {
        self.config
    }
}

/// Output of one symbol part.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PartOutput {
    /// Instructions drawn before everything drawn so far.
    pub pre: Vec<Node>,
    /// Instructions drawn after everything drawn so far.
    pub post: Vec<Node>,
    /// Bounds to merge into the symbol's bounding box.
    pub bbox: PartialBBox,
    /// Set when the part could not find a valid icon.
    pub invalid_icon: bool,
}

impl PartOutput {
    /// Instructions to draw behind and in front of the symbol so far, and
    /// the bounds they add.
    pub fn new(pre: Vec<Node>, post: Vec<Node>, bbox: impl Into<PartialBBox>) -> Self {
        PartOutput {
            pre,
            post,
            bbox: bbox.into(),
            invalid_icon: false,
        }
    }
}

/// A stage of symbol composition (upstream `ms.addSymbolPart`).
///
/// ```
/// use milsymbol::ir::Node;
/// use milsymbol::{PartError, PartOutput, PartialBBox, Renderer, SymbolPart, SymbolState};
///
/// /// Marks the symbol's anchor with a dot, drawn over everything else.
/// struct AnchorDot;
///
/// impl SymbolPart for AnchorDot {
///     fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, PartError> {
///         let dot = Node::circle(100.0, 100.0, 4.0);
///         Ok(PartOutput::new(vec![], vec![dot], PartialBBox::default()))
///     }
/// }
///
/// let r = Renderer::builder().symbol_part(AnchorDot).build();
/// assert!(r.symbol("10031000001211000000").render()?.to_svg().contains("<circle"));
/// # Ok::<(), milsymbol::RenderError>(())
/// ```
pub trait SymbolPart: Send + Sync {
    /// Draws this part for the symbol.
    ///
    /// A failure aborts rendering with [`RenderError::Part`], which keeps
    /// this error as its source.
    fn draw(&self, symbol: &SymbolState<'_>) -> Result<PartOutput, crate::PartError>;
}

/// The built-in symbol parts, in upstream order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuiltinPart {
    /// Stacked frames behind the symbol.
    Stack,
    /// The frame.
    BaseGeometry,
    /// The icon and its sector modifiers.
    Icon,
    /// HQ staff, task force, installation, feint/dummy, echelon, mobility.
    Modifier,
    /// Operational condition bar or damaged/destroyed slashes.
    StatusModifier,
    /// Engagement bar.
    Engagement,
    /// Exercise/simulation letters and unknown-dimension question mark.
    AffiliationDimension,
    /// Text amplifiers.
    TextFields,
    /// Direction of movement arrow or speed leader.
    DirectionArrow,
    /// The icon octagon, for debugging (upstream `ms.showOctagon`).
    Octagon,
}

impl BuiltinPart {
    /// The default pipeline.
    pub const DEFAULT: [BuiltinPart; 9] = [
        BuiltinPart::Stack,
        BuiltinPart::BaseGeometry,
        BuiltinPart::Icon,
        BuiltinPart::Modifier,
        BuiltinPart::StatusModifier,
        BuiltinPart::Engagement,
        BuiltinPart::AffiliationDimension,
        BuiltinPart::TextFields,
        BuiltinPart::DirectionArrow,
    ];
}

impl SymbolPart for BuiltinPart {
    fn draw(&self, s: &SymbolState<'_>) -> Result<PartOutput, crate::PartError> {
        self.render(s).map_err(Into::into)
    }
}

impl BuiltinPart {
    pub(crate) fn render(&self, s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
        match self {
            BuiltinPart::Stack => stack::draw(s),
            BuiltinPart::BaseGeometry => base_geometry::draw(s),
            BuiltinPart::Icon => icon::draw(s),
            BuiltinPart::Modifier => modifier::draw(s),
            BuiltinPart::StatusModifier => status::draw(s),
            BuiltinPart::Engagement => engagement::draw(s),
            BuiltinPart::AffiliationDimension => affiliation::draw(s),
            BuiltinPart::TextFields => textfields::draw(s),
            BuiltinPart::DirectionArrow => direction::draw(s),
            BuiltinPart::Octagon => Ok(octagon()),
        }
    }
}

fn octagon() -> PartOutput {
    let mut n = crate::ir::PathNode {
        d: crate::ir::PathData::new(
            "m 120,60 0,80 m -40,-80 0,80 m -20,-20 80,0 m 0,-40 -80,0 M 100,50 135.35534,64.64466 150,100 135.35534,135.35534 100,150.00002 64.644661,135.35534 50,100 64.644661,64.64466 z",
        ),
        style: Style::default(),
    };
    n.style.fill = Some(Paint::None);
    n.style.stroke = Some(Paint::color("rgb(0,0,255)"));
    n.style.stroke_width = Some(Num::Number(1.0));
    PartOutput::new(Vec::new(), alloc::vec![Node::Path(n)], BBox::default())
}

// ---------------------------------------------------------------------------
// Helpers shared by the parts.

impl SymbolState<'_> {
    /// Affiliation key for colour lookups (upstream `this.metadata.affiliation`).
    pub(crate) fn aff(&self) -> Option<crate::domain::Affiliation> {
        self.metadata.affiliation.known()
    }

    /// `colors.<mode>[metadata.affiliation]`.
    pub(crate) fn color_of(&self, mode: &crate::color::ColorMode) -> Option<Paint> {
        self.aff().and_then(|a| mode.for_affiliation(a)).cloned()
    }

    /// Upstream's outline colour argument.
    pub(crate) fn outline_color(&self) -> Option<Paint> {
        style_color_value(self.options.style.outline_color.as_ref(), self.aff())
    }

    /// `ms.outline(geom, outlineWidth, strokeWidth, outlineColor)`.
    pub(crate) fn outline(&self, nodes: &[Node]) -> Result<Node, RenderError> {
        let st = &self.options.style;
        outline_list(
            nodes,
            st.outline_width,
            st.stroke_width,
            &self.outline_color(),
        )
    }

    /// `ms.outline` applied to a single instruction.
    pub(crate) fn outline_one(&self, node: &Node) -> Result<Node, RenderError> {
        let st = &self.options.style;
        outline_node(
            node,
            st.outline_width,
            st.stroke_width,
            &self.outline_color(),
        )
    }
}

/// Value of a style colour for an affiliation: the string itself, or the
/// per-affiliation entry.
pub(crate) fn style_color_value(
    c: Option<&ColorChoice>,
    aff: Option<crate::domain::Affiliation>,
) -> Option<Paint> {
    match c {
        None => Some(Paint::Color(Str::Borrowed(""))),
        Some(ColorChoice::Uniform(c)) => Some(Paint::Color(c.to_str())),
        Some(ColorChoice::PerAffiliation(m)) => aff.and_then(|a| m.for_affiliation(a)).cloned(),
    }
}

/// `ms.outline` over an instruction array.
pub(crate) fn outline_list(
    nodes: &[Node],
    outline: f64,
    stroke: f64,
    color: &Option<Paint>,
) -> Result<Node, RenderError> {
    Ok(Node::Group(
        nodes
            .iter()
            .map(|n| outline_node(n, outline, stroke, color))
            .collect::<Result<_, _>>()?,
    ))
}

/// `ms.outline` over one instruction.
pub(crate) fn outline_node(
    node: &Node,
    outline: f64,
    stroke: f64,
    color: &Option<Paint>,
) -> Result<Node, RenderError> {
    let strip = |s: &Style| Style {
        fill: None,
        fill_opacity: None,
        ..s.clone()
    };
    let leaf = |s: &Style| {
        let mut st = strip(s);
        let width = if st.stroke != Some(Paint::None) {
            let w = st
                .stroke_width
                .as_ref()
                .filter(|w| js_truthy_num(w))
                .map_or(stroke, Num::value);
            w + 2.0 * outline
        } else {
            2.0 * outline
        };
        st.stroke_width = Some(Num::Number(width));
        st.stroke = color.clone();
        st.fill = Some(Paint::None);
        st.line_cap = Some(Cow::Borrowed("round"));
        st
    };
    let kids = |d: &[Node]| -> Result<Vec<Node>, RenderError> {
        d.iter()
            .map(|n| outline_node(n, outline, stroke, color))
            .collect()
    };
    Ok(match node {
        Node::Group(v) => Node::Group(kids(v)?),
        Node::Translate(n) => Node::Translate(crate::ir::TranslateNode {
            x: n.x.clone(),
            y: n.y.clone(),
            draw: kids(&n.draw)?,
            style: strip(&n.style),
        }),
        Node::Rotate(n) => Node::Rotate(crate::ir::RotateNode {
            degree: n.degree.clone(),
            x: n.x.clone(),
            y: n.y.clone(),
            draw: kids(&n.draw)?,
            style: strip(&n.style),
        }),
        Node::Scale(n) => Node::Scale(crate::ir::ScaleNode {
            factor: n.factor.clone(),
            draw: kids(&n.draw)?,
            style: strip(&n.style),
        }),
        Node::Path(n) => Node::Path(crate::ir::PathNode {
            style: leaf(&n.style),
            ..n.clone()
        }),
        Node::Circle(n) => Node::Circle(crate::ir::CircleNode {
            style: leaf(&n.style),
            ..n.clone()
        }),
        Node::Text(n) => Node::Text(crate::ir::TextNode {
            style: leaf(&n.style),
            ..n.clone()
        }),
        Node::Clip(n) => Node::Clip(crate::ir::ClipNode {
            style: leaf(&n.style),
            ..n.clone()
        }),
        Node::TrustedSvg(_) => node.clone(),
        Node::Bare(st) => Node::Bare(leaf(st)),
        Node::Scalar(_) => Node::Bare(leaf(&Style::default())),
        Node::Missing => {
            return Err(RenderError::upstream(
                "Cannot read properties of undefined (reading 'type')",
            ));
        }
    })
}

/// JavaScript truthiness of a numeric attribute.
pub(crate) fn js_truthy_num(n: &Num) -> bool {
    match n {
        Num::Number(v) => *v != 0.0 && !v.is_nan(),
        Num::Text(s) => !s.is_empty(),
        Num::Bool(b) => *b,
    }
}

/// `ms._translate(x, y, instruction)`.
pub(crate) fn translate(x: f64, y: f64, instruction: Node) -> Node {
    Node::translate(x, y, alloc::vec![instruction])
}

/// `ms._scale(factor, instruction, nonScalingStroke)`.
pub(crate) fn scale(
    factor: f64,
    mut instruction: Node,
    non_scaling: bool,
) -> Result<Node, RenderError> {
    if non_scaling {
        recurse_scale_value(&mut instruction, 1.0 / factor)?;
    }
    let inner = Node::Scale(crate::ir::ScaleNode {
        factor: Num::Number(factor),
        draw: alloc::vec![instruction],
        style: Style::default(),
    });
    Ok(Node::translate(
        100.0 - factor * 100.0,
        100.0 - factor * 100.0,
        alloc::vec![inner],
    ))
}

/// Upstream `recurse_scale` applied to a JavaScript value (an instruction or
/// an array of instructions).
fn recurse_scale_value(v: &mut Node, nss: f64) -> Result<(), RenderError> {
    match v {
        Node::Group(list) => recurse_scale_array(list, nss),
        other => set_nss(other, nss),
    }
}

fn recurse_scale_array(list: &mut [Node], nss: f64) -> Result<(), RenderError> {
    for d in list.iter_mut() {
        match d {
            // Setting a property on an array is a no-op; then each element is visited.
            Node::Group(inner) => {
                for e in inner.iter_mut() {
                    recurse_scale_value(e, nss)?;
                }
            }
            other => {
                set_nss(other, nss)?;
                let draw = match other {
                    Node::Translate(n) => Some(&mut n.draw),
                    Node::Rotate(n) => Some(&mut n.draw),
                    Node::Scale(n) => Some(&mut n.draw),
                    Node::Clip(n) => Some(&mut n.draw),
                    _ => None,
                };
                if let Some(draw) = draw {
                    recurse_scale_array(draw, nss)?;
                }
            }
        }
    }
    Ok(())
}

fn set_nss(n: &mut Node, nss: f64) -> Result<(), RenderError> {
    match n {
        Node::Missing => Err(RenderError::upstream(
            "Cannot set properties of undefined (setting 'non_scaling_stroke')",
        )),
        Node::Scalar(_) => Err(RenderError::upstream(
            "Cannot create property 'non_scaling_stroke' on primitive",
        )),
        Node::TrustedSvg(_) => Ok(()),
        other => {
            if let Some(st) = other.style_mut() {
                st.non_scaling_stroke = Some(nss);
            }
            Ok(())
        }
    }
}

/// String attribute helper.
pub(crate) fn s(v: &'static str) -> Option<Str> {
    Some(Cow::Borrowed(v))
}
