//! Bounding boxes in symbol units.

/// An axis-aligned bounding box (upstream `ms.BBox`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    /// Left.
    pub x1: f64,
    /// Top.
    pub y1: f64,
    /// Right.
    pub x2: f64,
    /// Bottom.
    pub y2: f64,
}

impl Default for BBox {
    /// Upstream's default: every coordinate at the symbol centre (100).
    fn default() -> Self {
        BBox {
            x1: 100.0,
            y1: 100.0,
            x2: 100.0,
            y2: 100.0,
        }
    }
}

/// A bounding box whose coordinates may be missing, as returned by symbol
/// parts that only extend some edges.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PartialBBox {
    /// Left.
    pub x1: Option<f64>,
    /// Top.
    pub y1: Option<f64>,
    /// Right.
    pub x2: Option<f64>,
    /// Bottom.
    pub y2: Option<f64>,
}

impl From<BBox> for PartialBBox {
    fn from(b: BBox) -> Self {
        PartialBBox {
            x1: Some(b.x1),
            y1: Some(b.y1),
            x2: Some(b.x2),
            y2: Some(b.y2),
        }
    }
}

impl PartialBBox {
    /// Completes missing coordinates with upstream's default of 100.
    pub fn complete(self) -> BBox {
        BBox {
            x1: self.x1.unwrap_or(100.0),
            y1: self.y1.unwrap_or(100.0),
            x2: self.x2.unwrap_or(100.0),
            y2: self.y2.unwrap_or(100.0),
        }
    }
}

impl BBox {
    /// Width.
    pub fn width(&self) -> f64 {
        self.x2 - self.x1
    }

    /// Height.
    pub fn height(&self) -> f64 {
        self.y2 - self.y1
    }

    /// Grows the box to include `other`'s present coordinates.
    ///
    /// Comparisons follow upstream (`box.x1 <= this.x1`), so a missing or NaN
    /// coordinate never changes the box.
    pub fn merge(&mut self, other: PartialBBox) {
        if other.x1.is_some_and(|v| v <= self.x1) {
            self.x1 = other.x1.unwrap_or(self.x1);
        }
        if other.y1.is_some_and(|v| v <= self.y1) {
            self.y1 = other.y1.unwrap_or(self.y1);
        }
        if other.x2.is_some_and(|v| v >= self.x2) {
            self.x2 = other.x2.unwrap_or(self.x2);
        }
        if other.y2.is_some_and(|v| v >= self.y2) {
            self.y2 = other.y2.unwrap_or(self.y2);
        }
    }
}
