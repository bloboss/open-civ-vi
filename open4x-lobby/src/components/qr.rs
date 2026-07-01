//! QR code → SVG string. Phase 5 polish helper for the Show
//! invite QR popup (Profile + tile menu).
//!
//! Self-contained: encodes the input via the `qrcode` crate (no
//! image/svg features), walks the resulting matrix once, and emits
//! a single `<svg>` element with one `<rect>` per dark module.

use qrcode::QrCode;

/// Render `text` as a square SVG QR code at `pixel_size` pixels per
/// side. Returns the raw SVG string the caller embeds in `inner_html`.
pub fn qr_svg(text: &str, pixel_size: u32) -> String {
    let code = match QrCode::new(text.as_bytes()) {
        Ok(c) => c,
        Err(_) => {
            // Encoding failed (almost always: input too long).
            // Render a placeholder square the caller can swap in for
            // real error UI.
            return format!(
                "<svg width=\"{pixel_size}\" height=\"{pixel_size}\" viewBox=\"0 0 1 1\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"1\" height=\"1\" fill=\"#fbbf24\"/></svg>"
            );
        }
    };

    let width = code.width();
    let modules: Vec<bool> = code
        .to_colors()
        .into_iter()
        .map(|c| c == qrcode::Color::Dark)
        .collect();

    // 1-module quiet zone on each side per the spec.
    let pad = 1usize;
    let total = width + pad * 2;

    let mut svg = String::new();
    use std::fmt::Write as _;
    let _ = write!(
        svg,
        "<svg width=\"{pixel_size}\" height=\"{pixel_size}\" viewBox=\"0 0 {total} {total}\" \
         xmlns=\"http://www.w3.org/2000/svg\" shape-rendering=\"crispEdges\">\
         <rect width=\"{total}\" height=\"{total}\" fill=\"#faf8f3\"/>",
    );
    for y in 0..width {
        for x in 0..width {
            if modules[y * width + x] {
                let _ = write!(
                    svg,
                    "<rect x=\"{x}\" y=\"{y}\" width=\"1\" height=\"1\" fill=\"#16170d\"/>",
                    x = x + pad,
                    y = y + pad,
                );
            }
        }
    }
    svg.push_str("</svg>");
    svg
}
