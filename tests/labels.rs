//! Public label constructors and custom placement bounds.

use milsymbol::labels::{Label, LabelField};
use milsymbol::options::TextField;
use milsymbol::{IconExtension, Renderer};
use std::collections::BTreeMap;

struct Placements(Label);

impl IconExtension for Placements {
    fn letter_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        out.insert("S-G-UCI---".into(), self.fields());
    }

    fn number_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        for entity in ["130100", "121100"] {
            out.insert(entity.into(), self.fields());
        }
    }
}

impl Placements {
    fn fields(&self) -> Vec<LabelField> {
        vec![LabelField::for_field(
            &TextField::UniqueDesignation,
            vec![self.0.clone()],
        )]
    }
}

#[test]
fn default_label_placement_includes_text_in_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let mut omitted = Label::default();
    omitted.x = Some(300.0);
    omitted.y = Some(-300.0);
    for label in [Label::at(300.0, -300.0), omitted] {
        check_bounds(label)?;
    }
    Ok(())
}

fn check_bounds(label: Label) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::builder().icons(Placements(label)).build();
    for sidc in ["SFGPUCI-----", "10032500001301000000"] {
        let symbol = renderer
            .symbol(sidc)
            .text(TextField::UniqueDesignation, "VISIBLE")
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

#[test]
fn omitted_label_coordinates_use_the_origin() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = Renderer::builder()
        .icons(Placements(Label::default()))
        .build()
        .symbol("SFGPUCI-----")
        .text(TextField::UniqueDesignation, "VISIBLE")
        .render()?;
    assert!(
        symbol
            .to_svg()
            .contains("x=\"0\" y=\"0\" text-anchor=\"start\" font-size=\"12\"")
    );
    assert_eq!(symbol.bounding_box().x1, 0.0);
    assert_eq!(symbol.bounding_box().y1, -12.0);
    Ok(())
}

#[test]
fn numeric_unit_labels_keep_the_standard_layout() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::builder()
        .icons(Placements(Label::at(300.0, -300.0)))
        .build();
    let render = |r: &Renderer| {
        r.symbol("10031000001211000000")
            .text(TextField::UniqueDesignation, "VISIBLE")
            .render()
    };
    let custom = render(&renderer)?;
    let standard = render(&Renderer::default())?;
    assert_eq!(custom.to_svg(), standard.to_svg());
    assert_eq!(custom.bounding_box(), standard.bounding_box());
    Ok(())
}
