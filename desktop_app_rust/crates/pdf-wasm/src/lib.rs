//! The website's PDF export: the editor's SVG (its text already as outlines) as PDF, by the same
//! `svg2pdf` as the desktop app (`src-tauri/src/export.rs`), so that both give the same file.
//! Loaded by the page only when a PDF is exported.

use wasm_bindgen::prelude::*;

/// An SVG as PDF; its size in pt stays the page's size.
#[wasm_bindgen]
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, JsError> {
    let tree = svg2pdf::usvg::Tree::from_str(svg, &svg2pdf::usvg::Options::default())
        .map_err(|e| JsError::new(&e.to_string()))?;
    // usvg reads pt as 4/3 px; 96 "dpi" turns them back into pt
    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions { dpi: 96.0 })
        .map_err(|e| JsError::new(&format!("{e:?}")))
}
