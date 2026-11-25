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
    /// The operator's view while the posted code plays on a machine: the camera's target held
    /// fixed in the machine's base frame (so the table and the part move under a still head, as
    /// they do in the shop), carried into the part's frame each frame; re-anchored where the
    /// person pans. Off: the part stays still and the machine moves about it.
    pub shop_eye: bool,
    /// The finer stock about the tool (`Sim::detail_at`), in the material's colours.
    pub detail: bool,
    /// (the target in the machine's base frame, the target last set in the part's frame).
    pub eye_anchor: Option<([f64; 3], [f32; 3])>,
    /// Cutting soft jaws: the part they will hold drawn faint where it will sit (off: it is not
    /// there while they are cut).
    pub jaw_part: bool,
    /// The tool's label shown while paused without the pointer on the tool (review shots).
    pub label_always: bool,
    pub path_width: f32,
    pub tab: Tab,
    /// The move list's selection (a piece), drawn wider.
    pub picked: Option<usize>,
    /// The move list scrolls to the piece playing once.
    scroll_to: Option<usize>,
    /// The panel's width; folded to its tabs.
    pub folded: bool,
    /// Drawn this frame by the environment's docked properties (`Shell::camsim_properties`):
    /// the floating panel stands down.
    pub hosted: bool,
    /// Docked last frame: the panel is in the environment's properties column, not over the view.
    pub docked: bool,
    /// The drawn tool's turn about its axis, radians (advanced while playing).
    pub spin: f64,
    /// Each op's cycle time on the machine (the workspace's run, forge-cam-quality: the lower
    /// bound and the estimate), by name; set by the host as it draws the panel.
    pub cycles: Vec<(String, (f64, f64))>,
    /// The workspace's word on each op (name, the verdict's words, its colour) and its notes
    /// (what is left over, the run's warnings), for the Checks tab; set by the host.
    pub op_checks: Vec<(String, String, egui::Color32)>,
    pub check_notes: Vec<String>,
    /// The place picked in the Checks tab (a marker's index): its kind's row opens to it, and it
    /// is highlighted there; a click on a place picks it.
    pub issue: Option<usize>,
    /// Picked from outside the tab (a command, the view): its row is opened and scrolled to once.
    pub issue_new: bool,
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
            shop_eye: true,
            detail: true,
            eye_anchor: None,
            jaw_part: false,
            label_always: false,
            path_width: 2.0,
            tab: Tab::Program,
            picked: None,
            scroll_to: None,
            folded: false,
            all_errors: false,
            selected_op: None,
            spin: 0.0,
            hosted: false,
            docked: false,
            cycles: vec![],
            op_checks: vec![],
            check_notes: vec![],
            issue: None,
            issue_new: false,
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
        // the flutes turn with the spindle of the op playing (slowed to read as turning)
        let rpm = self
            .sim
            .tl
            .op_at(self.t)
            .and_then(|o| self.sim.cam.ops.get(o))
            .and_then(|o| o.feeds.as_ref())
            .map_or(0.0, |f| f.rpm);
        let turns = crate::camsim::tools::turns_per_s(rpm, self.speed, true);
        self.spin = (self.spin + std::f64::consts::TAU * turns * dt).rem_euclid(std::f64::consts::TAU);
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

/// A marker's colour: a failure red, material left over amber, stock left by the plan neutral.
fn marker_colour(m: &Marker) -> Color32 {
    if m.severe() {
        srgb(look::FAIL)
    } else if m.planned {
        srgb(look::NO_VALUE)
    } else {
        srgb(look::LEFTOVER)
    }
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

/// How much of the view's right the panel takes: none when docked in the environment's
/// properties column, a strip when folded.
fn covered_w(st: &CamSimUi) -> f32 {
    if st.docked {
        -8.0
    } else if st.folded {
        36.0
    } else {
        PANEL_W
    }
}

impl Shell {
    /// The simulation's panel drawn in an environment's docked properties column `r` (the
    /// Manufacture environment hosts it as its Simulation section): while it is, the panel no
    /// longer floats over the view. False if there is no simulation to show.
    pub fn camsim_properties(&mut self, ui: &mut Ui, r: Rect) -> bool {
        let Some(mut st) = self.camsim.take() else {
            return false;
        };
        let t = self.theme;
        st.hosted = true;
        st.folded = false;
        if let Some(w) = self.cam.as_ref() {
            (st.op_checks, st.check_notes) = crate::shell::cam::check_lines(w, &t);
        }
        st.cycles = self
            .cam
            .as_ref()
            .and_then(|w| w.run.as_ref())
            .map(|r| {
                r.ops
                    .iter()
                    .filter_map(|o| Some((o.name.clone(), r.cycle([o.name.as_str()])?)))
                    .collect()
            })
            .unwrap_or_default();
        ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
            ui.set_width(r.width());
            camsim_panel(ui, &mut st, &t, r.width());
        });
        self.camsim = Some(st);
        true
    }

    /// The part of view `v` the simulation's panel (right) and timeline (bottom) leave free, for
    /// the workspace's windows to keep inside.
    pub(crate) fn camsim_free(&self, v: Rect) -> Rect {
        let Some(st) = self.camsim.as_deref() else {
            return v;
        };
        // a machining document is in Manufacture, whose properties column hosts the panel
        // (outside `v`), from before the column has first drawn it
        let panel_w = if self.cam.is_some() { -8.0 } else { covered_w(st) };
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
        // hosted by the environment's properties this frame (drawn before this): docked
        st.docked = std::mem::take(&mut st.hosted);
        st.keys(ctx);
        // the timeline: across the view's bottom, short of the panel
        let panel_w = covered_w(&st);
        let extra = 18.0 * machine_notes(&st).len() as f32;
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
        // the panel: down the view's right, unless the environment docks it
        if !st.docked {
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
                            camsim_panel(ui, &mut st, &t, pr.width() - 16.0)
                        });
                });
            self.view_blocks.push(r.response.rect);
        }
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
                let c = marker_colour(m);
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
                    jump = Some((k, m.t));
                }
                self.view_blocks.push(hit);
            }
            // a place clicked in the view: picked in the Checks tab too
            if let Some((k, tm)) = jump {
                st.issue = Some(k);
                st.issue_new = true;
                st.jump(tm);
            }
            // the tool and its holder in words, a leader label beside the tool's top (where the
            // holder meets it), out to the side with more room; the records it was drawn from
            // (keys, sources, what is left out) are its hover. Only while the pointer is on the
            // tool and the playback is still: never a label travelling with the tool (David);
            // the status line names the tool at all times
            let on_tool = |p: [f64; 3], reach: f64, top: f64| {
                let hover = ctx.pointer_hover_pos()?;
                let a = to_px([p[0] - reach, p[1], p[2]])?;
                let b = to_px([p[0] + reach, p[1], p[2] + top])?;
                let c = to_px([p[0], p[1], p[2] + top])?;
                let d = to_px([p[0], p[1], p[2]])?;
                let r = Rect::from_points(&[a, b, c, d]).expand(6.0);
                Some(r.contains(hover))
            };
            if st.tool
                && !st.playing
                && let Some((tool, p, _)) = st.tool_now()
                && let Some(lk) = st.sim.looks.get(tool)
                && let Some((at, note)) = lk.note.as_ref()
                && let Some(tu) = st.sim.cam.sim_tools().get(tool)
                && (st.label_always || on_tool(p, lk.reach, at[1] + 40.0) == Some(true))
            {
                let (z, r) = (p[2] + at[1], at[0].max(0.5 * tu.tool.dc));
                // the leader runs out past the widest of the drawing (the nut, the flange)
                let out = |side: f64| to_px([p[0] + side * lk.reach, p[1], z]);
                let left = to_px([p[0] - r, p[1], z]);
                let right = to_px([p[0] + r, p[1], z]);
                if let (Some(l), Some(rt)) = (left, right) {
                    let (l, rt) = if l.x <= rt.x { (l, rt) } else { (rt, l) };
                    let (far_l, far_r) = match (out(-1.0), out(1.0)) {
                        (Some(a), Some(b)) => (a.x.min(b.x).min(l.x), a.x.max(b.x).max(rt.x)),
                        _ => (l.x, rt.x),
                    };
                    let panel_w = if self.cam.is_some() { -8.0 } else { covered_w(&st) };
                    // clear of the view's own tool strip along its right edge (52 px) too
                    let free = Rect::from_min_max(
                        v.min,
                        pos2(v.right() - panel_w - 60.0, v.bottom() - TIMELINE_H - 16.0),
                    );
                    let room_r = free.right() - far_r;
                    let room_l = far_l - free.left();
                    let room = room_r.max(room_l) - 26.0;
                    // the whole label where it fits beside the tool, else the tool's words alone,
                    // else none (never over the tool: the hover has it all the same)
                    let font = FontId::proportional(12.0);
                    let galley = [lk.label.clone(), crate::camsim::tools::tool_words(tu)]
                        .into_iter()
                        .map(|w| painter.layout_no_wrap(w, font.clone(), t.ink))
                        .find(|g| g.size().x <= room);
                    if let Some(galley) = galley {
                        let (wd, ht) = (galley.size().x, galley.size().y);
                        let to_right = room_r >= wd + 26.0 && (room_r >= room_l || room_l < wd + 26.0);
                        let (a, end, rect) = if to_right {
                            // level with the holder's foot: under the nut, not across it
                            let end = pos2(far_r + 16.0, rt.y);
                            (
                                rt,
                                end,
                                Rect::from_min_size(pos2(end.x + 4.0, end.y - 0.5 * ht), galley.size()),
                            )
                        } else {
                            let end = pos2(far_l - 16.0, l.y);
                            (
                                l,
                                end,
                                Rect::from_min_size(pos2(end.x - 4.0 - wd, end.y - 0.5 * ht), galley.size()),
                            )
                        };
                        // kept inside the free view
                        let dx = (free.left() + 6.0 - rect.left()).max(0.0)
                            - (rect.right() - free.right() + 6.0).max(0.0);
                        let dy = (free.top() + 6.0 - rect.top()).max(0.0)
                            - (rect.bottom() - free.bottom() + 6.0).max(0.0);
                        let rect = rect.translate(vec2(dx, dy));
                        let end = end + vec2(dx, dy);
                        if v.contains(a) {
                            painter.line_segment([a, end], Stroke::new(1.0, t.ink2));
                            painter.circle_filled(a, 2.0, t.ink2);
                            painter.rect_filled(
                                rect.expand2(vec2(5.0, 3.0)),
                                4.0,
                                t.surface.gamma_multiply(0.85),
                            );
                            painter.galley(rect.min, galley, t.ink);
                            let hit = rect.expand2(vec2(5.0, 3.0));
                            egui::Area::new(egui::Id::unique("camsim:toolnote"))
                                .fixed_pos(hit.min)
                                .order(egui::Order::Background)
                                .fade_in(false)
                                .show(ctx, |ui| {
                                    let (_, r) = ui.allocate_exact_size(hit.size(), Sense::hover());
                                    r.on_hover_text(note.as_str());
                                });
                            self.view_blocks.push(hit);
                            self.places.push(("camsim:toolnote".into(), hit));
                        }
                    }
                }
            }
        }
        if st.playing {
            ctx.request_repaint();
        }
        self.camsim = Some(st);
        true
    }
}

/// What the view of a machine run says under the timeline, a line each: the placement it
/// assumed (the setup gives no `at:`), a part held by nothing (no workholding: drawn
/// unclamped, never a vise made up for it), and a workholding whose table plane is unknown.
pub fn machine_notes(st: &CamSimUi) -> Vec<String> {
    let Some(mv) = st.sim.machine() else {
        return vec![];
    };
    let mut out = vec![];
    if !mv.run.wcs_given {
        out.push(format!(
            "Posted code on {}: placement assumed ({} centred on the table datum; the setup gives no `at:`)",
            mv.run.def.name,
            if mv.run.holding.is_some() {
                "the workholding"
            } else {
                "the stock"
            }
        ));
    }
    if mv.run.holding.is_none() {
        out.push("No workholding defined: the part is shown unclamped".into());
    } else if mv.run.seat.is_none() {
        out.push(
            "This workholding's record has no body down to the table: where it stands on the table is unknown"
                .into(),
        );
    }
    out
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
        if b(ui, "camsim:back", Glyph::Back, "Back a move (Left arrow)") {
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
        if b(ui, "camsim:fwd", Glyph::Forward, "On a move (Right arrow)") {
            let to = st.sim.tl.step(st.t, false);
            st.jump(to);
        }
        if b(ui, "camsim:end", Glyph::End, "To the end (End)") {
            st.jump(total);
        }
        ui.separator();
        // a narrow view (a small window with the properties column open): the time now, the
        // total on hover, and the speed without its label, so the row stays in the bar
        let compact = w < 440.0;
        let now = if compact {
            clock(st.t)
        } else {
            format!("{} / {}", clock(st.t), clock(total))
        };
        // the playback clock runs at feed time (feeds and rapids at their programmed rates): not
        // the cycle time, which the Program tab gives (the bound and the estimate)
        // with the run's cycle time beside it where every op was timed on the machine, so the two
        // totals are read together (the cycle time is the longer: it adds what feed time leaves out)
        let timed: Option<(f64, f64)> = (!st.cycles.is_empty()
            && st.sim.tl.ops.iter().all(|o| st.cycles.iter().any(|c| c.0 == o.name)))
        .then(|| {
            st.cycles
                .iter()
                .fold((0.0, 0.0), |a, c| (a.0 + c.1.0, a.1 + c.1.1))
        });
        let cycle = match timed {
            Some((lo, est)) => format!(
                "The cycle time on the machine is \u{2265} {} \u{b7} est. {} (the Program tab, per op).",
                mmss(lo),
                mmss(est)
            ),
            None => "The cycle time on the machine is in the Program tab, per op.".to_string(),
        };
        let tip = format!(
            "{} of {} at feed time: feeds at their programmed rates, rapids at the machine's rapid rates, each tool change at the machine's change time (up, swap, across and down). It leaves out dwells and acceleration, so it is shorter than the cycle time. {cycle}",
            clock(st.t),
            clock(total)
        );
        ui.label(egui::RichText::new(now).monospace().color(t.ink)).on_hover_text(&tip);
        if !compact {
            ui.label(egui::RichText::new("feed time").size(11.0).color(t.ink2)).on_hover_text(&tip);
        }
        ui.separator();
        if !compact {
            ui.label(egui::RichText::new("Speed").color(t.ink2));
        }
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
            // the tool in people's words (the label never travels with the tool: it is here)
            let named = tu.map_or(String::new(), |u| {
                format!("T{} · {}", u.t, crate::camsim::tools::tool_words(u))
            });
            let s = match st.sim.tl.changes.iter().find(|c| st.t >= c.t0 && st.t < c.t1) {
                Some(c) => {
                    let to = st.sim.cam.sim_tools().get(c.to as usize).map_or(String::new(), |u| {
                        format!("T{} · {}", u.t, crate::camsim::tools::tool_words(u))
                    });
                    format!(
                        "Tool change to {to} · {}",
                        if c.cited {
                            format!("{:.1} s, the machine's", c.t1 - c.t0)
                        } else {
                            "the machine record gives no change time: a pause shown".into()
                        }
                    )
                }
                None => format!(
                    "{} · {named} · {} · F {:.0} mm/min · X {:.3}  Y {:.3}  Z {:.3} mm",
                    op.name,
                    g.kind.name(),
                    g.f,
                    p[0],
                    p[1],
                    p[2]
                ),
            };
            ui.add(egui::Label::new(egui::RichText::new(&s).color(t.ink2)).truncate())
                .on_hover_text(&s);
        }
    });
    // a machine run's placement, while the document gives none; a part held by nothing
    for n in machine_notes(st) {
        ui.label(egui::RichText::new(n).size(11.0).color(t.warn));
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
    // tool changes: a tick each, the new tool's number beside it where there is room (not over
    // the one before: the status line names the tool during each change)
    let mut room = f32::NEG_INFINITY;
    for c in &st.sim.tl.changes {
        let x = x_of(c.t0);
        painter.line_segment(
            [pos2(x, band.top()), pos2(x, band.bottom())],
            Stroke::new(1.5, Color32::WHITE),
        );
        if x < room {
            continue;
        }
        room = x + 26.0;
        if let Some(u) = st.sim.cam.sim_tools().get(c.to as usize) {
            painter.text(
                pos2(x + 2.0, band.bottom() - 1.0),
                Align2::LEFT_BOTTOM,
                format!("T{}", u.t),
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }
    // markers
    let mut hover: Option<&Marker> = None;
    let pointer = resp.hover_pos();
    for m in &st.sim.markers {
        let x = x_of(m.t);
        let c = marker_colour(m);
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

/// The panel: a fold button and the tabs as one control, then the tab's body. Public for an
/// environment that docks it (`Shell::camsim_properties`).
pub fn camsim_panel(ui: &mut Ui, st: &mut CamSimUi, t: &crate::shell::theme::Theme, w: f32) {
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
    let (c, s) = if !cam.ok && !cam.errors.is_empty() {
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
    } else if !cam.ok {
        (t.error, "Not certified".to_string())
    } else {
        (t.success, String::new())
    };
    if s.is_empty() {
        // certified: the workspace's words (a rule left unchecked named, its meaning on hover)
        let (words, hover, amber) = crate::shell::cam::certified_words(cam.certification());
        let r = ui.colored_label(
            if amber {
                t.warn
            } else if hover.is_some() {
                t.ink2
            } else {
                c
            },
            words,
        );
        if let Some(h) = hover {
            r.on_hover_text(h);
        }
    } else {
        ui.colored_label(c, s);
    }
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
        let what = crate::camsim::tidy(&what);
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
                "{} lines of {} code · {} motions · {}",
                r.text.lines().count(),
                r.dialect,
                r.tl.segments.len(),
                crate::shell::cam::cycle_words(crate::shell::cam::run::timeline_cycle(&r.tl))
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
                            // its time, in a right-aligned column: the cycle time on the
                            // machine (the bound and the estimate, together) once timed, else
                            // the feed time the playback runs at
                            let cycle = st.cycles.iter().find(|c| c.0 == out.name).map(|c| c.1);
                            let (words, tip) = match cycle {
                                Some((lo, est)) => (
                                    format!("\u{2265} {} \u{b7} est. {}", mmss(lo), mmss(est)),
                                    crate::shell::cam::CYCLE_TIP.to_string(),
                                ),
                                // not timed on the machine (its setup's program was not
                                // posted): one format in the column, the feed time on hover
                                None => (
                                    "\u{2014}".to_string(),
                                    format!(
                                        "Not timed on the machine: this setup's program was not posted (an op is refused). Feed time {}: feeds at their programmed rates, rapids at the machine's",
                                        duration(op.t1 - op.t0)
                                    ),
                                ),
                            };
                            let g = ui.painter().layout_no_wrap(words.clone(), FontId::proportional(12.0), t.ink2);
                            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x.max(78.0), 18.0), Sense::hover());
                            ui.painter().text(
                                pos2(r.right(), r.center().y),
                                Align2::RIGHT_CENTER,
                                words,
                                FontId::proportional(12.0),
                                t.ink2,
                            );
                            resp.on_hover_text(tip);
                        });
                    });
                    if let Some(r) = &out.refusal {
                        // in words: the first failed check's sentence, else the refusal tidied;
                        // the run's own text on hover
                        let said = out
                            .checks
                            .iter()
                            .find(|c| c.verdict != "pass")
                            .map(crate::camsim::check_sentence)
                            .unwrap_or_else(|| crate::camsim::tidy(r));
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(32.0);
                            ui.add(egui::Label::new(egui::RichText::new(said).size(11.0).color(t.error)).wrap())
                                .on_hover_text(r);
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
    if ui
        .checkbox(&mut st.detail, "Fine detail at the tool")
        .on_hover_text("The stock about the tool on a grid of about 0.03 mm, so a ball's cusps and the stepover show; the rest of the stock on the coarser grid. Display only.")
        .changed()
    {
        st.drawn_key = None;
    }
    if st.sim.machine().is_some() {
        ui.checkbox(&mut st.shop_eye, "Operator's view")
            .on_hover_text("The camera stays put in the machine: the table and the part move under the head, as in the shop. Off: the part stays still and the machine moves about it.");
    }
    if st.sim.setups.iter().any(|s| s.jaws_for.is_some()) {
        ui.checkbox(&mut st.jaw_part, "Finished part in jaws")
            .on_hover_text("Shows the part faint where it will sit, and seated once the jaws are cut");
    }
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
                (Kind::Rapid | Kind::Change, _) => look::RAPID,
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
    let (line, failed) = checked_line(&st.sim.markers, cam.leftover_done);
    ui.label(
        egui::RichText::new(line)
            .size(11.0)
            .color(if failed { t.error } else { t.ink2 }),
    )
    .on_hover_text(format!(
        "{pairs} tool–face pairs certified by the checker: G1 gouge, G2 holder and fixture, G3 rapids; leftover (G4) {}",
        if cam.leftover_done { "decided exactly" } else { "not decided" }
    ));
    // each op's verdict (the workspace's: certified, refused, not posted), then the notes
    if !st.op_checks.is_empty() {
        heading(ui, t, "Ops");
        for (name, words, c) in &st.op_checks {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(name).size(12.0).color(t.ink));
                // right-aligned, as wide as the Program tab's time column (78 px at least)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.set_min_width(78.0);
                    ui.label(egui::RichText::new(words).size(12.0).color(*c));
                });
            });
        }
    }
    for n in &st.check_notes {
        let words = note_words(n);
        let r = ui.label(egui::RichText::new(&words).size(11.0).color(t.warn));
        if words != *n {
            r.on_hover_text(n.as_str());
        }
    }
    // the markers by kind: an issue's kind (a failed check's rule), material left over, and the
    // stock left by the plan apart; each kind one row, opened to its places
    let kind = |m: &Marker| -> (u8, String) {
        if m.severe() {
            (0, m.rule.clone())
        } else if m.planned {
            (2, "planned".into())
        } else {
            (1, "leftover".into())
        }
    };
    let mut groups: Vec<((u8, String), Vec<usize>)> = vec![];
    for (k, m) in st.sim.markers.iter().enumerate() {
        let key = kind(m);
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(k),
            None => groups.push((key, vec![k])),
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    let issues: usize = groups.iter().filter(|g| g.0.0 < 2).map(|g| g.1.len()).sum();
    if issues == 0 {
        // one line, in the header's colour, and nothing more to say
        heading(ui, t, "No issues");
    } else {
        heading(ui, t, &format!("Issues ({issues})"));
    }
    let mut jump = None;
    let mut pick = None;
    egui::ScrollArea::vertical()
        .id_salt("camsim:checks")
        .show(ui, |ui| {
            for ((rank, rule), ks) in &groups {
                let ms: Vec<&Marker> = ks.iter().map(|&k| &st.sim.markers[k]).collect();
                let n = ms.len();
                let thick = ms
                    .iter()
                    .filter_map(|m| m.region.map(|r| r.2 - r.1))
                    .fold(0.0, f64::max);
                let ops: Vec<String> = {
                    let mut v: Vec<String> = ms
                        .iter()
                        .filter_map(|m| m.op.and_then(|o| st.sim.tl.ops.get(o)).map(|o| o.name.clone()))
                        .collect();
                    v.dedup();
                    v
                };
                let setups: Vec<&str> = {
                    let mut v: Vec<&str> = ms
                        .iter()
                        .filter_map(|m| st.sim.setups.get(m.setup).map(|s| s.name.as_str()))
                        .collect();
                    v.dedup();
                    v
                };
                // the tool, where the kind's places name one: once in the heading if all share it
                let one_tool = {
                    let mut v: Vec<&str> = ms.iter().map(|m| m.tool.as_str()).collect();
                    v.dedup();
                    (v.len() == 1 && !v[0].is_empty()).then(|| v[0].to_string())
                };
                let plural = |w: &str| if n == 1 { w.to_string() } else { format!("{w}s") };
                let title = match *rank {
                    2 => format!(
                        "Stock left as planned \u{b7} {n} {} under the part \u{b7} up to {thick:.1} mm thick",
                        plural("region")
                    ),
                    1 => format!(
                        "Material left on the part \u{b7} {n} {} \u{b7} up to {thick:.1} mm thick \u{b7} in {}",
                        plural("region"),
                        setups.join(", ")
                    ),
                    _ => format!(
                        "{} \u{b7} {n} \u{b7} in {}{}",
                        rule_title(rule),
                        if ops.is_empty() { setups.join(", ") } else { ops.join(", ") },
                        one_tool.as_ref().map_or(String::new(), |w| format!(" \u{b7} {w}"))
                    ),
                };
                let c = marker_colour(ms[0]);
                let has_pick = st.issue.is_some_and(|p| ks.contains(&p));
                egui::CollapsingHeader::new(egui::RichText::new(title).size(12.0).color(c))
                    .id_salt(("camsim:group", *rank, rule.as_str()))
                    .default_open(*rank == 0 && n <= 4)
                    .open(has_pick.then_some(true).filter(|_| st.issue_new))
                    .show(ui, |ui| {
                        // a long kind lists 8 places (around the picked one) until all are asked for
                        let all_id = ui.scope_id().with("all");
                        let all = ui.data(|d| d.get_temp::<bool>(all_id).unwrap_or(false));
                        const SHOWN: usize = 8;
                        let at = ks.iter().position(|&k| st.issue == Some(k)).unwrap_or(0);
                        let from = if all || n <= SHOWN {
                            0
                        } else {
                            at.saturating_sub(SHOWN / 2).min(n - SHOWN)
                        };
                        let to = if all { n } else { (from + SHOWN).min(n) };
                        for (&k, m) in ks.iter().zip(&ms).skip(from).take(to - from) {
                            let picked = st.issue == Some(k);
                            let mut text = egui::RichText::new(if m.tool.is_empty() || one_tool.is_some() {
                                format!("{} \u{b7} {}", clock(m.t), m.row)
                            } else {
                                format!("{} \u{b7} {} \u{b7} {}", clock(m.t), m.tool, m.row)
                            })
                                .size(11.0)
                                .color(if *rank == 2 { t.ink2 } else { c });
                            if picked {
                                text = text.background_color(look_selected(t));
                            }
                            let r = ui.add(egui::Label::new(text).sense(Sense::click()));
                            if picked && st.issue_new {
                                r.scroll_to_me(Some(egui::Align::Center));
                            }
                            if r.on_hover_text(format!("{}\nGo there", m.text)).clicked() {
                                pick = Some(k);
                                jump = Some(m.t);
                            }
                        }
                        if to - from < n {
                            let more = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("Show all {n}"))
                                        .size(11.0)
                                        .color(t.accent),
                                )
                                .sense(Sense::click()),
                            );
                            if more.clicked() {
                                ui.data_mut(|d| d.insert_temp(all_id, true));
                            }
                        }
                    });
            }
        });
    st.issue_new = false;
    if let Some(k) = pick {
        st.issue = Some(k);
    }
    if let Some(tm) = jump {
        st.jump(tm);
    }
}

/// A picked row's background: the selection's blue, faint under the row's own colour.
fn look_selected(t: &crate::shell::theme::Theme) -> Color32 {
    t.accent.gamma_multiply(0.25)
}

/// The Checks tab's first line, in words: what every move was checked for and what it found
/// (each failure in its issue kind's words), and whether the leftover was measured; true if any
/// check failed. The checker's pair count and rule codes are its hover.
pub fn checked_line(markers: &[Marker], leftover_done: bool) -> (String, bool) {
    // each failed kind counted, said by its title ("24 rapids into the stock")
    let said = |rules: &[&str], clear: &str| -> (String, usize) {
        let mut parts = vec![];
        let mut all = 0;
        for r in rules {
            let k = markers.iter().filter(|m| m.rule == *r).count();
            if k > 0 {
                all += k;
                let title = rule_title(r).to_lowercase();
                let title = if k == 1 { singular(&title) } else { title };
                parts.push(format!("{k} {title}"));
            }
        }
        if parts.is_empty() {
            (clear.to_string(), 0)
        } else {
            (parts.join(", "), all)
        }
    };
    let (g, gn) = said(&["gouge"], "no gouges");
    let (h, hn) = said(
        &[
            "holder_collision",
            "shank_collision",
            "fixture_collision",
            "holder_into_stock",
            "holder_into_stock_grid",
            "machine_collision",
            "workholding_off_table",
        ],
        "holder and fixture clear",
    );
    let (r, rn) = said(
        &[
            "rapid_into_stock",
            "rapid_into_stock_grid",
            "rapid_into_part",
            "rapid_into_workholding",
            "rapid_clear",
        ],
        "rapids clear",
    );
    let leftover = if leftover_done {
        "Leftover stock measured exactly."
    } else {
        "Leftover stock not measured."
    };
    (
        format!("Every move checked: {g}, {h}, {r}. {leftover}"),
        gn + hn + rn > 0,
    )
}

/// A kind's title for one of it: "rapids into the stock" → "rapid into the stock".
fn singular(title: &str) -> String {
    let (head, rest) = title.split_once(' ').unwrap_or((title, ""));
    let head = head.strip_suffix('s').unwrap_or(head);
    if rest.is_empty() {
        head.to_string()
    } else {
        format!("{head} {rest}")
    }
}

/// A run's warning in people's words (UX-LEAD: what is wrong and what to do; the syntax and the
/// rule's code are its hover): a setup not placed on the machine, a part whose material is only
/// a group. Else the warning as it is.
pub fn note_words(raw: &str) -> String {
    if let Some(i) = raw.find("setup `")
        && raw.contains("is not placed")
    {
        let name = raw[i + 7..].split('`').next().unwrap_or("");
        return format!(
            "Setup {name} isn't placed on the machine. Place it to check machine travel, or allow uncertified output in NC Programs."
        );
    }
    if let Some(i) = raw.find("names only a material group (") {
        // the group by its name in the tool library's cutting data, never the bare key
        let g = raw[i + 29..].split(')').next().unwrap_or("");
        static LIB: std::sync::OnceLock<forge_tooling::Library> = std::sync::OnceLock::new();
        let lib = LIB.get_or_init(forge_tooling::Library::builtin);
        let group = lib
            .group(g)
            .map(|gr| format!(": {}", gr.name))
            .unwrap_or_default();
        return format!(
            "The part's material is only a group{group}. Pick the alloy for realistic feeds and tool life."
        );
    }
    raw.to_string()
}

/// A check's rule as a group's title.
pub fn rule_title(rule: &str) -> String {
    match rule {
        "holder_collision" => "Holder hits".into(),
        "fixture_collision" => "Fixture hits".into(),
        "probe_clear" => "Probe too close".into(),
        "containment" => "Outside the stock".into(),
        "shank_collision" => "Shank hits".into(),
        "gouge" => "Gouges".into(),
        "rapid_clear" => "Rapids too close".into(),
        "rapid_into_stock" => "Rapids into the stock".into(),
        "rapid_into_part" => "Rapids into the part".into(),
        "rapid_into_workholding" => "Rapids into the workholding".into(),
        "holder_into_stock" => "Holder into the stock".into(),
        // the display grid's, not proven: said so in their heading
        "rapid_into_stock_grid" => "Rapids into the stock (approximate)".into(),
        "holder_into_stock_grid" => "Holder into the stock (approximate)".into(),
        "machine_collision" => "Machine collisions".into(),
        "workholding_off_table" => "Workholding off the table".into(),
        "refused" => "Refused ops".into(),
        "overtravel" => "Beyond the machine's travel".into(),
        r => {
            let w = r.replace('_', " ");
            w.get(0..1).map_or(w.clone(), |f| f.to_uppercase() + &w[1..])
        }
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
/// m:ss, whole seconds rounded.
fn mmss(secs: f64) -> String {
    let s = secs.round();
    format!("{}:{:02}", (s / 60.0).floor(), (s % 60.0) as u32)
}

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
