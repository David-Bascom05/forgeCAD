//! Parasolid transmit files (`.x_t`, `.x_b`; `.xmt_txt`, `.xmt_bin`), read from Siemens' published
//! Parasolid XT Format Reference. Text, neutral, typed and bare binary; the schema each file carries
//! as edits of the base SCH_13006.

mod build;
mod geom;
mod known;
mod lex;
mod nodes;
mod schema;
mod spcurve;

pub use lex::LexError;
pub use nodes::{Header, XtError, is_xt};

/// The node types a file holds and how many of each, with its header: for surveys.
pub fn census(b: &[u8], lim: &crate::Limits) -> Result<(Header, Vec<(u16, usize)>), XtError> {
    let s = nodes::read(b, lim)?;
    Ok((s.header.clone(), s.census.clone()))
}

/// A schema's name and each node type's record.
pub type SchemaRecords = (String, Vec<(u16, Vec<u8>)>);

/// An embedded-schema text file's schema number and each type's record as it carries it: what
/// `known.rs` is made from (research/parasolid/known_schemas.py).
pub fn schema_records(b: &[u8], lim: &crate::Limits) -> Result<SchemaRecords, XtError> {
    let s = nodes::read_keeping(b, lim, true)?;
    Ok((s.header.schema.clone(), s.records.clone()))
}

/// Reads a transmit file's solids, placed as its assemblies place them.
pub fn import(b: &[u8], lim: &crate::Limits) -> Result<crate::Imported, crate::ImportError> {
    build::import(b, lim, true)
}

/// As `import`, but each body where it stands in its own part, its instances' transforms not
/// applied: as a STEP read leaves an assembly's parts.
pub fn import_unplaced(b: &[u8], lim: &crate::Limits) -> Result<crate::Imported, crate::ImportError> {
    build::import(b, lim, false)
}
