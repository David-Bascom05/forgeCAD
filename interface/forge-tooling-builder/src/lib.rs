//! forge-tooling's records as library kinds (research/cam/library-builder.md): a tool, holder,
//! collet, collet nut, pull stud, insert or turning holder kept at the project's
//! `cam/<folder>/`, the user's `<config>/forgecad/cam/<folder>/` or built in, derived from any
//! record the levels see (forge_tooling::build::derive) and checked (build::check).
//!
//! A record is its TOML table as read plus the typed record forge-tooling makes of it; the
//! writer writes the table canonically (`id` first, then the keys in order, then the nested
//! tables), so `write(parse(write(r))) == write(r)` and `parse(write(r)) == r`.
pub mod cli;
pub mod write;

use forge_library::{Catalog, Kind, Level};
use forge_tooling::Library;
use forge_tooling::build::Record;
use forge_tooling::toml::Table;
use std::path::Path;

/// One record: its table and what forge-tooling reads it as.
#[derive(Clone, Debug, PartialEq)]
pub struct Rec {
    pub id: String,
    pub table: Table,
    pub typed: Record,
}

/// The typed record of `kind` and `id` in a library.
pub fn typed(lib: &Library, kind: &str, id: &str) -> Option<Record> {
    fn find<T: Clone>(v: &[T], id: &str, of: impl Fn(&T) -> &str) -> Option<T> {
        v.iter().find(|x| of(x) == id).cloned()
    }
    Some(match kind {
        "tool" => Record::Tool(find(&lib.tools, id, |x| &x.id)?),
        "holder" => Record::Holder(find(&lib.holders, id, |x| &x.id)?),
        "collet" => Record::Collet(find(&lib.collets, id, |x| &x.id)?),
        "collet_nut" => Record::ColletNut(find(&lib.collet_nuts, id, |x| &x.id)?),
        "pull_stud" => Record::PullStud(find(&lib.pull_studs, id, |x| &x.id)?),
        "insert" => Record::Insert(find(&lib.inserts, id, |x| &x.id)?),
        "turn_holder" => Record::TurnHolder(find(&lib.turn_holders, id, |x| &x.id)?),
        _ => return None,
    })
}

/// The records of `kind` in a file's text.
pub fn parse_kind(kind: &str, text: &str) -> Result<Vec<Rec>, String> {
    let mut lib = Library::default();
    lib.add("file", text).map_err(|e| e.to_string())?;
    lib.sources
        .iter()
        .filter(|(k, _, _)| k == kind)
        .map(|(_, id, t)| {
            Ok(Rec {
                id: id.clone(),
                table: t.clone(),
                typed: typed(&lib, kind, id).ok_or_else(|| format!("{kind} `{id}` did not read"))?,
            })
        })
        .collect()
}

/// The built-in records of `kind`, as one file of them (forge-tooling embeds its vendor files;
/// the library keeps each record's table, which is what is written here).
pub fn builtin_of(kind: &str) -> Vec<(String, String)> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type Texts = Mutex<HashMap<String, Vec<(String, String)>>>;
    static DONE: OnceLock<Texts> = OnceLock::new();
    let done = DONE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = done.lock().expect("no panics while held").get(kind) {
        return v.clone();
    }
    let v = builtin_text(kind);
    done.lock()
        .expect("no panics while held")
        .insert(kind.to_string(), v.clone());
    v
}

fn builtin_lib() -> &'static Library {
    static LIB: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    LIB.get_or_init(Library::builtin)
}

fn builtin_text(kind: &str) -> Vec<(String, String)> {
    let lib = builtin_lib();
    let recs: Vec<Rec> = lib
        .sources
        .iter()
        .filter(|(k, _, _)| k == kind)
        .filter_map(|(_, id, t)| {
            Some(Rec {
                id: id.clone(),
                table: t.clone(),
                typed: typed(lib, kind, id)?,
            })
        })
        .collect();
    if recs.is_empty() {
        return Vec::new();
    }
    let text = write::file(kind, "", &recs).expect("the built-in records write");
    vec![(format!("{kind}s.toml"), text)]
}

macro_rules! kinds {
    ($($ty:ident, $word:literal, $folder:literal, $title:literal;)*) => {
        $(
            #[doc = concat!("The `", $word, "` kind.")]
            pub struct $ty;
            impl Kind for $ty {
                type Record = Rec;
                const WORD: &'static str = $word;
                const FOLDER: &'static str = $folder;
                fn builtin() -> Vec<(String, String)> {
                    builtin_of($word)
                }
                fn parse(text: &str) -> Result<Vec<Rec>, String> {
                    parse_kind($word, text)
                }
                fn write(header: &str, rs: &[Rec]) -> Result<String, String> {
                    write::file($word, header, rs)
                }
                fn id(r: &Rec) -> &str {
                    &r.id
                }
            }
        )*
        /// Every kind: (TOML key, folder, title).
        pub const KINDS: &[(&str, &str, &str)] = &[$(($word, $folder, $title)),*];
    };
}

kinds! {
    Tools, "tool", "tools", "Cutting tool";
    Holders, "holder", "holders", "Holder";
    Collets, "collet", "collets", "Collet";
    ColletNuts, "collet_nut", "collet_nuts", "Collet nut";
    PullStuds, "pull_stud", "pull_studs", "Pull stud";
    Inserts, "insert", "inserts", "Turning insert";
    TurnHolders, "turn_holder", "turn_holders", "Turning holder";
}

/// Every record the levels see, as one forge-tooling Library (each id once: the project's over
/// the user's over the built-in), for derive, the checks and CAM. Also the files that did not
/// load, by kind.
pub fn library(
    project: Option<&Path>,
    user_cam: Option<&Path>,
) -> (Library, Vec<(String, Level, String, String)>) {
    let mut lib = builtin_lib().clone();
    let mut errors = Vec::new();
    // replace each kind's records by the catalog's (which has the levels applied)
    macro_rules! level {
        ($ty:ty, $word:literal, $vec:ident, $variant:ident) => {{
            let user = user_cam.map(|u| u.join(<$ty as Kind>::FOLDER));
            let cat = Catalog::<$ty>::load(project, user.as_deref());
            for (l, f, e) in cat.errors {
                errors.push(($word.to_string(), l, f, e));
            }
            lib.$vec.clear();
            lib.sources.retain(|(k, _, _)| k != $word);
            for e in cat.entries {
                if let Record::$variant(x) = &e.def.typed {
                    lib.$vec.push(x.clone());
                }
                lib.sources
                    .push(($word.into(), e.def.id.clone(), e.def.table.clone()));
            }
        }};
    }
    level!(Tools, "tool", tools, Tool);
    level!(Holders, "holder", holders, Holder);
    level!(Collets, "collet", collets, Collet);
    level!(ColletNuts, "collet_nut", collet_nuts, ColletNut);
    level!(PullStuds, "pull_stud", pull_studs, PullStud);
    level!(Inserts, "insert", inserts, Insert);
    level!(TurnHolders, "turn_holder", turn_holders, TurnHolder);
    (lib, errors)
}

/// The user's `cam` folder: `<config>/forgecad/cam` (each kind in its own folder under it).
pub fn user_cam() -> Option<std::path::PathBuf> {
    Some(forge_dirs::config()?.join("forgecad").join("cam"))
}
