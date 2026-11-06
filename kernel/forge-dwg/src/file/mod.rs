//! Whole files: the containers each version keeps its sections in, and the drawing they hold.

pub mod r2000;

use crate::bits::{R, bad};
use crate::object::Object;
use crate::str::Str;
use crate::version::Version;
use std::collections::BTreeMap;

/// A class: a type numbered from 500 by its place in the list.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Class {
    pub number: i16,
    /// R2000: version; R2004+: proxy flags.
    pub flags: i16,
    pub app: Str,
    pub cpp: Str,
    pub dxf: Str,
    pub zombie: bool,
    /// 0x1F2 entities, 0x1F3 objects.
    pub item: i16,
    /// R2004+.
    pub count: i32,
    pub dwg_version: i16,
    pub maint_version: i16,
    pub unknown: [i32; 2],
}

/// A drawing as read: its version, header variables, classes and objects, and the file's other
/// sections kept as they were.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Drawing {
    pub version: Option<Version>,
    pub maint: u8,
    pub codepage: u16,
    /// The header variables' bits as stored (decoded by `header`).
    pub header: Vec<u8>,
    pub classes: Vec<Class>,
    pub objects: BTreeMap<u32, Object>,
    /// Sections kept as read: name to bytes.
    pub other: Vec<(String, Vec<u8>)>,
}

impl Drawing {
    pub fn ver(&self) -> Version {
        self.version.unwrap_or(Version::R2000)
    }
    /// (DXF name, makes entities) per class, for typing objects.
    pub fn class_list(&self) -> Vec<(String, bool)> {
        self.classes.iter().map(|c| (c.dxf.text(), c.item == 0x1F2)).collect()
    }
}

/// Reads a DWG file, any version.
pub fn read(buf: &[u8]) -> R<Drawing> {
    let Some(v) = buf.get(0..6).and_then(Version::from_magic) else {
        return bad("not a DWG file this reads (AC1015..AC1032)");
    };
    match v {
        Version::R2000 => r2000::read(buf),
        _ => bad(format!("{v:?} files are not read yet")),
    }
}
