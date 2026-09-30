//! Affine transforms.

use crate::ir::Point;

/// A 2D affine transform. A point `(x, y)` maps to
/// `(a·x + c·y + e, b·x + d·y + f)`, the same layout as SVG's `matrix()`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Horizontal scale and rotation term.
    pub a: f64,
    /// Vertical shear term.
    pub b: f64,
    /// Horizontal shear term.
    pub c: f64,
    /// Vertical scale and rotation term.
    pub d: f64,
    /// Horizontal translation.
    pub e: f64,
    /// Vertical translation.
    pub f: f64,
}

impl Transform {
    /// The identity.
    pub const IDENTITY: Transform = Transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// A translation.
    pub fn translate(x: f64, y: f64) -> Self {
        Transform {
            e: x,
            f: y,
            ..Transform::IDENTITY
        }
    }

    /// A uniform scale about the origin.
    pub fn scale(factor: f64) -> Self {
        Transform {
            a: factor,
            d: factor,
            ..Transform::IDENTITY
        }
    }

    /// A rotation by `degrees` about `(x, y)`, clockwise on screen.
    pub fn rotate(degrees: f64, x: f64, y: f64) -> Self {
        let (sin, cos) = libm::sincos(degrees.to_radians());
        let rotation = Transform {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        };
        Transform::translate(x, y)
            .then(rotation)
            .then(Transform::translate(-x, -y))
    }

    /// The transform that applies `inner` first and `self` second.
    pub fn then(self, inner: Transform) -> Transform {
        Transform {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }

    /// Maps a point.
    pub fn apply(&self, p: Point) -> Point {
        Point {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

impl Transform {
    /// Maps a segment. Only similarity transforms (translation, rotation,
    /// uniform scale), the ones symbols use, keep an arc an arc.
    pub(crate) fn map_segment(&self, seg: &crate::ir::Segment) -> crate::ir::Segment {
        use crate::ir::Segment;
        match *seg {
            Segment::MoveTo(p) => Segment::MoveTo(self.apply(p)),
            Segment::LineTo(p) => Segment::LineTo(self.apply(p)),
            Segment::QuadTo { ctrl, to } => Segment::QuadTo {
                ctrl: self.apply(ctrl),
                to: self.apply(to),
            },
            Segment::CubicTo { ctrl1, ctrl2, to } => Segment::CubicTo {
                ctrl1: self.apply(ctrl1),
                ctrl2: self.apply(ctrl2),
                to: self.apply(to),
            },
            Segment::ArcTo {
                rx,
                ry,
                rotation,
                large_arc,
                sweep,
                to,
            } => {
                let det = self.a * self.d - self.b * self.c;
                let scale = libm::sqrt(det.abs());
                Segment::ArcTo {
                    rx: rx * scale,
                    ry: ry * scale,
                    rotation: rotation + libm::atan2(self.b, self.a).to_degrees(),
                    large_arc,
                    sweep: if det < 0.0 { !sweep } else { sweep },
                    to: self.apply(to),
                }
            }
            Segment::Close => Segment::Close,
        }
    }
}
