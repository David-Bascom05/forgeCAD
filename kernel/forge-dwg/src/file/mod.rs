//! Whole files: the containers each version keeps its sections in, and the drawing they hold.

pub mod r2000;
pub mod r2004;

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

/// A file's sections, out of their container: what every version has the same way.
pub struct Sections {
    pub ver: Version,
    pub maint: u8,
    pub codepage: u16,
    /// The header variables and classes sections, sentinels and all.
    pub header: Vec<u8>,
    pub classes: Vec<u8>,
    /// The object map's bytes.
    pub handles: Vec<u8>,
    /// Where the objects are: the whole file before R2004 (map offsets are file offsets), the
    /// AcDb:AcDbObjects section since.
    pub objects: Vec<u8>,
    /// The other sections, by name, as stored.
    pub other: Vec<(String, Vec<u8>)>,
}

/// Takes a file's sections out of its container.
pub fn sections(buf: &[u8]) -> R<Sections> {
    let Some(ver) = buf.get(0..6).and_then(Version::from_magic) else {
        return bad("not a DWG file this reads (AC1015..AC1032)");
    };
    let maint = buf[11];
    let codepage = u16::from_le_bytes([buf[0x13], buf[0x14]]);
    match ver {
        Version::R2000 => r2000::sections(buf),
        Version::R2007 => bad("R2007 files are not read yet"),
        _ => {
            let l = r2004::layout(buf)?;
            let take = |name: &str| -> R<Vec<u8>> {
                r2004::section_data(buf, &l, name)?.ok_or(crate::bits::Bad(format!("no {name} section")))
            };
            let header = take("AcDb:Header")?;
            let classes = take("AcDb:Classes")?;
            let handles = take("AcDb:Handles")?;
            let objects = take("AcDb:AcDbObjects")?;
            let mut other = vec![];
            for s in &l.sections {
                if !["AcDb:Header", "AcDb:Classes", "AcDb:Handles", "AcDb:AcDbObjects"].contains(&s.name.as_str()) {
                    other.push((s.name.clone(), take(&s.name)?));
                }
            }
            Ok(Sections { ver, maint, codepage, header, classes, handles, objects, other })
        }
    }
}

/// Reads a DWG file, any version.
pub fn read(buf: &[u8]) -> R<Drawing> {
    let s = sections(buf)?;
    let classes = r2000::classes(&s.classes, s.ver, s.maint)?;
    let mut d = Drawing {
        version: Some(s.ver),
        maint: s.maint,
        codepage: s.codepage,
        header: s.header,
        classes,
        other: s.other,
        ..Drawing::default()
    };
    let list = d.class_list();
    let cl = crate::object::Classes { list: &list };
    for (handle, at) in r2000::read_map(&s.handles, 0, s.handles.len())? {
        let o = crate::object::read_object(&s.objects, at, s.ver, &cl, None)
            .map_err(|e| crate::bits::Bad(format!("{handle:#x}: {}", e.0)))?;
        d.objects.insert(handle, o);
    }
    Ok(d)
}
