//! The drafting commands' machinery, AutoCAD's way: a command is a state machine that asks with
//! a prompt ("Specify next point or [Undo/Close]:") and is fed what answers it: a point (picked
//! and snapped, or typed `x,y`, `@dx,dy`, `@d<a`, or a distance along the cursor's direction),
//! a number, an option keyword, a line of text, a selection (picked, windowed, crossed, ended
//! with Enter; or what was chosen before the command, noun-verb), or an entity picked. Enter
//! and Space end or repeat, Esc calls off. Each command is one undo step; what it changes is
//! redrawn by itself (`DwgDoc::sync`). While picking, a command draws its rubber band as an
//! overlay (`Ghost`), never into the scene.
//!
//! Object snaps come from the entities near the cursor (the view's grid of segments, so a big
//! drawing costs no more than a small one), ranked as AutoSnap does; ORTHO, POLAR tracking, the
//! grid's SNAP and the last point are applied after.

use super::dwgdoc::DwgDoc;
use crate::dwgview::Geom;
use forge_dwg::draft::commit::snap_items;
use forge_dwg::draft::grip::Grip;
use forge_dwg::draft::snap::{Snap, SnapKind, SnapQuery, snap};
use forge_dwg::draft::{Curve, V2, v2};
use forge_dwg::edit::History;
use forge_dwg::file::Drawing;
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// What a prompt takes.
pub mod want {
    pub const POINT: u8 = 1;
    pub const NUM: u8 = 2;
    pub const TEXT: u8 = 4;
    pub const SEL: u8 = 8;
    pub const PICK: u8 = 16;
    /// Enter means something (a default, the end).
    pub const ENTER: u8 = 32;
    /// TEXT on several lines (MTEXT's editor).
    pub const LINES: u8 = 64;
}

/// A command's question.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Prompt {
    pub text: String,
    /// Option keywords, AutoCAD's way: the capitals are what to type ("eXit": X).
    pub opts: Vec<&'static str>,
    pub dflt: Option<String>,
    pub want: u8,
    /// The point the cursor rubber-bands from (relative input, ORTHO, POLAR, dynamic input).
    pub base: Option<V2>,
}

impl Prompt {
    pub fn new(text: impl Into<String>, want: u8) -> Prompt {
        Prompt {
            text: text.into(),
            want,
            ..Prompt::default()
        }
    }
    pub fn opts(mut self, o: &[&'static str]) -> Prompt {
        self.opts = o.to_vec();
        self
    }
    pub fn dflt(mut self, d: impl Into<String>) -> Prompt {
        self.dflt = Some(d.into());
        self.want |= want::ENTER;
        self
    }
    pub fn base(mut self, b: Option<V2>) -> Prompt {
        self.base = b;
        self
    }
    pub fn wants(&self, w: u8) -> bool {
        self.want & w != 0
    }
    /// As the command line shows it.
    pub fn line(&self) -> String {
        let mut s = self.text.clone();
        if !self.opts.is_empty() {
            s.push_str(&format!(" or [{}]", self.opts.join("/")));
        }
        if let Some(d) = &self.dflt {
            s.push_str(&format!(" <{d}>"));
        }
        s.push(':');
        s
    }
    /// The option a typed word names: its capitals, or the whole word (any case).
    pub fn option(&self, w: &str) -> Option<&'static str> {
        let w = w.trim();
        if w.is_empty() {
            return None;
        }
        self.opts.iter().copied().find(|o| {
            let caps: String = o.chars().filter(|c| c.is_ascii_uppercase()).collect();
            let word: String = o.chars().filter(|c| c.is_alphanumeric()).collect();
            caps.eq_ignore_ascii_case(w) || word.eq_ignore_ascii_case(w)
        })
    }
}

/// What a command is fed.
#[derive(Clone, Debug, PartialEq)]
pub enum Inp {
    Point(V2),
    Num(f64),
    Key(&'static str),
    Text(String),
    Enter,
    Sel(Vec<u32>),
    /// An entity picked, and where.
    Pick(u32, V2),
}

/// What a command says after being fed.
#[derive(Clone, Debug, PartialEq)]
pub enum Flow {
    More,
    Done,
    /// Not taken: why (the prompt is asked again).
    Bad(String),
}

/// A rubber band: drawn over the canvas while picking.
#[derive(Clone, Debug, PartialEq)]
pub enum Ghost {
    Curve(Curve),
    /// Entities as drawn, through a map (MOVE, ROTATE, ...).
    Ents(Vec<u32>, forge_dwg::draft::Affine),
    /// A text at a point (TEXT's, MTEXT's box): its height.
    Text(V2, f64, String),
    /// A dashed line (a mirror line, a reference).
    Dashed(V2, V2),
}

/// The drafting settings: snaps, ortho, polar, grid, and the commands' remembered values.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub osnap: bool,
    /// The running snaps (`SnapKind::bit`s).
    pub osmode: u16,
    pub ortho: bool,
    pub polar: bool,
    /// POLARANG, radians.
    pub polar_inc: f64,
    pub grid: bool,
    pub snap: bool,
    /// SNAPUNIT; 0 picks one from the zoom.
    pub snap_unit: f64,
    pub dyn_input: bool,
    pub fillet_r: f64,
    pub chamfer: (f64, f64),
    /// OFFSETDIST: None is Through.
    pub offset: Option<f64>,
    pub text_h: f64,
    pub hatch: String,
    pub hatch_scale: f64,
    pub hatch_angle: f64,
    pub polygon_sides: usize,
    pub mirrtext: bool,
    pub lengthen: forge_dwg::draft::ops::Lengthen,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            osnap: true,
            osmode: [
                SnapKind::Endpoint,
                SnapKind::Midpoint,
                SnapKind::Center,
                SnapKind::Intersection,
                SnapKind::Extension,
            ]
            .iter()
            .map(|k| k.bit())
            .sum(),
            ortho: false,
            polar: true,
            polar_inc: std::f64::consts::FRAC_PI_2,
            grid: false,
            snap: false,
            snap_unit: 0.0,
            dyn_input: true,
            fillet_r: 0.0,
            chamfer: (0.0, 0.0),
            offset: None,
            text_h: 2.5,
            hatch: "ANSI31".into(),
            hatch_scale: 1.0,
            hatch_angle: 0.0,
            polygon_sides: 4,
            mirrtext: false,
            lengthen: forge_dwg::draft::ops::Lengthen::Delta(0.0),
        }
    }
}

/// What the cursor snapped to, or tracks along.
#[derive(Clone, Debug, PartialEq)]
pub enum Track {
    Snap(Snap),
    /// POLAR or ORTHO: along the ray from the base at this angle.
    Polar {
        base: V2,
        angle: f64,
    },
    /// The grid's SNAP.
    Grid,
}

/// The view as commands query it: the entities near a point, in a box, along a segment, as
/// drawn by the space's scene (less what edits hid) and the edits' own.
pub struct ViewQ {
    pub base: Arc<Geom>,
    pub delta: Arc<Geom>,
    pub hidden: Arc<HashSet<u32>>,
    pub edited: Arc<BTreeSet<u32>>,
    base_shown: Vec<bool>,
    delta_shown: Vec<bool>,
    /// Drawing units a pixel (point) of the view.
    pub px: f64,
}

impl ViewQ {
    fn ok_base(&self, item: u32) -> bool {
        let it = &self.base.scene.items[item as usize];
        self.base_shown.get(it.layer).copied().unwrap_or(true) && !self.hidden.contains(&it.entity)
    }
    fn ok_delta(&self, item: u32) -> bool {
        let it = &self.delta.scene.items[item as usize];
        self.delta_shown.get(it.layer).copied().unwrap_or(true)
    }
    /// The entity nearest a point within `tol`.
    pub fn pick(&self, p: V2, tol: f64) -> Option<u32> {
        let a = self.base.pick_items(p.arr(), tol, &|i| self.ok_base(i));
        let b = self.delta.pick_items(p.arr(), tol, &|i| self.ok_delta(i));
        match (a, b) {
            (Some(a), Some(b)) => Some(if b.1 <= a.1 { b.0 } else { a.0 }),
            (a, b) => a.or(b).map(|x| x.0),
        }
    }
    pub fn near(&self, p: V2, r: f64) -> Vec<u32> {
        let mut out = vec![];
        self.base.near(p.arr(), r, &|i| self.ok_base(i), &mut out);
        self.delta.near(p.arr(), r, &|i| self.ok_delta(i), &mut out);
        out.sort_unstable();
        out.dedup();
        out
    }
    pub fn in_box(&self, a: V2, b: V2) -> Vec<u32> {
        let mut out = vec![];
        self.base.in_box(a.arr(), b.arr(), &|i| self.ok_base(i), &mut out);
        self.delta
            .in_box(a.arr(), b.arr(), &|i| self.ok_delta(i), &mut out);
        out.sort_unstable();
        out.dedup();
        out
    }
    pub fn along(&self, a: V2, b: V2) -> Vec<u32> {
        let mut out = vec![];
        self.base.along(a.arr(), b.arr(), &|i| self.ok_base(i), &mut out);
        self.delta
            .along(a.arr(), b.arr(), &|i| self.ok_delta(i), &mut out);
        out.sort_unstable();
        out.dedup();
        out
    }
    /// AutoCAD's window or crossing selection.
    pub fn select_box(&self, a: V2, b: V2, crossing: bool) -> Vec<u32> {
        let bs = |l: usize| self.base_shown.get(l).copied().unwrap_or(true);
        let ds = |l: usize| self.delta_shown.get(l).copied().unwrap_or(true);
        let mut out = self
            .base
            .select_box_where(a.arr(), b.arr(), crossing, &bs, &|e| !self.hidden.contains(&e));
        out.extend(self.delta.select_box(a.arr(), b.arr(), crossing, &ds));
        out.sort_unstable();
        out.dedup();
        out
    }
    /// The geometry drawing an entity now.
    pub fn geom_of(&self, e: u32) -> &Geom {
        if self.edited.contains(&e) {
            &self.delta
        } else {
            &self.base
        }
    }
    /// An entity's segments as drawn.
    pub fn segments(&self, e: u32, max: usize, out: &mut Vec<([f64; 2], [f64; 2])>) {
        self.geom_of(e).segments(e, max, out);
    }
    /// The space's extents as drawn.
    pub fn extents(&self) -> Option<(V2, V2)> {
        let a = self.base.scene.extents;
        let b = self.delta.scene.extents;
        let (lo, hi) = match (a, b) {
            (Some(a), Some(b)) => (
                [a.0[0].min(b.0[0]), a.0[1].min(b.0[1])],
                [a.1[0].max(b.1[0]), a.1[1].max(b.1[1])],
            ),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return None,
        };
        Some((V2::from(lo), V2::from(hi)))
    }
}

/// What a command works in when fed.
pub struct Cx<'a> {
    pub d: &'a mut Drawing,
    pub hist: &'a mut History,
    pub q: &'a ViewQ,
    pub set: &'a mut Settings,
    /// Lines for the command line.
    pub out: Vec<String>,
    /// The current layer and the space's block record.
    pub layer: u32,
    pub space: u32,
    /// Shift held (TRIM's extend, FILLET's corner).
    pub shift: bool,
    /// What should be chosen when the command ends.
    pub select_after: Option<Vec<u32>>,
    /// The last selection window's corners.
    pub window: Option<(V2, V2)>,
}

impl Cx<'_> {
    pub fn say(&mut self, s: impl Into<String>) {
        self.out.push(s.into());
    }
}

/// What a preview sees.
pub struct Pv<'a> {
    pub d: &'a Drawing,
    pub q: &'a ViewQ,
    pub set: &'a Settings,
}

pub trait Command {
    fn prompt(&self) -> Prompt;
    fn feed(&mut self, cx: &mut Cx, inp: Inp) -> Flow;
    fn preview(&self, _pv: &Pv, _at: V2) -> Vec<Ghost> {
        vec![]
    }
    /// Esc: anything to put back.
    fn cancel(&mut self, _cx: &mut Cx) {}
}

/// Is this one of the drafting commands (a state machine here)?
pub fn is_drafting(name: &str) -> bool {
    super::dwgcmds::make(name).is_some()
}

/// The drafting state of a document.
#[derive(Default)]
pub struct Editor {
    pub hist: History,
    cmd: Option<Box<dyn Command>>,
    pub name: &'static str,
    pub prompt: Option<Prompt>,
    pub set: Settings,
    pub show_osnap: bool,
    /// The cursor as the command gets it (snapped, tracked), and the raw one.
    pub cursor: Option<V2>,
    pub raw: Option<V2>,
    pub track: Option<Track>,
    /// LASTPOINT.
    pub last_point: Option<V2>,
    /// Entities the cursor paused on (extension and parallel track them).
    pub acquired: Vec<u32>,
    /// The selection's grips, and the one held.
    pub grips: Vec<(u32, Grip)>,
    pub grip_hot: Option<usize>,
    /// The command took the selection made before it (noun-verb).
    took_selection: bool,
    /// The last window or crossing a selection made (STRETCH's).
    pub last_window: Option<(V2, V2, bool)>,
    /// What the last selection was (the Previous option).
    pub previous: Vec<u32>,
    /// The view's scale when last hovered (points a unit), for apertures.
    pub px: f64,
    /// Shift as last seen.
    pub shift: bool,
    /// What each step cost: (what, ms), the last hundred.
    pub times: Vec<(String, f64)>,
}

impl Editor {
    pub fn active(&self) -> bool {
        self.cmd.is_some()
    }
    fn time(&mut self, what: impl Into<String>, ms: f64) {
        self.times.push((what.into(), ms));
        if self.times.len() > 100 {
            self.times.remove(0);
        }
    }
}

/// A typed point: `x,y` (`x,y,z`), `@dx,dy`, `@d<a`, `d<a` (degrees), `@` (the base itself).
pub fn parse_point(s: &str, base: Option<V2>) -> Option<V2> {
    let s = s.trim();
    let (rel, rest) = match s.strip_prefix('@') {
        Some(r) => (true, r.trim()),
        None => (false, s),
    };
    let origin = if rel { base.unwrap_or(V2::ZERO) } else { V2::ZERO };
    if rel && rest.is_empty() {
        return Some(origin);
    }
    if let Some((d, a)) = rest.split_once('<') {
        let d: f64 = d.trim().parse().ok()?;
        let a: f64 = a.trim().parse().ok()?;
        return Some(origin + V2::polar(a.to_radians(), d));
    }
    let mut it = rest.split(',');
    let x: f64 = it.next()?.trim().parse().ok()?;
    let y: f64 = it.next()?.trim().parse().ok()?;
    if let Some(z) = it.next() {
        z.trim().parse::<f64>().ok()?;
    }
    if it.next().is_some() {
        return None;
    }
    Some(origin + v2(x, y))
}

/// A number as AutoCAD shows it: four places.
pub fn n4(v: f64) -> String {
    format!("{v:.4}")
}

impl DwgDoc {
    /// The view as commands query it.
    pub fn viewq(&self) -> ViewQ {
        let base_shown = (0..self.geom.scene.layers.len())
            .map(|i| self.layer_shown(i))
            .collect();
        let delta_shown = (0..self.delta.scene.layers.len())
            .map(|i| self.layer_shown_in(&self.delta, i))
            .collect();
        ViewQ {
            base: self.geom.clone(),
            delta: self.delta.clone(),
            hidden: self.hidden.clone(),
            edited: Arc::new(self.edited.clone()),
            base_shown,
            delta_shown,
            px: if self.ed.px > 0.0 {
                1.0 / self.ed.px
            } else {
                self.view.map_or(1.0, |v| 1.0 / v.scale)
            },
        }
    }

    /// The current layer (CLAYER), or layer 0.
    pub fn current_layer(&self) -> u32 {
        self.drawing
            .current("CLAYER")
            .or_else(|| self.drawing.layer("0"))
            .unwrap_or(0)
    }

    /// Is the entity's layer locked (it can be snapped to, not chosen)?
    pub fn locked(&self, h: u32) -> bool {
        let Some(l) = self
            .drawing
            .objects
            .get(&h)
            .and_then(|o| o.ent.as_ref())
            .map(|e| e.layer.handle)
        else {
            return false;
        };
        matches!(self.drawing.objects.get(&l).map(|o| &o.data), Some(forge_dwg::types::Data::Layer(x)) if x.flags & 8 != 0)
    }

    /// Starts a drafting command by its name.
    pub fn start_command(&mut self, name: &'static str) {
        let Some(cmd) = super::dwgcmds::make(name) else {
            return;
        };
        self.ed.cmd = Some(cmd);
        self.ed.name = name;
        self.ed.hist.begin();
        self.ed.took_selection = false;
        self.ed.grip_hot = None;
        self.refresh_prompt();
        // noun-verb: what was chosen before is the command's selection
        if self.ed.prompt.as_ref().is_some_and(|p| p.wants(want::SEL)) {
            let chosen: Vec<u32> = self
                .selected
                .iter()
                .copied()
                .filter(|h| !self.locked(*h))
                .collect();
            if !chosen.is_empty() {
                self.ed.took_selection = true;
                self.say(format!("{} found", chosen.len()));
                self.feed(Inp::Sel(chosen));
            } else {
                self.clear_selection();
            }
        }
    }

    fn refresh_prompt(&mut self) {
        let p = self.ed.cmd.as_ref().map(|c| c.prompt());
        if let Some(p) = &p
            && self.ed.prompt.as_ref() != Some(p)
        {
            self.say(p.line());
        }
        self.ed.prompt = p;
    }

    /// Feeds the running command, then redraws what it changed.
    pub fn feed(&mut self, inp: Inp) {
        let Some(mut cmd) = self.ed.cmd.take() else { return };
        let t0 = crate::viewer::now_ms();
        let q = self.viewq();
        if let Inp::Point(p) = &inp {
            self.ed.last_point = Some(*p);
        }
        let layer = self.current_layer();
        let space = self.space_block();
        let mut cx = Cx {
            d: &mut self.drawing,
            hist: &mut self.ed.hist,
            q: &q,
            set: &mut self.ed.set,
            out: vec![],
            layer,
            space,
            shift: self.ed.shift,
            select_after: None,
            window: self.ed.last_window.map(|w| (w.0, w.1)),
        };
        let flow = cmd.feed(&mut cx, inp);
        let out = std::mem::take(&mut cx.out);
        let after = cx.select_after.take();
        let t1 = crate::viewer::now_ms();
        for l in out {
            self.say(l);
        }
        let touched = self.ed.hist.touched();
        self.sync(&touched);
        let t2 = crate::viewer::now_ms();
        self.ed.time(format!("{} step", self.ed.name), t1 - t0);
        self.ed.time(
            format!("{} redraw ({} entities)", self.ed.name, self.last_sync.1),
            t2 - t1,
        );
        match flow {
            Flow::More => {
                self.ed.cmd = Some(cmd);
                self.ed.prompt = None;
                self.refresh_prompt();
            }
            Flow::Bad(why) => {
                self.say(why);
                self.ed.cmd = Some(cmd);
                if let Some(p) = &self.ed.prompt {
                    let l = p.line();
                    self.say(l);
                }
            }
            Flow::Done => {
                self.ed.prompt = None;
                self.finish(after);
            }
        }
    }

    fn finish(&mut self, after: Option<Vec<u32>>) {
        self.ed.hist.end();
        self.ed.cmd = None;
        self.ed.track = None;
        if let Some(name) = Some(self.ed.name) {
            self.last_command = super::dwgdoc::command_of(name).map(|c| c.name);
        }
        self.clear_selection();
        if let Some(s) = after {
            self.select(&s, true);
        }
        self.refresh_grips();
    }

    /// Esc while a command runs.
    pub fn cancel_command(&mut self) {
        if let Some(mut cmd) = self.ed.cmd.take() {
            let q = self.viewq();
            let layer = self.current_layer();
            let space = self.space_block();
            let mut cx = Cx {
                d: &mut self.drawing,
                hist: &mut self.ed.hist,
                q: &q,
                set: &mut self.ed.set,
                out: vec![],
                layer,
                space,
                shift: false,
                select_after: None,
                window: None,
            };
            cmd.cancel(&mut cx);
            let touched = self.ed.hist.touched();
            self.sync(&touched);
        }
        self.say("*Cancel*");
        self.ed.prompt = None;
        self.finish(None);
    }

    /// A line typed while a command runs.
    pub fn typed_input(&mut self, line: &str) {
        let Some(p) = self.ed.prompt.clone() else { return };
        let raw = line.trim_end_matches(['\n', '\r']);
        let w = raw.trim();
        if !w.is_empty() {
            self.say(format!("{} {w}", p.line()));
        }
        if p.wants(want::TEXT) {
            if w.is_empty() && !p.wants(want::LINES) {
                self.feed(Inp::Enter);
            } else {
                self.feed(Inp::Text(raw.to_string()));
            }
            return;
        }
        if w.is_empty() {
            self.enter();
            return;
        }
        if let Some(k) = p.option(w) {
            self.feed(Inp::Key(k));
            return;
        }
        if p.wants(want::SEL) {
            match w.to_ascii_uppercase().as_str() {
                "ALL" => {
                    let all = self.all_selectable();
                    self.say(format!("{} found", all.len()));
                    self.select(&all, true);
                }
                "P" | "PREVIOUS" => {
                    let prev = self.ed.previous.clone();
                    self.select(&prev, true);
                }
                "L" | "LAST" => {
                    if let Some(h) = self.last_entity() {
                        self.select(&[h], true);
                    }
                }
                _ => self.say("*Invalid selection*  Expects a point or Window/Last/Crossing/ALL/Previous"),
            }
            return;
        }
        let base = p.base.or(self.ed.last_point);
        if p.wants(want::POINT)
            && let Some(pt) = parse_point(w, base)
        {
            self.feed(Inp::Point(pt));
            return;
        }
        if let Ok(x) = w.parse::<f64>() {
            if p.wants(want::NUM) {
                self.feed(Inp::Num(x));
                return;
            }
            // direct distance entry: along the cursor's direction from the base
            if p.wants(want::POINT)
                && let (Some(b), Some(c)) = (p.base, self.ed.cursor)
                && let Some(u) = (c - b).unit()
            {
                self.feed(Inp::Point(b + u * x));
                return;
            }
        }
        self.say(if p.wants(want::POINT) {
            "Point or option keyword required."
        } else if p.wants(want::NUM) {
            "Requires a number or an option keyword."
        } else {
            "Invalid option keyword."
        });
        let l = p.line();
        self.say(l);
    }

    /// Everything that can be chosen: drawn, on a layer shown and unlocked.
    pub fn all_selectable(&self) -> Vec<u32> {
        let q = self.viewq();
        let mut out: Vec<u32> = self
            .geom
            .by_entity
            .keys()
            .copied()
            .filter(|e| !self.hidden.contains(e))
            .chain(self.delta.by_entity.keys().copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        let _ = q;
        out.retain(|h| !self.locked(*h));
        out
    }

    /// The newest entity drawn (the Last option).
    fn last_entity(&self) -> Option<u32> {
        let space = self.space_block();
        forge_dwg::draft::commit::block_entities(&self.drawing, space)
            .into_iter()
            .rev()
            .find(|h| self.geom.by_entity.contains_key(h) || self.edited.contains(h))
    }

    /// Enter (or Space on an empty line) while a command runs.
    pub fn enter(&mut self) {
        let Some(p) = self.ed.prompt.clone() else { return };
        if p.wants(want::SEL) {
            let s: Vec<u32> = self.selected.iter().copied().collect();
            if s.is_empty() && !p.wants(want::ENTER) {
                // nothing chosen: the command ends
                self.cancel_quietly();
                return;
            }
            self.ed.previous = s.clone();
            self.clear_selection();
            self.feed(Inp::Sel(s));
            return;
        }
        self.feed(Inp::Enter);
    }

    fn cancel_quietly(&mut self) {
        self.ed.cmd = None;
        self.ed.prompt = None;
        self.finish(None);
    }

    /// A click on the canvas at a world point: the command's point or pick, a selection's pick,
    /// or (no command) a grip taken.
    pub fn click(&mut self, at: V2, shift: bool) -> bool {
        self.ed.shift = shift;
        let Some(p) = self.ed.prompt.clone() else {
            return self.click_idle(at);
        };
        let q = self.viewq();
        let tol = q.px * 5.0;
        if p.wants(want::SEL) {
            if let Some(h) = q.pick(at, tol).filter(|h| !self.locked(*h)) {
                let on = !(shift && self.selected.contains(&h));
                self.select(&[h], on);
                if on {
                    self.say("1 found");
                }
            }
            return true;
        }
        if p.wants(want::PICK) {
            if let Some(h) = q.pick(at, tol).filter(|h| !self.locked(*h)) {
                self.feed(Inp::Pick(h, at));
            } else if p.wants(want::POINT) {
                let pt = self.ed.cursor.unwrap_or(at);
                self.feed(Inp::Point(pt));
            } else {
                self.say("*Invalid selection*");
            }
            return true;
        }
        if p.wants(want::POINT) {
            let pt = self.ed.cursor.unwrap_or(at);
            self.feed(Inp::Point(pt));
            return true;
        }
        true
    }

    /// A click with no command: a grip taken (the GRIP command), else false (the window
    /// selects as it always has).
    fn click_idle(&mut self, at: V2) -> bool {
        let q = self.viewq();
        let r = q.px * 5.0;
        if let Some(k) = self.ed.grips.iter().position(|(_, g)| g.at.dist(at) <= r) {
            self.ed.grip_hot = Some(k);
            let (_, g) = self.ed.grips[k];
            let held: Vec<(u32, Grip)> = self
                .ed
                .grips
                .iter()
                .filter(|(_, x)| x.at.dist(g.at) <= 1e-9 * (1.0 + g.at.abs_max()))
                .copied()
                .collect();
            let sel: Vec<u32> = self.selected.iter().copied().collect();
            self.ed.cmd = Some(Box::new(super::dwgcmds::GripEdit::new(g.at, held, sel)));
            self.ed.name = "GRIP";
            self.ed.hist.begin();
            self.ed.last_point = Some(g.at);
            self.refresh_prompt();
            return true;
        }
        false
    }

    /// A window or crossing (by the corners' order on screen) while a command runs.
    pub fn window(&mut self, a: V2, b: V2, crossing: bool) {
        let Some(p) = self.ed.prompt.clone() else { return };
        self.ed.last_window = Some((a, b, crossing));
        if p.wants(want::SEL) {
            let q = self.viewq();
            let got: Vec<u32> = q
                .select_box(a, b, crossing)
                .into_iter()
                .filter(|h| !self.locked(*h))
                .collect();
            self.say(format!("{} found", got.len()));
            let add = !self.ed.shift;
            self.select(&got, add);
        }
    }

    /// The cursor moved (world point; the view's points a unit): snaps and tracking.
    pub fn hover(&mut self, raw: V2, scale: f64) {
        let t0 = crate::viewer::now_ms();
        self.ed.raw = Some(raw);
        self.ed.px = scale;
        let p = self.ed.prompt.clone();
        let wants_point = p.as_ref().is_some_and(|p| p.wants(want::POINT));
        self.ed.track = None;
        if !wants_point {
            self.ed.cursor = Some(raw);
            return;
        }
        let base = p.as_ref().and_then(|p| p.base);
        let px = 1.0 / scale.max(1e-300);
        let mut at = raw;
        // object snaps first
        if self.ed.set.osnap {
            let ap = px * 10.0;
            let q = self.viewq();
            let mut cands = q.near(raw, ap);
            for a in &self.ed.acquired {
                if !cands.contains(a) && self.drawing.objects.contains_key(a) {
                    cands.push(*a);
                }
            }
            if !cands.is_empty() {
                let items = snap_items(&self.drawing, &cands);
                let sq = SnapQuery {
                    cursor: raw,
                    aperture: ap,
                    kinds: self.ed.set.osmode,
                    base,
                    acquired: self.ed.acquired.clone(),
                };
                if let Some(s) = snap(&items, &sq).into_iter().next() {
                    at = s.point;
                    if matches!(s.kind, SnapKind::Endpoint | SnapKind::Midpoint) {
                        for h in &s.handles {
                            if !self.ed.acquired.contains(h) {
                                self.ed.acquired.push(*h);
                                if self.ed.acquired.len() > 4 {
                                    self.ed.acquired.remove(0);
                                }
                            }
                        }
                    }
                    self.ed.track = Some(Track::Snap(s));
                }
            }
        }
        if self.ed.track.is_none()
            && let Some(b) = base
        {
            let d = raw - b;
            if d.len() > 0.0 {
                if self.ed.set.ortho {
                    let a = if d.x.abs() >= d.y.abs() {
                        if d.x >= 0.0 { 0.0 } else { std::f64::consts::PI }
                    } else if d.y >= 0.0 {
                        std::f64::consts::FRAC_PI_2
                    } else {
                        -std::f64::consts::FRAC_PI_2
                    };
                    let u = V2::polar(a, 1.0);
                    at = b + u * d.dot(u);
                    self.ed.track = Some(Track::Polar { base: b, angle: a });
                } else if self.ed.set.polar && self.ed.set.polar_inc > 0.0 {
                    let inc = self.ed.set.polar_inc;
                    let a = d.angle();
                    let k = (a / inc).round();
                    let snapped = k * inc;
                    let u = V2::polar(snapped, 1.0);
                    let off = (d - u * d.dot(u)).len();
                    // within a few pixels of the polar ray
                    if off <= px * 8.0 && d.dot(u) > 0.0 {
                        at = b + u * d.dot(u);
                        self.ed.track = Some(Track::Polar {
                            base: b,
                            angle: snapped,
                        });
                    }
                }
            }
        }
        if self.ed.track.is_none() && self.ed.set.snap {
            let s = self.snap_unit();
            at = v2((raw.x / s).round() * s, (raw.y / s).round() * s);
            self.ed.track = Some(Track::Grid);
        }
        self.ed.cursor = Some(at);
        let ms = crate::viewer::now_ms() - t0;
        if ms > 1.0 {
            self.ed.time("snap", ms);
        }
    }

    /// SNAP's step: SNAPUNIT, else the grid's.
    pub fn snap_unit(&self) -> f64 {
        if self.ed.set.snap_unit > 0.0 {
            return self.ed.set.snap_unit;
        }
        grid_step(self.ed.px)
    }

    /// The running command's rubber band at the cursor.
    pub fn ghosts(&self) -> Vec<Ghost> {
        let (Some(cmd), Some(at)) = (&self.ed.cmd, self.ed.cursor) else {
            return vec![];
        };
        let q = self.viewq();
        let pv = Pv {
            d: &self.drawing,
            q: &q,
            set: &self.ed.set,
        };
        cmd.preview(&pv, at)
    }

    /// U: the last command undone.
    pub fn undo(&mut self) {
        if self.ed.active() {
            self.cancel_command();
        }
        if self.ed.hist.undo(&mut self.drawing) {
            let t = self.ed.hist.last_undone();
            self.sync(&t);
            self.say("UNDO  1 command undone");
        } else {
            self.say("Everything has been undone");
        }
        self.refresh_grips();
    }

    pub fn redo(&mut self) {
        if self.ed.active() {
            self.cancel_command();
        }
        if self.ed.hist.redo(&mut self.drawing) {
            let t = self.ed.hist.last_done();
            self.sync(&t);
            self.say("REDO  1 command redone");
        } else {
            self.say("Nothing to redo");
        }
        self.refresh_grips();
    }

    /// The grips of what's chosen (none past a hundred entities, as AutoCAD's GRIPOBJLIMIT).
    pub fn refresh_grips(&mut self) {
        self.ed.grips.clear();
        self.ed.grip_hot = None;
        if self.ed.active() || self.selected.len() > 100 {
            return;
        }
        for &h in &self.selected {
            if let Some(o) = self.drawing.objects.get(&h) {
                for g in forge_dwg::draft::grip::grips(&o.data) {
                    self.ed.grips.push((h, g));
                }
            }
        }
    }
}

/// The grid's step for a zoom (points a unit): a power of ten at least a dozen points apart.
pub fn grid_step(scale: f64) -> f64 {
    let px = 1.0 / scale.max(1e-300);
    let want = px * 12.0;
    let mut s = forge_math::dmath::powf(10.0, want.log10().ceil());
    if s / 2.0 >= want {
        s /= 2.0;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_points() {
        let b = Some(v2(10.0, 20.0));
        assert_eq!(parse_point("3,4", b), Some(v2(3.0, 4.0)));
        assert_eq!(parse_point("@3,4", b), Some(v2(13.0, 24.0)));
        let p = parse_point("@10<90", b).unwrap();
        assert!(p.dist(v2(10.0, 30.0)) < 1e-12);
        assert_eq!(parse_point("@", b), b);
        assert_eq!(parse_point("3,4,5", None), Some(v2(3.0, 4.0)));
        assert_eq!(parse_point("x", None), None);
        assert_eq!(parse_point("3", None), None);
    }

    #[test]
    fn options_by_their_capitals() {
        let p = Prompt::new("Specify next point", want::POINT).opts(&["Undo", "Close", "eXit"]);
        assert_eq!(p.line(), "Specify next point or [Undo/Close/eXit]:");
        assert_eq!(p.option("u"), Some("Undo"));
        assert_eq!(p.option("CLOSE"), Some("Close"));
        assert_eq!(p.option("x"), Some("eXit"));
        assert_eq!(p.option("q"), None);
    }
}
