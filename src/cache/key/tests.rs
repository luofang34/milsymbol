use super::{KeyBuf, write_key};
use crate::color::ColorMode;
use crate::ir::Paint;
use crate::options::{StyleColor, SymbolOptions};
use std::vec::Vec;

fn key(o: &SymbolOptions) -> Vec<u8> {
    let mut k = KeyBuf::default();
    write_key(&mut k, "10031000001211000000", o);
    k.as_slice().to_vec()
}

/// A named change to the default options.
type Change = (&'static str, fn(&mut SymbolOptions));

#[test]
fn every_option_changes_the_key() {
    let mode = || StyleColor::PerAffiliation(ColorMode::uniform(Some(Paint::color("red"))));
    let changes: [Change; 38] = [
        ("text", |o| {
            o.set_text("uniqueDesignation", "A");
        }),
        ("direction", |o| o.direction = Some(45.0)),
        ("speed_leader", |o| o.speed_leader = 10.0),
        ("stack", |o| o.stack = Some(2.0)),
        ("country_flag", |o| o.country_flag = Some("US".into())),
        ("full_frame_flag", |o| o.full_frame_flag = Some(true)),
        ("signature", |o| o.signature = Some("!".into())),
        ("alternate_medal", |o| o.style.alternate_medal = true),
        ("civilian_color", |o| o.style.civilian_color = false),
        ("color_mode", |o| {
            o.style.color_mode = StyleColor::from("Dark")
        }),
        ("fill", |o| o.style.fill = false),
        ("fill_color", |o| o.style.fill_color = "red".into()),
        ("fill_opacity", |o| o.style.fill_opacity = 0.5),
        ("font_family", |o| o.style.font_family = "Serif".into()),
        ("frame", |o| o.style.frame = false),
        ("frame_color", |o| {
            o.style.frame_color = StyleColor::from("red")
        }),
        ("hq_staff_length", |o| o.style.hq_staff_length = 50.0),
        ("icon", |o| o.style.icon = false),
        ("icon_text_uses_font_family", |o| {
            o.style.icon_text_uses_font_family = true
        }),
        ("icon_color", |o| {
            o.style.icon_color = StyleColor::from("red")
        }),
        ("info_background", |o| {
            o.style.info_background = StyleColor::from("red")
        }),
        ("info_background_frame", |o| {
            o.style.info_background_frame = StyleColor::from("red")
        }),
        ("info_color", |o| {
            o.style.info_color = StyleColor::from("red")
        }),
        ("info_fields", |o| o.style.info_fields = false),
        ("info_outline_color", |o| {
            o.style.info_outline_color = "red".into()
        }),
        ("info_outline_width", |o| {
            o.style.info_outline_width = Some(2.0)
        }),
        ("info_size", |o| o.style.info_size = 30.0),
        ("mono_color", |o| o.style.mono_color = "red".into()),
        ("outline_color", |o| {
            o.style.outline_color = StyleColor::from("red")
        }),
        ("outline_width", |o| o.style.outline_width = 2.0),
        ("padding", |o| o.style.padding = 2.0),
        ("simple_status_modifier", |o| {
            o.style.simple_status_modifier = true
        }),
        ("size", |o| o.style.size = 50.0),
        ("square", |o| o.style.square = true),
        ("standard", |o| {
            o.style.standard = Some(crate::Standard::App6)
        }),
        ("stroke_width", |o| o.style.stroke_width = 2.0),
        ("style_fill", |o| o.style.style_fill = true),
        ("negative zero", |o| o.style.padding = -0.0),
    ];
    let base = key(&SymbolOptions::default());
    for (name, change) in changes {
        let mut o = SymbolOptions::default();
        change(&mut o);
        assert_ne!(key(&o), base, "{name}");
    }
    // Per-affiliation colours differ from a colour string.
    let mut o = SymbolOptions::default();
    o.style.icon_color = mode();
    assert_ne!(key(&o), base);
}

#[test]
fn long_text_spills_to_the_heap_losslessly() {
    let mut o = SymbolOptions::default();
    o.set_text("additionalInformation", "x".repeat(4 * KeyBuf::INLINE));
    let long = key(&o);
    assert!(long.len() > KeyBuf::INLINE);
    assert_eq!(long, key(&o));
}
