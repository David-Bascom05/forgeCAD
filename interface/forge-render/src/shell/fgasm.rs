//! An assembly (.fg) in the app: its browser (occurrences grouped by subsystem, then by part type,
//! real subassemblies under their names, an eye on every row), the instance chosen (hide, isolate,
//! its part file and anchors), the joints with a slider for each drive (a SetParam), the mobility,
//! the expects, the BOM, and the overlap check on request. Typed results (redundant, conflicts,
//! interferences, intended contacts) are the panel's issues, each with a Show that highlights what
//! it names.

use super::Shell;
use super::fg::{FgAct, FgPanel, From, disclosure, fold_head, short};
use super::icons;
use super::theme::{medium, mono, sans};
use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};
use forge_lang::asm::{ExpectOut, Issue as AsmIssue, JointOut};
use forge_lang::error::{Literal, Op};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct OccRow {
    pub path: String,
    pub file: String,
    /// It has a solid to draw.
    pub drawn: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A subsystem: the part files' folder (Frame, Hardware & fasteners), or Subassemblies.
    Group,
    /// A real subassembly's instance.
    Sub,
    /// Every occurrence of one part file in its group.
    Type,
    Occ,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// What it's chosen and named by: an occurrence's path, `g:<group>`, `t:<file>`, `s:<instance>`.
    pub key: String,
    pub label: String,
    /// Words at its right: an occurrence alone in its type shows the type.
    pub detail: String,
    pub kind: NodeKind,
    pub depth: usize,
    pub parent: Option<usize>,
    pub kids: Vec<usize>,
    /// The occurrences under it, by index into the panel's.
    pub occ: Vec<usize>,
}

/// The browser's rows, made once per solve: occurrences by subsystem then part type, and real
/// subassemblies again under their names (the same occurrences, as they're built).
#[derive(Clone, Debug, Default)]
pub struct AsmTree {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    by_key: HashMap<String, usize>,
    /// Each occurrence's own row in the grouped tree.
    pub home: Vec<usize>,
    /// Each occurrence's browser group, in the browser's order (each its own when there's one).
    pub group: Vec<usize>,
    /// The real subassembly each is in, if any.
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
const SUBS: &str = "Subassemblies";

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
    let n = file.rsplit('/').next().unwrap_or(file);
    n.trim_end_matches(".fg").to_string()
}

/// A part type as a person reads it: `iso7380_m6x12` is "ISO 7380 M6×12", `cage_nut_m6` is
/// "Cage nut M6".
pub fn type_label(file: &str) -> String {
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

/// Text in the order a person counts: `pin[2]` before `pin[10]`.
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    fn chunks(s: &str) -> Vec<(bool, &str)> {
        let mut out = vec![];
        let mut start = 0;
        let bytes = s.as_bytes();
        for k in 1..=bytes.len() {
            if k == bytes.len() || bytes[k].is_ascii_digit() != bytes[k - 1].is_ascii_digit() {
                out.push((bytes[start].is_ascii_digit(), &s[start..k]));
                start = k;
            }
        }
        out
    }
    let (x, y) = (chunks(a), chunks(b));
    for (p, q) in x.iter().zip(&y) {
        let o = match (p.0, q.0) {
            (true, true) => {
                p.1.parse::<u64>()
                    .unwrap_or(0)
                    .cmp(&q.1.parse::<u64>().unwrap_or(0))
                    .then(p.1.cmp(q.1))
            }
            _ => p.1.cmp(q.1),
        };
        if o.is_ne() {
            return o;
        }
    }
    x.len().cmp(&y.len())
}

impl AsmTree {
    pub fn build(occ: &[OccRow], dir: Option<&Path>) -> AsmTree {
        let mut t = AsmTree {
            home: vec![usize::MAX; occ.len()],
            ..AsmTree::default()
        };
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
        // groups in their order, each its files' occurrences
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
        t.axis = occ
            .iter()
            .zip(&words)
            .map(|(o, w)| {
                HARDWARE
                    .contains(&w.as_str())
                    .then(|| if screw_like(&o.file) { -1.0 } else { 1.0 })
            })
            .collect();
        t.group = vec![0; occ.len()];
        for (g, (_, ks)) in groups.iter().enumerate() {
            for &k in ks {
                t.group[k] = if flat { k } else { g };
            }
        }
        for ((_, label), ks) in &groups {
            let parent = (!flat).then(|| {
                t.push(
                    None,
                    format!("g:{label}"),
                    label.clone(),
                    String::new(),
                    NodeKind::Group,
                )
            });
            t.types(parent, "", occ, ks, true);
        }
        // real subassemblies, by the instance their paths start with
        let mut subs: Vec<(String, Vec<usize>)> = vec![];
        for (k, o) in occ.iter().enumerate() {
            let Some((inst, _)) = o.path.split_once('.') else {
                continue;
            };
            match subs.iter_mut().find(|s| s.0 == inst) {
                Some(s) => s.1.push(k),
                None => subs.push((inst.to_string(), vec![k])),
            }
        }
        t.sub = vec![None; occ.len()];
        for (u, (_, ks)) in subs.iter().enumerate() {
            for &k in ks {
                t.sub[k] = Some(u);
            }
        }
        if !subs.is_empty() {
            subs.sort_by(|a, b| natural(&a.0, &b.0));
            let g = t.push(
                None,
                format!("g:{SUBS}"),
                SUBS.into(),
                String::new(),
                NodeKind::Group,
            );
            for (inst, ks) in &subs {
                let s = t.push(
                    Some(g),
                    format!("s:{inst}"),
                    inst.clone(),
                    String::new(),
                    NodeKind::Sub,
                );
                t.types(Some(s), &format!("s:{inst}/"), occ, ks, false);
            }
        }
        // each node's occurrences, from its leaves up
        for k in (0..t.nodes.len()).rev() {
            let kids = t.nodes[k].kids.clone();
            let mut all: Vec<usize> = t.nodes[k].occ.clone();
            for c in kids {
                all.extend(t.nodes[c].occ.iter().copied());
            }
            t.nodes[k].occ = all;
        }
        t
    }

    fn push(
        &mut self,
        parent: Option<usize>,
        key: String,
        label: String,
        detail: String,
        kind: NodeKind,
    ) -> usize {
        let depth = parent.map_or(0, |p| self.nodes[p].depth + 1);
        let k = self.nodes.len();
        self.by_key.insert(key.clone(), k);
        self.nodes.push(Node {
            key,
            label,
            detail,
            kind,
            depth,
            parent,
            kids: vec![],
            occ: vec![],
        });
        match parent {
            Some(p) => self.nodes[p].kids.push(k),
            None => self.roots.push(k),
        }
        k
    }

    /// Under `parent`: a row per part type (an occurrence alone in its type stands for it), its
    /// occurrences under it.
    fn types(&mut self, parent: Option<usize>, prefix: &str, occ: &[OccRow], ks: &[usize], home: bool) {
        let mut by_file: Vec<(String, Vec<usize>)> = vec![];
        for &k in ks {
            match by_file.iter_mut().find(|f| f.0 == occ[k].file) {
                Some(f) => f.1.push(k),
                None => by_file.push((occ[k].file.clone(), vec![k])),
            }
        }
        let mut by_file: Vec<(String, String, Vec<usize>)> = by_file
            .into_iter()
            .map(|(f, ks)| (type_label(&f), f, ks))
            .collect();
        by_file.sort_by(|a, b| natural(&a.0, &b.0));
        for (label, file, mut ks) in by_file {
            ks.sort_by(|a, b| natural(&occ[*a].path, &occ[*b].path));
            let under = if ks.len() == 1 {
                parent
            } else {
                Some(self.push(
                    parent,
                    format!("{prefix}t:{file}"),
                    label.clone(),
                    String::new(),
                    NodeKind::Type,
                ))
            };
            for k in ks {
                let detail = if under == parent {
                    label.clone()
                } else {
                    String::new()
                };
                let n = self.push(
                    under,
                    format!("{prefix}{}", occ[k].path),
                    occ[k].path.clone(),
                    detail,
                    NodeKind::Occ,
                );
                self.nodes[n].occ = vec![k];
                if home {
                    self.home[k] = n;
                }
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<usize> {
        self.by_key.get(key).copied()
    }
}

#[derive(Default)]
pub struct AsmPanel {
    pub occ: Vec<OccRow>,
    /// The browser's rows, made again when the occurrences change.
    pub tree: AsmTree,
    /// The assembly's folder: where its part files' folders are seen from.
    pub dir: Option<PathBuf>,
    pub joints: Vec<JointOut>,
    pub mobility: usize,
    pub bom: Vec<(String, usize)>,
    pub expects: Vec<ExpectOut>,
    /// The overlap check's counts once it ran: interferences, intended contacts.
    pub overlap: Option<(usize, usize)>,
    /// The overlap check is running.
    pub checking: bool,
    pub timings: String,
    /// The browser row chosen, by its key: an occurrence's path, or a group's (all under it).
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
    /// Which lists are open (joints, expects, BOM), once chosen.
    pub open: std::collections::HashMap<&'static str, bool>,
    /// A drive's slider value while it's moved, before it's let go.
    pub dragging: Option<(String, f64)>,
    /// How far apart the exploded view's asked to be: 0 home, 1 fully apart (the view follows).
    pub explode: f32,
    /// Trails from each exploded part's home.
    pub trails: bool,
    /// The grounded top-level instances: their group stays when exploding.
    pub grounded: Vec<String>,
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

    /// The solve's occurrences: the browser made again if they changed, what was hidden kept.
    pub fn set_occ(&mut self, occ: Vec<OccRow>, dir: Option<PathBuf>) {
        if occ != self.occ || dir != self.dir || self.tree.home.len() != occ.len() {
            self.tree = AsmTree::build(&occ, dir.as_deref());
            self.occ = occ;
            self.dir = dir;
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
        }
        self.sync();
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
                self.expanded.insert(self.tree.nodes[p].key.clone());
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
            let open = !n.kids.is_empty() && (found_under || self.expanded.contains(&n.key));
            out.push((k, open));
            if open {
                stack.extend(n.kids.iter().rev());
            }
        }
        out
    }
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

const TOOLBAR_H: f32 = 34.0;
const INDENT: f32 = 14.0;

impl Shell {
    /// The assembly's browser, in the side panel's place: a filter and Show all above its rows.
    pub(super) fn asm_tree(&mut self, ui: &mut Ui, area: Rect) {
        let Some(mut fg) = self.fg.take() else { return };
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
        let shown_mask = a.isolated.as_ref().map(|_| a.shown_mask());
        scroll.show_rows(&mut child, super::ROW_H, rows.len(), |ui, range| {
            for &(k, open) in &rows[range] {
                let n = &a.tree.nodes[k];
                let (r, resp) = ui.allocate_exact_size(vec2(list.width(), super::ROW_H), Sense::click());
                let on = a.selected.as_deref() == Some(n.key.as_str());
                resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::TreeItem, true, on, &n.label));
                let p = ui.painter();
                if on {
                    p.rect_filled(r, 0.0, t.on);
                    p.rect_filled(Rect::from_min_size(r.min, vec2(2.0, super::ROW_H)), 0.0, t.focus);
                } else if resp.hovered() {
                    p.rect_filled(r, 0.0, t.hover);
                }
                let x = r.left() + 10.0 + n.depth as f32 * INDENT;
                if !n.kids.is_empty() {
                    let arrow = Rect::from_center_size(pos2(x, r.center().y), vec2(16.0, super::ROW_H));
                    let ar = ui.interact(arrow, ui.scope_id().with(("asm-open", &n.key)), Sense::click());
                    disclosure(
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
                let state = a.eye(k);
                let isolated_out = shown_mask.as_ref().is_some_and(|m| n.occ.iter().all(|&o| !m[o]));
                let dim = state == Eye::Hidden || isolated_out;
                let ink = if dim {
                    t.ink2.gamma_multiply(0.55)
                } else if on {
                    t.on_ink
                } else if n.occ.iter().any(|&o| a.flagged(&a.occ[o].path)) {
                    t.error
                } else {
                    t.ink
                };
                let font = match n.kind {
                    NodeKind::Group => medium(13.0),
                    _ if on => medium(13.0),
                    _ => sans(13.0),
                };
                let g = ui.fonts_mut(|f| f.layout_no_wrap(n.label.clone(), font, ink));
                let lx = x + 10.0;
                let gw = g.size().x;
                ui.painter()
                    .galley(pos2(lx, r.center().y - g.size().y * 0.5), g, ink);
                // a count after a group's name; a lone occurrence's type at the right
                let count = match n.kind {
                    NodeKind::Type => format!("×{}", n.occ.len()),
                    NodeKind::Group | NodeKind::Sub => n.occ.len().to_string(),
                    NodeKind::Occ => String::new(),
                };
                let eye_r = Rect::from_center_size(pos2(r.right() - 16.0, r.center().y), vec2(20.0, 20.0));
                if !count.is_empty() {
                    ui.painter().text(
                        pos2(lx + gw + 6.0, r.center().y),
                        Align2::LEFT_CENTER,
                        count,
                        sans(11.0),
                        t.ink2,
                    );
                }
                if !n.detail.is_empty() {
                    ui.painter().text(
                        pos2(eye_r.left() - 6.0, r.center().y),
                        Align2::RIGHT_CENTER,
                        &n.detail,
                        sans(10.0),
                        t.ink2,
                    );
                }
                let er = ui.interact(eye_r, ui.scope_id().with(("asm-eye", &n.key)), Sense::click());
                let tip = if state == Eye::Hidden { "Show" } else { "Hide" };
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
                if er.clicked() {
                    eye = Some(n.key.clone());
                } else if resp.double_clicked() && !n.kids.is_empty() {
                    let open = a.expanded.contains(&n.key);
                    toggle = Some((n.key.clone(), open));
                } else if resp.clicked() {
                    chosen = Some(n.key.clone());
                }
                places.push((format!("asm:row:{}", n.key), r));
            }
        });
        if let Some((k, open)) = toggle {
            if open {
                a.expanded.remove(&k);
            } else {
                a.expanded.insert(k);
            }
        }
        if let Some(k) = eye {
            a.flip_eye(&k);
            changed = true;
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
        self.places.extend(places);
        self.fg = Some(fg);
    }

    /// The assembly's summary and its checks (run on request): first, as what matters most.
    pub(super) fn asm_checks(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let Some(a) = &mut fg.asm else { return };
        let summary = format!(
            "{} occurrences · {} parts · DOF {} · {}",
            a.occ.len(),
            a.bom.len(),
            a.mobility,
            a.timings
        );
        ui.label(egui::RichText::new(summary).font(sans(11.0)).color(t.ink2));
        // the overlap check and the expects, on request (slow on a big assembly)
        Self::fg_heading(ui, t, "Interference and expects");
        match (a.overlap, a.checking) {
            (_, true) => {
                ui.label(egui::RichText::new("checking…").font(sans(11.0)).color(t.ink2));
            }
            (Some((n, c)), false) => {
                ui.label(
                    egui::RichText::new(format!("{n} interferences, {c} intended contacts"))
                        .font(sans(11.0))
                        .color(if n > 0 { t.error } else { t.ink }),
                );
            }
            (None, false) => {
                if Self::fg_button(ui, "Run checks", "asm:check".into(), places) {
                    a.checking = true;
                    out.push(FgAct::AsmCheck);
                }
            }
        }
        if !a.expects.is_empty() {
            let fails = a.expects.iter().filter(|e| !e.ok).count();
            let r = ui.label(
                egui::RichText::new(format!(
                    "expects: {} of {} pass",
                    a.expects.len() - fails,
                    a.expects.len()
                ))
                .font(sans(11.0))
                .color(if fails > 0 { t.error } else { t.ink }),
            );
            places.push(("asm:expects".into(), r.rect));
        }
    }

    /// The assembly's sections of the panel: the instance chosen, joints and drives, DOF, expects,
    /// the BOM, the overlap check.
    pub(super) fn asm_panel(
        ui: &mut Ui,
        t: &super::theme::Theme,
        fg: &mut FgPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) {
        let feats = fg.feats.clone();
        let Some(a) = &mut fg.asm else { return };
        // the instance chosen
        if let Some(sel) = a.selected.clone() {
            let label = a
                .tree
                .find(&sel)
                .map_or(sel.clone(), |n| a.tree.nodes[n].label.clone());
            Self::fg_heading(ui, t, &label);
            let files: Vec<usize> = a.occ_of(&sel);
            let one = files.len() == 1;
            if one {
                let r = ui.label(
                    egui::RichText::new(&a.occ[files[0]].file)
                        .font(mono(11.0))
                        .color(t.ink),
                );
                places.push(("asm:file".into(), r.rect));
            } else {
                ui.label(
                    egui::RichText::new(format!("{} occurrences", files.len()))
                        .font(sans(11.0))
                        .color(t.ink2),
                );
            }
            ui.horizontal_wrapped(|ui| {
                let hidden = !files.is_empty() && files.iter().all(|&k| a.hidden.contains(&a.occ[k].path));
                if Self::fg_button(
                    ui,
                    if hidden { "Show" } else { "Hide" },
                    "asm:hide".into(),
                    places,
                ) {
                    a.set_hidden(&sel, !hidden);
                    out.push(FgAct::AsmShow);
                }
                let iso = a.isolated.as_deref() == Some(sel.as_str());
                if Self::fg_button(
                    ui,
                    if iso { "Show all" } else { "Isolate" },
                    "asm:isolate".into(),
                    places,
                ) {
                    a.isolated = if iso { None } else { Some(sel.clone()) };
                    out.push(FgAct::AsmShow);
                }
                if one && Self::fg_button(ui, "Anchors", "asm:anchors".into(), places) {
                    out.push(FgAct::AsmAnchors(a.occ[files[0]].file.clone()));
                }
            });
            if let Some((file, got)) = &a.anchors {
                match got {
                    Ok(list) => {
                        let r = ui.label(
                            egui::RichText::new(format!("{} anchors of {}", list.len(), part_word(file)))
                                .font(medium(11.0))
                                .color(t.ink),
                        );
                        places.push(("asm:anchorlist".into(), r.rect));
                        for x in list.iter().take(200) {
                            ui.label(egui::RichText::new(x).font(mono(10.0)).color(t.ink2));
                        }
                    }
                    Err(why) => {
                        ui.label(egui::RichText::new(why).font(sans(11.0)).color(t.error));
                    }
                }
            }
        } else if (a.isolated.is_some() || !a.hidden.is_empty())
            && Self::fg_button(ui, "Show all", "asm:showall".into(), places)
        {
            a.show_all();
            out.push(FgAct::AsmShow);
        }
        // the exploded view
        Self::fg_heading(ui, t, "Explode");
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
            let r = ui.add(egui::Slider::new(&mut a.explode, 0.0..=1.0).show_value(false));
            places.push(("asm:explode".into(), r.rect));
            let r = ui.toggle_value(&mut a.trails, "Trails");
            places.push(("asm:trails".into(), r.rect));
        });
        // joints, each drive a slider
        let open = Self::asm_fold(
            ui,
            t,
            a,
            "joints",
            &format!("Joints ({}), mobility {}", a.joints.len(), a.mobility),
            a.joints.len(),
            places,
        );
        for j in a.joints.iter().filter(|_| open) {
            let flagged = a.highlight_items.contains(&j.id);
            let vals: Vec<String> = j
                .values
                .iter()
                .map(|(w, v, ang)| {
                    if *ang {
                        format!("{w} {}°", short(v.to_degrees()))
                    } else {
                        format!("{w} {}", short(*v))
                    }
                })
                .collect();
            let r = ui.label(
                egui::RichText::new(format!(
                    "{} {} · {} · free {}",
                    j.id,
                    j.kind,
                    vals.join(", "),
                    j.free
                ))
                .font(mono(11.0))
                .color(if flagged { t.error } else { t.ink }),
            );
            places.push((format!("asm:joint:{}", j.id), r.rect));
            // a drive: the joint's dimension a param drives
            let drives: Vec<(String, forge_lang::error::ParamKey, f64, forge_lang::values::Dim)> = feats
                .iter()
                .filter(|f| f.id == j.id)
                .flat_map(|f| &f.dims)
                .filter_map(|d| match &d.from {
                    From::Driving(k) => Some((d.what.clone(), k.clone(), d.value, d.dim)),
                    _ => None,
                })
                .collect();
            for (what, key, value, dim) in drives {
                let angle = dim == forge_lang::values::Dim::Angle;
                let (lo, hi) = j
                    .limits
                    .iter()
                    .find(|l| l.0 == what)
                    .map(|l| {
                        if angle {
                            (l.1.to_degrees(), l.2.to_degrees())
                        } else {
                            (l.1, l.2)
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
                let name = format!("{}:{what}", j.id);
                let mut v = match &a.dragging {
                    Some((n, x)) if *n == name => *x,
                    _ => value,
                };
                let resp = ui.add(
                    egui::Slider::new(&mut v, lo.min(value)..=hi.max(value))
                        .text(format!("{key}"))
                        .suffix(if angle { "°" } else { " mm" }),
                );
                places.push((format!("asm:drive:{name}"), resp.rect));
                if resp.dragged() {
                    a.dragging = Some((name.clone(), v));
                }
                let let_go = resp.drag_stopped() || (resp.changed() && !resp.dragged());
                if let_go {
                    a.dragging = None;
                    if v != value {
                        out.push(FgAct::Apply(Op::SetParam {
                            param: key.clone(),
                            value: Literal { value: v, dim },
                        }));
                    }
                }
            }
        }
        // expects
        if !a.expects.is_empty() {
            let fails = a.expects.iter().filter(|e| !e.ok).count();
            let head = format!("Expects: {} of {} pass", a.expects.len() - fails, a.expects.len());
            let open = Self::asm_fold(ui, t, a, "expects", &head, a.expects.len(), places);
            for (k, e) in a.expects.iter().enumerate().filter(|_| open) {
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
                    .color(if e.ok { t.ink2 } else { t.error }),
                );
                places.push((format!("asm:expect:{k}"), r.rect));
            }
        }
        // the BOM
        let head = format!("BOM ({} parts)", a.bom.iter().map(|b| b.1).sum::<usize>());
        let open = Self::asm_fold(ui, t, a, "bom", &head, a.bom.len(), places);
        for (file, n) in a.bom.iter().filter(|_| open) {
            let r = ui.label(
                egui::RichText::new(format!("{n:>4} × {}", part_word(file)))
                    .font(mono(11.0))
                    .color(t.ink),
            );
            places.push((format!("asm:bom:{}", part_word(file)), r.rect));
        }
    }

    /// A list's heading that opens and shuts it; a long one starts shut.
    fn asm_fold(
        ui: &mut Ui,
        t: &super::theme::Theme,
        a: &mut AsmPanel,
        key: &'static str,
        head: &str,
        len: usize,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let open = *a.open.entry(key).or_insert(len <= 20);
        ui.add_space(6.0);
        let r = fold_head(ui, t, head, open, t.ink);
        places.push((format!("asm:open:{key}"), r.rect));
        if r.clicked() {
            a.open.insert(key, !open);
        }
        open
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
        let Some(a) = fg_asm else { return };
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
    fn counted_names_sort_by_their_numbers() {
        let mut v = vec!["pin[10]", "pin[2]", "pin[1]", "nut"];
        v.sort_by(|a, b| natural(a, b));
        assert_eq!(v, ["nut", "pin[1]", "pin[2]", "pin[10]"]);
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

    #[test]
    fn the_tree_groups_types_and_subassemblies() {
        let occ = |p: &str, f: &str| OccRow {
            path: p.into(),
            file: f.into(),
            drawn: true,
        };
        let rows = vec![
            occ("base", "/a/frame/base.fg"),
            occ("bolt[1]", "/a/hw/iso4017_m8x20.fg"),
            occ("bolt[0]", "/a/hw/iso4017_m8x20.fg"),
            occ("kit.post", "/a/frame/post.fg"),
            occ("kit.nut", "/a/hw/iso10511_m8.fg"),
            occ("d01", "/a/asm/dummy_d01.fg"),
            occ("d02", "/a/asm/dummy_d02.fg"),
        ];
        let t = AsmTree::build(&rows, Some(Path::new("/a/asm")));
        let roots: Vec<(&str, usize)> = t
            .roots
            .iter()
            .map(|&n| (t.nodes[n].label.as_str(), t.nodes[n].occ.len()))
            .collect();
        assert_eq!(
            roots,
            [
                ("Frame", 2),
                ("Hardware & fasteners", 3),
                ("Dummies", 2),
                ("Subassemblies", 2)
            ]
        );
        let hw = &t.nodes[t.roots[1]];
        let bolts = &t.nodes[hw.kids[0]];
        assert_eq!(
            (bolts.label.as_str(), bolts.kind, bolts.occ.len()),
            ("ISO 4017 M8×20", NodeKind::Type, 2)
        );
        // counted occurrences in order; a lone one stands for its type
        let kids: Vec<&str> = bolts.kids.iter().map(|&k| t.nodes[k].key.as_str()).collect();
        assert_eq!(kids, ["bolt[0]", "bolt[1]"]);
        let nut = &t.nodes[hw.kids[1]];
        assert_eq!(
            (nut.key.as_str(), nut.detail.as_str()),
            ("kit.nut", "ISO 10511 M8")
        );
        // every occurrence's own row, and the fasteners' ways out
        assert!(t.home.iter().all(|&h| h != usize::MAX));
        assert_eq!(t.axis[1], Some(-1.0));
        assert_eq!(t.axis[4], Some(1.0));
        assert_eq!(t.axis[0], None);
        assert_eq!(t.sub[3], t.sub[4]);
        assert!(t.sub[3].is_some() && t.sub[0].is_none());
    }

    #[test]
    fn hiding_a_group_and_the_filter() {
        let occ = |p: &str, f: &str| OccRow {
            path: p.into(),
            file: f.into(),
            drawn: true,
        };
        let mut a = AsmPanel::default();
        a.set_occ(
            vec![
                occ("base", "/a/frame/base.fg"),
                occ("bolt[0]", "/a/hw/iso4017_m8x20.fg"),
                occ("bolt[1]", "/a/hw/iso4017_m8x20.fg"),
            ],
            Some(PathBuf::from("/a/asm")),
        );
        a.flip_eye("g:Hardware & fasteners");
        assert_eq!(a.shown_mask(), [true, false, false]);
        assert_eq!(a.eye(a.tree.roots[1]), Eye::Hidden);
        a.set_hidden("bolt[0]", false);
        assert_eq!(a.eye(a.tree.roots[1]), Eye::Some);
        a.isolated = Some("base".into());
        assert_eq!(a.shown_mask(), [true, false, false]);
        a.show_all();
        assert_eq!(a.shown_mask(), [true, true, true]);
        // shut by default; the filter opens what holds what it finds
        assert_eq!(a.rows().len(), 2);
        a.filter = "BOLT[1".into();
        let rows: Vec<String> = a.rows().iter().map(|r| a.tree.nodes[r.0].key.clone()).collect();
        assert_eq!(
            rows,
            ["g:Hardware & fasteners", "t:/a/hw/iso4017_m8x20.fg", "bolt[1]"]
        );
    }
}
