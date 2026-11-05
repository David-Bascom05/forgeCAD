//! The app tools' host side (`agent/apptools.rs`): each call done on the frame's thread on the
//! active document, saying what changed. A call that changes the view is one view step,
//! `undo_view` taking it back; none edits a file but `keep_position`, asked for by name.

use super::Gui;
use crate::agent::apptools::{AppCall, Reply};
use crate::render::VisualStyle;
use crate::shell::fg::{FgAct, From};
use crate::shell::thermo_ui::Analysis;
use forge_agent::json::Json;
use forge_lang::error::{Literal, Op};
use std::collections::{HashMap, HashSet};

/// The view as the app tools change it, kept before each change.
#[derive(Clone)]
pub struct ViewSnap {
    asm: Option<(HashSet<String>, Option<String>, Option<String>)>,
    eyes: HashMap<String, bool>,
    body_hidden: bool,
    selected: Option<String>,
    overlays: crate::shell::Overlays,
    style: VisualStyle,
    section: bool,
    analysis: Analysis,
    camera: Option<crate::camera::Camera>,
    driven: Option<Vec<Op>>,
}

#[derive(Default)]
pub struct AppHost {
    pub undo: Vec<ViewSnap>,
    /// The joints driven for a look, as ops the file doesn't hold.
    pub driven: Option<Vec<Op>>,
    /// Pictures asked for: taken once the next frame's drawn.
    shots: Vec<std::sync::mpsc::Sender<Reply>>,
    /// The agent turned the view itself this turn: a driven joint leaves the camera alone.
    pub view_set: bool,
}

fn s<'a>(j: &'a Json, k: &str) -> &'a str {
    j.get(k).and_then(Json::str).unwrap_or("")
}

fn b(j: &Json, k: &str) -> Option<bool> {
    match j.get(k) {
        Some(Json::Bool(v)) => Some(*v),
        _ => None,
    }
}

impl Gui {
    fn view_snap(&self) -> ViewSnap {
        let p = self.shell.fg.as_ref();
        ViewSnap {
            asm: p
                .and_then(|p| p.asm.as_ref())
                .map(|a| (a.hidden.clone(), a.isolated.clone(), a.selected.clone())),
            eyes: p.map(|p| p.eyes.clone()).unwrap_or_default(),
            body_hidden: p.is_some_and(|p| p.body_hidden),
            selected: p.and_then(|p| p.selected.clone()),
            overlays: self.shell.overlays(),
            style: self.shell.view.style,
            section: self.shell.view.section,
            analysis: p.map_or(Analysis::None, |p| p.thermo.analysis),
            camera: self.view.as_ref().map(|v| v.nav.camera),
            driven: self.apps.driven.clone(),
        }
    }

    fn view_restore(&mut self, v: ViewSnap) {
        if let Some(p) = &mut self.shell.fg {
            if let (Some(a), Some((hidden, isolated, selected))) = (&mut p.asm, v.asm) {
                a.show_all();
                for h in &hidden {
                    a.set_hidden(h, true);
                }
                a.isolated = isolated;
                a.selected = selected;
                p.out.push(FgAct::AsmShow);
            }
            p.eyes = v.eyes;
            p.body_hidden = v.body_hidden;
            p.selected = v.selected;
            p.thermo.analysis = v.analysis;
        }
        *self.shell.overlays_mut() = v.overlays;
        if self.shell.view.style != v.style {
            self.shell.out.push(crate::shell::Action::VisualStyle(v.style));
        }
        if self.shell.view.section != v.section {
            self.shell.out.push(crate::shell::Action::Section);
        }
        if let (Some(c), Some(view)) = (v.camera, &mut self.view) {
            view.nav.camera = c;
            view.mark_dirty();
        }
        if self.apps.driven != v.driven {
            self.apps.driven = v.driven.clone();
            if let Some(p) = &mut self.shell.fg {
                p.out.push(FgAct::AsmPreview(v.driven));
            }
        }
    }

    /// The app tools asked for since the last frame, done; pictures asked last frame, taken.
    pub(crate) fn app_calls(&mut self) {
        for r in std::mem::take(&mut self.apps.shots) {
            let _ = r.send(self.screenshot());
        }
        let calls: Vec<AppCall> = self
            .agent
            .app_rx
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        for c in calls {
            if c.name == "screenshot" {
                if let Some(v) = &mut self.view {
                    v.mark_dirty();
                }
                self.apps.shots.push(c.reply);
                self.ctx.request_repaint();
                continue;
            }
            let before = self.view_snap();
            let got = self.app_tool(&c.name, &c.args);
            let steps = !matches!(
                c.name.as_str(),
                "tree" | "get_selection" | "thermo_report" | "undo_view" | "open_document" | "keep_position"
            );
            if steps && !got.error {
                self.apps.undo.push(before);
            }
            if let Some(v) = &mut self.view {
                v.mark_dirty();
            }
            let _ = c.reply.send(got);
        }
    }

    /// What the person sees in the view, as a PNG.
    fn screenshot(&mut self) -> Reply {
        let Some(v) = &self.view else {
            return Reply::err("nothing is in view");
        };
        let (w, h) = v.size;
        let rgb = v.target.read_rgb(&v.renderer);
        match crate::agent::apptools::png(w, h, &rgb) {
            Ok(png) => Reply {
                text: format!("The view, {w}×{h} px"),
                png: Some(png),
                error: false,
            },
            Err(e) => Reply::err(format!("couldn't make the picture: {e}")),
        }
    }

    pub(crate) fn app_tool(&mut self, name: &str, a: &Json) -> Reply {
        let is_asm = self.shell.is_asm();
        match name {
            "tree" => self.app_tree(s(a, "filter"), b(a, "relationships").unwrap_or(false)),
            "set_visibility" => {
                let ids: Vec<String> = match a.get("ids") {
                    Some(Json::Arr(v)) => v.iter().filter_map(|x| x.str().map(str::to_string)).collect(),
                    _ => return Reply::err("set_visibility takes `ids`, a list"),
                };
                let Some(on) = b(a, "visible") else {
                    return Reply::err("set_visibility takes `visible`");
                };
                let Some(p) = &mut self.shell.fg else {
                    return Reply::err("no Forge document is open");
                };
                let mut done = Vec::new();
                let mut unknown = Vec::new();
                for id in &ids {
                    if let Some(asm) = &mut p.asm {
                        if asm.occ_of(id).is_empty() {
                            unknown.push(id.clone());
                        } else {
                            asm.set_hidden(id, !on);
                            done.push(id.clone());
                        }
                    } else if id == "solid" || id == "body" {
                        p.body_hidden = !on;
                        done.push(id.clone());
                    } else if p.feats.iter().any(|f| f.id == *id) {
                        p.eyes.insert(id.clone(), on);
                        done.push(id.clone());
                    } else {
                        unknown.push(id.clone());
                    }
                }
                if is_asm {
                    p.out.push(FgAct::AsmShow);
                }
                let word = if on { "Shown" } else { "Hidden" };
                let mut t = format!("{word}: {}", done.join(", "));
                if !unknown.is_empty() {
                    t.push_str(&format!(". No such row: {} (see `tree`)", unknown.join(", ")));
                }
                Reply {
                    text: t,
                    png: None,
                    error: done.is_empty(),
                }
            }
            "isolate" | "show_all" => {
                let id = s(a, "id").to_string();
                let Some(asm) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) else {
                    if name == "show_all"
                        && let Some(p) = &mut self.shell.fg
                    {
                        p.body_hidden = false;
                        p.eyes.retain(|_, v| *v);
                        return Reply::ok("Shown: everything");
                    }
                    return Reply::err("isolate works on an assembly's occurrences");
                };
                if name == "show_all" {
                    asm.show_all();
                } else {
                    if asm.occ_of(&id).is_empty() {
                        return Reply::err(format!("no occurrence `{id}` (see `tree`)"));
                    }
                    asm.isolated = Some(id.clone());
                }
                self.shell.fg.as_mut().expect("open").out.push(FgAct::AsmShow);
                Reply::ok(if name == "show_all" {
                    "Shown: everything".to_string()
                } else {
                    format!("Only {id} shown")
                })
            }
            "set_overlay" => {
                let n = s(a, "name");
                let Some(on) = b(a, "on") else {
                    return Reply::err("set_overlay takes `on`");
                };
                if !crate::shell::OVERLAY_SWITCHES.iter().any(|x| x.0 == n) {
                    return Reply::err(format!("no overlay `{n}`"));
                }
                let o = self.shell.overlays_mut();
                if o.get(n) != on {
                    o.flip(n);
                }
                Reply::ok(format!("{n} {}", if on { "on" } else { "off" }))
            }
            "select" | "clear_selection" => {
                let id = (name == "select").then(|| s(a, "id").to_string());
                if let Some(d) = &mut self.shell.drawing {
                    d.selected = id.clone();
                } else if let Some(p) = &mut self.shell.fg {
                    if let Some(asm) = &mut p.asm {
                        if let Some(i) = &id
                            && asm.occ_of(i).is_empty()
                            && asm.tree.find(i).is_none()
                        {
                            return Reply::err(format!("no row `{i}` (see `tree`)"));
                        }
                        asm.selected = id.clone();
                        p.out.push(FgAct::AsmShow);
                    } else {
                        if let Some(i) = &id
                            && !p.feats.iter().any(|f| f.id == *i)
                        {
                            return Reply::err(format!("no feature `{i}` (see `tree`)"));
                        }
                        p.selected = id.clone();
                    }
                } else {
                    return Reply::err("no document is open");
                }
                Reply::ok(match id {
                    Some(i) => format!("Selected {i}"),
                    None => "Nothing selected".into(),
                })
            }
            "get_selection" => Reply::ok(match self.shell.agent_selection() {
                Some(s) => format!("Selected: {s}"),
                None => "Nothing is selected".into(),
            }),
            "set_view" => {
                let Some(v) = &mut self.view else {
                    return Reply::err("nothing is in view");
                };
                let region: Option<[i8; 3]> = match s(a, "view") {
                    "front" => Some([0, -1, 0]),
                    "back" => Some([0, 1, 0]),
                    "right" => Some([1, 0, 0]),
                    "left" => Some([-1, 0, 0]),
                    "top" => Some([0, 0, 1]),
                    "bottom" => Some([0, 0, -1]),
                    "iso" => Some([1, -1, 1]),
                    "home" => {
                        v.nav.home(&v.scene);
                        None
                    }
                    "fit" => None,
                    w => return Reply::err(format!("no view `{w}`")),
                };
                if let Some(r) = region {
                    v.nav.camera = crate::nav::region_view_as(v.nav.camera, r, &v.nav.orient);
                }
                v.nav.fit_all(&v.scene);
                v.mark_dirty();
                self.apps.view_set = true;
                Reply::ok(format!("View: {}", s(a, "view")))
            }
            "visual_style" => {
                let w = s(a, "style");
                let Some(m) = VisualStyle::ALL.iter().copied().find(|v| v.key() == w) else {
                    return Reply::err(format!("no visual style `{w}`"));
                };
                self.shell.out.push(crate::shell::Action::VisualStyle(m));
                Reply::ok(format!("Visual style: {}", m.name()))
            }
            "section" => {
                let Some(on) = b(a, "on") else {
                    return Reply::err("section takes `on`");
                };
                if self.shell.view.section != on {
                    self.shell.out.push(crate::shell::Action::Section);
                }
                Reply::ok(format!("Section {}", if on { "on" } else { "off" }))
            }
            "thermo_view" => {
                let Some(p) = &mut self.shell.fg else {
                    return Reply::err("no Forge part is open");
                };
                if p.thermos.is_empty() {
                    return Reply::err("the part has no thermoform");
                }
                p.thermo.analysis = match s(a, "analysis") {
                    "none" => Analysis::None,
                    "draft" => Analysis::Draft,
                    "thickness" => Analysis::Thickness,
                    w => return Reply::err(format!("no analysis `{w}`")),
                };
                Reply::ok(format!("Thermoform analysis: {}", s(a, "analysis")))
            }
            "thermo_report" => {
                let Some(r) = self.shell.fg.as_ref().and_then(|p| p.thermo_row()) else {
                    return Reply::err("the part has no thermoform (or it hasn't been made yet)");
                };
                let mut t = format!(
                    "{}: wall {:.2} to {:.2} mm (limit {:.2} mm), {} {}",
                    r.id, r.min.0, r.max.0, r.limit, r.material, r.model
                );
                for (k, v) in &r.facts {
                    t.push_str(&format!("\n{k}: {v}"));
                }
                Reply::ok(t)
            }
            "drive_joint" => self.app_drive(a),
            "return_to_design" => {
                if self.apps.driven.take().is_none() {
                    return Reply::ok("Nothing was driven");
                }
                if let Some(p) = &mut self.shell.fg {
                    p.out.push(FgAct::AsmPreview(None));
                }
                Reply::ok("Back to the design's position")
            }
            "keep_position" => {
                let Some(ops) = self.apps.driven.take() else {
                    return Reply::err("nothing is driven to keep");
                };
                if let Some(p) = &mut self.shell.fg {
                    p.out.push(FgAct::AsmPreview(None));
                    p.out.push(FgAct::AsmOps(ops));
                }
                Reply::ok("Kept: the driven values are in the file, as one undo step (not saved yet)")
            }
            "open_document" => {
                let path = s(a, "path");
                let dir = self
                    .agent_path()
                    .and_then(|p| std::path::Path::new(&p).parent().map(|d| d.to_path_buf()));
                let want = std::fs::canonicalize(path).ok();
                match (want, dir) {
                    (Some(w), Some(d))
                        if w.parent() == Some(d.as_path()) && w.extension().is_some_and(|x| x == "fg") =>
                    {
                        self.open_document(&w.display().to_string(), true);
                        Reply::ok(format!("Opened {}", w.display()))
                    }
                    _ => Reply::err("open_document opens a .fg file in the open document's folder"),
                }
            }
            "undo_view" => match self.apps.undo.pop() {
                Some(v) => {
                    self.view_restore(v);
                    Reply::ok("Took back the last view step")
                }
                None => Reply::ok("No view step to take back"),
            },
            _ => Reply::err(format!("no app tool `{name}`")),
        }
    }

    fn app_tree(&self, filter: &str, links: bool) -> Reply {
        if let Some(d) = &self.shell.drawing {
            return Reply::ok(format!(
                "A drawing, {}; selected: {}",
                d.name(),
                d.selected.as_deref().unwrap_or("nothing")
            ));
        }
        let Some(p) = &self.shell.fg else {
            return Reply::err("no Forge document is open");
        };
        let mut t = String::new();
        if let Some(asm) = &p.asm {
            let shown = asm.shown_mask();
            use crate::shell::fgasm::NodeKind as K;
            let mut lines = 0;
            for n in &asm.tree.nodes {
                let keep = match n.kind {
                    K::Top | K::Origin | K::OriginItem(_) => false,
                    K::Relationships | K::LinkKind | K::Link(_) => links,
                    _ => true,
                };
                // top-level rows unless a filter asks for what's under them
                let top = n.depth <= 1 || (links && matches!(n.kind, K::LinkKind | K::Link(_)));
                if !keep
                    || (filter.is_empty() && !top)
                    || (!filter.is_empty() && !n.key.contains(filter) && !n.label.contains(filter))
                {
                    continue;
                }
                lines += 1;
                if lines > 400 {
                    t.push_str("… more rows: narrow it with `filter`\n");
                    break;
                }
                let vis = if n.occ.is_empty() {
                    String::new()
                } else {
                    let on = n
                        .occ
                        .iter()
                        .filter(|&&k| shown.get(k).copied().unwrap_or(true))
                        .count();
                    match on {
                        0 => " hidden".into(),
                        x if x == n.occ.len() => " shown".into(),
                        x => format!(" {x}/{} shown", n.occ.len()),
                    }
                };
                let kind = match n.kind {
                    K::Occ => "part",
                    K::Sub => "subassembly",
                    K::Pattern => "pattern",
                    K::Link(_) => "relationship",
                    _ => "group",
                };
                t.push_str(&format!(
                    "{}{} ({kind}{}){vis}\n",
                    "  ".repeat(n.depth.saturating_sub(1)),
                    n.key,
                    if n.detail.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", n.detail)
                    }
                ));
            }
            if let Some(i) = &asm.isolated {
                t.push_str(&format!("isolated: {i}\n"));
            }
        } else {
            t.push_str(&format!(
                "solid {}\n",
                if p.body_hidden { "hidden" } else { "shown" }
            ));
            let planes = self.shell.overlays().planes;
            for f in &p.feats {
                let eye = match f.kind.as_str() {
                    "sketch" | "plane" | "workplane" | "axis" | "point" => {
                        let on = p.eyes.get(&f.id).copied().unwrap_or(planes);
                        if on { " shown" } else { " hidden" }
                    }
                    _ => "",
                };
                t.push_str(&format!("{} ({}){eye}\n", f.id, f.kind));
            }
        }
        Reply::ok(t)
    }

    /// A joint moved for a look: its driving param set in the build only, the file kept.
    fn app_drive(&mut self, a: &Json) -> Reply {
        // a relationship's browser key (`l:473`) names its joint too
        let link = s(a, "joint")
            .strip_prefix("l:")
            .and_then(|k| k.parse::<usize>().ok())
            .and_then(|k| {
                self.shell
                    .fg
                    .as_ref()?
                    .asm
                    .as_ref()?
                    .links
                    .get(k)
                    .map(|l| l.id.clone())
            });
        let joint = link.as_deref().unwrap_or(s(a, "joint"));
        let Some(value) = a.get("value").and_then(Json::num) else {
            return Reply::err("drive_joint takes `value`");
        };
        let what = s(a, "what");
        let Some(p) = &mut self.shell.fg else {
            return Reply::err("no Forge document is open");
        };
        if p.asm.is_none() {
            return Reply::err("drive_joint moves an assembly's joints");
        }
        let Some(f) = p.feats.iter().find(|f| f.id == joint) else {
            return Reply::err(format!("no joint `{joint}` in the open file"));
        };
        let Some(d) = f
            .dims
            .iter()
            .find(|d| matches!(d.from, From::Driving(_)) && (what.is_empty() || d.what == what))
        else {
            return Reply::err(format!(
                "`{joint}` has no driving value{}: it needs a param to drive (say so and ask before editing the file)",
                if what.is_empty() {
                    String::new()
                } else {
                    format!(" `{what}`")
                }
            ));
        };
        let From::Driving(key) = &d.from else {
            unreachable!()
        };
        let op = Op::SetParam {
            param: key.clone(),
            value: Literal { value, dim: d.dim },
        };
        let mut ops = self.apps.driven.take().unwrap_or_default();
        ops.retain(|o| !matches!(o, Op::SetParam { param, .. } if param == key));
        ops.push(op);
        p.out.push(FgAct::AsmPreview(Some(ops.clone())));
        self.apps.driven = Some(ops);
        // what moves: the joint's second side, framed in an iso view wide enough for its swing
        let moving = p
            .asm
            .as_ref()
            .and_then(|a| a.links.iter().find(|l| l.id == joint))
            .map(|l| l.b.0.clone());
        let (d_what, d_value) = (d.what.clone(), d.value);
        if !self.apps.view_set
            && let Some(m) = moving
        {
            self.app_frame(&m);
        }

        Reply::ok(format!(
            "Driven {joint}.{d_what} to {value} for a look (the file's {d_value}), framed in an iso view: \
             return_to_design puts it back, keep_position keeps it"
        ))
    }

    /// The view turned iso and fitted to an occurrence (and what's under it), room for its swing.
    fn app_frame(&mut self, path: &str) {
        let (Some((occ, _)), Some(v)) = (&self.fg_asm, &mut self.view) else {
            return;
        };
        let spheres: Vec<([f32; 3], f32)> = occ
            .iter()
            .filter(|o| crate::shell::fgasm::under(&o.path, path))
            .filter_map(|o| o.placement)
            .filter(|&k| k < v.scene.placements.len())
            .map(|k| v.scene.sphere(k))
            .collect();
        if spheres.is_empty() {
            return;
        }
        let n = spheres.len() as f32;
        let c = spheres.iter().fold([0.0f32; 3], |a, (p, _)| {
            [a[0] + p[0] / n, a[1] + p[1] / n, a[2] + p[2] / n]
        });
        let r = spheres
            .iter()
            .map(|(p, r)| ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt() + r)
            .fold(0.0f32, f32::max);
        v.nav.camera = crate::nav::region_view_as(v.nav.camera, [1, -1, 1], &v.nav.orient);
        v.nav.fit_sphere(c, r * 2.0);
        v.mark_dirty();
    }
}
