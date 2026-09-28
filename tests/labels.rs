//! Public label constructors and custom placement bounds.

use milsymbol::labels::{Label, LabelField};
use milsymbol::options::field;
use milsymbol::{IconExtension, Renderer};
use std::collections::BTreeMap;

struct OutsideFrame;

impl IconExtension for OutsideFrame {
    fn letter_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        out.insert("S-G-UCI---".into(), placements());
    }

    fn number_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        out.insert("130100".into(), placements());
    }
}

fn placements() -> Vec<LabelField> {
    vec![LabelField::new(
        field::UNIQUE_DESIGNATION,
        vec![Label::at(300.0, -300.0)],
    )]
}

#[test]
fn default_label_placement_includes_text_in_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default().with_icons(OutsideFrame);
    for sidc in ["SFGPUCI-----", "10032500001301000000"] {
        let symbol = renderer
            .symbol(sidc)
            .text(field::UNIQUE_DESIGNATION, "VISIBLE")
            .render()?;
        let svg = symbol.to_svg();
        assert!(svg.contains("x=\"300\" y=\"-300\" text-anchor=\"start\" font-size=\"12\""));
        assert!(svg.contains(">VISIBLE</text>"));
        let bbox = symbol.bounding_box();
        assert!(bbox.x2 > 300.0, "{sidc}: {bbox:?}");
        assert_eq!(bbox.y1, -312.0, "{sidc}: {bbox:?}");
    }
    Ok(())
}
