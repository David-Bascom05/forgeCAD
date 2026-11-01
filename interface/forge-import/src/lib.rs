//! Other systems' part histories as ForgeCAD part files. A file is read as data: nothing in it
//! runs. Each body becomes one `.fc` history the kernel rebuilds, or says why it could not.
//!
//! FreeCAD `.FCStd` is the first; the translation is the one tools/find-parts/recreate/freecad.py
//! made, line for line.

pub mod arrangement;
pub mod fcstd;
mod py;
mod state;
mod zip;

/// Why a body did not translate whole.
#[derive(Clone, Debug, PartialEq)]
pub enum Fail {
    /// A form `.fc` has no line for.
    Unsupported(String),
    /// A reference this could not resolve against the part.
    Unresolved(String),
    /// The kernel stopped on the lines so far.
    Stop(String),
    /// Data this reader did not expect.
    Bug(String),
}

impl Fail {
    pub fn why(&self) -> &str {
        match self {
            Fail::Unsupported(s) | Fail::Unresolved(s) | Fail::Stop(s) | Fail::Bug(s) => s,
        }
    }
}

pub(crate) fn bug<T>(what: &str) -> Result<T, Fail> {
    Err(Fail::Bug(what.into()))
}

/// One body's outcome.
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    /// The feature kinds of its chain, as the file names them.
    pub features: Vec<String>,
    /// Splines read as fine polylines.
    pub approx: bool,
    /// The part file: every line when it translated, the lines before the kernel stopped when
    /// `fail` is a Stop, none otherwise.
    pub fc: Option<String>,
    pub fail: Option<Fail>,
}

impl Body {
    pub fn status(&self) -> &'static str {
        match &self.fail {
            None => "translated",
            Some(Fail::Stop(_)) => "stopped",
            Some(_) => "untranslated",
        }
    }
}
