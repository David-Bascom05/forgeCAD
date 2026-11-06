//! Whole files: the containers each version keeps its sections in, and the drawing they hold.

pub mod r2000;
pub mod r2004;
pub mod r2007;

use crate::bits::{R, bad};
use crate::header::Header;
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

/// A section kept as stored, with what its container said about it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Extra {
    pub name: String,
    pub data: Vec<u8>,
    /// R2004+: its page size, whether compressed (2) or not (1), encrypted, and the unknown long.
    pub max_page: u32,
    pub compressed: u32,
    pub encrypted: u32,
    pub unknown: u32,
    /// R2007: its hash code and encoding (1 plain, 4 RS coded and compressed); its page size,
    /// encryption and unknown value are in the fields above.
    pub hash: u32,
    pub encoding: u32,
}

/// A drawing: its version, header variables, classes and objects, and the file's other sections
/// kept as they were.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Drawing {
    pub version: Option<Version>,
    pub maint: u8,
    pub codepage: u16,
    /// The first 0x80 bytes of the file as read (application version, security flags, ..).
    pub file_header: Vec<u8>,
    pub header: Header,
    pub classes: Vec<Class>,
    pub objects: BTreeMap<u32, Object>,
    /// The other sections, as read.
    pub other: Vec<Extra>,
    /// What didn't decode (those objects are kept as read).
    pub warnings: Vec<String>,
}

impl Drawing {
    pub fn ver(&self) -> Version {
        self.version.unwrap_or(Version::R2000)
    }
    /// (DXF name, makes entities) per class, for typing objects.
    pub fn class_list(&self) -> Vec<(String, bool)> {
        self.classes
            .iter()
            .map(|c| (c.dxf.text(), c.item == 0x1F2))
            .collect()
    }
    pub fn extra(&self, name: &str) -> Option<&Extra> {
        self.other.iter().find(|e| e.name == name)
    }
    /// The next free handle.
    pub fn handseed(&self) -> u32 {
        match self.header.get("HANDSEED") {
            Some(crate::header::Val::H(h)) => h.handle,
            _ => self.objects.keys().max().map_or(1, |h| h + 1),
        }
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
    /// The other sections, as stored.
    pub other: Vec<Extra>,
    /// The first 0x80 bytes of the file.
    pub file_header: Vec<u8>,
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
        Version::R2007 => {
            let l = r2007::layout(buf)?;
            let take = |name: &str| -> R<Vec<u8>> {
                r2007::section_data(buf, &l, name)?.ok_or(crate::bits::Bad(format!("no {name} section")))
            };
            let main = ["AcDb:Header", "AcDb:Classes", "AcDb:Handles", "AcDb:AcDbObjects"];
            let mut other = vec![];
            for s in &l.sections {
                if !s.name.is_empty() && !main.contains(&s.name.as_str()) {
                    let mut data = take(&s.name)?;
                    if let Some(at) = l.section_address(&s.name).filter(|_| s.name == "AcDb:Preview") {
                        // kept with addresses from the section's start
                        r2000::relocate_preview(&mut data, -(at as i64))?;
                    }
                    other.push(Extra {
                        name: s.name.clone(),
                        data,
                        max_page: s.max_size as u32,
                        encrypted: s.encryption as u32,
                        unknown: s.unknown as u32,
                        hash: s.hash as u32,
                        encoding: s.encoding as u32,
                        ..Extra::default()
                    });
                }
            }
            let [header, classes, handles, objects] = main.map(take);
            Ok(Sections {
                ver,
                maint,
                codepage,
                header: header?,
                classes: classes?,
                handles: handles?,
                objects: objects?,
                other,
                file_header: buf[..0x80.min(buf.len())].to_vec(),
            })
        }
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
                if !["AcDb:Header", "AcDb:Classes", "AcDb:Handles", "AcDb:AcDbObjects"]
                    .contains(&s.name.as_str())
                {
                    other.push(Extra {
                        name: s.name.clone(),
                        data: take(&s.name)?,
                        max_page: s.max_page,
                        compressed: s.compressed,
                        encrypted: s.encrypted,
                        unknown: s.unknown,
                        ..Extra::default()
                    });
                }
            }
            let file_header = buf[..0x80.min(buf.len())].to_vec();
            Ok(Sections {
                ver,
                maint,
                codepage,
                header,
                classes,
                handles,
                objects,
                other,
                file_header,
            })
        }
    }
}

/// Reads a DWG file, any version.
pub fn read(buf: &[u8]) -> R<Drawing> {
    let s = sections(buf)?;
    let classes = r2000::classes(&s.classes, s.ver, s.maint)?;
    let header = crate::header::read_header(&s.header, s.ver, s.maint)?;
    let mut d = Drawing {
        version: Some(s.ver),
        maint: s.maint,
        codepage: s.codepage,
        file_header: s.file_header,
        header,
        classes,
        other: s.other,
        ..Drawing::default()
    };
    let list = d.class_list();
    let cl = crate::object::Classes { list: &list };
    for (handle, at) in r2000::read_map(&s.handles, 0, s.handles.len())? {
        let (o, warn) = crate::object::read_object_lenient(&s.objects, at, s.ver, &cl)
            .map_err(|e| crate::bits::Bad(format!("{handle:#x}: {}", e.0)))?;
        if let Some(w) = warn {
            d.warnings.push(format!(
                "{handle:#x} ({}): {w}",
                cl.name(o.type_code).unwrap_or("?")
            ));
        }
        d.objects.insert(handle, o);
    }
    Ok(d)
}

/// Writes a drawing in its own version.
pub fn write(d: &Drawing) -> R<Vec<u8>> {
    match d.ver() {
        Version::R2000 => r2000::write(d),
        Version::R2007 => r2007::write(d),
        _ => r2004::write(d),
    }
}
