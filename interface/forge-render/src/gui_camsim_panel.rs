//! The simulation's chrome over the view: the timeline along its bottom (ops as bands, the
//! certified checks' markers, the playhead; play, step, speed, follow) and a panel along its
//! right (the program with each setup's and op's visibility, the display, the verdict and its
//! markers, the move list). The state (`CamSimUi`) is the Shell's while a machining document is
//! open; the view's geometry is made from it in `gui_camsim.rs`.
//!
//! The words it uses about the program are the run's: a check's verdict and the program's
//! certification come from the certified checker; the stock's colours are labelled a preview.

use crate::camsim::draw::{StockColour, look};
use crate::camsim::timeline::{Kind, clock};
use crate::camsim::{Marker, Sim};
use crate::shell::Shell;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};

/// A simulation made off the window's thread from the workspace's document: the text it is for
/// and the result; and the workspace's focus last taken.
#[derive(Default)]
pub struct SimJob {
    pub slot: std::sync::Arc<std::sync::Mutex<Option<(String, Sim)>>>,
    /// The text being simulated now.
    pub running: Option<String>,
    pub focus: Option<crate::shell::cam::Focus>,
    /// What the toolpath's colours were made from (the colour-by choice, the simulated text,
    /// whether the run's quality was measured).
    pub colour: Option<String>,
}

/// Where the playback is, for the workspace's browser to highlight (op by name, as the
/// workspace names them; the move of the posted path; the time).
#[derive(Clone, Debug, PartialEq)]
pub struct SimCursor {
    pub op: String,
    pub move_index: usize,
    pub t: f64,
}

/// How the part is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartShow {
    Hidden,
    Ghost,
    Solid,
}

/// The panel's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Program,
    Display,
    Checks,
    Moves,
}

/// What a machining document's simulation holds while it is open.
pub struct CamSimUi {
    pub sim: Sim,
    pub path: String,
    /// The document text it was made from.
    pub src: String,
    /// The time shown, seconds into the program.
    pub t: f64,
    pub playing: bool,
    /// Program seconds per second.
    pub speed: f64,
    pub show_ops: Vec<bool>,
    pub rapids: bool,
    pub played_only: bool,
    pub stock: bool,
    pub colour: StockColour,
    pub part: PartShow,
    pub tool: bool,
    pub fixture: bool,
    /// The machine's bodies, when a machine run plays.
    pub machine: bool,
    /// The toolpath coloured by a measured value per move (the workspace's colour-by).
    pub values: Option<crate::camsim::draw::Values>,
    pub follow: bool,
    pub path_width: f32,
    pub tab: Tab,
    /// The move list's selection (a piece), drawn wider.
    pub picked: Option<usize>,
    /// The move list scrolls to the piece playing once.
    scroll_to: Option<usize>,
    /// The panel's width; folded to its tabs.
    pub folded: bool,
    /// The op the workspace selected (its path drawn in the selection's blue).
    pub selected_op: Option<usize>,
    /// Every problem of the document listed, not the first few.
    pub all_errors: bool,
    /// What drew last: the time, the cells, the colouring (the stock's mesh is kept until one
    /// changes).
    pub drawn_key: Option<(u64, usize, bool, bool)>,
    pub stock_mesh: Vec<crate::origin::WorldVert>,
    /// Stop when a marker's time is reached while playing (as a machine operator would).
    pub stop_at_markers: bool,
    /// The wall clock at the last tick while playing, ms.
    pub last_ms: Option<f64>,
}

impl CamSimUi {
    pub fn new(sim: Sim, path: &str) -> CamSimUi {
        let n = sim.tl.ops.len();
        CamSimUi {
            sim,
            path: path.into(),
            src: String::new(),
            t: 0.0,
            playing: false,
            speed: 10.0,
            show_ops: vec![true; n],
            rapids: true,
            played_only: false,
            stock: true,
            colour: StockColour::Material,
            part: PartShow::Ghost,
            tool: true,
            fixture: true,
            machine: true,
            values: None,
            follow: false,
            path_width: 2.0,
            tab: Tab::Program,
            picked: None,
            scroll_to: None,
            folded: false,
            all_errors: false,
            selected_op: None,
            drawn_key: None,
            stock_mesh: vec![],
            stop_at_markers: true,
            last_ms: None,
        }
    }

    /// Play, or pause; playing from the end starts over.
    pub fn toggle_play(&mut self) {
        if !self.playing && self.t >= self.total() {
            self.t = 0.0;
        }
        self.playing = !self.playing;
    }

    /// The transport's keys: Space plays or pauses, ← and → step a move, Home and End go to the
    /// start and the end. Not while a text field has the keyboard.
    pub fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::Key;
        let (space, left, right, home, end) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::Space),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowLeft),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowRight),
                i.consume_key(egui::Modifiers::NONE, Key::Home),
                i.consume_key(egui::Modifiers::NONE, Key::End),
            )
        });
        if space {
            self.toggle_play();
        }
        if left {
            let to = self.sim.tl.step(self.t, true);
            self.jump(to);
        }
        if right {
            let to = self.sim.tl.step(self.t, false);
            self.jump(to);
        }
        if home {
            self.jump(0.0);
        }
        if end {
            let to = self.total();
            self.jump(to);
        }
    }

    pub fn total(&self) -> f64 {
        self.sim.tl.total
    }

    /// Plays on by `dt` seconds of the wall clock.
    pub fn tick(&mut self, dt: f64) -> bool {
        if !self.playing {
            return false;
        }
        let was = self.t;
        let to = (self.t + dt * self.speed).min(self.total());
        // a marker passed: stop on it
        if self.stop_at_markers
            && let Some(m) = self
                .sim
                .markers
                .iter()
                .find(|m| m.t > was && m.t <= to && m.severe())
        {
            self.t = m.t;
            self.playing = false;
            return true;
        }
        self.t = to;
        if self.t >= self.total() {
            self.playing = false;
        }
        true
    }

    /// Jumps the playback to op `op`'s path `path`, move `mv`, fraction `s` of it (the
    /// interface the CAM workspace calls: a click on a check, a move, an op).
    pub fn locate(&mut self, op: usize, path: usize, mv: usize, s: f64) -> bool {
        match self.sim.tl.locate(op, path, mv, s) {
            Some(t) => {
                self.jump(t);
                true
            }
            None => false,
        }
    }

    /// Shows op `op` alone, from its start.
    pub fn select_op(&mut self, op: usize) {
        self.selected_op = Some(op).filter(|&k| k < self.sim.tl.ops.len());
        if let Some(sp) = self.sim.tl.ops.get(op) {
            let t = sp.t0;
            self.jump(t);
        }
    }

    pub fn jump(&mut self, t: f64) {
        self.t = t.clamp(0.0, self.total());
        self.playing = false;
        self.scroll_to = self.sim.tl.seg_at(self.t);
    }

    /// Where the playback is (`SimCursor`).
    pub fn cursor(&self) -> Option<SimCursor> {
        let i = self.sim.tl.seg_at(self.t)?;
        let g = self.sim.tl.segs[i];
        Some(SimCursor {
            op: self.sim.tl.ops[g.op as usize].name.clone(),
            move_index: g.mv as usize,
            t: self.t,
        })
    }

    /// Takes the workspace's focus: its setup or op shown (and only it, if asked), a check
    /// located, the stock after an op.
    pub fn take_focus(&mut self, f: &crate::shell::cam::Focus) {
        let op_of = |name: &str| self.sim.tl.ops.iter().position(|o| o.name == name);
        let setup_of = |name: &str| self.sim.setups.iter().position(|s| s.name == name);
        let op = f.op.as_deref().and_then(op_of);
        let setup = f.setup.as_deref().and_then(setup_of);
        self.selected_op = op;
        for (k, o) in self.sim.tl.ops.iter().enumerate() {
            self.show_ops[k] = match (op, setup) {
                (Some(i), _) if f.only_selected => k == i,
                (_, Some(s)) => o.setup == s,
                _ => true,
            };
        }
        if let Some(l) = &f.locate
            && let Some(i) = op_of(&l.op)
        {
            // the witness is in the setup's WCS: the part's frame less the origin
            let o = self
                .sim
                .setups
                .get(self.sim.tl.ops[i].setup)
                .map_or([0.0; 3], |s| s.origin);
            let at = [l.at[0] + o[0], l.at[1] + o[1], l.at[2] + o[2]];
            let paths = self.sim.cam.sim_paths(i).len();
            let best = (0..paths)
                .filter_map(|p| self.sim.tl.locate(i, p, l.mov, 0.5))
                .min_by(|a, b| {
                    let d = |t: f64| {
                        let q = self.sim.tl.pos(t).map_or([f64::INFINITY; 3], |x| x.0);
                        (0..3).map(|k| (q[k] - at[k]).powi(2)).sum::<f64>()
                    };
                    d(*a).total_cmp(&d(*b))
                })
                .or_else(|| self.sim.tl.nearest(i, at));
            if let Some(t) = best {
                self.jump(t);
            }
        } else if let Some(i) = f.stock_at.as_deref().and_then(op_of) {
            let t = self.sim.tl.ops[i].t1;
            self.jump(t);
        } else if let Some(i) = op {
            self.select_op(i);
        }
    }

    /// The tool playing now: its index and its tip.
    pub fn tool_now(&self) -> Option<(usize, [f64; 3], usize)> {
        let (p, i) = self.sim.tl.pos(self.t)?;
        Some((self.sim.tl.segs[i].tool as usize, p, i))
    }

    /// The setup playing now.
    pub fn setup_now(&self) -> usize {
        self.sim.tl.op_at(self.t).map_or(0, |o| self.sim.tl.ops[o].setup)
    }
}

/// An op's colour on the timeline (cycled; distinct from the motion colours).
fn op_colour(i: usize) -> Color32 {
    const C: [u32; 6] = [0x5B7FA6, 0x7A6AA8, 0x4F9A8E, 0xA0794E, 0x6F8F4A, 0x9A5C77];
    let c = C[i % C.len()];
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn srgb(c: [f32; 3]) -> Color32 {
    let e = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        let s = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    Color32::from_rgb(e(c[0]), e(c[1]), e(c[2]))
}

pub(crate) const TIMELINE_H: f32 = 78.0;
pub(crate) const PANEL_W: f32 = 312.0;

impl Shell {
    /// The part of view `v` the simulation's panel (right) and timeline (bottom) leave free, for
    /// the workspace's windows to keep inside.
    pub(crate) fn camsim_free(&self, v: Rect) -> Rect {
        let Some(st) = self.camsim.as_deref() else {
            return v;
        };
        let panel_w = if st.folded { 36.0 } else { PANEL_W };
        Rect::from_min_max(
            v.min,
            pos2(v.right() - panel_w - 8.0, v.bottom() - TIMELINE_H - 16.0),
        )
    }

    /// The simulation's timeline and panel, over the view; true if a machining document is open.
    pub fn camsim_ui(&mut self, ctx: &egui::Context) -> bool {
        // the Manufacture tab's: elsewhere the document's other tabs are left alone
        if self.tab != crate::shell::CAM_TAB {
            return false;
        }
        let Some(mut st) = self.camsim.take() else {
            return false;
        };
        let v = self.view_rect;
        let t = self.theme;
        st.keys(ctx);
        // the timeline: across the view's bottom, short of the panel
        let panel_w = if st.folded { 36.0 } else { PANEL_W };
        let extra = if st.sim.machine().is_some_and(|m| !m.run.wcs_given) {
            18.0
        } else {
            0.0
        };
        let bar = Rect::from_min_max(
            pos2(v.left() + 8.0, v.bottom() - TIMELINE_H - extra - 8.0),
            pos2(v.right() - panel_w - 16.0, v.bottom() - 8.0),
        );
        let r = egui::Area::new(egui::Id::unique("camsim:timeline"))
            .fixed_pos(bar.min)
            .order(egui::Order::Middle)
            .fade_in(false)
            .show(ctx, |ui| {
                ui.set_width(bar.width());
                ui.set_height(bar.height());
                egui::Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(1.0, t.line))
                    .corner_radius(6.0)
                    .inner_margin(6.0)
                    .show(ui, |ui| timeline(ui, &mut st, &t, bar.width() - 12.0));
            });
        self.view_blocks.push(r.response.rect);
        // the panel: down the view's right
        let pr = Rect::from_min_max(
            pos2(v.right() - panel_w - 8.0, v.top() + 44.0),
            pos2(v.right() - 8.0, v.bottom() - 8.0),
        );
        let r = egui::Area::new(egui::Id::unique("camsim:panel"))
            .fixed_pos(pr.min)
            .order(egui::Order::Middle)
            .fade_in(false)
            .show(ctx, |ui| {
                ui.set_width(pr.width());
                ui.set_max_height(pr.height());
                egui::Frame::new()
                    .fill(t.surface)
                    .stroke(Stroke::new(1.0, t.line))
                    .corner_radius(6.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_min_height(pr.height() - 18.0);
                        panel(ui, &mut st, &t, pr.width() - 16.0)
                    });
            });
        self.view_blocks.push(r.response.rect);
        // the markers in the view: a ring at each, clickable
        if let Some((cam, size)) = self.camera {
            let to_px = |p: [f64; 3]| {
                cam.pixel_of([p[0] as f32, p[1] as f32, p[2] as f32], size.0, size.1)
                    .map(|(x, y)| {
                        pos2(
                            v.left() + x / size.0 as f32 * v.width(),
                            v.top() + y / size.1 as f32 * v.height(),
                        )
                    })
            };
            let setup = st.setup_now();
            let mut jump = None;
            // in the view's own layer: under every window and panel drawn over the view (CAM-UI)
            let painter = ctx.layer_painter(egui::LayerId::background()).with_clip_rect(v);
            for (k, m) in st.sim.markers.iter().enumerate() {
                if m.setup != setup || m.t > st.t + 1e-9 && m.severe() && st.played_only {
                    continue;
                }
                let Some(p) = to_px(m.at) else { continue };
                if !v.contains(p) {
                    continue;
                }
                let c = if m.severe() {
                    srgb(look::FAIL)
                } else {
                    srgb(look::LEFTOVER)
                };
                painter.circle_stroke(p, 7.0, Stroke::new(2.0, c));
                painter.circle_filled(p, 2.5, c);
                let id = egui::Id::unique("camsim:mark").with(k);
                let hit = Rect::from_center_size(p, vec2(16.0, 16.0));
                let resp = egui::Area::new(id)
                    .fixed_pos(hit.min)
                    .order(egui::Order::Background)
                    .fade_in(false)
                    .show(ctx, |ui| {
                        let (_, r) = ui.allocate_exact_size(hit.size(), Sense::click());
                        r.on_hover_text(&m.text)
                    });
                if resp.inner.clicked() {
                    jump = Some(m.t);
                }
                self.view_blocks.push(hit);
            }
            if let Some(tm) = jump {
                st.jump(tm);
            }
        }
        if st.playing {
            ctx.request_repaint();
        }
        self.camsim = Some(st);
        true
    }
}

/// The timeline: transport, the bands, markers and the playhead.
fn timeline(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme, w: f32) {
    let total = st.total();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let b = |ui: &mut Ui, id: &str, g: Glyph, tip: &str| glyph_button(ui, t, id, g, tip, true).clicked();
        if b(ui, "camsim:start", Glyph::Start, "To the start (Home)") {
            st.jump(0.0);
        }
        if b(ui, "camsim:back", Glyph::Back, "Back a move (←)") {
            let to = st.sim.tl.step(st.t, true);
            st.jump(to);
        }
        let (g, tip) = if st.playing {
            (Glyph::Pause, "Pause (Space)")
        } else {
            (Glyph::Play, "Play (Space)")
        };
        if b(ui, "camsim:play", g, tip) {
            st.toggle_play();
        }
        if b(ui, "camsim:fwd", Glyph::Forward, "On a move (→)") {
            let to = st.sim.tl.step(st.t, false);
            st.jump(to);
        }
        if b(ui, "camsim:end", Glyph::End, "To the end (End)") {
            st.jump(total);
        }
        ui.separator();
        ui.label(
            egui::RichText::new(format!("{} / {}", clock(st.t), clock(total)))
                .monospace()
                .color(t.ink),
        );
        ui.separator();
        ui.label(egui::RichText::new("Speed").color(t.ink2));
        egui::ComboBox::from_id_salt("camsim:speed")
            .width(64.0)
            .selected_text(format!("{}×", st.speed))
            .show_ui(ui, |ui| {
                for s in [0.25, 1.0, 5.0, 10.0, 25.0, 100.0, 500.0] {
                    ui.selectable_value(&mut st.speed, s, format!("{s}×"));
                }
            });
        // the playback toggles where the bar has room for them (always in the Display tab)
        if ui.available_width() > 250.0 {
            ui.checkbox(&mut st.follow, "Follow tool");
            ui.checkbox(&mut st.stop_at_markers, "Stop at issues");
        }
        // what's playing: one line, cut short with the whole of it on hover
        if let Some((tool, p, i)) = st.tool_now()
            && ui.available_width() > 120.0
        {
            let g = st.sim.tl.segs[i];
            let op = &st.sim.tl.ops[g.op as usize];
            let tu = st.sim.cam.sim_tools().get(tool);
            ui.separator();
            let s = format!(
                "{} · T{} {} · {} · F {:.0} mm/min · X {:.3}  Y {:.3}  Z {:.3} mm",
                op.name,
                tu.map_or(0, |u| u.t),
                tu.map_or("", |u| u.tool.id.as_str()),
                g.kind.name(),
                g.f,
                p[0],
                p[1],
                p[2]
            );
            ui.add(egui::Label::new(egui::RichText::new(&s).color(t.ink2)).truncate())
                .on_hover_text(&s);
        }
    });
    // a machine run's placement, while the document gives none
    if let Some(mv) = st.sim.machine()
        && !mv.run.wcs_given
    {
        ui.label(
            egui::RichText::new(format!(
                "Posted code on {}: stock placement assumed (centred on the table datum; the setup gives no `at:`)",
                mv.run.def.name
            ))
            .size(11.0)
            .color(t.warn),
        );
    }
    // the bands
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, t.field);
    let x_of = |tm: f64| rect.left() + (tm / total.max(1e-9)) as f32 * rect.width();
    let band = Rect::from_min_max(
        pos2(rect.left(), rect.top() + 12.0),
        pos2(rect.right(), rect.bottom() - 2.0),
    );
    for (k, op) in st.sim.tl.ops.iter().enumerate() {
        let r = Rect::from_min_max(
            pos2(x_of(op.t0), band.top()),
            pos2(x_of(op.t1).max(x_of(op.t0) + 1.0), band.bottom()),
        );
        let mut c = op_colour(k);
        if !st.show_ops.get(k).copied().unwrap_or(true) {
            c = c.gamma_multiply(0.35);
        }
        painter.rect_filled(r.shrink2(vec2(0.5, 0.0)), 2.0, c);
        if op.status != "built" {
            painter.rect_stroke(
                r,
                2.0,
                Stroke::new(1.5, srgb(look::FAIL)),
                egui::StrokeKind::Inside,
            );
        }
        if r.width() > 40.0 {
            painter.text(
                pos2(r.left() + 4.0, r.center().y),
                Align2::LEFT_CENTER,
                &op.name,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }
    // setups: a tick where each starts
    let mut last = usize::MAX;
    for op in &st.sim.tl.ops {
        if op.setup != last {
            let x = x_of(op.t0);
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1.0, t.ink2),
            );
            let name = st.sim.setups.get(op.setup).map_or("", |s| s.name.as_str());
            painter.text(
                pos2(x + 3.0, rect.top() + 1.0),
                Align2::LEFT_TOP,
                name,
                FontId::proportional(11.0),
                t.ink2,
            );
            last = op.setup;
        }
    }
    // markers
    let mut hover: Option<&Marker> = None;
    let pointer = resp.hover_pos();
    for m in &st.sim.markers {
        let x = x_of(m.t);
        let c = if m.severe() {
            srgb(look::FAIL)
        } else {
            srgb(look::LEFTOVER)
        };
        let tri = vec![
            pos2(x, band.top() - 1.0),
            pos2(x - 4.5, rect.top() + 2.0),
            pos2(x + 4.5, rect.top() + 2.0),
        ];
        painter.add(egui::Shape::convex_polygon(tri, c, Stroke::NONE));
        if pointer.is_some_and(|p| (p.x - x).abs() < 5.0 && p.y < band.top() + 2.0) {
            hover = Some(m);
        }
    }
    // the playhead
    let x = x_of(st.t);
    painter.line_segment(
        [pos2(x, rect.top()), pos2(x, rect.bottom())],
        Stroke::new(2.0, t.accent),
    );
    painter.circle_filled(pos2(x, rect.top() + 4.0), 4.0, t.accent);
    if let Some(m) = hover {
        let text = m.text.clone();
        let tm = m.t;
        let resp = resp.on_hover_text(text);
        if resp.clicked() {
            st.jump(tm);
        }
    } else if let Some(p) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        let f = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
        st.t = f * total;
        st.playing = false;
        st.scroll_to = st.sim.tl.seg_at(st.t);
    }
}

fn heading(ui: &mut Ui, t: &crate::shell::theme::Theme, s: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(s).strong().color(t.ink));
}

/// The panel: a fold button and the tabs as one control, then the tab's body.
fn panel(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme, w: f32) {
    ui.horizontal(|ui| {
        let (tip, g) = if st.folded {
            ("Open the simulation panel", Glyph::ChevronLeft)
        } else {
            ("Fold the simulation panel", Glyph::ChevronRight)
        };
        if glyph_button(ui, t, "camsim:fold", g, tip, true).clicked() {
            st.folded = !st.folded;
        }
        if st.folded {
            return;
        }
        let tabs = [
            (Tab::Program, "Program"),
            (Tab::Display, "Display"),
            (Tab::Checks, "Checks"),
            (Tab::Moves, "Moves"),
        ];
        let items: Vec<(&str, bool)> = tabs.iter().map(|(k, n)| (*n, st.tab == *k)).collect();
        let mut places = vec![];
        if let Some(k) = crate::shell::settings::segmented(
            ui,
            t,
            &items,
            &|k| format!("camsim:tab:{}", tabs[k].1),
            &mut places,
        ) {
            st.tab = tabs[k].0;
        }
    });
    if st.folded {
        return;
    }
    ui.separator();
    ui.set_width(w);
    match st.tab {
        Tab::Program => program(ui, st, t),
        Tab::Display => display(ui, st, t),
        Tab::Checks => checks(ui, st, t),
        Tab::Moves => moves(ui, st, t),
    }
}

/// The verdict line: the program's certification as the run states it.
fn verdict(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme) {
    let cam = &st.sim.cam;
    let refused = cam.ops.iter().filter(|o| o.status != "built").count();
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    let (c, s) = if !cam.ok {
        let n = cam.errors.len();
        (
            t.error,
            format!(
                "Not certified: the document doesn't run ({n} problem{})",
                plural(n)
            ),
        )
    } else if refused > 0 {
        (
            t.error,
            format!("Not certified: {refused} op{} refused", plural(refused)),
        )
    } else {
        (t.success, format!("Certified ({})", cam.certification()))
    };
    ui.colored_label(c, s);
    // the posted code on the machine: its own result, beside the toolpath's certificate
    if let Some(mv) = st.sim.machine() {
        let n = mv.markers.len();
        let (c, s) = if n == 0 {
            (
                t.success,
                format!("Posted code on {}: no collision or limit", mv.run.def.name),
            )
        } else {
            (
                t.error,
                format!(
                    "Posted code on {}: {n} issues (collisions, limits)",
                    mv.run.def.name
                ),
            )
        };
        ui.colored_label(c, s);
    }
    // the document's problems: one line each (its line, then what), the first few shown, the
    // rest behind "more"; the whole message on hover
    let shown = if st.all_errors {
        cam.errors.len()
    } else {
        cam.errors.len().min(3)
    };
    for e in &cam.errors[..shown] {
        let (at, what) = problem(e);
        ui.horizontal(|ui| {
            if let Some(at) = at {
                ui.label(egui::RichText::new(at).size(11.0).color(t.ink2));
            }
            ui.add(egui::Label::new(egui::RichText::new(what).color(t.error)).truncate())
                .on_hover_text(e);
        });
    }
    if cam.errors.len() > shown && ui.link(format!("{} more", cam.errors.len() - shown)).clicked() {
        st.all_errors = true;
    }
}

/// A run's error as (where, what): "line 7: `x` is not …" is ("Line 7", "`x` is not …"), the
/// list of what would have been accepted (a parenthesis ending the message) left to the hover.
fn problem(e: &str) -> (Option<String>, String) {
    let (at, rest) = match e.split_once(": ") {
        Some((l, r)) if l.starts_with("line ") && l[5..].chars().all(|c| c.is_ascii_digit()) => {
            (Some(format!("Line {}", &l[5..])), r)
        }
        _ => (None, e),
    };
    let what = match rest.rfind(" (") {
        Some(k) if rest.ends_with(')') && k > 0 => &rest[..k],
        _ => rest,
    };
    (at, what.to_string())
}

/// What plays: the toolpath, or a posted program on its machine.
fn source(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme) {
    let mut want = st.sim.source;
    // Toolpath | Posted code (one per program run on a machine), the machine named beneath
    let mut items: Vec<(String, Option<usize>, bool)> = vec![("Toolpath".into(), None, true)];
    for (k, m) in st.sim.machines.iter().enumerate() {
        let name = if st.sim.machines.len() > 1 {
            format!("Posted code {}", k + 1)
        } else {
            "Posted code".into()
        };
        items.push((name, Some(k), m.is_ok()));
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let seg: Vec<(&str, bool)> = items.iter().map(|(n, k, _)| (n.as_str(), *k == want)).collect();
        let mut places = vec![];
        if let Some(i) =
            crate::shell::settings::segmented(ui, t, &seg, &|i| format!("camsim:source:{i}"), &mut places)
            && items[i].2
        {
            want = items[i].1;
        }
        // a run that could not be made: why, on the control
        for ((_, k, ok), (_, r)) in items.iter().zip(&places) {
            if let (false, Some(k)) = (ok, k)
                && let Err(e) = &st.sim.machines[*k]
            {
                ui.interact(*r, ui.scope_id().with(("camsim:source:why", *k)), Sense::hover())
                    .on_hover_text(format!("Can't run the posted code on its machine: {e}"));
                ui.painter().rect_filled(*r, 4.0, t.surface.gamma_multiply(0.6));
            }
        }
    });
    let sub = match want.and_then(|k| st.sim.machines.get(k)) {
        Some(Ok(mv)) => format!("{} ({}) on {}", mv.run.program, mv.run.dialect, mv.run.def.name),
        _ => match st.sim.machines.first() {
            Some(Ok(mv)) => format!(
                "The planned toolpath · the posted code runs on {}",
                mv.run.def.name
            ),
            Some(Err(_)) => "The planned toolpath · the posted code can't run on its machine (hover)".into(),
            None => "The planned toolpath · no program posted".into(),
        },
    };
    ui.label(egui::RichText::new(sub).size(11.0).color(t.ink2));
    if want != st.sim.source {
        let t0 = st.t / st.total().max(1e-9);
        if let Err(e) = st.sim.set_source(want) {
            ui.colored_label(t.error, e);
        }
        st.drawn_key = None;
        let total = st.total();
        st.jump(t0 * total);
    }
    if let Some(mv) = st.sim.machine() {
        let r = &mv.run;
        let mm = |v: f64| {
            format!("{:.3}", v)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        };
        let offset = r
            .control
            .letters
            .iter()
            .zip(&r.wcs)
            .map(|(l, v)| format!("{l} {}", mm(*v)))
            .collect::<Vec<_>>()
            .join("  ");
        let contacts = r.contacts.contacts.len();
        let lines = [
            format!(
                "{} lines of {} code · {} motions · {} (at least {})",
                r.text.lines().count(),
                r.dialect,
                r.tl.segments.len(),
                duration(r.tl.t_stop),
                duration(r.tl.t_min)
            ),
            if r.wcs_given {
                format!("Work offset G54: {offset} mm (from the setup)")
            } else {
                format!(
                    "Work offset G54: {offset} mm (assumed: the setup isn't placed, so the stock is centred on the table)"
                )
            },
            if contacts == 0 && r.tl.violations.is_empty() {
                "Machine, tool and workholding checked through every motion: no contact, no limit".into()
            } else {
                format!(
                    "Machine, tool and workholding checked through every motion: {contacts} contact{}, {} limit{}",
                    if contacts == 1 { "" } else { "s" },
                    r.tl.violations.len(),
                    if r.tl.violations.len() == 1 { "" } else { "s" }
                )
            },
        ];
        for l in lines {
            ui.add(egui::Label::new(egui::RichText::new(l).size(11.0).color(t.ink2)).wrap());
        }
        // the axes now, each with its unit
        let joints = mv.joints_at(st.t);
        let unit = |n: &str| {
            r.def.joints.iter().find(|j| j.name == n).map_or("", |j| {
                if j.ty == forge_machine::def::JointType::Rotary {
                    "°"
                } else {
                    " mm"
                }
            })
        };
        let axes = joints
            .iter()
            .map(|(n, v)| format!("{n} {v:.3}{}", unit(n)))
            .collect::<Vec<_>>()
            .join("   ");
        ui.add(egui::Label::new(egui::RichText::new(axes).monospace().size(11.0).color(t.ink)).wrap());
    }
}

fn program(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme) {
    verdict(ui, st, t);
    source(ui, st, t);
    ui.separator();
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt("camsim:program")
        .show(ui, |ui| {
            for (si, s) in st.sim.setups.iter().enumerate() {
                let ops: Vec<usize> = (0..st.sim.tl.ops.len())
                    .filter(|&k| st.sim.tl.ops[k].setup == si)
                    .collect();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mut all = ops.iter().all(|&k| st.show_ops[k]);
                    if ui
                        .checkbox(&mut all, "")
                        .on_hover_text("Show this setup's toolpaths")
                        .changed()
                    {
                        for &k in &ops {
                            st.show_ops[k] = all;
                        }
                    }
                    ui.label(
                        egui::RichText::new(format!("Setup {}", s.name))
                            .strong()
                            .color(t.ink),
                    );
                });
                let st_desc = format!(
                    "stock {:.1} × {:.1} × {:.1}{}{}",
                    s.stock.max[0] - s.stock.min[0],
                    s.stock.max[1] - s.stock.min[1],
                    s.stock.max[2] - s.stock.min[2],
                    s.fixture_name
                        .as_deref()
                        .map(|f| format!(", in {f}"))
                        .unwrap_or_default(),
                    if s.turned.is_some() { ", turned over" } else { "" }
                );
                ui.label(egui::RichText::new(st_desc).size(11.0).color(t.ink2));
                for &k in &ops {
                    let op = &st.sim.tl.ops[k];
                    let out = &st.sim.cam.ops[k];
                    let now = st.sim.tl.op_at(st.t) == Some(k);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.checkbox(&mut st.show_ops[k], "");
                        let c = if op.status == "built" { t.success } else { t.error };
                        let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, c);
                        let label = egui::RichText::new(format!("{} ({})", op.name, op.kind)).color(if now {
                            t.accent
                        } else {
                            t.ink
                        });
                        if ui
                            .add(egui::Label::new(label).sense(Sense::click()))
                            .on_hover_text(match &out.refusal {
                                Some(r) => format!("refused: {r}"),
                                None => format!(
                                    "{} moves, {:.0} mm cut, {:.0} mm rapid, least distance {:.4} mm",
                                    out.moves, out.length_cut, out.length_rapid, out.least
                                ),
                            })
                            .clicked()
                        {
                            jump = Some(op.t0);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // the in-process stock as this op leaves it
                            if glyph_button(
                                ui,
                                t,
                                &format!("camsim:after:{k}"),
                                Glyph::Stock,
                                "Show the stock after this op",
                                true,
                            )
                            .clicked()
                            {
                                jump = Some(op.t1);
                            }
                            // its time, in a right-aligned column
                            let (r, _) = ui.allocate_exact_size(vec2(78.0, 18.0), Sense::hover());
                            ui.painter().text(
                                pos2(r.right(), r.center().y),
                                Align2::RIGHT_CENTER,
                                duration(op.t1 - op.t0),
                                FontId::proportional(12.0),
                                t.ink2,
                            );
                        });
                    });
                    if let Some(r) = &out.refusal {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(32.0);
                            ui.label(egui::RichText::new(r).size(11.0).color(t.error));
                        });
                    }
                }
            }
            ui.add_space(6.0);
            for p in &st.sim.cam.programs {
                let s = match (&p.error, p.round_trip) {
                    (Some(e), _) => format!("{} ({}): {e}", p.name, p.post),
                    (None, true) => format!("{} ({}): {} lines, read back exact", p.name, p.post, p.lines),
                    (None, false) => format!("{} ({}): not released", p.name, p.post),
                };
                ui.label(egui::RichText::new(s).size(11.0).color(t.ink2));
            }
        });
    if let Some(tm) = jump {
        st.jump(tm);
    }
}

fn display(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme) {
    heading(ui, t, "Playback");
    ui.checkbox(&mut st.follow, "Follow the tool");
    ui.checkbox(&mut st.stop_at_markers, "Stop at issues");
    // the toolpath coloured by a measured value: its legend
    if let Some(v) = &st.values {
        heading(ui, t, &format!("Coloured by {}", v.name));
        ui.horizontal_wrapped(|ui| {
            for (_, c, word) in &v.bands {
                let (r, _) = ui.allocate_exact_size(vec2(14.0, 4.0), Sense::hover());
                ui.painter().rect_filled(r, 1.0, srgb(*c));
                ui.label(egui::RichText::new(word).size(11.0));
            }
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 4.0), Sense::hover());
            ui.painter().rect_filled(r, 1.0, srgb(look::NO_VALUE));
            ui.label(egui::RichText::new("not measured").size(11.0));
        });
        ui.label(
            egui::RichText::new(format!("unit: {}", v.unit))
                .size(11.0)
                .color(t.ink2),
        );
    }
    heading(ui, t, "Toolpath");
    ui.checkbox(&mut st.rapids, "Rapids");
    ui.checkbox(&mut st.played_only, "Only what has played");
    ui.horizontal(|ui| {
        ui.label("Width");
        ui.add(egui::Slider::new(&mut st.path_width, 1.0..=6.0).suffix(" px"));
    });
    ui.horizontal_wrapped(|ui| {
        for (k, name) in [
            (Kind::Cut, "feed"),
            (Kind::Cut, "arc"),
            (Kind::Plunge, "plunge"),
            (Kind::Link, "link"),
            (Kind::Rapid, "rapid"),
        ] {
            let c = match (k, name) {
                (Kind::Cut, "arc") => look::ARC,
                (Kind::Cut, _) => look::CUT,
                (Kind::Plunge, _) => look::PLUNGE,
                (Kind::Link, _) => look::LINK,
                (Kind::Rapid, _) => look::RAPID,
            };
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 4.0), Sense::hover());
            ui.painter().rect_filled(r, 1.0, srgb(c));
            ui.label(egui::RichText::new(name).size(11.0));
        }
    });
    heading(ui, t, "Stock");
    ui.checkbox(&mut st.stock, "Show the stock");
    ui.horizontal(|ui| {
        ui.radio_value(&mut st.colour, StockColour::Material, "Material");
        ui.radio_value(&mut st.colour, StockColour::Compare, "Compare with the part");
    });
    if st.colour == StockColour::Compare {
        ui.horizontal_wrapped(|ui| {
            for (c, name) in [
                (look::UNDER, "left on"),
                (look::IN_TOL, "within tolerance"),
                (look::GOUGE, "below the part"),
            ] {
                let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, srgb(c));
                ui.label(egui::RichText::new(name).size(11.0));
            }
        });
    }
    let mut cells = st.sim.cells() as f64;
    ui.horizontal(|ui| {
        ui.label("Accuracy");
        let r = ui.add(
            egui::Slider::new(&mut cells, 20_000.0..=1_000_000.0)
                .logarithmic(true)
                .show_value(false),
        );
        if r.drag_stopped() || (r.changed() && !r.dragged()) {
            st.sim.set_cells(cells as usize);
            st.drawn_key = None;
        }
    });
    let cell = st
        .sim
        .setups
        .get(st.setup_now())
        .map(|s| {
            let (w, d) = (s.stock.max[0] - s.stock.min[0], s.stock.max[1] - s.stock.min[1]);
            (w * d / st.sim.cells() as f64).sqrt()
        })
        .unwrap_or(0.0);
    ui.label(
        egui::RichText::new(format!(
            "A preview on a {cell:.2} mm grid: its colours are not the verdict. The verdict is the certified checks' (Checks)."
        ))
        .size(11.0)
        .color(t.ink2),
    );
    heading(ui, t, "Show");
    ui.horizontal(|ui| {
        ui.label("Part");
        ui.radio_value(&mut st.part, PartShow::Hidden, "Hidden");
        ui.radio_value(&mut st.part, PartShow::Ghost, "See-through");
        ui.radio_value(&mut st.part, PartShow::Solid, "Solid");
    });
    ui.checkbox(&mut st.tool, "Tool and holder");
    ui.checkbox(&mut st.fixture, "Workholding");
    if st.sim.machine().is_some() {
        ui.checkbox(&mut st.machine, "Machine");
    }
}

fn checks(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme) {
    verdict(ui, st, t);
    let cam = &st.sim.cam;
    let pairs: usize = cam.ops.iter().map(|o| o.pairs).sum();
    ui.label(
        egui::RichText::new(format!(
            "{pairs} tool–face pairs certified by the checker (G1 gouge, G2 holder and fixture, G3 rapids); leftover (G4) {}",
            if cam.leftover_done { "decided exactly" } else { "not decided" }
        ))
        .size(11.0)
        .color(t.ink2),
    );
    heading(ui, t, &format!("Issues ({})", st.sim.markers.len()));
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt("camsim:checks")
        .show(ui, |ui| {
            if st.sim.markers.is_empty() {
                ui.colored_label(t.success, "None: every move certified, nothing left over.");
            }
            for m in &st.sim.markers {
                let c = if m.severe() {
                    srgb(look::FAIL)
                } else {
                    srgb(look::LEFTOVER)
                };
                let r = ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{}  {}", clock(m.t), m.text))
                            .size(11.0)
                            .color(c),
                    )
                    .sense(Sense::click()),
                );
                if r.on_hover_text("Go there").clicked() {
                    jump = Some(m.t);
                }
            }
        });
    if let Some(tm) = jump {
        st.jump(tm);
    }
}

fn moves(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme) {
    // the op playing: its pieces
    let Some(oi) = st.sim.tl.op_at(st.t) else {
        ui.label("No moves.");
        return;
    };
    let sp = st.sim.tl.ops[oi].clone();
    ui.label(
        egui::RichText::new(format!("{} ({}): {} moves", sp.name, sp.kind, sp.end - sp.first)).color(t.ink),
    );
    let now = st.sim.tl.seg_at(st.t);
    let row_h = 18.0;
    let mut jump = None;
    let scroll = st.scroll_to.take();
    let mut area = egui::ScrollArea::vertical()
        .id_salt("camsim:moves")
        .auto_shrink([false, false]);
    if let Some(k) = scroll
        && k >= sp.first
        && k < sp.end
    {
        area = area.vertical_scroll_offset(
            ((k - sp.first) as f32 * (row_h + ui.spacing().item_spacing.y) - 120.0).max(0.0),
        );
    }
    area.show_rows(ui, row_h, sp.end - sp.first, |ui, rows| {
        for r in rows {
            let k = sp.first + r;
            let g = st.sim.tl.segs[k];
            let on = now == Some(k);
            let c = srgb(crate::camsim::draw::colour_of(&g));
            let text = format!(
                "{:>5} {:<6} {:>9.3} {:>9.3} {:>8.3} {:>6.0} {:>7}",
                g.mv,
                if g.arc.is_some() { "arc" } else { g.kind.name() },
                g.b[0],
                g.b[1],
                g.b[2],
                g.f,
                clock(g.t0)
            );
            let resp = ui.add(
                egui::Label::new(egui::RichText::new(text).monospace().size(10.5).color(if on {
                    t.accent
                } else {
                    c
                }))
                .sense(Sense::click()),
            );
            if resp.hovered() {
                st.picked = Some(k);
            }
            if resp.clicked() {
                jump = Some(g.t0);
            }
        }
    });
    if let Some(tm) = jump {
        st.jump(tm);
    }
}

/// A point on the view's screen for a world point, for tests and hosts.
pub fn screen_of(cam: &crate::camera::Camera, size: (u32, u32), v: Rect, p: [f64; 3]) -> Option<Pos2> {
    cam.pixel_of([p[0] as f32, p[1] as f32, p[2] as f32], size.0, size.1)
        .map(|(x, y)| {
            pos2(
                v.left() + x / size.0 as f32 * v.width(),
                v.top() + y / size.1 as f32 * v.height(),
            )
        })
}

/// The simulation's own small glyphs, painted (the transport's, the fold's, the stock's), until
/// the shared toolkit's transport lands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Glyph {
    Start,
    Back,
    Play,
    Pause,
    Forward,
    End,
    ChevronLeft,
    ChevronRight,
    Stock,
}

/// Paints a glyph into a square.
pub fn paint_glyph(p: &egui::Painter, r: Rect, g: Glyph, c: Color32) {
    let s = r.width().min(r.height()) * 0.5;
    let o = r.center();
    let at = |x: f32, y: f32| pos2(o.x + x * s, o.y + y * s);
    let tri = |p: &egui::Painter, pts: [(f32, f32); 3]| {
        p.add(egui::Shape::convex_polygon(
            pts.iter().map(|q| at(q.0, q.1)).collect(),
            c,
            Stroke::NONE,
        ));
    };
    let bar = |p: &egui::Painter, x: f32| {
        p.rect_filled(
            Rect::from_min_max(at(x - 0.09, -0.55), at(x + 0.09, 0.55)),
            0.5,
            c,
        );
    };
    let line = Stroke::new(1.6, c);
    match g {
        Glyph::Play => tri(p, [(-0.4, -0.6), (-0.4, 0.6), (0.6, 0.0)]),
        Glyph::Pause => {
            bar(p, -0.25);
            bar(p, 0.25);
        }
        Glyph::Back => {
            tri(p, [(0.05, -0.55), (0.05, 0.55), (-0.6, 0.0)]);
            tri(p, [(0.6, -0.55), (0.6, 0.55), (0.05, 0.0)]);
        }
        Glyph::Forward => {
            tri(p, [(-0.05, -0.55), (-0.05, 0.55), (0.6, 0.0)]);
            tri(p, [(-0.6, -0.55), (-0.6, 0.55), (-0.05, 0.0)]);
        }
        Glyph::Start => {
            bar(p, -0.5);
            tri(p, [(0.5, -0.55), (0.5, 0.55), (-0.35, 0.0)]);
        }
        Glyph::End => {
            bar(p, 0.5);
            tri(p, [(-0.5, -0.55), (-0.5, 0.55), (0.35, 0.0)]);
        }
        Glyph::ChevronLeft => {
            p.line(vec![at(0.2, -0.45), at(-0.2, 0.0), at(0.2, 0.45)], line);
        }
        Glyph::ChevronRight => {
            p.line(vec![at(-0.2, -0.45), at(0.2, 0.0), at(-0.2, 0.45)], line);
        }
        Glyph::Stock => {
            // a block with a step cut from its top: the stock as an op leaves it
            p.line(
                vec![
                    at(-0.6, 0.5),
                    at(-0.6, -0.5),
                    at(0.0, -0.5),
                    at(0.0, -0.05),
                    at(0.6, -0.05),
                    at(0.6, 0.5),
                    at(-0.6, 0.5),
                ],
                line,
            );
        }
    }
}

/// A flat square button with a glyph and a tooltip (shown off, with the reason, when disabled).
pub fn glyph_button(
    ui: &mut Ui,
    t: &crate::shell::theme::Theme,
    id: &str,
    g: Glyph,
    tip: &str,
    on: bool,
) -> egui::Response {
    let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
    let resp = ui.interact(
        r,
        ui.scope_id().with(id),
        if on { Sense::click() } else { Sense::hover() },
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, on, tip));
    if on && (resp.hovered() || resp.has_focus()) {
        ui.painter().rect_filled(r, 4.0, t.hover);
    }
    let c = if on { t.ink } else { t.ink2.gamma_multiply(0.5) };
    paint_glyph(ui.painter(), r.shrink(5.0), g, c);
    resp.on_hover_text(tip)
}

/// A duration in words: "45 s", "1 min 20 s", "2 h 05 min".
pub fn duration(secs: f64) -> String {
    let s = secs.max(0.0);
    if s < 60.0 {
        if s < 10.0 {
            format!("{s:.1} s")
        } else {
            format!("{:.0} s", s.round())
        }
    } else if s < 3600.0 {
        let m = (s / 60.0).floor();
        format!("{m:.0} min {:02.0} s", (s - 60.0 * m).floor())
    } else {
        let h = (s / 3600.0).floor();
        format!("{h:.0} h {:02.0} min", ((s - 3600.0 * h) / 60.0).floor())
    }
}
