//! Other systems' part histories as Forge (`.fg`) parts. A file is read as data: nothing in it
//! runs. Each body becomes one Forge program that builds, or says why it could not.
//!
//! FreeCAD `.FCStd` is the first: each feature read as tools/find-parts/recreate/freecad.py reads
//! it, written as Forge statements (a sketch and the feature it makes), a blend's edges named by
//! Forge selections found on the part as built so far.

pub mod arrangement;
pub mod fcstd;
mod py;
mod state;
mod zip;

/// Why a body did not translate whole.
#[derive(Clone, Debug, PartialEq)]
pub enum Fail {
    /// A form Forge has no statement for, or this reader does not read.
    Unsupported(String),
    /// A reference this could not resolve against the part.
    Unresolved(String),
    /// The program so far did not build.
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
    /// The part file: the whole program when it translated, the program as far as it built when
    /// `fail` is a Stop, none otherwise.
    pub fg: Option<String>,
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
