//! The core of ERDTeX: everything that doesn't need a window.  No I/O and no threads,
//! so that it builds for the desktop and for WebAssembly alike.
//!
//! Modules, in the direction of their dependencies:
//!
//! * [`text`]: string helpers that behave like the Python functions the format began with
//! * [`texscan`]: the scanner for the file syntax (a TeX-like subset)
//! * [`model`]: elements, coordinates, options, attribute items
//! * [`document`]: a diagram file: reading, lossless writing, editing; the table of commands
//! * [`latex_text`]: approximate typesetting of labels
//! * [`colors`], [`style`]: xcolor expressions; the packages' style keys
//! * [`fonts`]: text widths from the bundled Latin Modern fonts
//! * [`geometry`]: node shapes and edge segments as TikZ computes them
//! * [`cardinality`], [`generalisation`]: what labels and generalisation types mean
//! * [`rules`]: the modelling rules: what the editor refuses, and what a check reports
//! * [`layout`]: the layout engine: a document as drawing primitives
//! * [`svg`]: a layout as SVG (with the dark canvas's colours)
//! * [`history`], [`session`]: one open diagram with its undo history
//! * [`editor`]: editing: tools, selection, dragging, handles, the clipboard
//! * [`panel`]: the properties panel as data: the form for the selection and its changes
//! * [`problems`]: the Problems panel and the badges on the canvas
//! * [`subdomains`]: which box an element lies in
//! * [`export`]: the LaTeX job's wrapper and errors, the editor rendering as SVG
//! * [`relational`]: ER diagram -> relational schema, and its text, LaTeX and SQL forms
//! * [`gsm2er`]: GSM diagram -> ER diagram
//! * [`rmwindow`]: the Relational Schema / SQL window: decisions, data types, output
//! * [`newdefaults`]: what new diagrams start with
//! * [`shortcuts`]: the table of commands and their keys
//!
//! The core never panics on its input: errors are values (see the lints below).

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

pub mod arrange;
pub mod cardinality;
pub mod colors;
pub mod document;
pub mod editor;
pub mod export;
pub mod fonts;
pub mod generalisation;
pub mod geometry;
pub mod gsm2er;
pub mod history;
pub mod latex_text;
pub mod layout;
pub mod lineage;
pub mod model;
pub mod newdefaults;
pub mod panel;
pub mod problems;
pub mod relational;
pub mod rmwindow;
pub mod rules;
pub mod session;
pub mod shortcuts;
pub mod style;
pub mod subdomains;
pub mod svg;
pub mod texscan;
pub mod text;
