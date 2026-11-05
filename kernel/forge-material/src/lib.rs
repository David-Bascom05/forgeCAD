//! Engineering materials as data: density for mass, and the mechanical and thermal values a
//! drawing or a check reads, each value with its source. ForgeCAD's own library
//! (`data/materials.toml`) is read-only; a project's (`materials/*.toml` beside its files) and
//! the user's (`~/.config/forgecad/materials/*.toml`) are the same format, searched first.
//! Names are found without regard to case, spaces or punctuation (`6061 t6` is `6061-T6`), and
//! through aliases.
//!
//! The format, a subset of TOML: `[[material]]` records of `key = value` lines, a value a string,
//! a number, or a list of strings or of numbers. Keys: `name`, `aliases`, `family`, `density`
//! (kg/m^3, required), `youngs` (GPa), `poisson`, `yield` and `uts` (MPa), `cte` (1e-6/K),
//! `colour` (#RRGGBB), `source` (required: it covers every value), `note`, `from` (the material
//! it was made from). For one value: `source_<key>` (its own source) and `kind_<key>` (`cited`,
//! `user` or `stand-in`). Other tools' values are dotted keys (`thermo.window = [140, 180]`),
//! kept as written (`Material::extra`), with their own `source_` and `kind_` keys.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub const DATA: &str = include_str!("../data/materials.toml");

/// A value as written.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Strs(Vec<String>),
    Nums(Vec<f64>),
}

impl Value {
    fn toml(&self) -> String {
        let q = |s: &str| format!("{s:?}");
        match self {
            Value::Str(s) => q(s),
            Value::Num(v) => num_text(*v),
            Value::Strs(v) => format!("[{}]", v.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")),
            Value::Nums(v) => format!(
                "[{}]",
                v.iter().map(|x| num_text(*x)).collect::<Vec<_>>().join(", ")
            ),
        }
    }
}

fn num_text(v: f64) -> String {
    format!("{v}")
}

/// Where a value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A published source says so.
    Cited,
    /// Someone set it, for their own reasons (a supplier's lot, a shop's measurement).
    User,
    /// A placeholder until a source is found.
    StandIn,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Cited => "cited",
            Kind::User => "user",
            Kind::StandIn => "stand-in",
        }
    }

    fn from_word(w: &str) -> Option<Kind> {
        [Kind::Cited, Kind::User, Kind::StandIn]
            .into_iter()
            .find(|k| k.word() == w)
    }
}

/// The core values' keys, in the order a record writes them.
pub const VALUES: [&str; 6] = ["density", "youngs", "poisson", "yield", "uts", "cte"];

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    pub aliases: Vec<String>,
    pub family: String,
    /// kg/m^3.
    pub density: f64,
    /// GPa.
    pub youngs: Option<f64>,
    pub poisson: Option<f64>,
    /// MPa.
    pub yield_mpa: Option<f64>,
    pub uts: Option<f64>,
    /// 1e-6 / K.
    pub cte: Option<f64>,
    /// sRGB, 0..255.
    pub colour: [u8; 3],
    pub source: String,
    /// Sources of single values that are not `source`'s, by key.
    pub sources: Vec<(String, String)>,
    /// Values that are not cited, by key.
    pub kinds: Vec<(String, Kind)>,
    pub note: Option<String>,
    /// The material this one was made from (duplicated, then changed).
    pub from: Option<String>,
    /// Other tools' values (dotted keys), as written.
    pub extra: Vec<(String, Value)>,
}

impl Material {
    /// kg per mm^3, the factor from a volume in mm^3 to a mass in kg.
    pub fn kg_per_mm3(&self) -> f64 {
        self.density * 1e-9
    }

    /// Where a value comes from: its own source, or the record's.
    pub fn source_of(&self, key: &str) -> &str {
        self.sources
            .iter()
            .find(|s| s.0 == key)
            .map_or(self.source.as_str(), |s| s.1.as_str())
    }

    pub fn kind_of(&self, key: &str) -> Kind {
        self.kinds
            .iter()
            .find(|k| k.0 == key)
            .map_or(Kind::Cited, |k| k.1)
    }

    pub fn extra(&self, key: &str) -> Option<&Value> {
        self.extra.iter().find(|x| x.0 == key).map(|x| &x.1)
    }

    /// A core value by its key.
    pub fn value(&self, key: &str) -> Option<f64> {
        match key {
            "density" => Some(self.density),
            "youngs" => self.youngs,
            "poisson" => self.poisson,
            "yield" => self.yield_mpa,
            "uts" => self.uts,
            "cte" => self.cte,
            _ => None,
        }
    }

    pub fn set_value(&mut self, key: &str, v: Option<f64>) {
        match key {
            "density" => self.density = v.unwrap_or(self.density),
            "youngs" => self.youngs = v,
            "poisson" => self.poisson = v,
            "yield" => self.yield_mpa = v,
            "uts" => self.uts = v,
            "cte" => self.cte = v,
            _ => {}
        }
    }

    /// A value's source and kind set (a cited value with the record's source keeps no keys).
    pub fn set_source(&mut self, key: &str, source: &str, kind: Kind) {
        self.sources.retain(|s| s.0 != key);
        self.kinds.retain(|s| s.0 != key);
        if source != self.source {
            self.sources.push((key.into(), source.into()));
        }
        if kind != Kind::Cited {
            self.kinds.push((key.into(), kind));
        }
    }

    /// The record in the library format.
    pub fn to_toml(&self) -> String {
        let q = |s: &str| format!("{s:?}");
        let mut out = String::from("[[material]]\n");
        let mut line = |k: &str, v: String| out.push_str(&format!("{k} = {v}\n"));
        line("name", q(&self.name));
        line("aliases", Value::Strs(self.aliases.clone()).toml());
        line("family", q(&self.family));
        if let Some(f) = &self.from {
            line("from", q(f));
        }
        for k in VALUES {
            if let Some(v) = self.value(k) {
                line(k, num_text(v));
            }
        }
        line(
            "colour",
            q(&format!(
                "#{:02X}{:02X}{:02X}",
                self.colour[0], self.colour[1], self.colour[2]
            )),
        );
        for (k, v) in &self.extra {
            line(k, v.toml());
        }
        line("source", q(&self.source));
        for (k, s) in &self.sources {
            line(&format!("source_{k}"), q(s));
        }
        for (k, s) in &self.kinds {
            line(&format!("kind_{k}"), q(s.word()));
        }
        if let Some(n) = &self.note {
            line("note", q(n));
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub why: String,
}

fn string(s: &str) -> Option<(String, &str)> {
    let s = s.strip_prefix('"')?;
    let mut out = String::new();
    let mut it = s.char_indices();
    while let Some((i, c)) = it.next() {
        match c {
            '"' => return Some((out, &s[i + 1..])),
            '\\' => out.push(match it.next()?.1 {
                'n' => '\n',
                't' => '\t',
                c => c,
            }),
            c => out.push(c),
        }
    }
    None
}

/// What follows a value: nothing but a comment.
fn rest_ok(s: &str) -> bool {
    let s = s.trim();
    s.is_empty() || s.starts_with('#')
}

fn number(s: &str) -> Option<(f64, &str)> {
    let end = s.find([',', ']', '#', ' ']).unwrap_or(s.len());
    let v: f64 = s[..end].trim().parse().ok()?;
    v.is_finite().then_some((v, &s[end..]))
}

fn value(s: &str) -> Option<Value> {
    let s = s.trim();
    if s.starts_with('"') {
        let (v, rest) = string(s)?;
        return rest_ok(rest).then_some(Value::Str(v));
    }
    if let Some(mut s) = s.strip_prefix('[') {
        let (mut strs, mut nums) = (vec![], vec![]);
        loop {
            s = s.trim_start();
            if let Some(r) = s.strip_prefix(']') {
                if !rest_ok(r) || (!strs.is_empty() && !nums.is_empty()) {
                    return None;
                }
                return Some(if nums.is_empty() {
                    Value::Strs(strs)
                } else {
                    Value::Nums(nums)
                });
            }
            let r = if s.starts_with('"') {
                let (v, r) = string(s)?;
                strs.push(v);
                r
            } else {
                let (v, r) = number(s)?;
                nums.push(v);
                r
            };
            s = r.trim_start();
            s = s.strip_prefix(',').unwrap_or(s);
        }
    }
    let (v, rest) = number(s)?;
    rest_ok(rest).then_some(Value::Num(v))
}

fn colour(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#').filter(|h| h.len() == 6)?;
    let b = |k: usize| u8::from_str_radix(&h[k..k + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

/// A record's line: where, its key, its value.
type Line = (usize, String, Value);

fn record(keys: Vec<Line>, at: usize) -> Result<Material, ParseError> {
    let err = |line: usize, why: String| ParseError { line, why };
    let find = |k: &str| keys.iter().find(|x| x.1 == k);
    let text = |k: &str| -> Result<Option<String>, ParseError> {
        match find(k) {
            None => Ok(None),
            Some((_, _, Value::Str(s))) => Ok(Some(s.clone())),
            Some((l, _, _)) => Err(err(*l, format!("{k} is text"))),
        }
    };
    let num = |k: &str| -> Result<Option<f64>, ParseError> {
        match find(k) {
            None => Ok(None),
            Some((_, _, Value::Num(v))) if *v > 0.0 => Ok(Some(*v)),
            Some((l, _, _)) => Err(err(*l, format!("{k} is a positive number"))),
        }
    };
    let name = text("name")?.ok_or(err(at, "a material has a name".into()))?;
    let need = |k: &str, v: Option<String>| v.ok_or(err(at, format!("{name} has no {k}")));
    let aliases = match find("aliases") {
        None => vec![],
        Some((_, _, Value::Strs(v))) => v.clone(),
        Some((l, _, _)) => return Err(err(*l, "aliases is a list of text".into())),
    };
    let col = need("colour", text("colour")?)?;
    let (mut sources, mut kinds, mut extra) = (vec![], vec![], vec![]);
    const KNOWN: [&str; 13] = [
        "name", "aliases", "family", "density", "youngs", "poisson", "yield", "uts", "cte", "colour",
        "source", "note", "from",
    ];
    let valued = |of: &str| VALUES.contains(&of) || (of.contains('.') && find(of).is_some());
    for (l, k, v) in &keys {
        if let Some(of) = k.strip_prefix("source_") {
            if !valued(of) {
                return Err(err(*l, format!("{k}: no value {of} to source")));
            }
            match v {
                Value::Str(s) => sources.push((of.to_string(), s.clone())),
                _ => return Err(err(*l, format!("{k} is text"))),
            }
        } else if let Some(of) = k.strip_prefix("kind_") {
            let kind = match v {
                Value::Str(s) => Kind::from_word(s),
                _ => None,
            };
            match kind {
                Some(kind) if valued(of) => kinds.push((of.to_string(), kind)),
                _ => return Err(err(*l, format!("{k}: cited, user or stand-in, of a value given"))),
            }
        } else if k.contains('.') {
            extra.push((k.clone(), v.clone()));
        } else if !KNOWN.contains(&k.as_str()) {
            return Err(err(*l, format!("unknown key {k}")));
        }
    }
    Ok(Material {
        aliases,
        family: need("family", text("family")?)?,
        density: num("density")?.ok_or(err(at, format!("{name} has no density")))?,
        youngs: num("youngs")?,
        poisson: num("poisson")?,
        yield_mpa: num("yield")?,
        uts: num("uts")?,
        cte: num("cte")?,
        colour: colour(&col).ok_or(err(at, format!("{name}: colour is #RRGGBB")))?,
        source: need("source", text("source")?)?,
        sources,
        kinds,
        note: text("note")?,
        from: text("from")?,
        extra,
        name,
    })
}

/// Reads a library.
pub fn parse(src: &str) -> Result<Vec<Material>, ParseError> {
    let mut out = vec![];
    let mut cur: Option<(usize, Vec<Line>)> = None;
    for (k, raw) in src.lines().enumerate() {
        let line = k + 1;
        let l = raw.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if l == "[[material]]" {
            if let Some((at, keys)) = cur.take() {
                out.push(record(keys, at)?);
            }
            cur = Some((line, vec![]));
            continue;
        }
        let Some((key, v)) = l.split_once('=') else {
            return Err(ParseError {
                line,
                why: "key = value".into(),
            });
        };
        let Some((_, keys)) = &mut cur else {
            return Err(ParseError {
                line,
                why: "a value before the first [[material]]".into(),
            });
        };
        let key = key.trim().to_string();
        if keys.iter().any(|x| x.1 == key) {
            return Err(ParseError {
                line,
                why: format!("{key} given twice"),
            });
        }
        let v = value(v).ok_or(ParseError {
            line,
            why: format!("{key}: a string, a number, or a list of strings or of numbers"),
        })?;
        keys.push((line, key, v));
    }
    if let Some((at, keys)) = cur {
        out.push(record(keys, at)?);
    }
    Ok(out)
}

/// The library shipped with ForgeCAD.
pub fn library() -> &'static [Material] {
    static LIB: OnceLock<Vec<Material>> = OnceLock::new();
    LIB.get_or_init(|| parse(DATA).expect("materials.toml reads (its test says why not)"))
}

/// A name as compared: lower case letters and digits only.
pub fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn named(m: &Material, n: &str) -> bool {
    norm(&m.name) == n || m.aliases.iter().any(|a| norm(a) == n)
}

/// A material of ForgeCAD's library by its name or an alias.
pub fn find(name: &str) -> Option<&'static Material> {
    let n = norm(name);
    library().iter().find(|m| named(m, &n))
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

fn nearest<'a>(all: impl Iterator<Item = &'a Material>, name: &str) -> Vec<&'a str> {
    let n = norm(name);
    let mut scored: Vec<(usize, &'a str)> = all
        .filter_map(|m| {
            std::iter::once(&m.name)
                .chain(&m.aliases)
                .map(|x| {
                    let x = norm(x);
                    let d = distance(&n, &x);
                    if !n.is_empty() && (x.contains(&n) || n.contains(&x)) {
                        d.min(1)
                    } else {
                        d
                    }
                })
                .min()
                .filter(|&d| d <= (n.chars().count() / 3).max(2))
                .map(|d| (d, m.name.as_str()))
        })
        .collect();
    scored.sort();
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(3).map(|x| x.1).collect()
}

/// ForgeCAD's names nearest an unknown one, best first (at most three).
pub fn near(name: &str) -> Vec<&'static str> {
    nearest(library().iter(), name)
}

/// Where a material is kept, in the order names are searched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lib {
    Project,
    User,
    ForgeCad,
}

impl Lib {
    pub fn label(self) -> &'static str {
        match self {
            Lib::Project => "Project",
            Lib::User => "My materials",
            Lib::ForgeCad => "ForgeCAD (read-only)",
        }
    }
}

/// One library file, read.
#[derive(Clone, Debug)]
pub struct Set {
    pub lib: Lib,
    /// None: ForgeCAD's own, built in.
    pub file: Option<PathBuf>,
    pub materials: Vec<Material>,
}

/// The libraries a document sees: its project's, the user's, ForgeCAD's; and the files that
/// failed to read.
#[derive(Clone, Debug, Default)]
pub struct Libraries {
    pub sets: Vec<Set>,
    pub errors: Vec<(PathBuf, ParseError)>,
}

/// The user's library folder: `$XDG_CONFIG_HOME/forgecad/materials`, else `~/.config/..`.
pub fn user_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("forgecad").join("materials"))
}

/// A project's library folder: `materials/` beside its documents.
pub fn project_dir(doc_dir: &Path) -> PathBuf {
    doc_dir.join("materials")
}

fn toml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

impl Libraries {
    /// ForgeCAD's library alone.
    pub fn builtin() -> Libraries {
        Libraries {
            sets: vec![Set {
                lib: Lib::ForgeCad,
                file: None,
                materials: library().to_vec(),
            }],
            errors: vec![],
        }
    }

    /// The libraries for documents in `doc_dir` (none: the user's and ForgeCAD's).
    pub fn load(doc_dir: Option<&Path>) -> Libraries {
        let mut out = Libraries::default();
        let dirs = [(Lib::Project, doc_dir.map(project_dir)), (Lib::User, user_dir())];
        for (lib, dir) in dirs {
            let Some(dir) = dir else { continue };
            for f in toml_files(&dir) {
                let Ok(text) = std::fs::read_to_string(&f) else {
                    continue;
                };
                match parse(&text) {
                    Ok(materials) => out.sets.push(Set {
                        lib,
                        file: Some(f),
                        materials,
                    }),
                    Err(e) => out.errors.push((f, e)),
                }
            }
        }
        out.sets.extend(Libraries::builtin().sets);
        out
    }

    /// `load`, kept while the library files are unchanged: their names and bytes, read each time
    /// (never their sizes and times, which a replaced file can keep).
    pub fn cached(doc_dir: Option<&Path>) -> Arc<Libraries> {
        type Seen = (PathBuf, Option<Vec<u8>>);
        type Key = (Option<PathBuf>, Vec<Seen>);
        type Cache = Mutex<Vec<(Key, Arc<Libraries>)>>;
        static CACHE: OnceLock<Cache> = OnceLock::new();
        let stamp = |d: Option<PathBuf>| {
            d.map(|d| toml_files(&d))
                .unwrap_or_default()
                .into_iter()
                .map(|f| {
                    let bytes = std::fs::read(&f).ok();
                    (f, bytes)
                })
                .collect::<Vec<_>>()
        };
        let mut files = stamp(doc_dir.map(project_dir));
        files.extend(stamp(user_dir()));
        let key: Key = (doc_dir.map(Path::to_path_buf), files);
        let cache = CACHE.get_or_init(Default::default);
        let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(hit) = c.iter().find(|x| x.0 == key) {
            return hit.1.clone();
        }
        let libs = Arc::new(Libraries::load(doc_dir));
        c.retain(|x| x.0.0 != key.0);
        c.push((key, libs.clone()));
        libs
    }

    /// Every material, in search order, with its library.
    pub fn all(&self) -> impl Iterator<Item = (&Set, &Material)> {
        self.sets
            .iter()
            .flat_map(|s| s.materials.iter().map(move |m| (s, m)))
    }

    /// A material by name or alias: the project's first, then the user's, then ForgeCAD's.
    pub fn find(&self, name: &str) -> Option<(Lib, &Material)> {
        let n = norm(name);
        self.all().find(|(_, m)| named(m, &n)).map(|(s, m)| (s.lib, m))
    }

    /// The names nearest an unknown one, best first (at most three).
    pub fn near(&self, name: &str) -> Vec<String> {
        nearest(self.all().map(|x| x.1), name)
            .into_iter()
            .map(String::from)
            .collect()
    }
}

/// Writes `m` into a library file: replacing the record of the same name, or added at the end
/// (the file made if it isn't there). `old`: the name it had, if it was renamed.
pub fn save(file: &Path, m: &Material, old: Option<&str>) -> Result<(), String> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let mut list = parse(&text).map_err(|e| format!("{}:{}: {}", file.display(), e.line, e.why))?;
    let n = [norm(&m.name), old.map(norm).unwrap_or_default()];
    match list.iter_mut().find(|x| n.contains(&norm(&x.name))) {
        Some(x) => *x = m.clone(),
        None => list.push(m.clone()),
    }
    write(file, &list)
}

/// Takes the record named `name` out of a library file.
pub fn remove(file: &Path, name: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut list = parse(&text).map_err(|e| format!("{}:{}: {}", file.display(), e.line, e.why))?;
    list.retain(|x| norm(&x.name) != norm(name));
    write(file, &list)
}

/// A library file of these records.
pub fn write(file: &Path, list: &[Material]) -> Result<(), String> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let body: Vec<String> = list.iter().map(Material::to_toml).collect();
    let text = format!(
        "# ForgeCAD materials (forge-material's format)\n\n{}",
        body.join("\n")
    );
    std::fs::write(file, text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_subset_reads_and_refuses() {
        let ok = "# a library\n[[material]]\nname = \"X\" # ok\naliases = [\"Y\", \"Z z\"]\nfamily = \"steel\"\ndensity = 7850\nyoungs = 200\ncolour = \"#808080\"\nsource = \"a book\"\nsource_youngs = \"another\"\n";
        let m = &parse(ok).unwrap()[0];
        assert_eq!(m.aliases, ["Y", "Z z"]);
        assert_eq!(m.source_of("youngs"), "another");
        assert_eq!(m.source_of("density"), "a book");
        assert!((m.kg_per_mm3() - 7.85e-6).abs() < 1e-18);
        let bad = |s: &str| parse(&ok.replace("density = 7850", s)).unwrap_err();
        assert_eq!(bad("density = -1").line, 6);
        assert_eq!(bad("dens = 1").why, "unknown key dens");
        assert_eq!(bad("").why, "X has no density");
        assert!(parse(&ok.replace("source = \"a book\"\n", "")).is_err());
        assert!(parse(&format!("{ok}source_colour = \"x\"\n")).is_err());
        assert!(parse(&format!("{ok}kind_youngs = \"guess\"\n")).is_err());
    }

    #[test]
    fn other_tools_values_and_kinds_survive_a_round_trip() {
        let src = "[[material]]\nname = \"HIPS lot 7\"\naliases = []\nfamily = \"plastic\"\nfrom = \"HIPS\"\ndensity = 1040\ncolour = \"#EEEEEE\"\nthermo.window = [140, 180.5]\nthermo.law = \"ogden3\"\nsource = \"Acme datasheet\"\nsource_thermo.window = \"our oven\"\nkind_thermo.window = \"user\"\nkind_density = \"stand-in\"\n";
        let m = parse(src).unwrap().remove(0);
        assert_eq!(m.extra("thermo.window"), Some(&Value::Nums(vec![140.0, 180.5])));
        assert_eq!(m.kind_of("thermo.window"), Kind::User);
        assert_eq!(m.kind_of("density"), Kind::StandIn);
        assert_eq!(m.kind_of("youngs"), Kind::Cited);
        assert_eq!(m.from.as_deref(), Some("HIPS"));
        assert_eq!(parse(&m.to_toml()).unwrap(), vec![m.clone()]);
        // every shipped record writes and reads back the same
        for x in library() {
            assert_eq!(&parse(&x.to_toml()).unwrap()[0], x);
        }
    }

    // wasm's test runner has no writable folder
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn a_project_library_comes_before_forgecad_and_saves_in_place() {
        let d = std::env::temp_dir().join(format!("forge-material-{}", std::process::id()));
        let file = project_dir(&d).join("project.toml");
        let mut m = find("ABS").unwrap().clone();
        m.name = "ABS black".into();
        m.aliases = vec!["abs-b".into()];
        m.from = Some("ABS".into());
        m.set_source("density", "our scale", Kind::User);
        m.density = 1060.0;
        save(&file, &m, None).unwrap();
        let libs = Libraries::load(Some(&d));
        let (lib, got) = libs.find("abs b").unwrap();
        assert_eq!((lib, got), (Lib::Project, &m));
        assert_eq!(libs.find("ABS").unwrap().0, Lib::ForgeCad);
        m.density = 1070.0;
        save(&file, &m, None).unwrap();
        assert_eq!(
            Libraries::load(Some(&d)).find("ABS black").unwrap().1.density,
            1070.0
        );
        remove(&file, "ABS black").unwrap();
        assert!(Libraries::load(Some(&d)).find("ABS black").is_none());
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn names_are_found_loosely_and_unknown_ones_get_near_names() {
        let m = find("1008 cr steel").expect("an alias");
        assert_eq!(m.name, find("1008 CR").unwrap().name);
        assert!(find("6061 t6").is_some());
        assert!(find("unobtainium").is_none());
        assert!(near("6061-T5").contains(&"6061-T6"), "{:?}", near("6061-T5"));
        assert!(near("unobtainium").is_empty());
    }

    #[test]
    fn every_record_is_complete_sourced_and_named_once() {
        let lib = parse(DATA).unwrap_or_else(|e| panic!("materials.toml line {}: {}", e.line, e.why));
        assert!(lib.len() >= 35, "{}", lib.len());
        let mut names = std::collections::HashSet::new();
        for m in &lib {
            // a record's source names its publisher and where to find it; a value's own says where
            let s = &m.source;
            assert!(
                s.len() > 20
                    && (s.contains("http") || s.contains("ISO") || s.contains("ASTM") || s.contains("ASM")),
                "{}: {s}",
                m.name
            );
            for (key, s) in &m.sources {
                assert!(s.len() > 10, "{} {key}: {s}", m.name);
            }
            assert!((100.0..25000.0).contains(&m.density), "{}", m.name);
            assert!(m.poisson.is_none_or(|p| p < 0.5), "{}", m.name);
            // a drawn thermoplastic can break below its yield; a metal can't
            if let (Some(y), Some(u)) = (m.yield_mpa, m.uts)
                && m.family != "plastic"
            {
                assert!(y <= u, "{}: yield over UTS", m.name);
            }
            let own: std::collections::HashSet<String> = std::iter::once(&m.name)
                .chain(&m.aliases)
                .map(|n| norm(n))
                .collect();
            for n in own {
                assert!(names.insert(n.clone()), "{n} names two materials");
            }
        }
    }
}
