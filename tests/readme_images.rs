//! The SVGs shown in README.md are this crate's actual output.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod gallery_list;

#[test]
fn readme_images_match_renderer() {
    for item in gallery_list::ITEMS {
        let path = format!(
            "{}/docs/images/{}.svg",
            env!("CARGO_MANIFEST_DIR"),
            item.name
        );
        let Ok(committed) = std::fs::read_to_string(&path) else {
            // docs/ is not shipped in the crates.io package.
            return;
        };
        let symbol = gallery_list::render(item);
        assert!(
            symbol.is_valid(),
            "{} ({}) is invalid",
            item.name,
            item.sidc
        );
        assert_eq!(
            committed,
            symbol.to_svg(),
            "{path} is stale; run `cargo run --example readme_images`"
        );
    }
}
