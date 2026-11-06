//! The file versions: what each writes differently.

/// A DWG version by the AutoCAD release that introduced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    /// AC1015, AutoCAD 2000.
    R2000,
    /// AC1018, AutoCAD 2004: paged and compressed sections.
    R2004,
    /// AC1021, AutoCAD 2007: Unicode, Reed-Solomon coded pages.
    R2007,
    /// AC1024, AutoCAD 2010.
    R2010,
    /// AC1027, AutoCAD 2013.
    R2013,
    /// AC1032, AutoCAD 2018.
    R2018,
}

impl Version {
    pub const ALL: [Version; 6] = [
        Version::R2000,
        Version::R2004,
        Version::R2007,
        Version::R2010,
        Version::R2013,
        Version::R2018,
    ];
    pub fn magic(self) -> &'static [u8; 6] {
        match self {
            Version::R2000 => b"AC1015",
            Version::R2004 => b"AC1018",
            Version::R2007 => b"AC1021",
            Version::R2010 => b"AC1024",
            Version::R2013 => b"AC1027",
            Version::R2018 => b"AC1032",
        }
    }
    pub fn from_magic(m: &[u8]) -> Option<Version> {
        Version::ALL.into_iter().find(|v| v.magic() == m)
    }
}
