//! The tool library's model (the window is `ui::tools_window`): finding a tool (words, type,
//! diameter, material), the tools used recently, and a new tool, from nothing or derived from a
//! library one (forge-tooling's `derive`: the source's cites kept, the changed values cited
//! `user`), written as a `[[tool]]` into the project's `cam/tools.toml` and read back first.

use forge_tooling::toml::{Table, Val};
use forge_tooling::{Kind, Library, Tool};

/// The type chips: a name and the kinds it covers.
pub const KIND_CHIPS: [(&str, &[Kind]); 7] = [
    ("Flat", &[Kind::Flat]),
    ("Ball", &[Kind::Ball]),
    ("Bull", &[Kind::Bull]),
    ("Drill", &[Kind::Drill, Kind::Spot, Kind::Reamer]),
    ("Chamfer", &[Kind::Chamfer, Kind::Countersink, Kind::Engrave]),
    ("Thread", &[Kind::Tap, Kind::ThreadMill]),
    ("Other", &[Kind::Bore, Kind::Counterbore, Kind::Slot]),
];

/// What the library is narrowed to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Find {
    /// Words: in the id, the type's words, the maker, the material, or a diameter ("10", "Ø10").
    pub text: String,
    /// One of `KIND_CHIPS`.
    pub kind: Option<usize>,
    /// Diameter from / to, mm (empty: open).
    pub dmin: String,
    pub dmax: String,
    /// The tool's material (empty: any).
    pub material: String,
}

impl Find {
    pub fn is_open(&self) -> bool {
        *self == Find::default()
    }
}

/// The words a person searches a tool by.
pub fn words(t: &Tool) -> String {
    format!(
        "{} {} {} {} {} d{} ø{} {}",
        t.id,
        kind_words(t.kind),
        t.kind.name(),
        t.vendor,
        t.material,
        num(t.dc),
        num(t.dc),
        t.note
    )
    .to_lowercase()
}

/// A tool's type in words.
pub fn kind_words(k: Kind) -> &'static str {
    match k {
        Kind::Flat => "Flat end mill",
        Kind::Ball => "Ball end mill",
        Kind::Bull => "Bull nose end mill",
        Kind::Chamfer => "Chamfer mill",
        Kind::Spot => "Spot drill",
        Kind::Drill => "Drill",
        Kind::Tap => "Tap",
        Kind::ThreadMill => "Thread mill",
        Kind::Reamer => "Reamer",
        Kind::Bore => "Boring bar",
        Kind::Counterbore => "Counterbore",
        Kind::Countersink => "Countersink",
        Kind::Slot => "Slot cutter",
        Kind::Engrave => "Engraver",
    }
}

fn num(v: f64) -> String {
    let s = format!("{:.3}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Whether `t` is among what `f` narrows to.
pub fn matches(f: &Find, t: &Tool) -> bool {
    if let Some(k) = f.kind
        && !KIND_CHIPS.get(k).is_some_and(|c| c.1.contains(&t.kind))
    {
        return false;
    }
    let mm = |s: &str| s.trim().trim_start_matches(['Ø', 'ø']).trim().parse::<f64>().ok();
    if mm(&f.dmin).is_some_and(|lo| t.dc < lo - 1e-9) || mm(&f.dmax).is_some_and(|hi| t.dc > hi + 1e-9) {
        return false;
    }
    if !f.material.is_empty() && !t.material.eq_ignore_ascii_case(&f.material) {
        return false;
    }
    let w = words(t);
    f.text
        .to_lowercase()
        .split_whitespace()
        .all(|q| w.contains(q.trim_start_matches(['Ø', 'ø'])))
}

/// The tools' materials, each once, in order.
pub fn materials(lib: &Library) -> Vec<String> {
    let mut m: Vec<String> = lib
        .tools
        .iter()
        .map(|t| t.material.clone())
        .filter(|m| !m.is_empty())
        .collect();
    m.sort();
    m.dedup();
    m
}

/// The library's order: by type (as the chips), then diameter, then id.
pub fn sorted(tools: &[Tool]) -> Vec<&Tool> {
    let rank = |k: Kind| KIND_CHIPS.iter().position(|c| c.1.contains(&k)).unwrap_or(99);
    let mut v: Vec<&Tool> = tools.iter().collect();
    v.sort_by(|a, b| {
        (rank(a.kind), a.dc, &a.id)
            .partial_cmp(&(rank(b.kind), b.dc, &b.id))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

/// The fields a tool of `kind` is made with: (TOML key, label, unit).
pub fn form_fields(kind: Kind) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut f = vec![
        ("dc", "Diameter", "mm"),
        ("lcf", "Flute length", "mm"),
        ("nof", "Flutes", ""),
        ("dcon", "Shank diameter", "mm"),
        ("oal", "Overall length", "mm"),
    ];
    match kind {
        Kind::Bull => f.push(("re", "Corner radius", "mm")),
        Kind::Drill => f.push(("sig", "Point angle", "\u{b0}")),
        Kind::Chamfer | Kind::Engrave => {
            f.push(("angle", "Included angle", "\u{b0}"));
            f.push(("tip_d", "Tip diameter", "mm"));
        }
        Kind::Spot | Kind::Countersink => f.push(("angle", "Included angle", "\u{b0}")),
        Kind::Tap | Kind::ThreadMill => f.push(("tp", "Pitch", "mm")),
        _ => {}
    }
    f.push(("material", "Material", ""));
    f
}

/// A field's value on a tool, as the form shows it.
fn value_of(t: &Tool, key: &str) -> String {
    match key {
        "dc" => num(t.dc),
        "lcf" => num(t.lcf),
        "nof" => t.nof.to_string(),
        "dcon" => num(t.dcon),
        "oal" => num(t.oal),
        "re" => num(t.re),
        "sig" | "angle" => num(t.angle),
        "tip_d" => num(t.tip_d),
        "tp" => num(t.tp),
        "material" => t.material.clone(),
        _ => String::new(),
    }
}

/// A new tool being made: from nothing (`from` None) or derived from a library tool.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolForm {
    pub from: Option<String>,
    pub id: String,
    pub kind: Kind,
    /// (TOML key, the value typed).
    pub vals: Vec<(&'static str, String)>,
}

impl ToolForm {
    /// A tool from nothing: a 10 mm flat by default.
    pub fn blank(lib: &Library, kind: Kind) -> ToolForm {
        let id = free_id(lib, &format!("my-{}", kind.name().replace('_', "-")));
        let mut f = ToolForm {
            from: None,
            id,
            kind,
            vals: vec![],
        };
        f.set_kind(kind);
        f
    }

    /// The type changed: its fields, the common ones kept.
    pub fn set_kind(&mut self, kind: Kind) {
        let old = std::mem::take(&mut self.vals);
        self.kind = kind;
        let dflt = |k: &str| -> String {
            match k {
                "dc" => "10",
                "lcf" => "25",
                "nof" => {
                    if kind == Kind::Drill {
                        "2"
                    } else {
                        "3"
                    }
                }
                "dcon" => "10",
                "oal" => "75",
                "re" => "1",
                "sig" => "118",
                "angle" => "90",
                "tip_d" => "0.5",
                "tp" => "1.5",
                "material" => "carbide",
                _ => "",
            }
            .to_string()
        };
        self.vals = form_fields(kind)
            .into_iter()
            .map(|(k, _, _)| {
                (
                    k,
                    old.iter()
                        .find(|o| o.0 == k)
                        .map_or_else(|| dflt(k), |o| o.1.clone()),
                )
            })
            .collect();
    }

    /// A tool derived from `t`: its values, a new id.
    pub fn derived(lib: &Library, t: &Tool) -> ToolForm {
        ToolForm {
            from: Some(t.id.clone()),
            id: free_id(lib, &format!("{}-custom", t.id)),
            kind: t.kind,
            vals: form_fields(t.kind)
                .into_iter()
                .map(|(k, _, _)| (k, value_of(t, k)))
                .collect(),
        }
    }

    pub fn get(&self, k: &str) -> &str {
        self.vals.iter().find(|v| v.0 == k).map_or("", |v| v.1.as_str())
    }

    pub fn set(&mut self, k: &str, v: String) {
        if let Some(x) = self.vals.iter_mut().find(|x| x.0 == k) {
            x.1 = v;
        }
    }

    /// The `[[tool]]` it writes, read back by forge-tooling first (a typed refusal otherwise).
    pub fn toml(&self, lib: &Library) -> Result<String, String> {
        let id = self.id.trim();
        if id.is_empty() || id.contains(['"', '\\', '\n']) {
            return Err("give the tool an id (letters, digits, - and _)".into());
        }
        if lib.tools.iter().any(|t| t.id == id) {
            return Err(format!("`{id}` is taken: give another id"));
        }
        let val = |k: &str, s: &str| -> Result<Val, String> {
            if k == "material" {
                return Ok(Val::Str(s.trim().to_string()));
            }
            s.trim()
                .parse::<f64>()
                .map(Val::Num)
                .map_err(|_| format!("{}: `{s}` is not a number", label(self.kind, k)))
        };
        let table = match &self.from {
            Some(from) => {
                let src = lib
                    .tools
                    .iter()
                    .find(|t| &t.id == from)
                    .ok_or(format!("no library tool `{from}`"))?;
                // only what changed is an override (cited `user`); the rest keeps the source's cites
                let mut over = vec![];
                for (k, v) in &self.vals {
                    if v.trim() != value_of(src, k) && !(k == &"material" && v.trim().is_empty()) {
                        over.push((k.to_string(), val(k, v)?));
                    }
                }
                forge_tooling::build::derive_table(lib, "tool", from, id, &over)
                    .map_err(|e| format!("{e:?}"))?
                    .0
            }
            None => {
                let mut t = Table::new();
                t.insert("id".into(), Val::Str(id.into()));
                t.insert("type".into(), Val::Str(self.kind.name().into()));
                t.insert("real".into(), Val::Bool(false));
                t.insert("source".into(), Val::Str("made in the tool library".into()));
                for (k, v) in &self.vals {
                    if v.trim().is_empty() {
                        continue;
                    }
                    t.insert(k.to_string(), val(k, v)?);
                }
                t
            }
        };
        let text = toml_of("tool", &table);
        let mut check = lib.clone();
        check.add("cam/tools.toml", &text).map_err(|e| format!("{e:?}"))?;
        Ok(text)
    }
}

fn label(kind: Kind, k: &str) -> &'static str {
    form_fields(kind)
        .into_iter()
        .find(|f| f.0 == k)
        .map_or("value", |f| f.1)
}

/// `base`, or `base-2`, `base-3`.. : an id no library tool has.
pub fn free_id(lib: &Library, base: &str) -> String {
    let taken = |s: &str| lib.tools.iter().any(|t| t.id == s);
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|k| format!("{base}-{k}"))
        .find(|s| !taken(s))
        .unwrap_or_default()
}

/// A record's table as a library file writes it: `[[key]]`, its values, then its sub-tables
/// (`[[key.cite]]`, `[[key.preset]]`).
pub fn toml_of(key: &str, t: &Table) -> String {
    let mut out = format!("[[{key}]]\n");
    let mut subs = String::new();
    // id and type first, as the library files are written
    let mut keys: Vec<&String> = t.keys().collect();
    keys.sort_by_key(|k| match k.as_str() {
        "id" => 0,
        "type" => 1,
        _ => 2,
    });
    for k in keys {
        match &t[k] {
            Val::Tables(v) => {
                for sub in v {
                    subs.push_str(&format!("\n[[{key}.{k}]]\n"));
                    for (sk, sv) in sub {
                        subs.push_str(&format!("{sk} = {}\n", scalar(sv)));
                    }
                }
            }
            Val::Table(sub) => {
                subs.push_str(&format!("\n[{key}.{k}]\n"));
                for (sk, sv) in sub {
                    subs.push_str(&format!("{sk} = {}\n", scalar(sv)));
                }
            }
            v => out.push_str(&format!("{k} = {}\n", scalar(v))),
        }
    }
    out + &subs
}

fn scalar(v: &Val) -> String {
    match v {
        Val::Str(s) => format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        ),
        Val::Num(x) => {
            if x.fract() == 0.0 && x.abs() < 1e15 {
                format!("{}", *x as i64)
            } else {
                format!("{x}")
            }
        }
        Val::Bool(b) => b.to_string(),
        Val::Arr(a) => format!("[{}]", a.iter().map(scalar).collect::<Vec<_>>().join(", ")),
        // a table inside a value: not written by the library's records
        Val::Table(_) | Val::Tables(_) => "[]".into(),
    }
}

/// The recently used tools: `id` to the front, at most eight.
pub fn touch(recent: &mut Vec<String>, id: &str) {
    recent.retain(|r| r != id);
    recent.insert(0, id.to_string());
    recent.truncate(8);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_narrows_by_words_type_diameter_and_material() {
        let lib = Library::builtin();
        let f = |text: &str, kind: Option<usize>, lo: &str, hi: &str| Find {
            text: text.into(),
            kind,
            dmin: lo.into(),
            dmax: hi.into(),
            material: String::new(),
        };
        let ids = |f: &Find| {
            lib.tools
                .iter()
                .filter(|t| matches(f, t))
                .map(|t| t.id.clone())
                .collect::<Vec<_>>()
        };
        assert!(ids(&f("flat", None, "", "")).contains(&"flat-10".to_string()));
        assert!(ids(&f("Ø10", Some(0), "", "")).contains(&"flat-10".to_string()));
        let small = ids(&f("", Some(0), "", "6"));
        assert!(small.contains(&"flat-6".to_string()) && !small.contains(&"flat-10".to_string()));
        let balls = ids(&f("", Some(1), "", ""));
        assert!(
            !balls.is_empty()
                && balls
                    .iter()
                    .all(|i| lib.tools.iter().any(|t| &t.id == i && t.kind == Kind::Ball))
        );
    }

    #[test]
    fn a_derived_tool_writes_toml_that_reads_back_with_its_cites() {
        let lib = Library::builtin();
        let src = lib.tools.iter().find(|t| t.id == "flat-10").unwrap();
        let mut form = ToolForm::derived(&lib, src);
        assert_eq!(form.id, "flat-10-custom");
        form.set("lcf", "40".into());
        let text = form.toml(&lib).unwrap();
        assert!(text.starts_with("[[tool]]\nid = \"flat-10-custom\"\n"), "{text}");
        let mut l = lib.clone();
        l.add("cam/tools.toml", &text).unwrap();
        let t = l.tools.iter().find(|t| t.id == "flat-10-custom").unwrap();
        assert_eq!((t.dc, t.lcf, t.kind, t.real), (src.dc, 40.0, src.kind, false));
        assert!(
            t.cites
                .iter()
                .any(|c| format!("{c:?}").contains("lcf") && format!("{c:?}").contains("User")),
            "{:?}",
            t.cites
        );
        // taken ids and bad numbers are refused, nothing written
        form.id = "flat-10".into();
        assert!(form.toml(&lib).unwrap_err().contains("taken"));
        form.id = "x".into();
        form.set("dc", "ten".into());
        assert!(form.toml(&lib).unwrap_err().contains("not a number"));
    }

    #[test]
    fn a_new_tool_from_nothing_reads_back() {
        let lib = Library::builtin();
        let mut form = ToolForm::blank(&lib, Kind::Flat);
        form.set_kind(Kind::Bull);
        form.set("re", "0.5".into());
        let text = form.toml(&lib).unwrap();
        let mut l = lib.clone();
        l.add("cam/tools.toml", &text).unwrap();
        let t = l.tools.iter().find(|t| t.id == form.id).unwrap();
        assert_eq!((t.kind, t.re, t.dc), (Kind::Bull, 0.5, 10.0));
    }
}
