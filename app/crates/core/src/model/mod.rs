//! The document model: one element per command of a diagram file.

mod attr;
mod coord;
mod element;
mod options;

pub use attr::{AttrItem, ItemError, flatten_items, nest_items, split_type};
pub use coord::{Coord, parse_coord_list};
pub use element::{
    AttributeTree, Body, Edge, Element, ElementKey, Generalisation, Grid, Meta, Node, Note, Point, Raw,
    Setup, Subdomain, endpoint_id,
};
pub use options::Options;
