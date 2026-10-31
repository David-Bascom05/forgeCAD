//! A part as a document: parameters, sketches and feature lines in their unevaluated form,
//! printed losslessly as the `.fc` part file, edited through an API, and evaluated into the
//! plain-number part history the kernel regenerates.

pub mod doc;
pub mod expr;
pub mod sketch;
pub mod text;
pub mod wire;

pub use doc::{Arg, Body, Document, Item, ItemId, Op, Row};
