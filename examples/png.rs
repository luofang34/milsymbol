//! Rasterizes a symbol to PNG bytes with `resvg` and `tiny-skia`:
//! `cargo run --example png [output.png] [sidc]` (defaults to `symbol.png`).
//!
//! `milsymbol` keeps its `no_std + alloc` core; applications
//! that want PNG or premultiplied RGBA pixels pair `Symbol::to_svg` (or
//! `Symbol::drawing` with `Drawing::viewport_transform`) with their chosen
//! rasterizer. Enable `resvg`'s default `text` feature and load system or
//! bundled fonts into `usvg::Options::fontdb_mut()` when rasterizing symbols
//! that contain `<text>` amplifiers or text icons.

use milsymbol::Renderer;
use milsymbol::options::Color;

fn render_png(sidc: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let symbol = Renderer::default()
        .symbol(sidc)
        .size(80.0)
        .outline_width(4.0)
        .outline_color(Color::WHITE)
        .render()?;
    let mut svg = String::new();
    symbol.write_svg(&mut svg);

    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg, &options)?;
    let size = symbol.size();
    let (width, height) = (size.width.ceil() as u32, size.height.ceil() as u32);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("empty symbol dimensions")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}

fn write_png_blocking(path: &str, sidc: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, render_png(sidc)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out_path = args.next();
    let sidc = args.next();
    write_png_blocking(
        out_path.as_deref().unwrap_or("symbol.png"),
        sidc.as_deref().unwrap_or("10031002161211000000"),
    )
}
