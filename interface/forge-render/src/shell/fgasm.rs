//! An assembly (.fg) in the app, as Inventor's Model browser has it: the Origin and the
//! Relationships at the top, then the components in the order the file makes them, subassemblies
//! nested with their own Origin and Relationships, each occurrence holding the relationships that
//! hold it. An eye on every component and origin row; problems flagged on the rows that have them.
//! What Inventor puts in dialogs is in dialogs here: Parameters, Drive, iProperties, Analyze
//! Interference, the bill of materials, the explode controls.

use super::Shell;
use super::fg::{FgAct, From, Severity, short};
use super::icons;
use super::theme::{medium, mono, sans, semi};
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use forge_lang::asm::{ExpectOut, InstOut, Issue as AsmIssue, JointOut, LinkFamily, LinkOut};
use forge_lang::error::{Literal, Op};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct OccRow {
    pub path: String,
    pub file: String,
    /// It has a solid to draw.
    pub drawn: bool,
}

/// A problem on a row: Inventor's yellow and red.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Flag {
    Warning,
    Error,
}

/// What the issues flag: relationships by (owner, id), occurrences by path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flags {
    pub links: HashMap<(String, String), Flag>,
    pub occ: HashMap<String, Flag>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// The assembly itself, everything under it; open until shut.
    Top,
    /// An Origin folder, the assembly's or a subassembly's.
    Origin,
    /// One of its planes, axes or the centre point, by its index in `origin::ITEMS`.
    OriginItem(usize),
    Relationships,
    /// One kind of relationship, under Relationships.
    LinkKind,
    /// A relationship, by its index into the panel's links: under Relationships, or under an
    /// occurrence it holds.
    Link(usize),
    Sub,
    /// One statement's occurrences in a loop: `stud_a ×4`.
    Pattern,
    Occ,
    /// A part occurrence's own Origin folder and its items, as the part's browser has them.
    PartOrigin,
    PartOriginItem(usize),
    /// One of a part occurrence's features (or a loop's group of them), its key
    /// `<occurrence>>f:<feature>`.
    Feature,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// What it's chosen and named by: an occurrence's path, `s:<sub>`, `p:<pattern>`, `o:<sub>`,
    /// `r:<sub>`, `l:<link>`, `<occurrence>>l:<link>`.
    pub key: String,
    pub label: String,
    /// Words at its right: the part type, what a relationship ties.
    pub detail: String,
    pub kind: NodeKind,
    pub icon: &'static str,
    pub depth: usize,
    pub parent: Option<usize>,
    pub kids: Vec<usize>,
    /// The occurrences it stands for, by index into the panel's: a relationship's are its two.
    pub occ: Vec<usize>,
    pub flag: Option<Flag>,
    pub ground: bool,
    /// The subassembly occurrence it's in; empty for the top.
    pub at: String,
}

/// The browser's rows, made again when the solve or its problems change; and each occurrence's
/// subsystem, subassembly and way out, for the exploded view.
#[derive(Clone, Debug, Default)]
pub struct AsmTree {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    by_key: HashMap<String, usize>,
    /// Each occurrence's own row.
    pub home: Vec<usize>,
    /// Each occurrence's subsystem (its part file's folder), in the order they're listed.
    pub group: Vec<usize>,
    /// The top-level subassembly each is in, if any.
    pub sub: Vec<Option<usize>>,
    /// A fastener's way out along its own z: -1 a screw or stud (head at z < 0), +1 the rest.
    pub axis: Vec<Option<f32>>,
}

/// Hardware folders: what's in them is a fastener, and backs out along its axis.
const HARDWARE: [&str; 2] = ["hw", "hardware"];

/// A screw or a stud backs out towards its head (its z < 0); a nut or washer off the other way.
pub fn screw_like(file: &str) -> bool {
    let stem = part_word(file).to_ascii_lowercase();
    const SCREWS: [&str; 16] = [
        "iso4014", "iso4017", "iso4762", "iso7380", "iso7045", "iso7046", "iso7379", "iso10642", "iso1207",
        "iso4026", "iso4027", "iso4028", "iso4029", "iso14579", "iso14580", "iso14583",
    ];
    stem.contains("screw")
        || stem.contains("bolt")
        || stem.contains("stud")
        || stem.starts_with("pem_fh")
        || SCREWS.iter().any(|s| stem.starts_with(s))
}

/// Folder words and what the browser calls them, in the order it lists them.
const GROUPS: [(&str, &str); 7] = [
    ("frame", "Frame"),
    ("panels", "Panels & doors"),
    ("casters", "Casters"),
    ("fans", "Fan tray"),
    ("hw", "Hardware & fasteners"),
    ("hardware", "Hardware & fasteners"),
    ("dummy", "Dummies"),
];
const PARTS: &str = "Parts";

/// What a folder word is called, and where it sorts.
fn group_label(word: &str) -> (usize, String) {
    if let Some(k) = GROUPS.iter().position(|g| g.0 == word) {
        return (k, GROUPS[k].1.to_string());
    }
    if word.is_empty() {
        return (GROUPS.len() + 1, PARTS.to_string());
    }
    let mut c = word.chars();
    let first = c
        .next()
        .map(|f| f.to_uppercase().collect::<String>())
        .unwrap_or_default();
    (GROUPS.len(), format!("{first}{}", c.as_str().replace('_', " ")))
}

/// The folder a part file is in, as the assembly's folder sees it: the first folder past what
/// both share (`../hw/x.fg` is `hw`, `../fans/standin/y.fg` is `fans`); none beside it.
pub fn folder_word(file: &str, dir: Option<&Path>) -> String {
    let p = Path::new(file);
    let parent = match p.parent() {
        Some(q) if !q.as_os_str().is_empty() => q,
        _ => return String::new(),
    };
    let parent: PathBuf = match (parent.is_absolute(), dir) {
        (false, Some(d)) => d.join(parent),
        _ => parent.to_path_buf(),
    };
    let norm = |q: &Path| -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for c in q.components() {
            match c {
                std::path::Component::ParentDir => {
                    out.pop();
                }
                std::path::Component::CurDir => {}
                c => out.push(c.as_os_str().to_string_lossy().to_string()),
            }
        }
        out
    };
    let a = norm(&parent);
    let Some(d) = dir else {
        return a.last().cloned().unwrap_or_default();
    };
    let b = norm(d);
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    a.get(common).cloned().unwrap_or_default()
}

/// A part file's name as the tree shows it: its stem.
pub fn part_word(file: &str) -> String {
    if let Some(d) = std_designation(file) {
        return d;
    }
    let n = file.rsplit('/').next().unwrap_or(file);
    n.trim_end_matches(".fg").to_string()
}

/// A standard part's designation from its key: `std:ISO 4762|M6|20` is "ISO 4762 M6x20".
fn std_designation(file: &str) -> Option<String> {
    let spec = forge_lang::stdparts::spec_of(file)?;
    Some(forge_lang::stdparts::generate(&spec).ok()?.designation)
}

/// A part type as a person reads it: `iso7380_m6x12` is "ISO 7380 M6×12", `cage_nut_m6` is
/// "Cage nut M6".
pub fn type_label(file: &str) -> String {
    if let Some(d) = std_designation(file) {
        return d.replace('x', "\u{d7}");
    }
    let stem = part_word(file);
    let words: Vec<String> = stem
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            for std in ["iso", "din", "en"] {
                if let Some(n) = lower.strip_prefix(std)
                    && !n.is_empty()
                    && n.chars().all(|c| c.is_ascii_digit())
                {
                    return format!("{} {n}", std.to_ascii_uppercase());
                }
            }
            let times = |s: &str| {
                let b: Vec<char> = s.chars().collect();
                let mut out = String::new();
                for (k, c) in b.iter().enumerate() {
                    let digits = |i: Option<&char>| i.is_some_and(|c| c.is_ascii_digit());
                    if *c == 'x' && k > 0 && digits(b.get(k - 1)) && digits(b.get(k + 1)) {
                        out.push('×');
                    } else {
                        out.push(*c);
                    }
                }
                out
            };
            // a metric size: m6, m6x12, m4_0 stays as written
            if let Some(n) = lower.strip_prefix('m')
                && n.starts_with(|c: char| c.is_ascii_digit())
            {
                return format!("M{}", times(n));
            }
            if lower.starts_with(|c: char| c.is_ascii_digit()) {
                return times(&lower);
            }
            if lower == "pem" || (lower.len() <= 2 && lower.starts_with(|c: char| c.is_ascii_alphabetic())) {
                return lower.to_ascii_uppercase();
            }
            lower
        })
        .collect();
    let mut s = words.join(" ");
    if let Some(f) = s.chars().next()
        && f.is_ascii_lowercase()
    {
        s.replace_range(0..1, &f.to_ascii_uppercase().to_string());
    }
    s
}

/// A relationship kind as the browser heads it: `Revolute joints`, `Gear relations`.
pub fn kind_label(family: LinkFamily, kind: &str) -> String {
    let word = match kind {
        "pin_slot" => "Pin-slot".to_string(),
        "plane_plane" => "Mate".to_string(),
        "rack_pinion" => "Rack and pinion".to_string(),
        k => {
            let mut c = k.replace('_', " ").chars().collect::<Vec<char>>();
            if let Some(f) = c.first_mut() {
                *f = f.to_ascii_uppercase();
            }
            c.into_iter().collect()
        }
    };
    match family {
        LinkFamily::Joint => format!("{word} joints"),
        LinkFamily::Constraint => format!("{word} constraints"),
        LinkFamily::Relation => format!("{word} relations"),
        LinkFamily::Fit => "Fits".into(),
        LinkFamily::FitRule => "Fit rules".into(),
    }
}

/// Each kind its icon.
pub fn kind_icon(family: LinkFamily, kind: &str) -> &'static str {
    match (family, kind) {
        (LinkFamily::Joint, "fastened") => "jfastened",
        (LinkFamily::Joint, "revolute") => "jrevolute",
        (LinkFamily::Joint, "slider") => "jslider",
        (LinkFamily::Joint, "cylindrical") => "jcylindrical",
        (LinkFamily::Joint, "pin_slot") => "jpinslot",
        (LinkFamily::Joint, "planar") => "plane",
        (LinkFamily::Joint, "ball") => "ball",
        (LinkFamily::Joint, _) => "jrevolute",
        (LinkFamily::Constraint, "insert") => "jinsert",
        (LinkFamily::Constraint, "angle") => "jangle",
        (LinkFamily::Constraint, _) => "jmate",
        (LinkFamily::Relation, "rack_pinion") => "rackpinion",
        (LinkFamily::Relation, _) => "gear",
        (LinkFamily::Fit, _) => "fit",
        (LinkFamily::FitRule, _) => "rule",
    }
}

/// A path as seen from inside `owner`: `post_fl.post` from `post_fl` is `post`.
fn seen_from<'a>(path: &'a str, owner: &str) -> &'a str {
    if owner.is_empty() {
        return path;
    }
    path.strip_prefix(owner)
        .and_then(|r| r.strip_prefix('.'))
        .unwrap_or(path)
}

/// The id an occurrence path ends in, and the path it's in: `post_fl.stud_a[0]` is
/// (`post_fl`, `stud_a[0]`).
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('.') {
        Some(k) => (&path[..k], &path[k + 1..]),
        None => ("", path),
    }
}

/// One statement's name in a loop: `stud_a[0]` is `stud_a`.
fn stem(id: &str) -> &str {
    id.split('[').next().unwrap_or(id)
}

/// What the occurrences say when the solve gave no structure: each a top-level part, a
/// subassembly by its paths' first word.
fn insts_of(occ: &[OccRow]) -> Vec<InstOut> {
    let mut out: Vec<InstOut> = vec![];
    let mut seen: HashSet<String> = HashSet::new();
    for o in occ {
        let parts: Vec<&str> = o.path.split('.').collect();
        for d in 1..parts.len() {
            let p = parts[..d].join(".");
            if seen.insert(p.clone()) {
                out.push(InstOut {
                    path: p,
                    file: String::new(),
                    sub: true,
                    ground: false,
                    suppressed: false,
                });
            }
        }
        if seen.insert(o.path.clone()) {
            out.push(InstOut {
                path: o.path.clone(),
                file: o.file.clone(),
                sub: false,
                ground: false,
                suppressed: false,
            });
        }
    }
    out
}

impl AsmTree {
    /// The occurrences alone: the browser as their paths give it.
    pub fn build(occ: &[OccRow], dir: Option<&Path>) -> AsmTree {
        AsmTree::build_with(occ, dir, &[], &[], &Flags::default())
    }

    pub fn build_with(
        occ: &[OccRow],
        dir: Option<&Path>,
        insts: &[InstOut],
        links: &[LinkOut],
        flags: &Flags,
    ) -> AsmTree {
        AsmTree::build_full(occ, dir, insts, links, flags, &HashMap::new())
    }

    /// With each part file's features read so far, under its occurrences.
    pub fn build_full(
        occ: &[OccRow],
        dir: Option<&Path>,
        insts: &[InstOut],
        links: &[LinkOut],
        flags: &Flags,
        feats: &HashMap<String, PartFeats>,
    ) -> AsmTree {
        let mut t = AsmTree {
            home: vec![usize::MAX; occ.len()],
            ..AsmTree::default()
        };
        t.explode_groups(occ, dir);
        let made;
        let insts = if insts.is_empty() {
            made = insts_of(occ);
            &made[..]
        } else {
            insts
        };
        let index: HashMap<&str, usize> = occ
            .iter()
            .enumerate()
            .map(|(k, o)| (o.path.as_str(), k))
            .collect();
        let occ_under = |p: &str| -> Vec<usize> {
            match index.get(p) {
                Some(&k) => vec![k],
                None => (0..occ.len()).filter(|&k| under(&occ[k].path, p)).collect(),
            }
        };
        // each relationship's occurrences: a relation's are its two joints'
        let mut ends: Vec<Vec<usize>> = links
            .iter()
            .map(|l| match l.family {
                LinkFamily::Joint | LinkFamily::Constraint | LinkFamily::Fit => {
                    let mut v = occ_under(&l.a.0);
                    v.extend(occ_under(&l.b.0));
                    v
                }
                _ => vec![],
            })
            .collect();
        let by_id: HashMap<String, usize> = links
            .iter()
            .enumerate()
            .map(|(k, l)| (join(&l.owner, &l.id), k))
            .collect();
        for (k, l) in links.iter().enumerate() {
            if l.family == LinkFamily::Relation {
                let mut v = vec![];
                for j in [&l.a.0, &l.b.0] {
                    if let Some(&i) = by_id.get(j) {
                        v.extend(ends[i].clone());
                    }
                }
                ends[k] = v;
            }
        }
        let link_flag: Vec<Option<Flag>> = links
            .iter()
            .map(|l| flags.links.get(&(l.owner.clone(), l.id.clone())).copied())
            .collect();
        // what holds each occurrence and subassembly: relationships written beside it
        let mut holds: HashMap<String, Vec<usize>> = HashMap::new();
        for (k, l) in links.iter().enumerate() {
            if !matches!(
                l.family,
                LinkFamily::Joint | LinkFamily::Constraint | LinkFamily::Fit
            ) {
                continue;
            }
            for p in [&l.a.0, &l.b.0] {
                // the occurrence, and the subassembly beside the link it's inside
                holds.entry(p.clone()).or_default().push(k);
                if let Some((first, _)) = seen_from(p, &l.owner).split_once('.') {
                    holds.entry(join(&l.owner, first)).or_default().push(k);
                }
            }
        }
        let cx = Cx {
            occ,
            insts,
            links,
            ends: &ends,
            link_flag: &link_flag,
            holds: &holds,
            index: &index,
            flags,
            feats,
        };
        let top = t.push(
            None,
            TOP.into(),
            "Assembly".into(),
            String::new(),
            NodeKind::Top,
            "subasm",
            "",
        );
        t.nodes[top].occ = (0..occ.len()).collect();
        t.doc(&cx, Some(top), "");
        // a problem shows on everything above it, so a shut row still says it
        for k in (0..t.nodes.len()).rev() {
            if let (Some(f), Some(p)) = (t.nodes[k].flag, t.nodes[k].parent) {
                t.nodes[p].flag = t.nodes[p].flag.max(Some(f));
            }
        }
        t
    }

    /// Subsystems by the part files' folders, the subassemblies, and the fasteners' ways out:
    /// what the exploded view moves together and how.
    fn explode_groups(&mut self, occ: &[OccRow], dir: Option<&Path>) {
        // files beside the assembly: by a stem's first word when others share it (dummy_d01)
        let mut words: Vec<String> = occ.iter().map(|o| folder_word(&o.file, dir)).collect();
        let mut first_words: HashMap<String, HashSet<&str>> = HashMap::new();
        for (o, w) in occ.iter().zip(&words) {
            if w.is_empty() {
                let stem = part_word(&o.file);
                if let Some((f, _)) = stem.split_once('_') {
                    first_words.entry(f.to_string()).or_default().insert(&o.file);
                }
            }
        }
        for (o, w) in occ.iter().zip(words.iter_mut()) {
            if w.is_empty()
                && let Some((f, _)) = part_word(&o.file).split_once('_')
                && first_words.get(f).is_some_and(|s| s.len() > 1)
            {
                *w = f.to_string();
            }
        }
        let mut groups: Vec<((usize, String), Vec<usize>)> = vec![];
        for (k, w) in words.iter().enumerate() {
            let g = group_label(w);
            match groups.iter_mut().find(|x| x.0.1 == g.1) {
                Some(x) => x.1.push(k),
                None => groups.push((g, vec![k])),
            }
        }
        groups.sort_by(|a, b| a.0.cmp(&b.0));
        let flat = groups.len() == 1;
        self.axis = occ
            .iter()
            .zip(&words)
            .map(|(o, w)| {
                HARDWARE
                    .contains(&w.as_str())
                    .then(|| if screw_like(&o.file) { -1.0 } else { 1.0 })
            })
            .collect();
        self.group = vec![0; occ.len()];
        for (g, (_, ks)) in groups.iter().enumerate() {
            for &k in ks {
                self.group[k] = if flat { k } else { g };
            }
        }
        let mut subs: Vec<&str> = vec![];
        self.sub = occ
            .iter()
            .map(|o| {
                let (inst, _) = o.path.split_once('.')?;
                Some(match subs.iter().position(|s| *s == inst) {
                    Some(u) => u,
                    None => {
                        subs.push(inst);
                        subs.len() - 1
                    }
                })
            })
            .collect();
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        parent: Option<usize>,
        key: String,
        label: String,
        detail: String,
        kind: NodeKind,
        icon: &'static str,
        at: &str,
    ) -> usize {
        let depth = parent.map_or(0, |p| self.nodes[p].depth + 1);
        let k = self.nodes.len();
        self.by_key.insert(key.clone(), k);
        self.nodes.push(Node {
            key,
            label,
            detail,
            kind,
            icon,
            depth,
            parent,
            kids: vec![],
            occ: vec![],
            flag: None,
            ground: false,
            at: at.to_string(),
        });
        match parent {
            Some(p) => self.nodes[p].kids.push(k),
            None => self.roots.push(k),
        }
        k
    }

    /// One document's rows under `parent`: its Origin, its Relationships, what holds it in the
    /// document above, its components.
    fn doc(&mut self, cx: &Cx, parent: Option<usize>, owner: &str) {
        // the top's own document is the top's: nothing holds it
        let o = self.push(
            parent,
            format!("o:{owner}"),
            "Origin".into(),
            String::new(),
            NodeKind::Origin,
            "ucs",
            owner,
        );
        for (k, item) in crate::origin::ITEMS.into_iter().enumerate() {
            self.push(
                Some(o),
                format!("o:{owner}:{k}"),
                item.name().into(),
                String::new(),
                NodeKind::OriginItem(k),
                item.icon(),
                owner,
            );
        }
        // Relationships: by kind, problems first
        let mine: Vec<usize> = (0..cx.links.len())
            .filter(|&k| cx.links[k].owner == owner)
            .collect();
        if !mine.is_empty() {
            let r = self.push(
                parent,
                format!("r:{owner}"),
                "Relationships".into(),
                String::new(),
                NodeKind::Relationships,
                "relfolder",
                owner,
            );
            let mut kinds: Vec<((LinkFamily, &str), Vec<usize>)> = vec![];
            for &k in &mine {
                let l = &cx.links[k];
                let key = (l.family, l.kind.as_str());
                match kinds.iter_mut().find(|x| x.0 == key) {
                    Some(x) => x.1.push(k),
                    None => kinds.push((key, vec![k])),
                }
            }
            for x in &mut kinds {
                x.1.sort_by_key(|&k| std::cmp::Reverse(cx.link_flag[k]));
            }
            kinds.sort_by_key(|x| {
                (
                    std::cmp::Reverse(x.1.iter().map(|&k| cx.link_flag[k]).max().flatten()),
                    x.0.0,
                    std::cmp::Reverse(x.1.len()),
                )
            });
            for ((family, kind), ks) in kinds {
                let g = self.push(
                    Some(r),
                    format!("r:{owner}:{family:?}:{kind}"),
                    kind_label(family, kind),
                    String::new(),
                    NodeKind::LinkKind,
                    kind_icon(family, kind),
                    owner,
                );
                for k in ks {
                    self.link_row(cx, g, format!("l:{k}"), k, None);
                }
            }
        }
        if let Some(p) = parent.filter(|_| !owner.is_empty()) {
            self.held(cx, p, owner);
        }
        // components, in the order the file makes them; one statement's in a loop together
        let kids: Vec<&InstOut> = cx
            .insts
            .iter()
            .filter(|i| split_path(&i.path).0 == owner)
            .collect();
        let mut done: HashSet<usize> = HashSet::new();
        for (n, i) in kids.iter().enumerate() {
            if done.contains(&n) {
                continue;
            }
            let id = split_path(&i.path).1;
            let same: Vec<usize> = if i.sub || !id.contains('[') {
                vec![n]
            } else {
                (n..kids.len())
                    .filter(|&m| {
                        !kids[m].sub
                            && kids[m].file == i.file
                            && stem(split_path(&kids[m].path).1) == stem(id)
                    })
                    .collect()
            };
            done.extend(same.iter().copied());
            if same.len() > 1 {
                let p = self.push(
                    parent,
                    format!("p:{}", join(owner, stem(id))),
                    stem(id).into(),
                    type_label(&i.file),
                    NodeKind::Pattern,
                    "rect",
                    owner,
                );
                let mut all = vec![];
                for m in same {
                    let o = self.occ_row(cx, p, kids[m]);
                    all.extend(self.nodes[o].occ.clone());
                }
                self.nodes[p].occ = all;
            } else if i.sub {
                let s = self.push(
                    parent,
                    format!("s:{}", i.path),
                    id.into(),
                    part_word(&i.file),
                    NodeKind::Sub,
                    "subasm",
                    owner,
                );
                self.nodes[s].ground = i.ground;
                self.nodes[s].occ = (0..cx.occ.len())
                    .filter(|&k| under(&cx.occ[k].path, &i.path))
                    .collect();
                self.doc(cx, Some(s), &i.path);
            } else {
                self.occ_row(cx, parent.unwrap_or(usize::MAX), i);
            }
        }
    }

    /// A part occurrence's row, the relationships that hold it under it.
    fn occ_row(&mut self, cx: &Cx, parent: usize, i: &InstOut) -> usize {
        let parent = (parent != usize::MAX).then_some(parent);
        let (owner, id) = split_path(&i.path);
        let n = self.push(
            parent,
            i.path.clone(),
            id.into(),
            type_label(&i.file),
            NodeKind::Occ,
            "part",
            owner,
        );
        self.nodes[n].ground = i.ground;
        if let Some(&k) = cx.index.get(i.path.as_str()) {
            self.nodes[n].occ = vec![k];
            self.home[k] = n;
        }
        self.nodes[n].flag = cx.flags.occ.get(&i.path).copied();
        self.held(cx, n, &i.path);
        // the file as the solve resolved it, which the features are read by
        let file = cx.index.get(i.path.as_str()).map(|&k| cx.occ[k].file.as_str());
        match file.and_then(|f| cx.feats.get(f)) {
            Some(Ok(feats)) => self.part_rows(n, &i.path, feats, owner),
            Some(Err(e)) => {
                let k = self.push(
                    Some(n),
                    format!("{}>f:", i.path),
                    "Features not read".into(),
                    e.clone(),
                    NodeKind::Feature,
                    "warning",
                    owner,
                );
                self.nodes[k].flag = Some(Flag::Warning);
            }
            None => {}
        }
        n
    }

    /// A part occurrence's own tree under its row, as the part's browser has it: its Origin,
    /// then its features in the order the file makes them, a loop's under its name.
    fn part_rows(&mut self, n: usize, path: &str, feats: &[super::fg::FeatRow], owner: &str) {
        let o = self.push(
            Some(n),
            format!("{path}>o"),
            "Origin".into(),
            String::new(),
            NodeKind::PartOrigin,
            "ucs",
            owner,
        );
        for (k, item) in crate::origin::ITEMS.into_iter().enumerate() {
            self.push(
                Some(o),
                format!("{path}>o:{k}"),
                item.name().into(),
                String::new(),
                NodeKind::PartOriginItem(k),
                item.icon(),
                owner,
            );
        }
        let mut stack: Vec<usize> = vec![n];
        for r in super::fg::tree_rows(feats) {
            stack.truncate(r.depth + 1);
            let kind = r.feat.map(|f| feats[f].kind.as_str());
            let at = self.push(
                stack.last().copied(),
                format!("{path}>f:{}", r.key),
                r.label.clone(),
                kind.unwrap_or_default().to_string(),
                NodeKind::Feature,
                kind.map_or("pattern", super::panel::icon),
                owner,
            );
            self.nodes[at].occ = self.nodes[n].occ.clone();
            stack.push(at);
        }
    }

    /// Under a component: the relationships that hold it, problems first.
    fn held(&mut self, cx: &Cx, n: usize, path: &str) {
        let mut ks: Vec<usize> = cx.holds.get(path).cloned().unwrap_or_default();
        ks.dedup();
        ks.sort_by_key(|&k| std::cmp::Reverse(cx.link_flag[k]));
        for k in ks {
            self.link_row(cx, n, format!("{path}>l:{k}"), k, Some(path));
        }
    }

    /// A relationship's row: its name, and what it ties (from a component, the other side).
    fn link_row(&mut self, cx: &Cx, parent: usize, key: String, k: usize, from: Option<&str>) -> usize {
        let l = &cx.links[k];
        let at = &self.nodes[parent].at.clone();
        let side = |s: &(String, String)| -> String {
            let p = seen_from(&s.0, at);
            match l.family {
                LinkFamily::Relation => format!("{p}.{}", s.1),
                _ => p.to_string(),
            }
        };
        let detail = match from {
            Some(me) => {
                let me_a = under(&l.a.0, me);
                let other = if me_a { &l.b } else { &l.a };
                format!("{} ({})", l.kind.replace('_', " "), side(other))
            }
            None if l.family == LinkFamily::FitRule => {
                format!("({}, {})", part_word(&l.a.0), part_word(&l.b.0))
            }
            None => format!("({}, {})", side(&l.a), side(&l.b)),
        };
        let n = self.push(
            Some(parent),
            key,
            l.id.clone(),
            detail,
            NodeKind::Link(k),
            kind_icon(l.family, &l.kind),
            &l.owner,
        );
        self.nodes[n].occ = cx.ends[k].clone();
        self.nodes[n].flag = cx.link_flag[k];
        n
    }

    pub fn find(&self, key: &str) -> Option<usize> {
        self.by_key.get(key).copied()
    }
}

/// What the browser is made from, once per build.
struct Cx<'a> {
    occ: &'a [OccRow],
    insts: &'a [InstOut],
    links: &'a [LinkOut],
    ends: &'a [Vec<usize>],
    link_flag: &'a [Option<Flag>],
    holds: &'a HashMap<String, Vec<usize>>,
    index: &'a HashMap<&'a str, usize>,
    flags: &'a Flags,
    feats: &'a HashMap<String, PartFeats>,
}

/// A part file's features as its browser lists them, or why they couldn't be read.
pub type PartFeats = Result<Vec<super::fg::FeatRow>, String>;

/// The occurrence and feature a feature row's key names: `<occurrence>>f:<feature>`.
pub fn feature_of(key: &str) -> Option<(&str, &str)> {
    key.split_once(">f:")
}

/// The top row's key.
pub const TOP: &str = "a:";

fn join(owner: &str, id: &str) -> String {
    if owner.is_empty() {
        id.to_string()
    } else {
        format!("{owner}.{id}")
    }
}

#[derive(Default)]
pub struct AsmPanel {
    pub occ: Vec<OccRow>,
    /// The browser's rows, made again when the solve or its problems change.
    pub tree: AsmTree,
    /// The assembly's folder: where its part files' folders are seen from.
    pub dir: Option<PathBuf>,
    /// Every occurrence and relationship through subassemblies, and what the issues flag.
    pub insts: Vec<InstOut>,
    pub links: Vec<LinkOut>,
    pub flags: Flags,
    /// Each top-level instance's pose: where a subassembly's origin is drawn.
    pub poses: Vec<(String, forge_math::Frame)>,
    pub joints: Vec<JointOut>,
    pub mobility: usize,
    pub bom: Vec<(String, usize)>,
    pub expects: Vec<ExpectOut>,
    /// The overlap check's counts once it ran: interferences, intended contacts.
    pub overlap: Option<(usize, usize)>,
    /// The overlap check is running.
    pub checking: bool,
    pub timings: String,
    /// The browser row chosen, by its key: an occurrence's path, or a row standing for several.
    pub selected: Option<String>,
    /// Hidden occurrences, by path: kept through a solve.
    pub hidden: HashSet<String>,
    /// The same, by index: what the browser and the view read.
    hid: Vec<bool>,
    /// Only this row (and what's under it) shown.
    pub isolated: Option<String>,
    /// What an issue's Show picked out: occurrences, and joints or constraints by id.
    pub highlight: Vec<String>,
    pub highlight_items: Vec<String>,
    /// The chosen part's file and its anchors (or why not), when asked.
    pub anchors: Option<(String, Result<Vec<String>, String>)>,
    /// Browser rows opened: every row starts shut.
    pub expanded: HashSet<String>,
    /// The browser's filter, and which rows it keeps (made again when it changes).
    pub filter: String,
    kept: Option<(String, Vec<bool>)>,
    /// A row to scroll the browser to, once.
    pub reveal: Option<String>,
    /// Which lists are open in the dialogs (expects), once chosen.
    pub open: std::collections::HashMap<&'static str, bool>,
    /// A drive's slider value while it's moved, before it's let go.
    pub dragging: Option<(String, f64)>,
    /// How far apart the exploded view's asked to be: 0 home, 1 fully apart (the view follows).
    pub explode: f32,
    /// Trails from each exploded part's home.
    pub trails: bool,
    /// The grounded top-level instances: their group stays when exploding.
    pub grounded: Vec<String>,
    /// Subassemblies' origin items shown, by (subassembly, item).
    pub sub_origins: BTreeSet<(String, usize)>,
    /// The dialogs up: Drive (a relationship, by index), iProperties (an occurrence's path),
    /// Analyze Interference, the bill of materials, the explode controls.
    pub drive: Option<usize>,
    pub props: Option<String>,
    pub interference: bool,
    pub bom_open: bool,
    pub explode_open: bool,
    /// The authoring tool up: Place, Joint, Constrain, Pattern, Free Move or Free Rotate.
    pub tool: Option<super::asmtools::AsmTool>,
    /// What's under the pointer while a tool wants an origin (the host fills it each frame).
    pub hover: Option<super::asmtools::Hover>,
    /// Each part file's snap points, by its path as the solve names it (none yet: being found).
    pub snaps: HashMap<String, Result<std::sync::Arc<crate::snaps::PartSnaps>, String>>,
    /// Free Move's and Free Rotate's moves, by top-level instance: a world motion, not in the file.
    pub moved: std::collections::BTreeMap<String, forge_math::Frame>,
    /// The browser row last clicked, and when: a second click soon on it is a double click.
    pub last_click: Option<(String, f64)>,
    /// Each part file's features, by the file as the solve names it: read when one of its
    /// occurrences is first opened.
    pub part_feats: HashMap<String, PartFeats>,
    /// A part's feature edited in the assembly (Inventor's in-place edit).
    pub in_place: Option<super::asmpart::InPlace>,
    /// Where each relationship's two ends are, solved (`AsmOut::link_at`).
    pub link_at: Vec<[Option<forge_math::Frame>; 2]>,
    /// The relationships' glyphs in the view: which are off.
    pub glyphs: super::asmglyph::Glyphs,
    /// The relationship under the pointer, in the view or on its browser row.
    pub hover_link: Option<usize>,
}

/// Whether `path` is the row `key` or under it.
pub fn under(path: &str, key: &str) -> bool {
    path == key || path.starts_with(&format!("{key}.")) || path.starts_with(&format!("{key}["))
}

/// A browser row's eye: all shown, some, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eye {
    Shown,
    Some,
    Hidden,
}

impl AsmPanel {
    /// Whether occurrence `k` is (in) a grounded instance.
    pub fn grounded(&self, k: usize) -> bool {
        self.grounded.iter().any(|g| under(&self.occ[k].path, g))
    }

    /// The solve's occurrences alone.
    pub fn set_occ(&mut self, occ: Vec<OccRow>, dir: Option<PathBuf>) {
        let (insts, links, flags) = (self.insts.clone(), self.links.clone(), self.flags.clone());
        self.set_structure(occ, dir, insts, links, flags);
    }

    /// The solve's occurrences, relationships and problems: the browser made again if they
    /// changed, what was hidden kept.
    pub fn set_structure(
        &mut self,
        occ: Vec<OccRow>,
        dir: Option<PathBuf>,
        insts: Vec<InstOut>,
        links: Vec<LinkOut>,
        flags: Flags,
    ) {
        if occ != self.occ
            || dir != self.dir
            || insts != self.insts
            || links != self.links
            || flags != self.flags
            || self.tree.home.len() != occ.len()
        {
            self.tree = AsmTree::build_full(&occ, dir.as_deref(), &insts, &links, &flags, &self.part_feats);
            self.occ = occ;
            self.dir = dir;
            self.insts = insts;
            self.links = links;
            self.flags = flags;
            self.kept = None;
            if self
                .selected
                .as_ref()
                .is_some_and(|k| self.tree.find(k).is_none())
            {
                self.selected = None;
            }
            if self
                .isolated
                .as_ref()
                .is_some_and(|k| self.tree.find(k).is_none())
            {
                self.isolated = None;
            }
            if self.drive.is_some_and(|k| k >= self.links.len()) {
                self.drive = None;
            }
        }
        self.sync();
    }

    /// A relationship's Joint or Constrain dialog, filled from its statement.
    pub fn edit_link(&mut self, source: &str, id: &str) -> Result<(), String> {
        let mut f = super::asmtools::LinkForm::edit(source, id).ok_or(format!(
            "{id}: made in a loop or a call; edit its statement in the file"
        ))?;
        // each end's occurrence: the longest path its reference starts with
        for p in f.picks.iter_mut().flatten() {
            p.occ = self
                .insts
                .iter()
                .map(|i| i.path.as_str())
                .filter(|i| p.reference.starts_with(&format!("{i}.")))
                .max_by_key(|i| i.len())
                .unwrap_or_default()
                .to_string();
        }
        self.tool = Some(super::asmtools::AsmTool::Link(Box::new(f)));
        self.drive = None;
        self.in_place = None;
        Ok(())
    }

    /// A Relationships row's eye: its glyphs' in the view (the folder all of them, a kind's
    /// row its kind, a relationship's its own); None for other rows.
    pub fn glyph_eye(&self, node: usize) -> Option<Eye> {
        let n = &self.tree.nodes[node];
        let g = &self.glyphs;
        let one = |l: usize| {
            let lk = &self.links[l];
            super::asmglyph::drawn(lk) && g.shown(lk)
        };
        let drawn = |l: usize| super::asmglyph::drawn(&self.links[l]);
        match n.kind {
            NodeKind::Relationships => Some(if g.hidden { Eye::Hidden } else { Eye::Shown }),
            NodeKind::LinkKind => {
                let ls: Vec<usize> = n
                    .kids
                    .iter()
                    .filter_map(|&c| match self.tree.nodes[c].kind {
                        NodeKind::Link(l) => Some(l),
                        _ => None,
                    })
                    .filter(|&l| drawn(l))
                    .collect();
                let on = ls.iter().filter(|&&l| one(l)).count();
                (!ls.is_empty()).then_some(match on {
                    0 => Eye::Hidden,
                    x if x == ls.len() => Eye::Shown,
                    _ => Eye::Some,
                })
            }
            NodeKind::Link(l) if drawn(l) => Some(if one(l) { Eye::Shown } else { Eye::Hidden }),
            _ => None,
        }
    }

    pub fn flip_glyph_eye(&mut self, node: usize) {
        let n = self.tree.nodes[node].clone();
        match n.kind {
            NodeKind::Relationships => self.glyphs.hidden = !self.glyphs.hidden,
            NodeKind::LinkKind => {
                if let Some(l) = n.kids.iter().find_map(|&c| match self.tree.nodes[c].kind {
                    NodeKind::Link(l) => Some(l),
                    _ => None,
                }) {
                    let lk = self.links[l].clone();
                    self.glyphs.flip_kind(lk.family, &lk.kind);
                    // a kind turned on shows each of its own again
                    if !self
                        .glyphs
                        .kinds_off
                        .contains(&super::asmglyph::kind_key(lk.family, &lk.kind))
                    {
                        for &c in &n.kids {
                            if let NodeKind::Link(l) = self.tree.nodes[c].kind {
                                let k = super::asmglyph::link_key(&self.links[l]);
                                self.glyphs.off.remove(&k);
                            }
                        }
                    }
                }
            }
            NodeKind::Link(l) => {
                let lk = self.links[l].clone();
                self.glyphs.flip_link(&lk);
            }
            _ => {}
        }
    }

    /// The browser made again from what it was made of (a part's features read since).
    pub fn remake_tree(&mut self) {
        self.tree = AsmTree::build_full(
            &self.occ,
            self.dir.as_deref(),
            &self.insts,
            &self.links,
            &self.flags,
            &self.part_feats,
        );
        self.kept = None;
    }

    fn sync(&mut self) {
        self.hid = self.occ.iter().map(|o| self.hidden.contains(&o.path)).collect();
    }

    /// The occurrences a row (or an occurrence path, or a path an issue names) stands for.
    pub fn occ_of(&self, key: &str) -> Vec<usize> {
        match self.tree.find(key) {
            Some(n) => self.tree.nodes[n].occ.clone(),
            None => (0..self.occ.len())
                .filter(|&k| under(&self.occ[k].path, key))
                .collect(),
        }
    }

    /// Hides (or shows) everything a row stands for.
    pub fn set_hidden(&mut self, key: &str, hide: bool) {
        for k in self.occ_of(key) {
            let p = self.occ[k].path.clone();
            if hide {
                self.hidden.insert(p);
            } else {
                self.hidden.remove(&p);
            }
        }
        self.sync();
    }

    pub fn show_all(&mut self) {
        self.hidden.clear();
        self.isolated = None;
        self.sync();
    }

    pub fn eye(&self, node: usize) -> Eye {
        let occ = &self.tree.nodes[node].occ;
        let hidden = occ
            .iter()
            .filter(|&&k| self.hid.get(k).copied().unwrap_or(false))
            .count();
        match hidden {
            0 => Eye::Shown,
            h if h == occ.len() => Eye::Hidden,
            _ => Eye::Some,
        }
    }

    /// The row's eye clicked: all hidden if any was shown, else all shown.
    pub fn flip_eye(&mut self, key: &str) {
        let hide = self.tree.find(key).is_none_or(|n| self.eye(n) != Eye::Hidden);
        self.set_hidden(key, hide);
    }

    /// Which occurrences are drawn: not hidden, and inside what's isolated.
    pub fn shown_mask(&self) -> Vec<bool> {
        let mut m: Vec<bool> = (0..self.occ.len())
            .map(|k| !self.hid.get(k).copied().unwrap_or(false))
            .collect();
        if let Some(i) = &self.isolated {
            let mut inside = vec![false; m.len()];
            for k in self.occ_of(i) {
                inside[k] = true;
            }
            for (a, b) in m.iter_mut().zip(inside) {
                *a &= b;
            }
        }
        m
    }

    pub fn picked_mask(&self) -> Vec<bool> {
        let mut m = vec![false; self.occ.len()];
        if let Some(s) = &self.selected {
            for k in self.occ_of(s) {
                m[k] = true;
            }
        }
        m
    }

    /// Whether the occurrence at `path` is drawn.
    pub fn shown(&self, path: &str) -> bool {
        let Some(k) = self.occ.iter().position(|o| o.path == path) else {
            return false;
        };
        self.shown_mask()[k]
    }

    pub fn picked(&self, path: &str) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|k| self.occ_of(k).iter().any(|&i| self.occ[i].path == path))
    }

    pub fn flagged(&self, path: &str) -> bool {
        self.highlight.iter().any(|h| under(path, h))
    }

    /// An occurrence chosen in the view: chosen here too, its row opened to and scrolled to.
    pub fn reveal_occ(&mut self, k: Option<usize>) {
        let Some(k) = k else {
            self.selected = None;
            return;
        };
        let path = self.occ[k].path.clone();
        let mut at = self.tree.home.get(k).copied().filter(|&n| n != usize::MAX);
        let key = at.map_or(path.clone(), |n| self.tree.nodes[n].key.clone());
        while let Some(n) = at {
            at = self.tree.nodes[n].parent;
            if let Some(p) = at {
                if self.tree.nodes[p].kind == NodeKind::Top {
                    self.expanded.remove(TOP);
                } else {
                    self.expanded.insert(self.tree.nodes[p].key.clone());
                }
            }
        }
        self.selected = Some(key.clone());
        self.reveal = Some(key);
    }

    /// Which rows the filter keeps: those it finds and every row above them.
    fn kept(&mut self) -> Option<&Vec<bool>> {
        let f = self.filter.trim().to_lowercase();
        if f.is_empty() {
            return None;
        }
        if self.kept.as_ref().is_none_or(|k| k.0 != f) {
            let n = &self.tree.nodes;
            let mut keep: Vec<bool> = n
                .iter()
                .map(|x| x.label.to_lowercase().contains(&f) || x.detail.to_lowercase().contains(&f))
                .collect();
            // kids come after their parents: up from the leaves
            for k in (0..n.len()).rev() {
                if keep[k]
                    && let Some(p) = n[k].parent
                {
                    keep[p] = true;
                }
            }
            self.kept = Some((f, keep));
        }
        self.kept.as_ref().map(|k| &k.1)
    }

    /// The browser's rows in order, each with whether it's open: what's opened, and with a
    /// filter what holds a row it finds (a row it finds itself stays as it was).
    pub fn rows(&mut self) -> Vec<(usize, bool)> {
        let keep = self.kept().cloned();
        let f = self.filter.trim().to_lowercase();
        let mut out = vec![];
        let mut stack: Vec<usize> = self.tree.roots.iter().rev().copied().collect();
        while let Some(k) = stack.pop() {
            if keep.as_ref().is_some_and(|m| !m[k]) {
                continue;
            }
            let n = &self.tree.nodes[k];
            let found_under = keep
                .as_ref()
                .is_some_and(|m| n.kids.iter().any(|&c| m[c]) && !n.label.to_lowercase().contains(&f));
            // the top starts open: its key in `expanded` shuts it
            let open = !n.kids.is_empty()
                && (found_under || self.expanded.contains(&n.key) != (n.kind == NodeKind::Top));
            out.push((k, open));
            if open {
                stack.extend(n.kids.iter().rev());
            }
        }
        out
    }

    /// An origin row's state: an item's on or off; a folder's all, some or none.
    pub fn origin_eye(&self, n: usize, top: &[bool; 7]) -> Eye {
        let node = &self.tree.nodes[n];
        let on = |k: usize| {
            if node.at.is_empty() {
                top[k]
            } else {
                self.sub_origins.contains(&(node.at.clone(), k))
            }
        };
        let ks: Vec<usize> = match node.kind {
            NodeKind::OriginItem(k) => vec![k],
            _ => (0..7).collect(),
        };
        match ks.iter().filter(|&&k| on(k)).count() {
            0 => Eye::Hidden,
            c if c == ks.len() => Eye::Shown,
            _ => Eye::Some,
        }
    }

    /// The pose of a subassembly occurrence, when it's a top-level one.
    pub fn pose_of(&self, sub: &str) -> Option<forge_math::Frame> {
        self.poses.iter().find(|p| p.0 == sub).map(|p| p.1)
    }

    /// A relationship's id as issues name it: inside its subassembly, by its path.
    pub fn link_name(&self, k: usize) -> String {
        let l = &self.links[k];
        join(&l.owner, &l.id)
    }

    /// The joint as solved, when the top document writes it.
    /// The freedoms an occurrence has left: none grounded, its joints' free ones when held by
    /// joints, all six when nothing holds it.
    pub fn dof(&self, occ: &str) -> usize {
        let top = occ.split('.').next().unwrap_or(occ);
        if self.grounded.iter().any(|g| g == top) {
            return 0;
        }
        let held: Vec<&LinkOut> = self
            .links
            .iter()
            .filter(|l| l.owner.is_empty() && !l.suppressed)
            .filter(|l| matches!(l.family, LinkFamily::Joint | LinkFamily::Constraint))
            .filter(|l| under(&l.a.0, top) || under(&l.b.0, top))
            .collect();
        if held.is_empty() {
            return 6;
        }
        held.iter()
            .filter_map(|l| self.joints.iter().find(|j| j.id == l.id))
            .map(|j| j.free)
            .min()
            .unwrap_or(0)
    }

    pub fn joint_of(&self, k: usize) -> Option<&JointOut> {
        let l = &self.links[k];
        if !l.owner.is_empty() {
            return None;
        }
        self.joints.iter().find(|j| j.id == l.id)
    }
}

/// What the issues flag, Inventor's way: a sick relationship red or yellow, two parts that
/// interfere yellow.
pub fn issue_flags(issues: &[super::fg::Issue]) -> Flags {
    fn marks(issue: &AsmIssue, owner: &str, occ: &mut Vec<String>, items: &mut Vec<(String, String)>) {
        let mut item = |id: &str| items.push((owner.to_string(), id.to_string()));
        match issue {
            AsmIssue::Interference { a, b, .. } | AsmIssue::ShallowEngagement { a, b, .. } => {
                occ.push(join(owner, a));
                occ.push(join(owner, b));
            }
            AsmIssue::Conflict { items: is, .. } | AsmIssue::NotConverged { items: is, .. } => {
                for i in is {
                    item(&i.id);
                }
            }
            AsmIssue::Redundant { item: i, .. } => item(&i.id),
            AsmIssue::BrokenAnchor { item: i, .. }
            | AsmIssue::WrongReference { item: i, .. }
            | AsmIssue::SelfLink { item: i } => item(i),
            AsmIssue::LimitViolated { joint, .. } | AsmIssue::BadFreedom { joint, .. } => item(joint),
            AsmIssue::BadRelation { relation } => item(relation),
            AsmIssue::Inside { path, issue } => marks(issue, &join(owner, path), occ, items),
            AsmIssue::AtPastLimit { limit } => marks(limit, owner, occ, items),
            _ => {}
        }
    }
    let mut f = Flags::default();
    for is in issues {
        let Some(ai) = &is.asm else { continue };
        let flag = if is.severity == Severity::Error {
            Flag::Error
        } else {
            Flag::Warning
        };
        let (mut occ, mut items) = (vec![], vec![]);
        marks(ai, "", &mut occ, &mut items);
        for o in occ {
            let e = f.occ.entry(o).or_insert(flag);
            *e = (*e).max(flag);
        }
        for i in items {
            let e = f.links.entry(i).or_insert(flag);
            *e = (*e).max(flag);
        }
    }
    f
}

/// What an assembly issue names, to highlight: occurrences and items.
pub fn named(issue: &AsmIssue) -> (Vec<String>, Vec<String>) {
    match issue {
        AsmIssue::Interference { a, b, .. } | AsmIssue::ShallowEngagement { a, b, .. } => {
            (vec![a.clone(), b.clone()], vec![])
        }
        AsmIssue::IntendedContact { a, b, by } => (vec![a.clone(), b.clone()], vec![by.clone()]),
        AsmIssue::Conflict { items, .. } => (vec![], items.iter().map(|i| i.id.clone()).collect()),
        AsmIssue::Redundant { item, implied_by, .. } => (
            vec![],
            std::iter::once(item.id.clone())
                .chain(implied_by.iter().map(|i| i.id.clone()))
                .collect(),
        ),
        AsmIssue::NotConverged { items, .. } => (vec![], items.iter().map(|i| i.id.clone()).collect()),
        AsmIssue::BrokenAnchor { item, .. }
        | AsmIssue::WrongReference { item, .. }
        | AsmIssue::SelfLink { item } => (vec![], vec![item.clone()]),
        AsmIssue::LimitViolated { joint, .. } | AsmIssue::BadFreedom { joint, .. } => {
            (vec![], vec![joint.clone()])
        }
        AsmIssue::Inside { path, issue } => {
            let (occ, items) = named(issue);
            (occ.into_iter().map(|o| format!("{path}.{o}")).collect(), items)
        }
        _ => (vec![], vec![]),
    }
}

/// What a browser row's right-click menu asks.
enum MenuAct {
    Visibility(String),
    Isolate(String),
    Props(String),
    Drive(usize),
    Expand(String),
    /// An edit to the file: ground, suppress, delete.
    Ops(Vec<Op>),
    /// Another occurrence of this one's file.
    Again(String),
    Pattern(String),
    /// A relationship's dialog, filled from its statement.
    Edit(String),
    /// A part's feature, edited in the assembly.
    InPlace(String),
}

const TOOLBAR_H: f32 = 34.0;
const INDENT: f32 = 14.0;
const WARN: Color32 = Color32::from_rgb(0xE0, 0xA0, 0x30);

/// A dialog's frame, as the feature dialogs have it.
pub(super) fn dialog_frame(t: &super::theme::Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(6)
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(90),
        })
        .inner_margin(12)
}

/// A dialog's title row with its close box: true when closed.
pub(super) fn dialog_head(
    ui: &mut Ui,
    t: &super::theme::Theme,
    icon: &str,
    title: &str,
    name: &str,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let w = ui.available_width();
    let (head, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    icons::draw(
        ui.painter(),
        head.left_top() + vec2(0.0, 2.0),
        20.0,
        icon,
        1.5,
        t.ink,
    );
    ui.painter().text(
        pos2(head.left() + 28.0, head.center().y),
        Align2::LEFT_CENTER,
        title,
        semi(14.0),
        t.ink,
    );
    let x = Rect::from_center_size(pos2(head.right() - 10.0, head.center().y), vec2(20.0, 20.0));
    let r = ui.interact(x, ui.scope_id().with(("close", name)), Sense::click());
    icons::draw(
        ui.painter(),
        x.left_top() + vec2(2.0, 2.0),
        16.0,
        "close",
        1.5,
        if r.hovered() { t.ink } else { t.ink2 },
    );
    places.push((format!("{name}:close"), x));
    r.clicked()
}

impl Shell {
    /// The assembly's Model browser, in the side panel's place: a filter and Show all above its rows.
    pub(super) fn asm_tree(&mut self, ui: &mut Ui, area: Rect) {
        let Some(mut fg) = self.fg.take() else { return };
        let source = fg.source.clone();
        let mut pending: Vec<FgAct> = Vec::new();
        let Some(a) = &mut fg.asm else {
            self.fg = Some(fg);
            return;
        };
        let t = self.theme;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut changed = false;
        // the filter, and Show all while anything's hidden or isolated
        let bar = Rect::from_min_size(area.min, vec2(area.width(), TOOLBAR_H));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(8.0, 5.0))));
        child.horizontal(|ui| {
            let hiding = a.isolated.is_some() || !a.hidden.is_empty();
            let w = ui.available_width() - if hiding { 74.0 } else { 0.0 };
            let r = ui.add(
                egui::TextEdit::singleline(&mut a.filter)
                    .id(egui::Id::unique("asm-filter"))
                    .font(sans(12.0))
                    .hint_text("Filter")
                    .desired_width(w),
            );
            places.push(("asm:filter".into(), r.rect));
            if hiding && Self::fg_button(ui, "Show all", "asm:treeshowall".into(), &mut places) {
                a.show_all();
                changed = true;
            }
        });
        ui.painter()
            .hline(area.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, t.line));
        let list = Rect::from_min_max(pos2(area.left(), bar.bottom()), area.max);
        let rows = a.rows();
        let mut chosen = None;
        let mut toggle = None;
        let mut eye = None;
        let mut menu: Option<MenuAct> = None;
        let mut row_hover: Option<usize> = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("fg-asm-tree")
            .auto_shrink([false, false]);
        if let Some(k) = a.reveal.take()
            && let Some(i) = rows.iter().position(|&(n, _)| a.tree.nodes[n].key == k)
        {
            let y = (i as f32 * super::ROW_H - list.height() * 0.4).max(0.0);
            scroll = scroll.vertical_scroll_offset(y);
        }
        // a second click on the same row soon after the first: a double click, however egui
        // counted it
        let last = std::cell::Cell::new(a.last_click.take());
        let second = |key: &str, now: f64| {
            let was = last.take();
            let double = was.as_ref().is_some_and(|(k, t)| k == key && now - t < 0.5);
            last.set(if double {
                None
            } else {
                Some((key.to_string(), now))
            });
            double
        };
        let shown_mask = a.isolated.as_ref().map(|_| a.shown_mask());
        let top_origin = self.origin_shown;
        let top_name = self.name.clone();
        scroll.show_rows(&mut child, super::ROW_H, rows.len(), |ui, range| {
            for &(k, open) in &rows[range] {
                let n = &a.tree.nodes[k];
                let (r, resp) = ui.allocate_exact_size(vec2(list.width(), super::ROW_H), Sense::click());
                let on = a.selected.as_deref() == Some(n.key.as_str());
                resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::TreeItem, true, on, &n.label));
                let p = ui.painter();
                // a relationship lit in the view lights its rows too
                let lit = matches!(n.kind, NodeKind::Link(l) if a.hover_link == Some(l));
                if on {
                    p.rect_filled(r, 0.0, t.on);
                    p.rect_filled(Rect::from_min_size(r.min, vec2(2.0, super::ROW_H)), 0.0, t.focus);
                } else if resp.hovered() || lit {
                    p.rect_filled(r, 0.0, t.hover);
                }
                if lit {
                    p.rect_filled(
                        Rect::from_min_size(r.min, vec2(2.0, super::ROW_H)),
                        0.0,
                        Color32::from_rgb(0x40, 0x9E, 0xF2),
                    );
                }
                let x = r.left() + 10.0 + n.depth as f32 * INDENT;
                // a part's own tree is read when its row first opens
                if !n.kids.is_empty() || n.kind == NodeKind::Occ {
                    let arrow = Rect::from_center_size(pos2(x, r.center().y), vec2(16.0, super::ROW_H));
                    let ar = ui.interact(arrow, ui.scope_id().with(("asm-open", &n.key)), Sense::click());
                    super::fg::disclosure(
                        ui.painter(),
                        arrow.center(),
                        open,
                        if ar.hovered() { t.ink } else { t.ink2 },
                    );
                    if ar.clicked() {
                        toggle = Some((n.key.clone(), open));
                    }
                    places.push((format!("asm:open:{}", n.key), arrow));
                }
                let origin = matches!(n.kind, NodeKind::Origin | NodeKind::OriginItem(_));
                let has_eye = origin
                    || matches!(
                        n.kind,
                        NodeKind::Top | NodeKind::Occ | NodeKind::Pattern | NodeKind::Sub
                    );
                // a relationship row's eye is its glyph's in the view
                let glyph_eye = a.glyph_eye(k);
                let state = if origin {
                    a.origin_eye(k, &top_origin)
                } else if has_eye {
                    a.eye(k)
                } else {
                    glyph_eye.unwrap_or(Eye::Shown)
                };
                let isolated_out =
                    !origin && has_eye && shown_mask.as_ref().is_some_and(|m| n.occ.iter().all(|&o| !m[o]));
                let off = match n.kind {
                    NodeKind::Link(l) => a.links[l].suppressed,
                    NodeKind::Occ | NodeKind::Sub => a.insts.iter().any(|i| i.path == n.key && i.suppressed),
                    _ => false,
                };
                let dim = (has_eye && !origin && state == Eye::Hidden) || isolated_out || off;
                let flag_ink = match n.flag {
                    Some(Flag::Error) => Some(t.error),
                    Some(Flag::Warning) => Some(WARN),
                    None => None,
                };
                let ink = if dim {
                    t.ink2.gamma_multiply(0.55)
                } else if on {
                    t.on_ink
                } else if n.flag == Some(Flag::Error) {
                    t.error
                } else {
                    t.ink
                };
                // the row's icon: its kind's, an origin item in its own colour, red or yellow if sick
                let ix = x + 9.0;
                let icon_ink = match n.kind {
                    NodeKind::OriginItem(i) => {
                        let c = crate::origin::ITEMS[i].colour();
                        Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
                    }
                    _ => flag_ink.unwrap_or(if dim { t.ink2.gamma_multiply(0.55) } else { t.ink2 }),
                };
                icons::draw(
                    ui.painter(),
                    pos2(ix, r.center().y - 8.0),
                    16.0,
                    n.icon,
                    1.25,
                    icon_ink,
                );
                if let Some(c) = flag_ink {
                    let at = pos2(ix + 14.0, r.center().y + 5.0);
                    ui.painter().circle_filled(at, 4.5, c);
                    ui.painter()
                        .text(at, Align2::CENTER_CENTER, "!", medium(8.0), Color32::BLACK);
                }
                if n.ground {
                    icons::draw(
                        ui.painter(),
                        pos2(ix - 2.0, r.center().y - 1.0),
                        11.0,
                        "pin",
                        1.5,
                        t.focus,
                    );
                }
                let font = match n.kind {
                    NodeKind::Top | NodeKind::Origin | NodeKind::Relationships => medium(13.0),
                    _ if on => medium(13.0),
                    _ => sans(13.0),
                };
                let lx = ix + 22.0;
                let label = if n.kind == NodeKind::Top && !top_name.is_empty() {
                    top_name.clone()
                } else {
                    n.label.clone()
                };
                let g = ui.fonts_mut(|f| f.layout_no_wrap(label, font, ink));
                let gw = g.size().x;
                ui.painter()
                    .galley(pos2(lx, r.center().y - g.size().y * 0.5), g, ink);
                // a count after a folder's, a kind's or a pattern's name
                let count = match n.kind {
                    NodeKind::Pattern => format!("×{}", n.kids.len()),
                    NodeKind::LinkKind => n.kids.len().to_string(),
                    NodeKind::Relationships => n
                        .kids
                        .iter()
                        .map(|&c| a.tree.nodes[c].kids.len())
                        .sum::<usize>()
                        .to_string(),
                    _ => String::new(),
                };
                let eye_r = Rect::from_center_size(pos2(r.right() - 16.0, r.center().y), vec2(20.0, 20.0));
                let mut used = lx + gw;
                if !count.is_empty() {
                    let c = ui.painter().text(
                        pos2(used + 6.0, r.center().y),
                        Align2::LEFT_CENTER,
                        count,
                        sans(11.0),
                        t.ink2,
                    );
                    used = c.right();
                }
                // the detail where it fits, cut at its start if it doesn't
                let has_eye = has_eye || glyph_eye.is_some();
                let room = (if has_eye { eye_r.left() } else { r.right() - 8.0 }) - 6.0 - (used + 10.0);
                if !n.detail.is_empty() && room > 30.0 {
                    let mut d = n.detail.clone();
                    let wide = |s: &str, ui: &mut Ui| {
                        ui.fonts_mut(|f| f.layout_no_wrap(s.to_string(), sans(10.0), t.ink2))
                            .size()
                            .x
                    };
                    while wide(&d, ui) > room && d.chars().count() > 2 {
                        d = format!("…{}", d.chars().skip(2).collect::<String>());
                    }
                    ui.painter().text(
                        pos2(used + 10.0 + room, r.center().y),
                        Align2::RIGHT_CENTER,
                        d,
                        sans(10.0),
                        t.ink2,
                    );
                }
                let mut eye_hit = false;
                if has_eye {
                    let er = ui.interact(eye_r, ui.scope_id().with(("asm-eye", &n.key)), Sense::click());
                    let tip = match (glyph_eye.is_some(), state == Eye::Hidden) {
                        (true, true) => "Show in the view",
                        (true, false) => "Hide in the view",
                        (false, true) => "Show",
                        (false, false) => "Hide",
                    };
                    let er = er.on_hover_text(tip);
                    let eink = match state {
                        _ if er.hovered() => t.ink,
                        Eye::Shown => t.ink2,
                        Eye::Some => t.ink2.gamma_multiply(0.7),
                        Eye::Hidden => t.ink2.gamma_multiply(0.5),
                    };
                    icons::draw(
                        ui.painter(),
                        pos2(eye_r.left() + 2.0, eye_r.top() + 2.0),
                        16.0,
                        if state == Eye::Hidden { "hide" } else { "eye" },
                        1.25,
                        eink,
                    );
                    places.push((format!("asm:eye:{}", n.key), eye_r));
                    eye_hit = er.clicked();
                }
                // a relationship's row lights it in the view
                if let NodeKind::Link(l) = n.kind
                    && resp.hovered()
                {
                    row_hover = Some(l);
                }
                // what Inventor shows on hover: the file, what a relationship ties
                let resp = resp.on_hover_ui_at_pointer(|ui| {
                    ui.set_max_width(360.0);
                    Self::asm_row_tip(ui, &t, a, k);
                });
                resp.context_menu(|ui| {
                    ui.set_min_width(150.0);
                    let mut item = |ui: &mut Ui, label: &str, name: &str| {
                        let b = ui.button(egui::RichText::new(label).font(sans(12.0)));
                        places.push((format!("asm:menu:{name}"), b.rect));
                        b.clicked()
                    };
                    match n.kind {
                        NodeKind::Occ | NodeKind::Pattern | NodeKind::Sub => {
                            let vis = if state == Eye::Hidden {
                                "Show"
                            } else {
                                "Visibility off"
                            };
                            if item(ui, vis, "hide") {
                                menu = Some(MenuAct::Visibility(n.key.clone()));
                                ui.close();
                            }
                            let iso = a.isolated.as_deref() == Some(n.key.as_str());
                            if item(ui, if iso { "Undo Isolate" } else { "Isolate" }, "isolate") {
                                menu = Some(MenuAct::Isolate(n.key.clone()));
                                ui.close();
                            }
                            if n.kind == NodeKind::Occ && item(ui, "iProperties…", "props") {
                                menu = Some(MenuAct::Props(n.key.clone()));
                                ui.close();
                            }
                            // what the file says of a top-level component
                            if let Some(i) = a.insts.iter().find(|i| i.path == n.key && !i.path.contains('.'))
                            {
                                ui.separator();
                                let id = i.path.clone();
                                let ground = if i.ground { "Unground" } else { "Ground" };
                                if item(ui, ground, "ground") {
                                    menu =
                                        Some(MenuAct::Ops(vec![super::asmtools::ground_op(&id, !i.ground)]));
                                    ui.close();
                                }
                                let sup = if i.suppressed { "Unsuppress" } else { "Suppress" };
                                if item(ui, sup, "suppress") {
                                    menu = Some(MenuAct::Ops(super::asmtools::suppress_ops(
                                        a,
                                        &id,
                                        !i.suppressed,
                                    )));
                                    ui.close();
                                }
                                if item(ui, "Place again", "again") {
                                    menu = Some(MenuAct::Again(id.clone()));
                                    ui.close();
                                }
                                if !i.sub && item(ui, "Pattern…", "pattern") {
                                    menu = Some(MenuAct::Pattern(id.clone()));
                                    ui.close();
                                }
                                if item(ui, "Delete", "delete") {
                                    menu = Some(MenuAct::Ops(super::asmtools::delete_ops(a, &id)));
                                    ui.close();
                                }
                            }
                        }
                        NodeKind::Link(l) => {
                            let lk = &a.links[l];
                            let top = lk.owner.is_empty()
                                && matches!(
                                    lk.family,
                                    LinkFamily::Joint | LinkFamily::Constraint | LinkFamily::Relation
                                );
                            if top && item(ui, "Edit…", "edit") {
                                menu = Some(MenuAct::Edit(lk.id.clone()));
                                ui.close();
                            }
                            if item(ui, "Drive…", "drive") {
                                menu = Some(MenuAct::Drive(l));
                                ui.close();
                            }
                            if top && lk.family != LinkFamily::Relation {
                                let sup = if lk.suppressed { "Unsuppress" } else { "Suppress" };
                                if item(ui, sup, "suppress") {
                                    menu = Some(MenuAct::Ops(vec![super::asmtools::suppress_op(
                                        &lk.id,
                                        !lk.suppressed,
                                    )]));
                                    ui.close();
                                }
                            }
                            if lk.owner.is_empty() && item(ui, "Delete", "delete") {
                                let mut ops = vec![Op::DeleteFeature { id: lk.id.clone() }];
                                ops.extend(
                                    a.links
                                        .iter()
                                        .filter(|r| r.owner.is_empty() && r.family == LinkFamily::Relation)
                                        .filter(|r| r.a.0 == lk.id || r.b.0 == lk.id)
                                        .map(|r| Op::DeleteFeature { id: r.id.clone() }),
                                );
                                menu = Some(MenuAct::Ops(ops));
                                ui.close();
                            }
                        }
                        NodeKind::Feature if item(ui, "Edit Feature", "edit-feature") => {
                            menu = Some(MenuAct::InPlace(n.key.clone()));
                            ui.close();
                        }
                        _ => {}
                    }
                    if !n.kids.is_empty() && item(ui, if open { "Collapse" } else { "Expand" }, "expand") {
                        menu = Some(MenuAct::Expand(n.key.clone()));
                        ui.close();
                    }
                });
                if eye_hit {
                    eye = Some(k);
                } else if resp.double_clicked() || (resp.clicked() && second(&n.key, ui.input(|i| i.time))) {
                    match n.kind {
                        // Inventor's: a relationship's own dialog, filled; one in a subassembly drives
                        NodeKind::Link(l)
                            if a.links[l].owner.is_empty() && a.links[l].family != LinkFamily::Fit =>
                        {
                            menu = Some(MenuAct::Edit(a.links[l].id.clone()))
                        }
                        NodeKind::Link(l) => menu = Some(MenuAct::Drive(l)),
                        NodeKind::Occ => menu = Some(MenuAct::Props(n.key.clone())),
                        // Inventor's: a part's feature edited where it sits in the assembly
                        NodeKind::Feature if feature_of(&n.key).is_some_and(|(_, f)| !f.is_empty()) => {
                            menu = Some(MenuAct::InPlace(n.key.clone()))
                        }
                        _ if !n.kids.is_empty() => {
                            toggle = Some((n.key.clone(), a.expanded.contains(&n.key)));
                        }
                        _ => {}
                    }
                } else if resp.clicked() {
                    chosen = Some(n.key.clone());
                }
                places.push((format!("asm:row:{}", n.key), r));
            }
        });
        a.last_click = last.take();
        a.glyphs.row = row_hover;
        // opened or shut: the row's key in or out (the top's means shut)
        if let Some((k, _)) = toggle
            && !a.expanded.remove(&k)
        {
            a.read_part(&k);
            a.expanded.insert(k);
        }
        if let Some(k) = eye {
            let n = &a.tree.nodes[k];
            match n.kind {
                NodeKind::Relationships | NodeKind::LinkKind | NodeKind::Link(_) => a.flip_glyph_eye(k),
                NodeKind::Origin | NodeKind::OriginItem(_) => {
                    let ks: Vec<usize> = match n.kind {
                        NodeKind::OriginItem(i) => vec![i],
                        _ => (0..7).collect(),
                    };
                    let turn_on = a.origin_eye(k, &self.origin_shown) == Eye::Hidden;
                    let at = n.at.clone();
                    for i in ks {
                        if at.is_empty() {
                            self.origin_shown[i] = turn_on;
                        } else if turn_on {
                            a.sub_origins.insert((at.clone(), i));
                        } else {
                            a.sub_origins.remove(&(at.clone(), i));
                        }
                    }
                }
                _ => {
                    let key = n.key.clone();
                    a.flip_eye(&key);
                }
            }
            changed = true;
        }
        match menu {
            Some(MenuAct::Visibility(k)) => {
                a.flip_eye(&k);
                changed = true;
            }
            Some(MenuAct::Isolate(k)) => {
                a.isolated = if a.isolated.as_deref() == Some(k.as_str()) {
                    None
                } else {
                    Some(k)
                };
                changed = true;
            }
            Some(MenuAct::Props(k)) => {
                a.props = Some(k.clone());
                a.anchors = None;
                a.selected = Some(k);
                changed = true;
            }
            Some(MenuAct::Drive(l)) => a.drive = Some(l),
            Some(MenuAct::Ops(ops)) => pending.push(FgAct::AsmOps(ops)),
            Some(MenuAct::Again(id)) => {
                let file = a
                    .insts
                    .iter()
                    .find(|i| i.path == id)
                    .map(|i| i.file.clone())
                    .unwrap_or_default();
                pending.push(FgAct::AsmPlace { file, like: Some(id) });
            }
            Some(MenuAct::Pattern(id)) => {
                a.tool = Some(super::asmtools::AsmTool::Pattern(super::asmtools::PatternForm {
                    inst: id,
                    count: "3".into(),
                    spacing: "50".into(),
                    dir: 0,
                }));
            }
            Some(MenuAct::Edit(id)) => {
                if let Err(e) = a.edit_link(&source, &id) {
                    self.status = e;
                }
            }
            Some(MenuAct::InPlace(k)) => match a.edit_in_place(&k) {
                Ok(()) => {
                    a.tool = None;
                    a.drive = None;
                    changed = true;
                    self.status = format!("Editing in place: {}", k.replace(">f:", " › "));
                }
                Err(e) => self.status = e,
            },
            Some(MenuAct::Expand(k)) if !a.expanded.remove(&k) => {
                a.read_part(&k);
                a.expanded.insert(k);
            }
            Some(MenuAct::Expand(_)) => {}
            None => {}
        }
        if let Some(k) = chosen {
            a.selected = if a.selected.as_deref() == Some(k.as_str()) {
                None
            } else {
                Some(k)
            };
            a.anchors = None;
            changed = true;
        }
        if changed {
            fg.out.push(FgAct::AsmShow);
        }
        fg.out.extend(pending);
        self.places.extend(places);
        self.fg = Some(fg);
    }

    /// A row's hover: an occurrence's file and how it's held, what a relationship ties.
    fn asm_row_tip(ui: &mut Ui, t: &super::theme::Theme, a: &AsmPanel, k: usize) {
        let n = &a.tree.nodes[k];
        let line = |ui: &mut Ui, s: String, c: Color32| {
            ui.label(egui::RichText::new(s).font(sans(12.0)).color(c));
        };
        match n.kind {
            NodeKind::Occ | NodeKind::Sub => {
                let file = a
                    .insts
                    .iter()
                    .find(|i| i.path == n.key || format!("s:{}", i.path) == n.key)
                    .map(|i| i.file.clone())
                    .or_else(|| n.occ.first().map(|&o| a.occ[o].file.clone()))
                    .unwrap_or_default();
                line(ui, n.key.trim_start_matches("s:").to_string(), t.ink);
                line(ui, file, t.ink2);
                let held = n
                    .kids
                    .iter()
                    .filter(|&&c| matches!(a.tree.nodes[c].kind, NodeKind::Link(_)))
                    .count();
                let mut s = format!("held by {held} relationship{}", if held == 1 { "" } else { "s" });
                if n.ground {
                    s.push_str(" · grounded");
                }
                line(ui, s, t.ink2);
            }
            NodeKind::Link(l) => {
                let lk = &a.links[l];
                line(
                    ui,
                    format!("{} · {}", a.link_name(l), lk.kind.replace('_', " ")),
                    t.ink,
                );
                for s in [&lk.a, &lk.b] {
                    let text = if s.1.is_empty() {
                        s.0.clone()
                    } else {
                        format!("{} · {}", s.0, s.1)
                    };
                    ui.label(egui::RichText::new(text).font(mono(11.0)).color(t.ink2));
                }
                if let Some(j) = a.joint_of(l) {
                    line(ui, joint_values(j), t.ink2);
                }
            }
            NodeKind::Pattern => line(ui, format!("{} × {}", n.kids.len(), n.detail), t.ink2),
            _ => {
                ui.label(egui::RichText::new(&n.label).font(sans(12.0)).color(t.ink));
            }
        }
    }

    /// The assembly's dialogs: Drive, iProperties, Analyze Interference, the bill of materials,
    /// the explode controls.
    pub(super) fn asm_windows(&mut self, ctx: &egui::Context) {
        let Some(mut fg) = self.fg.take() else { return };
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut out: Vec<FgAct> = Vec::new();
        let mut blocks: Vec<Rect> = Vec::new();
        let feats = fg.feats.clone();
        let issues = fg.issues.clone();
        let source = fg.source.clone();
        if let Some(a) = &mut fg.asm {
            // the joints and constraints in the view; the one under the pointer lights its parts
            let hit = self.asm_link_glyphs(ctx, a);
            let lit = hit.hover.or(a.glyphs.row);
            if lit != a.hover_link {
                a.hover_link = lit;
                out.push(FgAct::AsmShow);
            }
            if let Some(l) = hit.click.or(hit.double) {
                let key = a.link_row_key(l);
                let lk = &a.links[l];
                for open in [
                    format!("r:{}", lk.owner),
                    format!("r:{}:{:?}:{}", lk.owner, lk.family, lk.kind),
                ] {
                    a.expanded.insert(open);
                }
                a.selected = Some(key.clone());
                a.reveal = Some(key);
                out.push(FgAct::AsmShow);
            }
            if let Some(l) = hit.double {
                let id = a.links[l].id.clone();
                if a.links[l].owner.is_empty() {
                    if let Err(e) = a.edit_link(&source, &id) {
                        self.status = e;
                    }
                } else {
                    a.drive = Some(l);
                }
            }
            if a.drive.is_some() {
                blocks.extend(Self::drive_window(ctx, &t, v, a, &feats, &mut out, &mut places));
            }
            if a.props.is_some() {
                blocks.extend(Self::props_window(ctx, &t, v, a, &mut out, &mut places));
            }
            if a.interference {
                blocks.extend(Self::interference_window(
                    ctx,
                    &t,
                    v,
                    a,
                    &issues,
                    &mut out,
                    &mut places,
                ));
            }
            if a.bom_open {
                blocks.extend(Self::bom_window(ctx, &t, v, a, &mut places));
            }
            if a.explode_open {
                blocks.extend(Self::explode_window(ctx, &t, v, a, &mut places));
            }
            if a.in_place.is_some() {
                blocks.extend(Self::in_place_window(ctx, &t, v, a, &mut out, &mut places));
            }
        }
        self.view_blocks.extend(blocks);
        self.places.extend(places);
        fg.out.extend(out);
        self.fg = Some(fg);
        self.asm_tool_window(ctx);
    }

    pub(super) fn window<'a>(
        title: &'a str,
        id: &str,
        t: &super::theme::Theme,
        at: egui::Pos2,
        w: f32,
    ) -> egui::Window<'a> {
        egui::Window::new(title)
            .id(egui::Id::unique(("asm-window", id)))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fade_in(false)
            .default_pos(at)
            .min_width(w)
            .max_width(w)
            .frame(dialog_frame(t))
    }

    fn drive_window(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        feats: &[super::fg::FeatRow],
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let l = a.drive?;
        let link = a.links.get(l)?.clone();
        let mut close = false;
        let r =
            Self::window("Drive", "drive", t, pos2(v.left() + 16.0, v.top() + 56.0), 340.0).show(ctx, |ui| {
                close = dialog_head(
                    ui,
                    t,
                    kind_icon(link.family, &link.kind),
                    "Drive",
                    "asm:drive",
                    places,
                );
                ui.label(
                    egui::RichText::new(format!("{} · {}", a.link_name(l), link.kind.replace('_', " ")))
                        .font(medium(12.0))
                        .color(t.ink),
                );
                ui.label(
                    egui::RichText::new(format!("{}  ·  {}", link.a.0, link.b.0))
                        .font(mono(11.0))
                        .color(t.ink2),
                );
                let joint = a.joint_of(l).cloned();
                if let Some(j) = &joint {
                    ui.label(
                        egui::RichText::new(joint_values(j))
                            .font(sans(11.0))
                            .color(t.ink2),
                    );
                }
                // the joint's dimensions a param drives, each a slider
                let drives: Vec<(String, forge_lang::error::ParamKey, f64, forge_lang::values::Dim)> = feats
                    .iter()
                    .filter(|f| link.owner.is_empty() && f.id == link.id)
                    .flat_map(|f| &f.dims)
                    .filter_map(|d| match &d.from {
                        From::Driving(k) => Some((d.what.clone(), k.clone(), d.value, d.dim)),
                        _ => None,
                    })
                    .collect();
                if drives.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(if link.family == LinkFamily::Joint && link.owner.is_empty() {
                            "No param drives this joint: give it one (turn: a) to drive it here."
                        } else if link.family == LinkFamily::Joint {
                            "Driven from inside its subassembly: open that file to drive it."
                        } else {
                            "Only a joint's values can be driven."
                        })
                        .font(sans(12.0))
                        .color(t.ink2),
                    );
                }
                for (what, key, value, dim) in drives {
                    let angle = dim == forge_lang::values::Dim::Angle;
                    let (lo, hi) = joint
                        .as_ref()
                        .and_then(|j| j.limits.iter().find(|x| x.0 == what))
                        .map(|x| {
                            if angle {
                                (x.1.to_degrees(), x.2.to_degrees())
                            } else {
                                (x.1, x.2)
                            }
                        })
                        .unwrap_or(if angle {
                            (-180.0, 180.0)
                        } else {
                            (
                                value - 100.0f64.max(value.abs()),
                                value + 100.0f64.max(value.abs()),
                            )
                        });
                    let name = format!("{}:{what}", link.id);
                    let mut x = match &a.dragging {
                        Some((n, x)) if *n == name => *x,
                        _ => value,
                    };
                    let unit = if angle { "°" } else { " mm" };
                    let (lo, hi) = (lo.min(value), hi.max(value));
                    // its param and what it drives, the slider across the dialog, its range under it
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("{key} · {what}"))
                            .font(medium(12.0))
                            .color(t.ink),
                    );
                    ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(120.0);
                    ui.visuals_mut().widgets.inactive.bg_fill = t.ink2.gamma_multiply(0.35);
                    let resp = ui.add(
                        egui::Slider::new(&mut x, lo..=hi)
                            .trailing_fill(true)
                            .suffix(unit),
                    );
                    ui.horizontal(|ui| {
                        ui.set_width(ui.spacing().slider_width);
                        ui.label(
                            egui::RichText::new(format!("{}{unit}", short(lo)))
                                .font(sans(10.0))
                                .color(t.ink2),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{}{unit}", short(hi)))
                                    .font(sans(10.0))
                                    .color(t.ink2),
                            );
                        });
                    });
                    places.push((format!("asm:drive:{name}"), resp.rect));
                    if resp.dragged() {
                        a.dragging = Some((name.clone(), x));
                    }
                    let let_go = resp.drag_stopped() || (resp.changed() && !resp.dragged());
                    if let_go {
                        a.dragging = None;
                        if x != value {
                            out.push(FgAct::Apply(Op::SetParam {
                                param: key.clone(),
                                value: Literal { value: x, dim },
                            }));
                        }
                    }
                }
            });
        if close {
            a.drive = None;
        }
        r.map(|r| r.response.rect)
    }

    fn props_window(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let key = a.props.clone()?;
        let inst = a.insts.iter().find(|i| i.path == key).cloned();
        let file = inst
            .as_ref()
            .map(|i| i.file.clone())
            .or_else(|| a.occ.iter().find(|o| o.path == key).map(|o| o.file.clone()))
            .unwrap_or_default();
        let mut close = false;
        let r = Self::window(
            "iProperties",
            "props",
            t,
            pos2(v.left() + 16.0, v.top() + 56.0),
            360.0,
        )
        .show(ctx, |ui| {
            close = dialog_head(ui, t, "part", &format!("{key} iProperties"), "asm:props", places);
            egui::Grid::new("asm-props")
                .num_columns(2)
                .spacing(vec2(12.0, 4.0))
                .show(ui, |ui| {
                    let row = |ui: &mut Ui, k: &str, val: String| {
                        ui.label(egui::RichText::new(k).font(sans(12.0)).color(t.ink2));
                        let r = ui.label(egui::RichText::new(val).font(mono(11.0)).color(t.ink));
                        ui.end_row();
                        r.rect
                    };
                    row(ui, "Occurrence", key.clone());
                    let r = row(ui, "File", file.clone());
                    places.push(("asm:file".into(), r));
                    row(ui, "Part", type_label(&file));
                    row(
                        ui,
                        "Grounded",
                        if inst.as_ref().is_some_and(|i| i.ground) {
                            "yes"
                        } else {
                            "no"
                        }
                        .into(),
                    );
                    let held = a.links.iter().filter(|l| l.a.0 == key || l.b.0 == key).count();
                    row(ui, "Relationships", held.to_string());
                });
            ui.add_space(6.0);
            if a.anchors.as_ref().is_none_or(|x| x.0 != file)
                && Self::fg_button(ui, "Anchors", "asm:anchors".into(), places)
            {
                out.push(FgAct::AsmAnchors(file.clone()));
            }
            if let Some((f, got)) = &a.anchors
                && *f == file
            {
                match got {
                    Ok(list) => {
                        let r = ui.label(
                            egui::RichText::new(format!("{} anchors", list.len()))
                                .font(medium(11.0))
                                .color(t.ink),
                        );
                        places.push(("asm:anchorlist".into(), r.rect));
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .max_height(220.0)
                            .show(ui, |ui| {
                                for x in list.iter().take(400) {
                                    ui.label(egui::RichText::new(x).font(mono(10.0)).color(t.ink2));
                                }
                            });
                    }
                    Err(why) => {
                        ui.label(egui::RichText::new(why).font(sans(11.0)).color(t.error));
                    }
                }
            }
        });
        if close {
            a.props = None;
        }
        r.map(|r| r.response.rect)
    }

    #[allow(clippy::too_many_arguments)]
    fn interference_window(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        issues: &[super::fg::Issue],
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let mut close = false;
        let at = pos2(v.right() - 476.0, v.top() + 56.0);
        let r = Self::window("Analyze Interference", "interference", t, at, 440.0).show(ctx, |ui| {
            close = dialog_head(ui, t, "check", "Analyze Interference", "asm:interference", places);
            ui.label(
                egui::RichText::new("Every pair of parts, and every expect in the file.")
                    .font(sans(12.0))
                    .color(t.ink2),
            );
            ui.add_space(4.0);
            match (a.overlap, a.checking) {
                (_, true) => {
                    ui.label(egui::RichText::new("Analyzing…").font(sans(12.0)).color(t.ink2));
                }
                (Some((n, c)), false) => {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {}",
                                super::fg::plural(n, "interference"),
                                super::fg::plural(c, "intended contact")
                            ))
                            .font(medium(12.0))
                            .color(if n > 0 { t.error } else { t.ink }),
                        );
                    });
                }
                (None, false) => {
                    if Self::fg_button(ui, "Analyze", "asm:check".into(), places) {
                        a.checking = true;
                        out.push(FgAct::AsmCheck);
                    }
                }
            }
            let found: Vec<(usize, &super::fg::Issue)> = issues
                .iter()
                .enumerate()
                .filter(|(_, i)| {
                    matches!(
                        i.asm.as_ref().map(innermost),
                        Some(AsmIssue::Interference { .. } | AsmIssue::ShallowEngagement { .. })
                    )
                })
                .collect();
            if !found.is_empty() {
                egui::ScrollArea::vertical()
                    .id_salt("asm-interf")
                    .auto_shrink([false, true])
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for (k, is) in found {
                            ui.horizontal(|ui| {
                                let r = ui.label(egui::RichText::new(&is.text).font(sans(11.0)).color(WARN));
                                places.push((format!("asm:interf:{k}"), r.rect));
                                if let Some(ai) = &is.asm {
                                    Self::asm_show_on(ui, a, ai, k, out, places);
                                }
                            });
                        }
                    });
            }
            if !a.expects.is_empty() {
                let fails = a.expects.iter().filter(|e| !e.ok).count();
                let head = format!("Expects: {} of {} pass", a.expects.len() - fails, a.expects.len());
                let open = *a.open.entry("expects").or_insert(fails > 0);
                ui.add_space(4.0);
                let r = super::fg::fold_head(ui, t, &head, open, if fails > 0 { t.error } else { t.ink });
                places.push(("asm:expects".into(), r.rect));
                if r.clicked() {
                    a.open.insert("expects", !open);
                }
                if open {
                    egui::ScrollArea::vertical()
                        .id_salt("asm-expects")
                        .auto_shrink([false, true])
                        .max_height(220.0)
                        .show(ui, |ui| {
                            for (k, e) in a.expects.iter().enumerate() {
                                let m = e.measured.map_or("-".to_string(), short);
                                let l = e.limit.map_or("-".to_string(), short);
                                let r = ui.label(
                                    egui::RichText::new(format!(
                                        "{} line {}: {} {m} (limit {l})",
                                        if e.ok { "pass" } else { "FAIL" },
                                        e.line,
                                        e.metric
                                    ))
                                    .font(mono(10.0))
                                    .color(if e.ok {
                                        t.ink2
                                    } else {
                                        t.error
                                    }),
                                );
                                places.push((format!("asm:expect:{k}"), r.rect));
                            }
                        });
                }
            }
        });
        if close {
            a.interference = false;
        }
        r.map(|r| r.response.rect)
    }

    fn bom_window(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let mut close = false;
        let at = pos2(v.center().x - 200.0, v.top() + 56.0);
        let r = Self::window("Bill of Materials", "bom", t, at, 400.0).show(ctx, |ui| {
            close = dialog_head(ui, t, "rule", "Bill of Materials", "asm:bomwin", places);
            let total: usize = a.bom.iter().map(|b| b.1).sum();
            ui.label(
                egui::RichText::new(format!("{} parts, {total} in all", a.bom.len()))
                    .font(sans(12.0))
                    .color(t.ink2),
            );
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(v.height() * 0.6)
                .show(ui, |ui| {
                    egui::Grid::new("asm-bom")
                        .num_columns(3)
                        .striped(true)
                        .spacing(vec2(14.0, 3.0))
                        .show(ui, |ui| {
                            for h in ["Qty", "Part", "File"] {
                                ui.label(egui::RichText::new(h).font(medium(11.0)).color(t.ink2));
                            }
                            ui.end_row();
                            for (k, (file, n)) in a.bom.iter().enumerate() {
                                ui.label(egui::RichText::new(n.to_string()).font(mono(11.0)).color(t.ink));
                                let r = ui.label(
                                    egui::RichText::new(type_label(file))
                                        .font(sans(12.0))
                                        .color(t.ink),
                                );
                                ui.label(
                                    egui::RichText::new(part_word(file))
                                        .font(mono(10.0))
                                        .color(t.ink2),
                                );
                                ui.end_row();
                                places.push((format!("asm:bom:{}", part_word(file)), r.rect));
                                let _ = k;
                            }
                        });
                });
        });
        if close {
            a.bom_open = false;
        }
        r.map(|r| r.response.rect)
    }

    fn explode_window(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let mut close = false;
        let at = pos2(v.center().x - 170.0, v.bottom() - 16.0);
        let r = Self::window("Explode", "explode", t, at, 340.0)
            .pivot(Align2::LEFT_BOTTOM)
            .show(ctx, |ui| {
                close = dialog_head(ui, t, "combine", "Explode", "asm:explodewin", places);
                ui.horizontal(|ui| {
                    let apart = a.explode >= 0.5;
                    if Self::fg_button(
                        ui,
                        if apart { "Collapse" } else { "Explode" },
                        "asm:explodeplay".into(),
                        places,
                    ) {
                        a.explode = if apart { 0.0 } else { 1.0 };
                    }
                    ui.spacing_mut().slider_width = 150.0;
                    ui.visuals_mut().widgets.inactive.bg_fill = t.ink2.gamma_multiply(0.35);
                    let r = ui.add(
                        egui::Slider::new(&mut a.explode, 0.0..=1.0)
                            .show_value(false)
                            .trailing_fill(true),
                    );
                    places.push(("asm:explode".into(), r.rect));
                    let r = ui.toggle_value(&mut a.trails, "Trails");
                    places.push(("asm:trails".into(), r.rect));
                });
            });
        if close {
            a.explode_open = false;
            a.explode = 0.0;
        }
        r.map(|r| r.response.rect)
    }

    /// An assembly issue's Show: highlights what it names.
    pub(super) fn asm_show_button(
        ui: &mut Ui,
        fg_asm: &mut Option<AsmPanel>,
        issue: &AsmIssue,
        k: usize,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        if let Some(a) = fg_asm {
            Self::asm_show_on(ui, a, issue, k, out, places);
        }
    }

    fn asm_show_on(
        ui: &mut Ui,
        a: &mut AsmPanel,
        issue: &AsmIssue,
        k: usize,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let (occ, items) = named(issue);
        if occ.is_empty() && items.is_empty() {
            return;
        }
        let on = a.highlight == occ && a.highlight_items == items;
        if Self::fg_button(
            ui,
            if on { "Hide highlight" } else { "Show" },
            format!("asm:show:{k}"),
            places,
        ) {
            if on {
                a.highlight.clear();
                a.highlight_items.clear();
            } else {
                a.highlight = occ;
                a.highlight_items = items;
            }
            out.push(FgAct::AsmShow);
        }
    }
}

/// An issue inside subassemblies, as it is where it happens.
fn innermost(i: &AsmIssue) -> &AsmIssue {
    match i {
        AsmIssue::Inside { issue, .. } => innermost(issue),
        i => i,
    }
}

/// A joint's values as solved: `turn 30°, free 1`.
pub fn joint_values(j: &JointOut) -> String {
    let mut vals: Vec<String> = j
        .values
        .iter()
        .map(|(w, v, ang)| {
            if *ang {
                format!("{w} {}°", short(v.to_degrees()))
            } else {
                format!("{w} {} mm", short(*v))
            }
        })
        .collect();
    vals.push(format!("free {}", j.free));
    vals.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_type_reads_as_a_person_names_it() {
        assert_eq!(type_label("/x/hw/iso7380_m6x12.fg"), "ISO 7380 M6×12");
        assert_eq!(type_label("hw/cage_nut_m6.fg"), "Cage nut M6");
        assert_eq!(type_label("nylon_spacer_8x14x8.fg"), "Nylon spacer 8×14×8");
        assert_eq!(type_label("pem_s_m6_0_t2.fg"), "PEM S M6 0 T2");
        assert_eq!(type_label("base_pan.fg"), "Base pan");
    }

    #[test]
    fn a_files_group_is_its_folder_seen_from_the_assembly() {
        let d = Path::new("/r/rack/asm");
        assert_eq!(folder_word("/r/rack/hw/a.fg", Some(d)), "hw");
        assert_eq!(folder_word("../fans/standin/b.fg", Some(d)), "fans");
        assert_eq!(folder_word("dummy_d01.fg", Some(d)), "");
        assert_eq!(folder_word("/r/rack/asm/sub/c.fg", Some(d)), "sub");
        assert_eq!(group_label("hw").1, "Hardware & fasteners");
        assert_eq!(group_label("brackets").1, "Brackets");
        assert_eq!(group_label("").1, "Parts");
    }

    #[test]
    fn screws_back_out_towards_their_heads() {
        assert!(
            screw_like("hw/iso7380_m6x12.fg")
                && screw_like("rack_screw_m6x16.fg")
                && screw_like("pem_fh_m6_20.fg")
        );
        assert!(
            !screw_like("hw/iso10511_m6.fg") && !screw_like("iso7089_6.fg") && !screw_like("cage_nut_m6.fg")
        );
    }

    fn occ(p: &str, f: &str) -> OccRow {
        OccRow {
            path: p.into(),
            file: f.into(),
            drawn: true,
        }
    }

    fn inst(p: &str, f: &str, sub: bool, ground: bool) -> InstOut {
        InstOut {
            path: p.into(),
            file: f.into(),
            sub,
            ground,
            suppressed: false,
        }
    }

    fn link(owner: &str, id: &str, kind: &str, a: &str, b: &str) -> LinkOut {
        LinkOut {
            owner: owner.into(),
            id: id.into(),
            family: LinkFamily::Joint,
            kind: kind.into(),
            a: (a.into(), "x".into()),
            b: (b.into(), "y".into()),
            suppressed: false,
        }
    }

    #[test]
    fn the_browser_is_inventors_origin_relationships_then_components() {
        let rows = vec![
            occ("base", "/a/frame/base.fg"),
            occ("bolt[0]", "/a/hw/iso4017_m8x20.fg"),
            occ("bolt[1]", "/a/hw/iso4017_m8x20.fg"),
            occ("kit.post", "/a/frame/post.fg"),
            occ("kit.nut", "/a/hw/iso10511_m8.fg"),
        ];
        let insts = vec![
            inst("base", "../frame/base.fg", false, true),
            inst("bolt[0]", "../hw/iso4017_m8x20.fg", false, false),
            inst("bolt[1]", "../hw/iso4017_m8x20.fg", false, false),
            inst("kit", "kit.fg", true, false),
            inst("kit.post", "../frame/post.fg", false, true),
            inst("kit.nut", "../hw/iso10511_m8.fg", false, false),
        ];
        let links = vec![
            link("", "j1", "revolute", "bolt[0]", "base"),
            link("", "j2", "fastened", "bolt[1]", "base"),
            link("", "j3", "fastened", "kit.post", "base"),
            link("kit", "at_nut", "fastened", "kit.nut", "kit.post"),
        ];
        let mut flags = Flags::default();
        flags.links.insert(("kit".into(), "at_nut".into()), Flag::Error);
        let t = AsmTree::build_with(&rows, Some(Path::new("/a/asm")), &insts, &links, &flags);
        let label = |n: usize| t.nodes[n].label.as_str();
        // one top row, the assembly, and everything under it
        assert_eq!(t.roots.len(), 1);
        assert_eq!(t.nodes[t.roots[0]].kind, NodeKind::Top);
        let top = t.nodes[t.roots[0]].kids.clone();
        let roots: Vec<&str> = top.iter().map(|&n| label(n)).collect();
        assert_eq!(roots, ["Origin", "Relationships", "base", "bolt", "kit"]);
        // the origin's seven, the relationships by kind with the most first
        assert_eq!(t.nodes[top[0]].kids.len(), 7);
        let rel = &t.nodes[top[1]];
        let kinds: Vec<(&str, usize)> = rel
            .kids
            .iter()
            .map(|&k| (label(k), t.nodes[k].kids.len()))
            .collect();
        assert_eq!(kinds, [("Fastened joints", 2), ("Revolute joints", 1)]);
        // a grounded part holds its relationships under it; one statement's loop is a pattern
        let base = &t.nodes[top[2]];
        assert!(base.ground && base.kind == NodeKind::Occ);
        let held: Vec<&str> = base.kids.iter().map(|&k| label(k)).collect();
        assert_eq!(held, ["j1", "j2", "j3"]);
        let bolts = &t.nodes[top[3]];
        assert_eq!(
            (bolts.kind, bolts.kids.len(), bolts.occ.len()),
            (NodeKind::Pattern, 2, 2)
        );
        // the subassembly: its own origin and relationships, what holds it, its parts; its sick
        // relationship flags it
        let kit = &t.nodes[top[4]];
        let kids: Vec<&str> = kit.kids.iter().map(|&k| label(k)).collect();
        assert_eq!(kids, ["Origin", "Relationships", "j3", "post", "nut"]);
        assert_eq!(kit.flag, Some(Flag::Error));
        assert_eq!(t.nodes[t.find("kit.nut").unwrap()].flag, Some(Flag::Error));
        // a relationship stands for its two occurrences
        let j1 = t.find("l:0").unwrap();
        assert_eq!(t.nodes[j1].occ, [1, 0]);
        assert!(t.home.iter().all(|&h| h != usize::MAX));
        // the exploded view's subsystems, subassemblies and fasteners as before
        assert_eq!(t.axis[1], Some(-1.0));
        assert_eq!(t.axis[4], Some(1.0));
        assert_eq!(t.axis[0], None);
        assert_eq!(t.sub[3], t.sub[4]);
        assert!(t.sub[3].is_some() && t.sub[0].is_none());
    }

    #[test]
    fn hiding_a_pattern_and_the_filter() {
        let mut a = AsmPanel::default();
        a.set_occ(
            vec![
                occ("base", "/a/frame/base.fg"),
                occ("bolt[0]", "/a/hw/iso4017_m8x20.fg"),
                occ("bolt[1]", "/a/hw/iso4017_m8x20.fg"),
            ],
            Some(PathBuf::from("/a/asm")),
        );
        a.flip_eye("p:bolt");
        assert_eq!(a.shown_mask(), [true, false, false]);
        let bolts = a.tree.find("p:bolt").unwrap();
        assert_eq!(a.eye(bolts), Eye::Hidden);
        a.set_hidden("bolt[0]", false);
        assert_eq!(a.eye(bolts), Eye::Some);
        a.isolated = Some("base".into());
        assert_eq!(a.shown_mask(), [true, false, false]);
        a.show_all();
        assert_eq!(a.shown_mask(), [true, true, true]);
        // the top open, the rest shut: it, Origin, base, the bolts; the filter opens what holds
        // what it finds
        assert_eq!(a.rows().len(), 4);
        a.filter = "BOLT[1".into();
        let rows: Vec<String> = a.rows().iter().map(|r| a.tree.nodes[r.0].key.clone()).collect();
        assert_eq!(rows, ["a:", "p:bolt", "bolt[1]"]);
        // the top shuts like any row
        a.filter.clear();
        a.expanded.insert(TOP.into());
        assert_eq!(a.rows().len(), 1);
    }

    #[test]
    fn issues_flag_relationships_and_parts_inside_subassemblies() {
        use forge_lang::asm::ItemName;
        let issue = |sev, i| super::super::fg::Issue {
            severity: sev,
            kind: String::new(),
            text: String::new(),
            line: 0,
            col: 0,
            fixes: vec![],
            feature: None,
            asm: Some(i),
        };
        let f = issue_flags(&[
            issue(
                Severity::Error,
                AsmIssue::Inside {
                    path: "kit".into(),
                    issue: Box::new(AsmIssue::Redundant {
                        item: ItemName {
                            id: "at_nut".into(),
                            part: None,
                        },
                        rows: 1,
                        implied_by: vec![],
                    }),
                },
            ),
            issue(
                Severity::Warning,
                AsmIssue::Interference {
                    a: "base".into(),
                    b: "bolt[0]".into(),
                    volume: None,
                    confirmed: true,
                    boolean_failed: None,
                },
            ),
        ]);
        assert_eq!(f.links.get(&("kit".into(), "at_nut".into())), Some(&Flag::Error));
        assert_eq!(f.occ.get("bolt[0]"), Some(&Flag::Warning));
    }

    /// A part occurrence opens to the part's own tree, its Origin then its features as the part's
    /// browser has them; a double click on one opens it in place, the rest of the assembly dimmed,
    /// and OK writes the part's file.
    #[test]
    fn a_part_opens_to_its_features_and_edits_one_in_place() {
        let dir = std::env::temp_dir().join(format!("fgasm-inplace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let part = "shared t = 6\nsketch s(on: xy) {\n  r = rect(center: (0, 0), size: (40, 20))\n}\nplate = extrude(s, t)\n";
        std::fs::write(dir.join("plate.fg"), part).unwrap();
        let rows = vec![occ("a", "plate.fg"), occ("b", "plate.fg")];
        // the file as the document writes it, the solve's resolved name on the occurrences
        let insts = vec![
            inst("a", "./plate.fg", false, true),
            inst("b", "./plate.fg", false, false),
        ];
        let mut p = AsmPanel::default();
        p.set_structure(rows, Some(dir.clone()), insts, vec![], Flags::default());
        assert!(p.tree.find("a>f:plate").is_none());
        p.read_part("a");
        let n = p.tree.find("a").unwrap();
        let kids: Vec<&str> = p.tree.nodes[n]
            .kids
            .iter()
            .map(|&k| p.tree.nodes[k].key.as_str())
            .collect();
        assert_eq!(kids, ["a>o", "a>f:s", "a>f:plate"]);
        assert_eq!(p.tree.nodes[p.tree.find("a>f:plate").unwrap()].detail, "extrude");
        // the other occurrence of the file has it too, read once
        assert!(p.tree.find("b>f:plate").is_some());
        assert!(
            p.edit_in_place("a>f:s").is_err(),
            "a sketch is edited in its own window"
        );
        p.edit_in_place("a>f:plate").unwrap();
        assert!(!p.dimmed(0) && p.dimmed(1));
        let ip = p.in_place.as_mut().unwrap();
        // the extrude's own tool dialog, filled from its statement: its distance as written
        let tool = ip.tool.as_mut().expect("Extrude's form reads the statement back");
        let d = tool
            .params
            .iter_mut()
            .find(|q| q.text == "t")
            .expect("the distance, as written");
        d.text = "8".into();
        assert!(ip.write().unwrap());
        let text = std::fs::read_to_string(dir.join("plate.fg")).unwrap();
        assert!(
            text.contains("param plate.d = 8\nplate = extrude(s, d)"),
            "{text}"
        );
        // a kind no tool writes: its dimensions, each set where it's written
        ip.source = text;
        ip.tool = None;
        let f = ip
            .fields
            .iter_mut()
            .find(|f| f.what == "depth")
            .expect("the extrude's depth");
        assert_eq!(
            f.write,
            super::super::asmpart::Write::Param(forge_lang::error::ParamKey::shared("t"))
        );
        f.text = "9".into();
        assert!(ip.write().unwrap());
        let text = std::fs::read_to_string(dir.join("plate.fg")).unwrap();
        assert!(text.contains("shared t = 9"), "{text}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Every joint and constraint has a glyph: the Relationships eye turns them all off, a kind's
    /// eye its kind, a row's its own; fits and fit rules have none.
    #[test]
    fn relationship_glyphs_turn_off_all_by_kind_or_one() {
        let rows = vec![occ("base", "base.fg"), occ("arm", "arm.fg"), occ("pin", "pin.fg")];
        let insts = vec![
            inst("base", "base.fg", false, true),
            inst("arm", "arm.fg", false, false),
            inst("pin", "pin.fg", false, false),
        ];
        let links = vec![
            link("", "j1", "revolute", "arm", "base"),
            link("", "j2", "revolute", "pin", "arm"),
            link("", "j3", "fastened", "pin", "base"),
        ];
        let mut p = AsmPanel::default();
        p.set_structure(rows, None, insts, links, Flags::default());
        let f = forge_math::Frame::WORLD;
        p.link_at = vec![[Some(f), Some(f)]; 3];
        let shown = |p: &AsmPanel| {
            p.glyph_list()
                .iter()
                .map(|g| p.links[g.0].id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(&p), ["j1", "j2", "j3"]);
        let kind = p.tree.find("r::Joint:revolute").unwrap();
        p.flip_glyph_eye(kind);
        assert_eq!(shown(&p), ["j3"]);
        assert_eq!(p.glyph_eye(kind), Some(Eye::Hidden));
        p.flip_glyph_eye(kind);
        let one = p.tree.find("l:1").unwrap();
        p.flip_glyph_eye(one);
        assert_eq!(shown(&p), ["j1", "j3"]);
        assert_eq!(p.glyph_eye(kind), Some(Eye::Some));
        p.flip_glyph_eye(p.tree.find("r:").unwrap());
        assert!(shown(&p).is_empty());
        // hovering j2 lights the pin and the arm
        assert_eq!(p.tied(1), [1, 2]);
    }
}
