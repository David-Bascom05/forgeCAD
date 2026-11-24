//! Appearances: how a surface looks, as data (`data/appearances.toml`, a project's
//! `materials/appearances.toml`, the user's `~/.config/forgecad/materials/appearances.toml`), each
//! value with its source and kind as forge-material's are. A part's appearance is the one it
//! names, else its material's default (by the material's name, then its family), else satin
//! plastic in the part's colour.

use super::bsdf::Bsdf;
use super::v3::{V3, v3};
use std::sync::OnceLock;

pub const DATA: &str = include_str!("../../data/appearances.toml");

pub use forge_material::Kind;

#[derive(Clone, Debug, PartialEq)]
pub struct Appearance {
    pub name: String,
    pub family: String,
    pub bsdf: Bsdf,
    /// A part's own colour replaces `base`.
    pub part_colour: bool,
    pub materials: Vec<String>,
    pub families: Vec<String>,
    pub source: String,
    pub sources: Vec<(String, String)>,
    pub kinds: Vec<(String, Kind)>,
    pub note: Option<String>,
}

impl Appearance {
    pub fn kind_of(&self, key: &str) -> Kind {
        self.kinds.iter().find(|k| k.0 == key).map_or(Kind::Cited, |k| k.1)
    }

    pub fn source_of(&self, key: &str) -> &str {
        self.sources.iter().find(|s| s.0 == key).map_or(&self.source, |s| &s.1)
    }

    /// The BSDF, in a part's colour (sRGB bytes) when this appearance takes it.
    pub fn bsdf_for(&self, colour: Option<[u8; 3]>) -> Bsdf {
        let mut b = self.bsdf;
        if self.part_colour
            && let Some(c) = colour
        {
            b.base = V3::from(c.map(srgb_to_linear));
        }
        b
    }
}

pub fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

#[derive(Clone, Debug, PartialEq)]
enum Val {
    Str(String),
    Num(f32),
    Bool(bool),
    Strs(Vec<String>),
    Nums(Vec<f32>),
}

fn value(s: &str) -> Option<Val> {
    let s = s.trim();
    if let Some(q) = s.strip_prefix('"') {
        return q.strip_suffix('"').map(|t| Val::Str(t.replace("\\\"", "\"")));
    }
    if s == "true" || s == "false" {
        return Some(Val::Bool(s == "true"));
    }
    if let Some(inner) = s.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        let items: Vec<&str> = inner.split(',').map(str::trim).filter(|t| !t.is_empty()).collect();
        if items.iter().all(|t| t.starts_with('"')) {
            return Some(Val::Strs(items.iter().map(|t| t.trim_matches('"').to_string()).collect()));
        }
        return items.iter().map(|t| t.parse().ok()).collect::<Option<Vec<f32>>>().map(Val::Nums);
    }
    s.parse().ok().map(Val::Num)
}

/// A line's comment cut off, outside quotes.
fn uncomment(l: &str) -> &str {
    let mut quoted = false;
    for (k, c) in l.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &l[..k],
            _ => {}
        }
    }
    l
}

/// A library's text to its appearances; an error names the line.
pub fn parse(src: &str) -> Result<Vec<Appearance>, String> {
    let mut out = Vec::new();
    let mut cur: Option<(usize, Vec<(String, Val)>)> = None;
    let finish = |at: usize, keys: Vec<(String, Val)>| record(at, keys);
    for (k, raw) in src.lines().enumerate() {
        let l = uncomment(raw).trim();
        if l.is_empty() {
            continue;
        }
        if l == "[[appearance]]" {
            if let Some((at, keys)) = cur.take() {
                out.push(finish(at, keys)?);
            }
            cur = Some((k + 1, Vec::new()));
            continue;
        }
        let (key, v) = l.split_once('=').ok_or(format!("line {}: key = value", k + 1))?;
        let v = value(v).ok_or(format!("line {}: {} is not a value", k + 1, v.trim()))?;
        let Some((_, keys)) = cur.as_mut() else {
            return Err(format!("line {}: outside an [[appearance]]", k + 1));
        };
        keys.push((key.trim().to_string(), v));
    }
    if let Some((at, keys)) = cur {
        out.push(finish(at, keys)?);
    }
    Ok(out)
}

const KEYS: [&str; 15] = [
    "name", "family", "base", "metal", "tint", "rough", "ior", "transmission", "coat", "coat_rough",
    "part_colour", "materials", "families", "source", "note",
];
const VALUED: [&str; 8] = ["base", "metal", "tint", "rough", "ior", "transmission", "coat", "coat_rough"];

fn record(at: usize, keys: Vec<(String, Val)>) -> Result<Appearance, String> {
    let get = |k: &str| keys.iter().find(|x| x.0 == k).map(|x| &x.1);
    let name = match get("name") {
        Some(Val::Str(s)) => s.clone(),
        _ => return Err(format!("line {at}: an appearance has a name")),
    };
    let e = |why: String| format!("{name}: {why}");
    let num = |k: &str, lo: f32, hi: f32, def: f32| -> Result<f32, String> {
        match get(k) {
            None => Ok(def),
            Some(Val::Num(v)) if (lo..=hi).contains(v) => Ok(*v),
            _ => Err(e(format!("{k} is a number in {lo}..{hi}"))),
        }
    };
    let rgb = |k: &str, def: V3| -> Result<V3, String> {
        match get(k) {
            None => Ok(def),
            Some(Val::Nums(v)) if v.len() == 3 && v.iter().all(|x| (0.0..=1.0).contains(x)) => Ok(v3(v[0], v[1], v[2])),
            _ => Err(e(format!("{k} is [r, g, b] in 0..1"))),
        }
    };
    let text = |k: &str| match get(k) {
        Some(Val::Str(s)) => Ok(Some(s.clone())),
        None => Ok(None),
        _ => Err(e(format!("{k} is text"))),
    };
    let list = |k: &str| match get(k) {
        Some(Val::Strs(v)) => Ok(v.clone()),
        None => Ok(Vec::new()),
        _ => Err(e(format!("{k} is a list of text"))),
    };
    let (mut sources, mut kinds) = (Vec::new(), Vec::new());
    for (k, v) in &keys {
        if let Some(of) = k.strip_prefix("source_") {
            match v {
                Val::Str(s) if VALUED.contains(&of) => sources.push((of.to_string(), s.clone())),
                _ => return Err(e(format!("{k}: a source of a value, as text"))),
            }
        } else if let Some(of) = k.strip_prefix("kind_") {
            let kind = match v {
                Val::Str(s) if s == "cited" => Kind::Cited,
                Val::Str(s) if s == "user" => Kind::User,
                Val::Str(s) if s == "stand-in" => Kind::StandIn,
                _ => return Err(e(format!("{k} is cited, user or stand-in"))),
            };
            if !VALUED.contains(&of) {
                return Err(e(format!("{k}: no value {of}")));
            }
            kinds.push((of.to_string(), kind));
        } else if !KEYS.contains(&k.as_str()) {
            return Err(e(format!("unknown key {k}")));
        }
    }
    let source = text("source")?.ok_or(e("has no source".into()))?;
    let bsdf = Bsdf {
        base: rgb("base", V3::splat(0.5))?,
        metal: num("metal", 0.0, 1.0, 0.0)?,
        f82: rgb("tint", V3::ONE)?,
        rough: num("rough", 0.0, 1.0, 0.5)?,
        ior: num("ior", 1.0, 4.0, 1.5)?,
        transmission: num("transmission", 0.0, 1.0, 0.0)?,
        coat: num("coat", 0.0, 1.0, 0.0)?,
        coat_rough: num("coat_rough", 0.0, 1.0, 0.03)?,
    };
    Ok(Appearance {
        family: text("family")?.unwrap_or_else(|| "other".into()),
        bsdf,
        part_colour: matches!(get("part_colour"), Some(Val::Bool(true))),
        materials: list("materials")?,
        families: list("families")?,
        source,
        sources,
        kinds,
        note: text("note")?,
        name,
    })
}

/// ForgeCAD's own appearances.
pub fn library() -> &'static [Appearance] {
    static ALL: OnceLock<Vec<Appearance>> = OnceLock::new();
    ALL.get_or_init(|| parse(DATA).expect("ForgeCAD's appearances read"))
}

/// Libraries searched in order: the project's, the user's, ForgeCAD's.
#[derive(Clone, Debug, Default)]
pub struct Appearances {
    pub sets: Vec<Vec<Appearance>>,
}

impl Appearances {
    pub fn builtin() -> Appearances {
        Appearances {
            sets: vec![library().to_vec()],
        }
    }

    /// The project's (`materials/appearances.toml` beside its files) and the user's, then
    /// ForgeCAD's; a file that doesn't read is skipped and said.
    pub fn load(doc_dir: Option<&std::path::Path>) -> (Appearances, Vec<String>) {
        let mut sets = Vec::new();
        let mut errors = Vec::new();
        let mut files = Vec::new();
        if let Some(d) = doc_dir {
            files.push(forge_material::project_dir(d).join("appearances.toml"));
        }
        if let Some(d) = forge_material::user_dir() {
            files.push(d.join("appearances.toml"));
        }
        for f in files {
            let Ok(text) = forge_dirs::fs::read(&f) else {
                continue;
            };
            match parse(&String::from_utf8_lossy(&text)) {
                Ok(v) => sets.push(v),
                Err(e) => errors.push(format!("{}: {e}", f.display())),
            }
        }
        sets.push(library().to_vec());
        (Appearances { sets }, errors)
    }

    pub fn all(&self) -> impl Iterator<Item = &Appearance> {
        self.sets.iter().flatten()
    }

    pub fn find(&self, name: &str) -> Option<&Appearance> {
        let n = forge_material::norm(name);
        self.all().find(|a| forge_material::norm(&a.name) == n)
    }

    /// A material's default appearance: one naming it, else one naming its family.
    pub fn for_material(&self, m: &forge_material::Material) -> Option<&Appearance> {
        let names: Vec<String> = std::iter::once(&m.name).chain(&m.aliases).map(|s| forge_material::norm(s)).collect();
        self.all()
            .find(|a| a.materials.iter().any(|x| names.contains(&forge_material::norm(x))))
            .or_else(|| self.all().find(|a| a.families.contains(&m.family)))
    }

    /// What a part looks like: its own appearance if it names one, else its material's, else
    /// satin plastic in its colour.
    pub fn resolve(&self, appearance: Option<&str>, material: Option<&forge_material::Material>, colour: Option<[u8; 3]>) -> Bsdf {
        let colour = colour.or(material.map(|m| m.colour));
        let a = appearance
            .and_then(|n| self.find(n))
            .or_else(|| material.and_then(|m| self.for_material(m)))
            .or_else(|| self.find("Plastic, satin"));
        a.map_or_else(Bsdf::default, |a| a.bsdf_for(colour))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_reads_and_every_value_is_accounted_for() {
        let lib = library();
        assert!(lib.len() >= 30, "{}", lib.len());
        let names: std::collections::HashSet<_> = lib.iter().map(|a| forge_material::norm(&a.name)).collect();
        assert_eq!(names.len(), lib.len(), "names are unique");
        for a in lib {
            // a metal has a tint; transparent things are dielectrics
            assert!(a.bsdf.metal == 0.0 || a.bsdf.metal == 1.0, "{}", a.name);
            assert!(!(a.bsdf.metal > 0.0 && a.bsdf.transmission > 0.0), "{}", a.name);
            // every roughness is marked a stand-in until measured
            assert_eq!(a.kind_of("rough") == Kind::StandIn || a.name == "Grey card" || a.name == "Rubber, black", true, "{}", a.name);
        }
    }

    #[test]
    fn every_forge_material_family_has_an_appearance() {
        let apps = Appearances::builtin();
        for m in forge_material::library() {
            let a = apps.for_material(m);
            assert!(a.is_some(), "{} ({})", m.name, m.family);
        }
        let brass = forge_material::find("C36000").unwrap();
        assert_eq!(apps.for_material(brass).unwrap().name, "Brass");
        let al = forge_material::find("6061-T6").unwrap();
        assert_eq!(apps.for_material(al).unwrap().name, "Aluminium, machined");
        // a plastic part takes its own colour
        let abs = forge_material::find("ABS").unwrap();
        let b = apps.resolve(None, Some(abs), Some([255, 0, 0]));
        assert_eq!(b.base, v3(1.0, 0.0, 0.0));
    }

    #[test]
    fn a_bad_record_says_where() {
        assert!(parse("[[appearance]]\nname = \"x\"\nrough = 2\nsource = \"s\"\n").unwrap_err().contains("rough"));
        assert!(parse("[[appearance]]\nname = \"x\"\n").unwrap_err().contains("source"));
        assert!(parse("[[appearance]]\nname = \"x\"\nsource = \"s\"\nkind_bogus = \"cited\"\n").is_err());
        let ok = parse("# mine\n[[appearance]]\nname = \"Mine\" # a comment\nbase = [0.1, 0.2, 0.3]\nsource = \"me\"\nkind_base = \"user\"\n").unwrap();
        assert_eq!(ok[0].bsdf.base, v3(0.1, 0.2, 0.3));
        assert_eq!(ok[0].kind_of("base"), Kind::User);
    }
}
