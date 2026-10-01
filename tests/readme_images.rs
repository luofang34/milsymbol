//! The SVGs shown in README.md are this crate's actual output plus a white
//! background rectangle.

mod gallery_list;

#[test]
fn readme_images_match_renderer() -> Result<(), Box<dyn std::error::Error>> {
    for item in gallery_list::ITEMS {
        let path = format!(
            "{}/docs/images/{}.svg",
            env!("CARGO_MANIFEST_DIR"),
            item.name
        );
        let Ok(committed) = std::fs::read_to_string(&path) else {
            // docs/ is not shipped in the crates.io package.
            return Ok(());
        };
        let symbol = gallery_list::render(item)?;
        assert!(
            symbol.sidc_validity().is_valid(),
            "{} ({}) is invalid",
            item.name,
            item.sidc
        );
        let svg = symbol.to_svg();
        assert_eq!(
            Some(committed.clone()),
            gallery_list::with_background(&svg),
            "{path} is stale; run `cargo run --example readme_images`"
        );
        let background = gallery_list::background(&svg).ok_or("svg without viewBox")?;
        let without = committed.replacen(&background, "", 1);
        assert_eq!(
            without, svg,
            "{path} differs from Symbol::to_svg() beyond the background"
        );
    }
    Ok(())
}
