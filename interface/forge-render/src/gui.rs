//! Chrome and viewport in one frame: the shell over the part's picture, the live worker behind
//! them. The host (a window, or `Headless` here) gives input and a target. Nothing here runs
//! unless the host asks for a frame, and the host asks only when something changed.

use crate::camera::Camera;
use crate::live::{self, Out};
use crate::nav::Nav;
use crate::render::{self, Entity, Mark, Offscreen, Overlay, Renderer, part_info};
use crate::scene::{Motion, Placement, Scene};
use crate::shell::SketchChild;
use crate::shell::dialog::{Dialog, DialogTarget};
use crate::shell::dim::DimEdit;
use crate::shell::sketching::{SkTool, SketchFor, SketchMode, project_refs};
use crate::shell::{Action, Shell};
use crate::viewcube;
use crate::viewer::now_ms;
use forge_agent::handles::Facts;
use forge_agent::live::{Event, Live};
use forge_agent::short;
use forge_agent::tools::{Call, Part, Target};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};

#[path = "gui_asm.rs"]
mod gui_asm;
#[path = "gui_explode.rs"]
mod gui_explode;
#[path = "gui_inspect.rs"]
pub mod gui_inspect;
#[path = "gui_overlay.rs"]
mod gui_overlay;
#[path = "gui_sheet.rs"]
mod gui_sheet;
#[path = "gui_thermo.rs"]
mod gui_thermo;

pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// The formats a target can be: blending in sRGB, as a browser's. The viewport's picture is sampled
/// as stored.
pub const HEADLESS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct Loaded {
    /// Its file, if it has one.
    file: Option<String>,
    name: String,
    /// The .fc part its worker acts on; none for a Forge part, which `fg` builds.
    part: Option<Part>,
    /// Nothing in it yet: the view holds the origin's planes whole.
    empty: bool,
    scene: Scene,
    /// Its pipelines take tens of milliseconds to make: on the loader's thread, never in a frame.
    renderer: Renderer,
    /// It built but can't be shown, and why.
    unshown: Option<String>,
}

/// The part in view.
pub struct View {
    pub renderer: Renderer,
    pub scene: Scene,
    pub nav: Nav,
    target: Offscreen,
    tex: egui::TextureId,
    size: (u32, u32),
    /// Its picture is out of date.
    dirty: bool,
    /// The .fc worker; none for a Forge (.fg) part, which `fg` builds.
    pub live: Option<Live<Out>>,
    pub motion: Option<Motion>,
    pub facts: Facts,
    /// What the view's click had selected, to see a new click by.
    clicked: Vec<(u32, Entity)>,
    clicks: u32,
    /// What of the origin was drawn last: its picture changes with this.
    origin_was: Vec<(crate::origin::Item, bool)>,
    sketches_was: Vec<(usize, bool)>,
    /// A dialog's result is what's on the screen.
    previewing: bool,
    /// A number's being dragged or has just been let go: the picture's the worker's, not the mesh's.
    pub live_drag: bool,
}

/// A frame's times, in milliseconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct Times {
    /// Of `ui`, taking in what the worker's said: a new part's mesh goes to the GPU here.
    pub events: f64,
    pub ui: f64,
    pub tess: f64,
    pub view: f64,
    pub draw: f64,
    pub view_drawn: bool,
}

pub struct FrameOut {
    pub platform: egui::PlatformOutput,
    /// What the host must do, the window's things.
    pub host: Vec<Action>,
    /// Draw again after this long; `None`: only when something happens.
    pub again_in: Option<std::time::Duration>,
    pub times: Times,
}

/// An assembly's occurrences, and each placement's mesh and solved pose.
pub type AsmPlaced = (Vec<crate::fg::Occ>, Vec<(usize, crate::scene::Rows)>);
/// A part file, and its anchors or why not.
type PartAnchors = (String, Result<Vec<String>, String>);

pub struct Gui {
    pub ctx: egui::Context,
    egui: egui_wgpu::Renderer,
    pub shell: Shell,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::AdapterInfo,
    pub view: Option<View>,
    loading: Option<Receiver<Result<Loaded, String>>>,
    waker: Waker,
    /// The view holds the pointer: a press began in it.
    captured: bool,
    over_view: bool,
    /// Where the pointer last was, in window pixels.
    pointer_px: Option<PhysicalPosition<f64>>,
    /// Tool calls made, for replay.
    pub calls: Vec<Call>,
    pixels_per_point: f32,
    /// The font atlas from before the first frame, still to go to the GPU.
    warm_textures: Option<egui::TexturesDelta>,
    /// Where the part's view is kept: none for a part with no file.
    view_file: Option<String>,
    /// The window, in device pixels.
    target_px: (u32, u32),
    tree_hover_was: Option<forge_doc::ItemId>,
    /// A feature to open once the sketch just finished is in.
    after_sketch: Option<&'static str>,
    /// When the right button went down in the view, for its menu to show if it stays down.
    mark_held: Option<f64>,
    /// A sketch's plane has been asked for.
    sketch_for: Option<(String, SketchFor, Vec<String>)>,
    /// What the `project` rows of the sketch being opened come to.
    sketch_sources: std::collections::HashMap<String, Result<Vec<forge_doc::sketch::Refd>, String>>,
    /// A Forge (.fg) part, and its first view waiting for the layout.
    pub fg: Option<crate::fg::Session>,
    fg_view: Option<Loaded>,
    /// The Forge part's last meshes, folded and (a sheet's) flat, and its sheets' DXF.
    fg_folded: Option<crate::mesh::PartMesh>,
    fg_flat: Option<crate::mesh::PartMesh>,
    pub fg_sheets: Vec<crate::fg::SheetInfo>,
    /// Each thermoform's wall-thickness map, world triangles shown while the panel asks.
    pub fg_walls: Vec<crate::thermo_view::Gfx>,
    /// What Forge names each face and edge of the part: what a pick becomes.
    pub fg_anchors: Option<Arc<crate::fg::AnchorView>>,
    /// An assembly's occurrences and their placements' poses as solved.
    pub fg_asm: Option<AsmPlaced>,
    /// A part's anchors being found, for the instance chosen.
    fg_asm_anchors: Option<Receiver<PartAnchors>>,
    /// The camera to keep when an assembly's view is made again.
    fg_keep_camera: Option<Camera>,
    /// The GPU refused something: said once, and no more views are drawn.
    gpu_failed: Arc<std::sync::Mutex<Option<String>>>,
    /// The view's switches' anchors, and what they were last drawn for.
    extras: gui_overlay::Extras,
    /// An assembly's meshes' boxes, each in its part's space.
    pub fg_asm_boxes: Vec<crate::explode::Aabb>,
    /// The assembly tools' state between frames.
    asm: gui_asm::AsmHost,
    /// The settings as last saved; none: not kept (tests, shots).
    settings_was: Option<(bool, bool)>,
    /// The exploded view: its plan, and how far apart it is now.
    pub explode: gui_explode::Explode,
    /// A Forge part's solid as it was before its eye hid it.
    body_rows: Option<crate::scene::Rows>,
    /// Measure, sections and physical properties: the exact bodies, the jobs' thread.
    inspector: gui_inspect::Inspector,
    /// The open tool's statement last shown as a preview.
    fg_previewed: Option<String>,
}

fn view_camera(center: [f32; 3], radius: f32, parallel: bool, size: (u32, u32)) -> Camera {
    let mut c = crate::nav::region_view(Camera::fit(center, radius), crate::nav::HOME_CORNER);
    if parallel {
        c.set_parallel(true, radius);
    }
    c.fit_to(radius * 1.1, size.0, size.1);
    c
}

fn name_of(path: &str) -> String {
    let n = path.rsplit('/').next().unwrap_or(path);
    if n.ends_with(".fc") || n.ends_with(".fg") {
        n.to_string()
    } else {
        format!("{n}.fc")
    }
}

impl View {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

/// The line a sketch reads the part above: its own, or where a new one would go.
fn place_of(shell: &Shell, edit: &SketchFor) -> Option<forge_doc::ItemId> {
    match edit {
        SketchFor::Line(id) | SketchFor::Inline(id, _) => Some(*id),
        SketchFor::New => {
            let (doc, end) = (shell.doc.as_ref()?, shell.end_at?);
            let mut lines = doc.features().map(|i| i.id);
            lines.find(|&i| i == end)?;
            lines.next()
        }
    }
}

fn sketch_place(shell: &Shell) -> Option<forge_doc::ItemId> {
    place_of(shell, &shell.sketch.as_ref()?.edit)
}

impl Gui {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: wgpu::AdapterInfo,
        format: wgpu::TextureFormat,
        waker: Waker,
        scale: f32,
    ) -> Gui {
        // never panic from a thread the GPU reports on: say it, and the window stays up
        let gpu_failed: Arc<std::sync::Mutex<Option<String>>> = Arc::default();
        let failed = gpu_failed.clone();
        device.on_uncaptured_error(Arc::new(move |e| {
            eprintln!("forge-app: GPU error: {e}");
            let mut f = failed.lock().expect("no panics while held");
            f.get_or_insert_with(|| e.to_string());
        }));
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::shell::theme::fonts());
        let egui = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
        // the type's first layout takes 40 ms or so: before the first frame, not in it
        let mut warm_shell = Shell::default();
        let mut warm_input = egui::RawInput::default();
        warm_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale);
        let warm = ctx.run_ui(warm_input, |ui| warm_shell.ui(ui));
        warm_shell.out.clear();
        Gui {
            ctx,
            egui,
            shell: Shell::default(),
            device,
            queue,
            adapter,
            view: None,
            loading: None,
            waker,
            captured: false,
            over_view: false,
            pointer_px: None,
            calls: Vec::new(),
            pixels_per_point: 1.0,
            warm_textures: Some(warm.textures_delta),
            view_file: None,
            target_px: (1, 1),
            tree_hover_was: None,
            sketch_for: None,
            sketch_sources: Default::default(),
            after_sketch: None,
            mark_held: None,
            fg: None,
            fg_view: None,
            fg_folded: None,
            fg_flat: None,
            fg_sheets: Vec::new(),
            fg_walls: Vec::new(),
            fg_anchors: None,
            fg_asm: None,
            fg_asm_anchors: None,
            fg_keep_camera: None,
            gpu_failed,
            extras: Default::default(),
            fg_asm_boxes: Vec::new(),
            asm: Default::default(),
            settings_was: None,
            explode: Default::default(),
            body_rows: None,
            inspector: Default::default(),
            fg_previewed: None,
        }
    }

    /// A key that means something while it's down, as the window sees it; true if it was one.
    pub fn held_key(&mut self, code: winit::keyboard::KeyCode, down: bool) -> bool {
        if code != winit::keyboard::KeyCode::F4 {
            return false;
        }
        // F4: the left button orbits while it's down
        if let Some(v) = &mut self.view {
            v.nav.orbit_held = down;
        }
        true
    }

    /// A step of a held orbit, for measuring.
    pub fn turn_view(&mut self) {
        if let Some(v) = &mut self.view {
            v.nav.camera.yaw += 0.01;
            v.dirty = true;
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading.is_some()
    }

    /// Opens a part in the background: the window never waits on the kernel.
    pub fn open(&mut self, path: &str) {
        // a drawing opens in the Drawing environment
        if path.ends_with(".fg")
            && std::fs::read_to_string(path).is_ok_and(|s| forge_lang::drawing::is_drawing(&s))
        {
            match crate::shell::drawing::DrawingEnv::open(path) {
                Ok(env) => self.shell.open_drawing(env),
                Err(e) => self.shell.status = e,
            }
            return;
        }
        if path.ends_with(".fg") {
            self.open_fg(path);
            return;
        }
        self.fg = None;
        self.fg_view = None;
        self.shell.fg = None;
        let (send, recv) = channel();
        self.loading = Some(recv);
        let (path, waker) = (path.to_string(), self.waker.clone());
        self.shell.status = format!("Opening {}", name_of(&path));
        let (device, queue, adapter) = (self.device.clone(), self.queue.clone(), self.adapter.clone());
        std::thread::spawn(move || {
            let got = (|| {
                let (mesh, _) = crate::load::open(&path, true)?;
                let part = crate::load::tool_part(&path)?;
                let parts = [mesh];
                let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
                scene.set_explode(0.0);
                let renderer = Renderer::new(
                    device,
                    queue,
                    adapter,
                    render::OFFSCREEN,
                    (16, 16),
                    &parts,
                    &scene,
                );
                let unshown = crate::load::MESH_FAILED.with(|f| f.borrow_mut().take());
                Ok(Loaded {
                    file: path.ends_with(".fc").then(|| path.clone()),
                    name: name_of(&path),
                    empty: part.model.lines.is_empty(),
                    part: Some(part),
                    scene,
                    renderer,
                    unshown,
                })
            })();
            let _ = send.send(got);
            waker();
        });
    }

    fn take_loaded(&mut self) {
        // the view takes its size from the layout: nothing to allocate until there is one
        if self.shell.view_rect.area() <= 0.0 {
            return;
        }
        let Some(r) = &self.loading else { return };
        let Ok(got) = r.try_recv() else { return };
        self.loading = None;
        let l = match got {
            Ok(l) => l,
            Err(why) => {
                self.shell.status = why;
                return;
            }
        };
        self.show_loaded(l);
    }

    /// A part's view, its worker (for .fc) and its saved orientation.
    fn show_loaded(&mut self, l: Loaded) {
        self.shell.unshown = l.unshown.is_some();
        let (scene, mut renderer) = (l.scene, l.renderer);
        let (center, radius) = scene.bounds();
        let size = self.view_px();
        eprintln!(
            "forge-app: opening {} with a view of {}x{}",
            l.name, size.0, size.1
        );
        // an empty part's view holds its planes whole
        let fit = if l.empty { radius * 1.6 } else { radius };
        let camera = view_camera(center, fit, self.shell.view.parallel, size);
        renderer.resize(size);
        self.theme_view(&mut renderer);
        let target = Offscreen::new(&self.device, size);
        let raw = raw_view(&target);
        let tex = self
            .egui
            .register_native_texture(&self.device, &raw, wgpu::FilterMode::Nearest);
        let live = l.part.map(|p| {
            let mut live = live::start(p);
            let w = self.waker.clone();
            live.on_ready(move || w());
            live
        });
        let mut nav = Nav::new(camera, size, self.pixels_per_point);
        nav.tool = self.shell.view.tool;
        if let Some(f) = &l.file {
            let path = format!("{f}.view");
            let (orient, home, eyes) = Self::load_view(&path);
            self.shell.sketch_eye = eyes;
            nav.orient = orient;
            nav.home_view = home;
            // it opens where Home is
            nav.camera = home.unwrap_or_else(|| {
                let mut c =
                    crate::nav::region_view_as(camera, orient.world(crate::nav::HOME_CORNER), &orient);
                c.target = camera.target;
                c
            });
            self.view_file = Some(path);
        } else {
            self.view_file = None;
        }
        self.view = Some(View {
            renderer,
            scene,
            nav,
            target,
            tex,
            size,
            dirty: true,
            live,
            motion: None,
            facts: Facts::default(),
            clicked: Vec::new(),
            clicks: 0,
            origin_was: Vec::new(),
            sketches_was: Vec::new(),
            previewing: false,
            live_drag: false,
        });
        self.shell.view_tex = Some(tex);
        self.shell.name = l.name;
        self.shell.status.clear();
        self.part_colours();
    }

    /// The parts' colours as the setting has them: an assembly's per occurrence's file, a part's
    /// by its own name.
    pub fn part_colours(&mut self) {
        if self.fg_asm.is_some() {
            self.asm_show();
            return;
        }
        let tint = crate::palette::part_colour(&self.shell.name, self.shell.view.pastel);
        if let Some(v) = &mut self.view {
            for p in &mut v.scene.placements {
                p.tint = tint;
            }
            v.renderer.set_tints(&v.scene);
            v.dirty = true;
        }
    }

    /// The settings kept between sessions (the app's own, not a test's): theme, part colours.
    pub fn load_settings(&mut self) {
        for (k, v) in crate::palette::settings::load() {
            match (k.as_str(), v.as_str()) {
                ("theme", "light") => self.shell.theme = crate::shell::theme::LIGHT,
                ("part_colours", "pastel") => self.shell.view.pastel = true,
                _ => {}
            }
        }
        self.settings_was = Some((self.shell.theme.dark, self.shell.view.pastel));
    }

    fn save_settings(&mut self) {
        let now = (self.shell.theme.dark, self.shell.view.pastel);
        if self.settings_was.is_none_or(|w| w == now) {
            return;
        }
        self.settings_was = Some(now);
        crate::palette::settings::save(&[
            ("theme", if now.0 { "dark" } else { "light" }),
            ("part_colours", if now.1 { "pastel" } else { "grey" }),
        ]);
    }

    fn theme_view(&self, r: &mut Renderer) {
        let t = self.shell.theme;
        r.back_colour = viewcube::rgb(t.view);
        r.cube_palette = if t.dark { viewcube::DARK } else { viewcube::LIGHT };
    }

    /// The viewport's size in device pixels, at least one.
    fn view_px(&self) -> (u32, u32) {
        let v = self.shell.view_rect;
        let p = self.pixels_per_point;
        (
            ((v.width() * p).round() as u32).max(1),
            ((v.height() * p).round() as u32).max(1),
        )
    }

    /// What the worker has finished.
    fn take_live(&mut self) {
        let Some(v) = &mut self.view else { return };
        let Some(l) = &mut v.live else { return };
        let events = l.poll();
        let part = v.scene.placements[0].part;
        for e in events {
            match e {
                Event::Doc(d) => self.shell.set_doc(&d.src, d.version, d.failed),
                Event::Done(d) => {
                    // the reply's first line says it
                    let first = d.text.lines().next().unwrap_or("").to_string();
                    if let Some(dlg) = &mut self.shell.dialog
                        && dlg.awaiting
                    {
                        dlg.awaiting = false;
                        if d.ok {
                            // + : the same kind again, from the start
                            let again = match dlg.target {
                                crate::shell::dialog::DialogTarget::New(kind) if dlg.again => Some(kind),
                                _ => None,
                            };
                            self.shell.dialog = None;
                            self.shell.out.extend(again.map(Action::NewFeature));
                        } else {
                            dlg.again = false;
                            dlg.error = Some(d.text.lines().nth(1).unwrap_or(&first).to_string());
                            self.calls.pop();
                        }
                    }
                    if let Some(e) = &mut self.shell.dim {
                        e.error = (!d.ok).then(|| d.text.lines().nth(1).unwrap_or(&first).to_string());
                    }
                    self.shell.status = first;
                }
                Event::Frame(f) => {
                    if f.seq > 0
                        && let Some(e) = &mut self.shell.dim
                    {
                        e.error = f.failure.clone();
                    }
                    if let Some(d) = &mut self.shell.dialog
                        && d.seq == f.seq
                    {
                        d.error = f.failure.as_deref().map(crate::shell::panel::plain_error);
                        // which side fills isn't for a person to find by getting it wrong
                        if d.error.is_some() && d.try_other_side() {
                            d.seq = 0;
                        }
                    }
                    if let Some(Out::Delta(d)) = f.out {
                        v.renderer.live_apply(part, &d);
                        if f.seq > 0 {
                            v.renderer.live_show(true);
                            v.scene.set_part(part, d.center, d.radius, vec![d.chord]);
                            v.dirty = true;
                        }
                    }
                }
                Event::Final(out) => {
                    if let Out::Whole(m) = out {
                        self.shell.unshown = false;
                        v.renderer.set_parts(std::slice::from_ref(&*m));
                        let chords = m.lods.iter().map(|x| x.chord).collect();
                        v.scene.set_part(part, m.center, m.radius, chords);
                    }
                    v.renderer.live_show(false);
                    v.live_drag = false;
                    v.dirty = true;
                }
                Event::Checking => {
                    self.shell.checking_since.get_or_insert(now_ms());
                }
                Event::Checked(flags) => {
                    self.shell.flags = flags;
                    self.shell.checking_since = None;
                }
                Event::SketchPlane(head, got) => {
                    let asked = self.sketch_for.take_if(|a| a.0 == head);
                    match (got, asked) {
                        (Ok(plane), Some((head, edit, rows))) => {
                            let mut m = SketchMode::new(edit, head, plane, rows, self.shell.doc.as_ref());
                            m.units = self.shell.units;
                            if m.rows.iter().any(|r| project_refs(r).is_some()) {
                                m.take_sources(std::mem::take(&mut self.sketch_sources));
                            }
                            self.shell.sketch = Some(m);
                            self.shell.origin_selected = None;
                            self.shell.tab = 1;
                            self.shell.status.clear();
                            // square on to it, as if drawing on a sheet
                            look_at(v, plane);
                        }
                        (Err(why), Some(_)) => self.shell.status = why,
                        _ => {}
                    }
                }
                Event::SketchSources { adding: false, got } => {
                    self.sketch_sources = got.into_iter().collect()
                }
                Event::SketchSources { adding: true, got } => {
                    if let Some(m) = &mut self.shell.sketch {
                        for (words, r) in got {
                            m.project(words, r);
                        }
                    }
                }
                Event::Facts(f) => {
                    if let Some(kind) = self.after_sketch.take() {
                        self.shell.out.push(Action::NewFeature(kind));
                    }
                    v.dirty |= f.sketches != self.shell.sketch_shapes;
                    v.facts = *f;
                    self.shell.sketch_shapes = v.facts.sketches.clone();
                    // what's marked for the selection reads these
                    v.dirty |= self.shell.selected.is_some();
                    if let Some(id) = self.shell.selected
                        && self.shell.dim.as_ref().is_none_or(|e| !e.dragging)
                    {
                        let old = self.shell.dim.take();
                        self.shell.dim = v
                            .facts
                            .handle(id)
                            .cloned()
                            .map(|h| DimEdit::in_units(h, self.shell.units));
                        if let (Some(e), Some(old)) = (&mut self.shell.dim, old) {
                            e.error = old.error;
                            // what's being typed stays, whatever the part has just become
                            if old.text != old.handle.text {
                                e.text = old.text;
                            }
                        }
                    }
                }
            }
        }
    }

    /// What of the origin is under the pointer in the view: what's shown, any time it's the axes or
    /// the point; a plane only where the part isn't hit.
    fn origin_under(&self) -> Option<crate::origin::Item> {
        let v = self.view.as_ref()?;
        if !self.over_view {
            return None;
        }
        let shown: Vec<crate::origin::Item> = crate::origin::ITEMS
            .into_iter()
            .enumerate()
            .filter(|(k, i)| {
                self.shell.origin_shown[*k]
                    || self.shell.origin_selected == Some(*i)
                    || (self.shell.rows.is_empty() && *k < 3)
            })
            .map(|(_, i)| i)
            .collect();
        let (c, r) = v.scene.bounds();
        let empty = v.nav.marks().iter().all(|m| m.selected);
        crate::origin::hit(
            &shown,
            crate::origin::reach(c, r),
            &v.nav.camera,
            v.size,
            v.nav.cursor(),
            empty,
        )
    }

    /// A click in the view chooses a face's feature; a row of the tree chooses its faces. With a
    /// dialog up, what's clicked goes to the field waiting for picks.
    fn sync_selection(&mut self) {
        // in a sketch only the sketch answers the pointer; the model's for what's looked at
        if self
            .shell
            .sketch
            .as_ref()
            .is_some_and(|m| m.tool != Some(SkTool::Project))
        {
            self.shell.origin_hover = None;
            if let Some(v) = &mut self.view {
                v.clicks = v.nav.clicks;
                v.nav.selected.clear();
                v.clicked.clear();
            }
            return;
        }
        let under = self.origin_under();
        self.shell.origin_hover = under;
        if self.fg.is_some() {
            self.fg_picks(under);
            return;
        }
        if self.shell.sketch_pick {
            let Some(v) = &mut self.view else { return };
            let clicked = v.nav.clicks != v.clicks;
            v.clicks = v.nav.clicks;
            let face = v.nav.selected.iter().rev().find_map(|&(_, e)| match e {
                Entity::Face(f) => v.facts.face_refs.get(f as usize).cloned(),
                _ => None,
            });
            v.nav.selected.clear();
            v.clicked.clear();
            // a sketch chosen, here or in the tree: that's the one to edit
            let sketch_row = |id: Option<forge_doc::ItemId>| {
                id.filter(|id| self.shell.rows.iter().any(|r| r.id == *id && r.kind == "sketch"))
            };
            let picked_sketch = sketch_row(self.shell.selected)
                .or_else(|| sketch_row(self.shell.sketch_hover.filter(|_| clicked)));
            if self.shell.selected.take().is_some() && picked_sketch.is_none() {
                self.shell.status =
                    "That's a feature. Pick a plane, a planar face, or a sketch to edit".to_string();
            }
            if let Some(id) = picked_sketch {
                self.shell.sketch_pick = false;
                self.shell.selected = Some(id);
                self.edit_sketch(id);
                return;
            }
            let chosen = self.shell.origin_selected.take();
            let plane = chosen.or(under.filter(|_| clicked)).filter(|i| {
                matches!(
                    i,
                    crate::origin::Item::XyPlane
                        | crate::origin::Item::XzPlane
                        | crate::origin::Item::YzPlane
                )
            });
            let on_face = face.filter(|_| clicked).map(|f| format!("on {f}"));
            if let Some(h) = plane.map(|i| i.reference().to_string()).or(on_face) {
                self.ask_plane(h, SketchFor::New, Vec::new());
            }
            return;
        }
        if self
            .shell
            .sketch
            .as_ref()
            .is_some_and(|m| m.tool == Some(SkTool::Project))
        {
            let at = sketch_place(&self.shell);
            let Some(v) = &mut self.view else { return };
            v.clicks = v.nav.clicks;
            if v.nav.selected != v.clicked {
                // a click's one, or all a window took
                let refs: Vec<String> = v
                    .nav
                    .selected
                    .iter()
                    .filter_map(|&(_, e)| match e {
                        Entity::Face(f) => v.facts.face_refs.get(f as usize).cloned(),
                        Entity::Edge(x) => v.facts.edge_refs.get(x as usize).cloned(),
                        Entity::Vertex(_) => None,
                    })
                    .collect();
                v.nav.selected.clear();
                v.clicked.clear();
                if !refs.is_empty()
                    && let Some(l) = &mut v.live
                {
                    l.sketch_sources(at, refs, true);
                }
                v.dirty = true;
            }
            return;
        }
        let origin_under = self.origin_under();
        let Some(v) = &mut self.view else { return };
        if let Some(d) = &mut self.shell.dialog {
            // a revolve's axis from the origin's axes or a work axis, in the view or the tree
            if d.kind == "revolve" && d.pick.as_ref().is_some_and(|p| p.0 == "Axis") {
                use crate::origin::Item;
                let clicked = v.nav.clicks != v.clicks;
                let origin = match self.shell.origin_selected {
                    Some(i @ (Item::XAxis | Item::YAxis | Item::ZAxis)) => Some(i),
                    _ => origin_under
                        .filter(|i| clicked && matches!(i, Item::XAxis | Item::YAxis | Item::ZAxis)),
                };
                let work = self
                    .shell
                    .selected
                    .and_then(|id| v.facts.work_axes.iter().find(|w| w.0 == id))
                    .map(|w| (w.1, w.2));
                let line = match origin {
                    Some(Item::XAxis) => Some(([0.0; 3], [1.0, 0.0, 0.0])),
                    Some(Item::YAxis) => Some(([0.0; 3], [0.0, 1.0, 0.0])),
                    Some(Item::ZAxis) => Some(([0.0; 3], [0.0, 0.0, 1.0])),
                    _ => work,
                };
                if let Some((at, dir)) = line {
                    let plane = crate::shell::dialog::profile_sketch(d).and_then(|o| {
                        let id = self.shell.doc.as_ref()?.find(&o)?;
                        self.shell
                            .sketch_shapes
                            .iter()
                            .find(|x| x.item == Some(id))
                            .map(|x| x.plane)
                    });
                    match plane.map(|pl| crate::shell::dialog::axis_in_plane(&pl, at, dir)) {
                        Some(Ok(t)) => {
                            d.error = None;
                            d.take_pick("Axis", t);
                        }
                        Some(Err(why)) => d.error = Some(why),
                        None => d.error = Some("Pick the profile first".to_string()),
                    }
                    self.shell.origin_selected = None;
                    self.shell.selected = None;
                    v.nav.selected.clear();
                    v.clicked.clear();
                    v.clicks = v.nav.clicks;
                    v.dirty = true;
                    return;
                }
            }
            // a feature from the tree, for what takes features: exact where bodies overlap
            let takes = d.pick.as_ref().map(|p| p.0.clone()).unwrap_or_default();
            if matches!(takes.as_str(), "Features" | "Target" | "Tools")
                && let Some(id) = self.shell.selected.take()
                && let Some(name) = self.shell.doc.as_ref().and_then(|doc| doc.name(id))
            {
                if takes == "Target" {
                    d.take_pick("Target", name);
                } else {
                    let had = d
                        .fields
                        .iter()
                        .find(|f| f.0 == takes)
                        .map_or(String::new(), |f| f.1.clone());
                    let mut names: Vec<String> = had.split_whitespace().map(str::to_string).collect();
                    match names.iter().position(|n| *n == name) {
                        Some(k) => {
                            names.remove(k);
                        }
                        None => names.push(name),
                    }
                    d.take_pick(&takes, names.join(" "));
                }
                v.dirty = true;
            }
            // a sketch from the tree, for a hole on points: all it has
            if d.kind == "hole"
                && d.hole
                    .as_ref()
                    .is_some_and(|h| h.placing == crate::shell::dialog::Placing::Sketch)
                && let Some(id) = self.shell.selected.take()
                && self.shell.rows.iter().any(|r| r.id == id && r.kind == "sketch")
                && let Some(name) = self.shell.doc.as_ref().and_then(|doc| doc.name(id))
                && let Some(shape) = self.shell.sketch_shapes.iter().find(|s| s.item == Some(id))
            {
                let (o, z): ([f64; 3], forge_math::Vec3) = {
                    let o = shape.plane.origin;
                    ([o.x, o.y, o.z], shape.plane.z().into())
                };
                let m = v.scene.bounds().0;
                let toward =
                    (m[0] as f64 - o[0]) * z.x + (m[1] as f64 - o[1]) * z.y + (m[2] as f64 - o[2]) * z.z;
                let on = if toward > 0.0 { "on" } else { "-on" };
                if let Some(f) = d.fields.iter_mut().find(|f| f.0 == "Points") {
                    f.1 = format!("{on} {name} pts centers");
                }
                v.dirty = true;
            }
            if v.nav.selected != v.clicked {
                v.clicked = v.nav.selected.clone();
                // a face that's what it goes to, as a line names it
                let names = |f: u32| -> Option<String> {
                    let owner = v.facts.face_item.get(f as usize).copied().flatten()?;
                    let name = self.shell.doc.as_ref()?.name(owner)?;
                    Some(format!("{name} {}", v.facts.face_refs.get(f as usize)?))
                };
                let picked = v.clicked.last().map(|c| c.1);
                // a straight edge of the part for a revolve's axis, where it lies in the sketch's plane
                if d.kind == "revolve"
                    && d.pick.as_ref().is_some_and(|p| p.0 == "Axis")
                    && let Some(Entity::Edge(x)) = picked
                {
                    let plane = crate::shell::dialog::profile_sketch(d).and_then(|o| {
                        let id = self.shell.doc.as_ref()?.find(&o)?;
                        self.shell
                            .sketch_shapes
                            .iter()
                            .find(|x| x.item == Some(id))
                            .map(|x| x.plane)
                    });
                    let got = match (v.facts.edge_line.get(x as usize), plane) {
                        (Some(Some((a, b))), Some(pl)) => crate::shell::dialog::axis_in_plane(
                            &pl,
                            *a,
                            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                        ),
                        (_, None) => Err("Pick the profile first".to_string()),
                        _ => Err("The axis must be straight: pick a straight edge".to_string()),
                    };
                    match got {
                        Ok(t) => {
                            d.error = None;
                            d.take_pick("Axis", t);
                        }
                        Err(why) => d.error = Some(why),
                    }
                    v.nav.selected.clear();
                    v.clicked.clear();
                    v.dirty = true;
                    return;
                }
                let wants_surface = d.kind == "extrude" && d.extent == crate::shell::dialog::Extent::Surface;
                const UNNAMED: &str = "No feature owns that face, so it can't be named here. Pick another.";
                if wants_surface {
                    if let Some(Entity::Face(f)) = picked {
                        match names(f) {
                            Some(t) => {
                                d.surface = t;
                                d.error = None;
                            }
                            None => d.error = Some(UNNAMED.to_string()),
                        }
                    }
                    v.nav.selected.clear();
                    v.clicked.clear();
                    v.dirty = true;
                    v.clicks = v.nav.clicks;
                    return;
                }
                if let Some(h) = &mut d.hole {
                    use crate::shell::dialog::{Placing, Termination};
                    let mut took = false;
                    // and again after one it couldn't reach
                    if h.termination == Termination::Surface
                        && (h.surface.is_empty() || d.error.is_some())
                        && let Some(Entity::Face(f)) = picked
                    {
                        match names(f) {
                            Some(t) => {
                                h.surface = t;
                                d.error = None;
                            }
                            None => d.error = Some(UNNAMED.to_string()),
                        }
                        took = true;
                    }
                    if !took {
                        match (h.placing, picked) {
                            (Placing::Concentric, Some(Entity::Edge(x))) => {
                                if let Some(Some((c, n, _))) = v.facts.edge_round.get(x as usize) {
                                    // its axis out of the part
                                    let m = v.scene.bounds().0;
                                    let side = (0..3).map(|k| (m[k] as f64 - c[k]) * n[k]).sum::<f64>();
                                    let out = if side > 0.0 { n.map(|a| -a) } else { *n };
                                    h.concentric = Some((*c, out));
                                }
                            }
                            (Placing::Linear, Some(Entity::Face(f))) => {
                                if let Some(Some(p)) = v.facts.face_plane.get(f as usize) {
                                    h.linear.face = Some(*p);
                                    h.linear.edges.clear();
                                    if let Some(q) = v.nav.hit_point() {
                                        h.linear.clicked = q.map(|a| a as f64);
                                    }
                                }
                            }
                            (Placing::Linear, Some(Entity::Edge(x))) => {
                                if h.linear.face.is_some()
                                    && let Some(Some(l)) = v.facts.edge_line.get(x as usize)
                                {
                                    if h.linear.edges.len() >= 2 {
                                        h.linear.edges.remove(0);
                                    }
                                    h.linear.edges.push(*l);
                                }
                            }
                            _ => {}
                        }
                    }
                    if h.placing != Placing::Sketch || took {
                        d.settle_hole();
                        v.nav.selected.clear();
                        v.clicked.clear();
                        v.dirty = true;
                        v.clicks = v.nav.clicks;
                        return;
                    }
                }
                // a field for edges takes edges, one for faces takes faces
                let wants = d.pick.as_ref().map(|p| p.0.clone()).unwrap_or_default();
                if d.kind == "thread" {
                    // the cylinder picked, and what made it: a hole's thread is the hole's
                    if let Some(&(_, Entity::Face(f))) = v.clicked.last()
                        && let Some(Some(r)) = v.facts.face_radius.get(f as usize)
                        && let Some(handle) = v.facts.face_refs.get(f as usize)
                        && let Some(owner) = v.facts.face_item.get(f as usize).copied().flatten()
                        && let Some(doc) = &self.shell.doc
                        && let Some(name) = doc.name(owner)
                    {
                        let bore = v.facts.face_bore.get(f as usize).copied().unwrap_or(false);
                        d.thread_on(format!("face {name} {handle}"), r * 2.0, bore);
                    }
                    v.dirty = true;
                    v.clicks = v.nav.clicks;
                    return;
                }
                // features by a face they made; references, on the part as it stands
                if matches!(wants.as_str(), "Features" | "Target" | "Tools") {
                    if let Some(&(_, Entity::Face(f))) = v.clicked.last()
                        && let Some(owner) = v.facts.face_item.get(f as usize).copied().flatten()
                        && let Some(doc) = &self.shell.doc
                        && let Some(name) = doc.name(owner)
                    {
                        // where bodies overlap, a face shared on the view reads as the first's: a
                        // body can't combine with itself, so it's the other's
                        let target = d
                            .fields
                            .iter()
                            .find(|f| f.0 == "Target")
                            .map_or(String::new(), |f| f.1.clone());
                        let name = if wants == "Tools" && name == target {
                            self.shell
                                .rows
                                .iter()
                                .filter(|r| {
                                    r.line.contains(" new ")
                                        || r.line.split_whitespace().nth(2) == Some("new")
                                })
                                .filter_map(|r| doc.name(r.id))
                                .find(|n| *n != target)
                                .unwrap_or(name)
                        } else {
                            name
                        };
                        if wants == "Target" {
                            d.take_pick("Target", name);
                        } else {
                            let had = d
                                .fields
                                .iter()
                                .find(|f| f.0 == wants)
                                .map_or(String::new(), |f| f.1.clone());
                            let mut names: Vec<String> = had.split_whitespace().map(str::to_string).collect();
                            match names.iter().position(|n| *n == name) {
                                Some(k) => {
                                    names.remove(k);
                                }
                                None => names.push(name),
                            }
                            d.take_pick(&wants, names.join(" "));
                        }
                    }
                    v.nav.selected.clear();
                    v.clicked.clear();
                    v.dirty = true;
                    v.clicks = v.nav.clicks;
                    return;
                }
                if matches!(d.kind.as_str(), "plane" | "axis" | "point") {
                    let last = self
                        .shell
                        .rows
                        .iter()
                        .rev()
                        .find(|r| {
                            !r.suppressed && !matches!(r.kind.as_str(), "sketch" | "plane" | "axis" | "point")
                        })
                        .map(|r| r.id)
                        .and_then(|id| self.shell.doc.as_ref().and_then(|doc| doc.name(id)));
                    if let (Some(&(_, e)), Some(feat)) = (v.clicked.last(), last) {
                        let text = match (d.kind.as_str(), e) {
                            ("plane" | "axis", Entity::Face(f)) => {
                                v.facts.face_refs.get(f as usize).map(|h| format!("{feat} {h}"))
                            }
                            ("axis", Entity::Edge(x)) => {
                                v.facts.edge_refs.get(x as usize).map(|h| format!("{feat} {h}"))
                            }
                            ("point", Entity::Edge(x)) => v
                                .facts
                                .edge_refs
                                .get(x as usize)
                                .map(|h| format!("mid {feat} {h}")),
                            ("point", Entity::Vertex(k)) => {
                                v.facts.vertex_refs.get(k as usize).map(|h| format!("{feat} {h}"))
                            }
                            _ => None,
                        };
                        if let Some(text) = text {
                            d.take_pick("Reference", text);
                        }
                    }
                    v.dirty = true;
                    v.clicks = v.nav.clicks;
                    return;
                }
                let refs: Vec<String> = v
                    .clicked
                    .iter()
                    .filter_map(|&(_, e)| match e {
                        Entity::Face(f) if wants != "Edges" => v.facts.face_refs.get(f as usize).cloned(),
                        Entity::Edge(x) if wants != "Faces" => v.facts.edge_refs.get(x as usize).cloned(),
                        _ => None,
                    })
                    .collect();
                d.picked(&refs);
                v.dirty = true;
            }
            if v.nav.clicks != v.clicks {
                v.clicks = v.nav.clicks;
            }
            return;
        }
        if v.nav.clicks != v.clicks {
            v.clicks = v.nav.clicks;
            if let Some(id) = self.shell.sketch_hover {
                self.shell.selected = Some(id);
                self.shell.origin_selected = None;
                v.nav.selected.clear();
                v.clicked.clear();
                return;
            }
            if let Some(i) = under {
                self.shell.origin_selected = Some(i);
                self.shell.selected = None;
                v.nav.selected.clear();
            } else {
                self.shell.origin_selected = None;
            }
        }
        if v.nav.selected != v.clicked {
            v.clicked = v.nav.selected.clone();
            let owner = v.clicked.iter().rev().find_map(|&(_, e)| match e {
                Entity::Face(f) => v.facts.face_item.get(f as usize).copied().flatten(),
                _ => None,
            });
            self.shell.selected = owner;
        }
        let want = self.shell.selected;
        if self.shell.dim.as_ref().map(|d| d.handle.item) != want {
            self.shell.dim = want.and_then(|id| v.facts.handle(id).cloned()).map(DimEdit::new);
            v.dirty = true;
        }
    }

    /// Is the view what's under this place, with nothing of the chrome over it?
    fn view_is_at(&self, position: PhysicalPosition<f64>) -> bool {
        let p = self.pixels_per_point;
        let at = egui::pos2(position.x as f32 / p, position.y as f32 / p);
        self.shell.view_rect.contains(at)
            && !self.shell.menu_open()
            && !self.ctx.egui_is_using_pointer()
            && !self.shell.view_blocks.iter().any(|r| r.contains(at))
            && self
                .ctx
                .layer_id_at(at)
                .is_none_or(|l| l.order == egui::Order::Background)
    }

    /// What one window event means for the view, if it's the view's.
    pub fn view_event(&mut self, event: &WindowEvent) {
        // a press decides from where the pointer is now: a menu may have closed, a dialog gone, since
        // it last moved
        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            device_id,
            ..
        } = event
            && !self.captured
            && let Some(position) = self.pointer_px
        {
            let device_id = *device_id;
            let over = self.view_is_at(position);
            let was = std::mem::replace(&mut self.over_view, over);
            if over && !was {
                let moved = WindowEvent::CursorMoved { device_id, position };
                self.over_view = true;
                self.view_event(&moved);
            }
        }
        let over_now = match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_px = Some(*position);
                self.view_is_at(*position)
            }
            _ => false,
        };
        let Some(v) = &mut self.view else { return };
        let p = self.pixels_per_point;
        let origin = self.shell.view_rect.min;
        let to_view =
            |x: f64, y: f64| PhysicalPosition::new(x - (origin.x * p) as f64, y - (origin.y * p) as f64);
        let forward = match event {
            WindowEvent::ModifiersChanged(_) => {
                v.nav.event(event);
                return;
            }
            WindowEvent::CursorMoved { device_id, position } => {
                if self.captured || over_now {
                    self.over_view = over_now;
                    Some(WindowEvent::CursorMoved {
                        device_id: *device_id,
                        position: to_view(position.x, position.y),
                    })
                } else if std::mem::take(&mut self.over_view) {
                    Some(WindowEvent::CursorLeft {
                        device_id: *device_id,
                    })
                } else {
                    None
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.over_view = false;
                self.captured = false;
                Some(event.clone())
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                ..
            } if self
                .shell
                .sketch
                .as_ref()
                .is_some_and(|m| m.tool.is_some_and(|t| t != SkTool::Project)) =>
            {
                None
            }
            WindowEvent::MouseInput { state, .. } => match state {
                ElementState::Pressed if self.over_view => {
                    self.captured = true;
                    Some(event.clone())
                }
                ElementState::Released if self.captured => {
                    self.captured = false;
                    Some(event.clone())
                }
                _ => None,
            },
            WindowEvent::MouseWheel { .. } if self.over_view => Some(event.clone()),
            _ => None,
        };
        if let Some(e) = forward {
            if v.nav.event(&e) {
                v.dirty = true;
            }
            if v.nav.home_if_asked(&v.scene) {
                v.dirty = true;
            }
            if let Some((from, to)) = v.nav.marked.take() {
                // a flick chooses from the marking menu without showing it; a click shows it
                let (items, _) = self.shell.marking_items();
                self.mark_held = None;
                match Shell::marking_flick(from, to, p) {
                    Some(k) => {
                        self.shell.marking = None;
                        self.shell.out.extend(items[k].2);
                    }
                    None => {
                        self.shell.marking = Some(egui::pos2(origin.x + from.0 / p, origin.y + from.1 / p))
                    }
                }
            }
            if let Some((a, b)) = v.nav.window.take() {
                v.dirty = true;
                let lo = (a.0.min(b.0), a.1.min(b.1));
                let hi = (a.0.max(b.0), a.1.max(b.1));
                let wants = self
                    .shell
                    .dialog
                    .as_ref()
                    .and_then(|d| d.pick.as_ref())
                    .map_or(String::new(), |p| p.0.clone());
                let camera = v.nav.camera;
                // only what can be seen: right to left what shows in it, left to right what shows
                // in it and nowhere else
                let crossing = b.0 < a.0;
                let found: Vec<Entity> = v
                    .renderer
                    .seen_in_box(&mut v.scene, &camera, lo, hi)
                    .into_iter()
                    .filter(|(e, _, _)| match e {
                        Entity::Edge(_) => wants == "Edges",
                        Entity::Face(_) => wants != "Edges",
                        Entity::Vertex(_) => false,
                    })
                    .filter(|(_, inside, outside)| *inside > 0 && (crossing || *outside == 0))
                    .map(|f| f.0)
                    .collect();
                if v.nav.ctrl_held() {
                    v.nav.selected.retain(|s| !found.contains(&s.1));
                } else {
                    if !v.nav.shift_held() {
                        v.nav.selected.clear();
                    }
                    for e in found {
                        if !v.nav.selected.contains(&(0, e)) {
                            v.nav.selected.push((0, e));
                        }
                    }
                }
            }
            if let Some(at) = v.nav.cube_menu.take() {
                self.shell.cube_region = v.nav.cube_menu_region.take();
                self.shell.cube_menu = Some(egui::pos2(origin.x + at.0 / p, origin.y + at.1 / p));
                self.shell.cube_menu_fresh = true;
            }
        }
    }

    /// A key, if no field has the keyboard.
    pub fn view_key(&mut self, code: winit::keyboard::KeyCode) -> bool {
        if self.ctx.egui_wants_keyboard_input() {
            return false;
        }
        let Some(v) = &mut self.view else { return false };
        use winit::keyboard::KeyCode as K;
        let changed = match code {
            K::KeyL => {
                v.renderer.line_mode = v.renderer.line_mode.next();
                self.shell.view.lines = v.renderer.line_mode;
                true
            }
            K::KeyO => {
                v.renderer.ao = !v.renderer.ao;
                true
            }
            _ => v.nav.key(code, &v.scene),
        };
        self.shell.view.section = v.nav.section.is_some();
        v.dirty |= changed;
        changed
    }

    /// Asks for what's under the cursor, and takes in the answer. True if that changed anything.
    pub fn picks(&mut self) -> bool {
        let Some(v) = &mut self.view else { return false };
        // what a pick would read is the mesh from before the drag
        if v.live_drag {
            v.nav.take_pick();
        } else if !v.renderer.pick_busy()
            && let Some(px) = v.nav.take_pick()
        {
            v.renderer.pick_start(&mut v.scene, &v.nav.camera, px);
        }
        if let Some((px, hit, _)) = v.renderer.pick_poll(&v.scene)
            && v.nav.on_pick(px, hit)
        {
            v.dirty = true;
            return true;
        }
        false
    }

    pub fn pick_busy(&self) -> bool {
        self.view.as_ref().is_some_and(|v| v.renderer.pick_busy())
    }

    /// The tool calls and view changes the chrome asked for; the window's things go to the host.
    fn apply(&mut self) -> Vec<Action> {
        let mut host = Vec::new();
        for a in std::mem::take(&mut self.shell.out) {
            if let Action::PartColours(on) = a {
                self.shell.view.pastel = on;
                self.part_colours();
                continue;
            }
            match a {
                Action::NewDrawing => {
                    match self.fg.as_ref().and_then(|s| s.path.clone()) {
                        Some(model) => {
                            self.shell.new_drawing = Some(crate::shell::drawing::NewDrawing {
                                model,
                                size: 1,
                                iso: false,
                            })
                        }
                        None => {
                            self.shell.status =
                                "New Drawing: open a saved Forge part or assembly first".into()
                        }
                    }
                    continue;
                }
                Action::CloseDrawing => {
                    self.shell.drawing = None;
                    self.shell.tab = 0;
                    continue;
                }
                Action::Undo | Action::Redo if self.shell.drawing.is_some() => {
                    if let Some(env) = &mut self.shell.drawing {
                        env.undo(a == Action::Redo);
                    }
                    continue;
                }
                _ => {}
            }
            if let Some(s) = &mut self.fg {
                // a Forge part's document changes only by typed ops, from its own panel
                match a {
                    Action::Undo => {
                        s.undo();
                        self.fg_text();
                        continue;
                    }
                    Action::Redo => {
                        s.redo();
                        self.fg_text();
                        continue;
                    }
                    Action::NewFeature(kind) => {
                        self.fg_open_tool(kind);
                        continue;
                    }
                    Action::DialogOk | Action::DialogCancel
                        if self.shell.fg.as_ref().is_some_and(|p| p.tool.is_some()) =>
                    {
                        let p = self.shell.fg.as_mut().expect("a Forge part");
                        if a == Action::DialogCancel {
                            p.tool = None;
                        } else if let Some(t) = &mut p.tool {
                            match t.text() {
                                Ok(text) => p.out.push(crate::shell::fg::FgAct::Tool(
                                    forge_lang::error::Op::AddFeature { before: None, text },
                                )),
                                Err(e) => t.error = Some(e),
                            }
                        }
                        self.apply_fg();
                        continue;
                    }
                    Action::StartSketch => {
                        self.shell.sketch_pick = true;
                        self.shell.status = "Pick a planar face or an origin plane to sketch on".into();
                        continue;
                    }
                    Action::FinishSketch => {
                        self.fg_finish_sketch();
                        continue;
                    }
                    Action::Fg(c) => {
                        self.shell.fg_command(c);
                        self.apply_fg();
                        continue;
                    }
                    Action::Inspect(c) => {
                        self.shell.inspect_command(c);
                        continue;
                    }
                    Action::SetUnits(_) => {
                        self.fg_refuse(crate::shell::fgtool::ToolError::NoForgeForm {
                            tool: "units",
                            why:
                                "a Forge part reads in mm and degrees; give a value a unit (`2in` is not one)"
                                    .into(),
                        });
                        continue;
                    }
                    a if fc_only(a) => continue,
                    _ => {}
                }
            }
            match a {
                Action::Undo | Action::Redo => {
                    let call = if a == Action::Undo { Call::Undo } else { Call::Redo };
                    if let Some(v) = &mut self.view {
                        self.calls.push(call.clone());
                        if let Some(l) = &mut v.live {
                            l.call(vec![call]);
                        }
                    }
                }
                Action::LookAt => {
                    let plane = self.shell.sketch.as_ref().map(|m| m.plane).or_else(|| {
                        let on = self.shell.selected;
                        self.shell
                            .sketch_shapes
                            .iter()
                            .find(|s| s.item == on)
                            .map(|s| s.plane)
                    });
                    if let (Some(plane), Some(v)) = (plane, &mut self.view) {
                        look_at(v, plane);
                    }
                }
                Action::SketchVisibility(id) => {
                    self.shell.toggle_sketch(id);
                    self.save_view();
                }
                Action::FitAll
                | Action::Home
                | Action::SetHome
                | Action::SetHomeAt(_)
                | Action::SetFaceAs(..)
                | Action::ResetOrientation
                | Action::Parallel(_)
                | Action::Lines(_)
                | Action::Section
                | Action::Tool(_) => {
                    self.apply_view(a);
                }
                Action::StartSketch => self.start_sketch(),
                Action::SketchTool(t) => {
                    if let Some(m) = &mut self.shell.sketch {
                        m.set_tool(Some(t));
                    }
                }
                Action::CancelSketch => {
                    self.shell.sketch = None;
                    self.shell.then_feature = None;
                    self.shell.tab = 0;
                    self.shell.status.clear();
                }
                Action::SetUnits(u) => self.set_units(u),
                Action::CancelPick => {
                    self.shell.sketch_pick = false;
                    self.shell.then_feature = None;
                    self.shell.status.clear();
                }
                Action::Deselect => {
                    self.shell.selected = None;
                    self.shell.origin_selected = None;
                    if let Some(v) = &mut self.view {
                        v.nav.selected.clear();
                        v.clicked.clear();
                        v.dirty = true;
                    }
                }
                Action::EditInline(id) => self.edit_inline(id),
                Action::FinishSketch => self.finish_sketch(),
                Action::Edit(id) if self.shell.rows.iter().any(|r| r.id == id && r.kind == "sketch") => {
                    self.edit_sketch(id);
                }
                Action::Edit(id) => {
                    self.shell.selected = Some(id);
                    let (kind, title) = match self.shell.rows.iter().find(|r| r.id == id) {
                        Some(r) => (r.kind.clone(), r.title.clone()),
                        None => continue,
                    };
                    let fields = self.shell.fields_of(id);
                    let names = self.param_names();
                    self.shell.dialog =
                        Some(Dialog::edit(id, &kind, title, fields).in_units(self.shell.units, names));
                    if let Some(v) = &mut self.view {
                        v.nav.selected.clear();
                        v.clicked.clear();
                    }
                }
                // a profile feature starts from a sketch: one there's no feature on, or a new one
                Action::NewFeature(kind)
                    if matches!(kind, "extrude" | "revolve") && self.free_sketch().is_none() =>
                {
                    self.shell.then_feature = Some(kind);
                    self.start_sketch();
                }
                Action::NewFeature(kind) => {
                    self.end_preview();
                    let names = self.param_names();
                    let mut d = Dialog::new_feature(kind).in_units(self.shell.units, names);
                    if kind == "thread" {
                        d.choose_thread(Some("ISO Metric"), None, None, None);
                    }
                    // with no sketch to place it on, it's placed on the part
                    if self.shell.sketch_shapes.is_empty()
                        && let Some(h) = &mut d.hole
                    {
                        h.placing = crate::shell::dialog::Placing::Linear;
                    }
                    if matches!(kind, "extrude" | "revolve")
                        && let Some(name) = self.free_sketch()
                    {
                        let doc = self.shell.doc.as_ref();
                        let regions = doc.and_then(|d| d.find(&name)).map_or(0, |id| {
                            self.shell
                                .sketch_shapes
                                .iter()
                                .find(|s| s.item == Some(id))
                                .map_or(0, |s| s.regions.len())
                        });
                        // which of several is the person's to say
                        if regions > 1 {
                            d.on_sketch("");
                            d.pick = Some(("Profile".to_string(), "Pick a closed region in the view"));
                        } else {
                            d.on_sketch(&name);
                        }
                    }
                    self.shell.dialog = Some(d);
                    if let Some(v) = &mut self.view {
                        v.nav.selected.clear();
                        v.clicked.clear();
                    }
                }
                Action::DialogOk => self.dialog_ok(),
                Action::DialogCancel => {
                    self.end_preview();
                    self.shell.dialog = None;
                }
                Action::StartRename(id) => self.shell.start_rename(id),
                Action::Delete(item) => self.tool(Call::Delete { item }),
                Action::Suppress(item) => self.tool(Call::Suppress { item }),
                Action::Unsuppress(item) => self.tool(Call::Unsuppress { item }),
                Action::Reorder(item, before) => self.tool(Call::Move { item, before }),
                Action::EndAt(to) => {
                    self.shell.end_at = to;
                    if let Some(l) = self.view.as_mut().and_then(|v| v.live.as_mut()) {
                        l.roll(to);
                    }
                }
                Action::DimDrag(value) => self.dim_drag(value),
                Action::DimCommit(value) => self.dim_commit(value),
                Action::DimCancel => self.dim_cancel(),
                Action::Fg(_) | Action::Inspect(_) => {}
                Action::Sheet(c) if c != crate::shell::sheet::SheetCmd::NewPart => self.sheet_cmd(c),
                other => host.push(other),
            }
        }
        if let Some(text) = self.shell.dim_typed.take() {
            self.dim_type(text);
        }
        if let Some((item, name)) = self.shell.renamed.take() {
            self.tool(Call::Rename { item, name });
        }
        self.dialog_preview();
        host
    }

    /// The sketch a new feature would read: the one chosen, else the last that nothing uses.
    fn free_sketch(&self) -> Option<String> {
        let doc = self.shell.doc.as_ref()?;
        let name = |r: &crate::shell::Row| doc.name(r.id);
        let used = |r: &crate::shell::Row| {
            self.shell
                .rows
                .iter()
                .any(|o| matches!(&o.sketch, Some(SketchChild::Line(id, _)) if *id == r.id))
        };
        let sketches = || self.shell.rows.iter().filter(|r| r.kind == "sketch");
        sketches()
            .find(|r| Some(r.id) == self.shell.selected)
            .and_then(name)
            .or_else(|| sketches().rfind(|r| !used(r)).and_then(name))
    }

    fn start_sketch(&mut self) {
        // on what's chosen, if it's a plane
        let head = self.shell.origin_selected.and_then(|i| match i {
            crate::origin::Item::XyPlane | crate::origin::Item::XzPlane | crate::origin::Item::YzPlane => {
                Some(i.reference().to_string())
            }
            _ => None,
        });
        match head {
            Some(h) => self.ask_plane(h, SketchFor::New, Vec::new()),
            None => {
                self.shell.sketch_pick = true;
                self.shell.selected = None;
                self.shell.status =
                    "Pick a plane, a planar face, or a sketch to edit. Esc cancels".to_string();
            }
        }
    }

    fn ask_plane(&mut self, head: String, edit: SketchFor, rows: Vec<String>) {
        self.shell.sketch_pick = false;
        let at = place_of(&self.shell, &edit);
        let refs: Vec<String> = rows.iter().filter_map(|r| project_refs(r)).collect();
        self.sketch_for = Some((head.clone(), edit, rows));
        if let Some(v) = &mut self.view {
            if !refs.is_empty()
                && let Some(l) = &mut v.live
            {
                l.sketch_sources(at, refs, false);
            }
            if let Some(l) = &mut v.live {
                l.sketch_plane(head);
            }
        }
    }

    /// A sketch from the tree: its own rows on its own plane.
    fn edit_sketch(&mut self, id: forge_doc::ItemId) {
        let Some(item) = self
            .shell
            .doc
            .as_ref()
            .and_then(|d| d.items.iter().find(|i| i.id == id))
        else {
            return;
        };
        let forge_doc::Body::Sketch { head, rows } = &item.body else {
            return;
        };
        let head: Vec<&str> = head.iter().map(|a| a.text.as_str()).collect();
        let rows: Vec<String> = rows
            .iter()
            .filter(|r| !r.args.is_empty())
            .map(|r| r.args.join(" "))
            .collect();
        self.ask_plane(head.join(" "), SketchFor::Line(id), rows);
    }

    /// A feature's inline profile, in sketch mode: Finish writes it as a sketch line.
    fn edit_inline(&mut self, id: forge_doc::ItemId) {
        let Some(row) = self.shell.rows.iter().find(|r| r.id == id) else {
            return;
        };
        let Some(SketchChild::Inline(p, _)) = &row.sketch else {
            return;
        };
        let (head, rest, rows) = (p.head.clone(), p.rest.clone(), p.rows.clone());
        self.ask_plane(head, SketchFor::Inline(id, rest), rows);
    }

    fn finish_sketch(&mut self) {
        let Some(m) = self.shell.sketch.take() else { return };
        // nothing drawn: nothing to keep
        if m.is_new() && m.rows.is_empty() {
            self.shell.then_feature = None;
            self.shell.tab = 0;
            self.shell.status.clear();
            return;
        }
        // looked at and left as it was: nothing to commit
        self.shell.status.clear();
        if m.unchanged() && m.edit != SketchFor::New {
            self.shell.tab = 0;
            return;
        }
        let Some(mut doc) = self.shell.doc.clone() else {
            return;
        };
        // a new sketch line goes above what reads it, or at the end-of-part marker
        let marker = self.shell.end_at.and_then(|e| {
            let k = doc.items.iter().position(|i| i.id == e)?;
            Some(doc.items.get(k + 1)?.id)
        });
        let mut items = doc.read_items(&m.block());
        let calls = match &m.edit {
            SketchFor::Line(id) => {
                let [item] = &mut items[..] else { return };
                let Some(old) = doc.items.iter().find(|i| i.id == *id) else {
                    return;
                };
                item.id = *id;
                item.label = old.label.clone();
                item.note = old.note.clone();
                vec![Call::Edit { item: item.clone() }]
            }
            SketchFor::Inline(id, rest) => {
                match crate::shell::panel::inline_calls(&doc, *id, &m.head, &m.rows, rest) {
                    Some(calls) => calls,
                    None => return,
                }
            }
            SketchFor::New => vec![Call::Add {
                before: marker,
                items,
            }],
        };
        if let Some(v) = &mut self.view {
            self.calls.extend(calls.iter().cloned());
            if let Some(l) = &mut v.live {
                l.call(calls);
            }
        }
        self.shell.tab = 0;
        self.after_sketch = self.shell.then_feature.take();
    }

    /// What a dialog's fields now say, as a tool call.
    fn dialog_call(&self) -> Option<Call> {
        let d = self.shell.dialog.as_ref()?;
        if d.waiting_for_pick() {
            return None;
        }
        let mut doc = self.shell.doc.clone()?;
        let mut items = doc.read_items(&d.line());
        match d.target {
            DialogTarget::Edit(id) => {
                let [item] = &mut items[..] else { return None };
                let old = doc.items.iter().find(|i| i.id == id)?;
                item.id = id;
                item.label = old.label.clone();
                item.note = old.note.clone();
                Some(Call::Edit { item: item.clone() })
            }
            DialogTarget::New(_) => {
                // a new feature goes in at the end-of-part marker
                let before = self.shell.end_at.and_then(|e| {
                    let k = self.shell.doc.as_ref()?.items.iter().position(|i| i.id == e)?;
                    Some(self.shell.doc.as_ref()?.items.get(k + 1)?.id)
                });
                Some(Call::Add { before, items })
            }
        }
    }

    /// Fields changed: the live result of what they say.
    fn dialog_preview(&mut self) {
        let Some(d) = &self.shell.dialog else { return };
        let line = d.line();
        if d.sent.as_deref() == Some(line.as_str()) || d.awaiting {
            return;
        }
        let call = self.dialog_call();
        let Some(v) = &mut self.view else { return };
        let Some(d) = &mut self.shell.dialog else { return };
        d.sent = Some(line);
        d.error = None;
        match call {
            Some(call) => {
                d.seq = v.live.as_mut().map_or(0, |l| l.preview(vec![call]));
                v.previewing = true;
            }
            // nothing to show yet: what was shown goes
            None if v.previewing => {
                v.previewing = false;
                if let Some(l) = &mut v.live {
                    l.preview_end();
                }
            }
            None => {}
        }
    }

    fn end_preview(&mut self) {
        if let Some(v) = &mut self.view
            && v.previewing
        {
            v.previewing = false;
            if let Some(l) = &mut v.live {
                l.preview_end();
            }
        }
    }

    /// OK: the same call commits; the dialog waits for the answer.
    fn dialog_ok(&mut self) {
        let Some(call) = self.dialog_call() else { return };
        let Some(v) = &mut self.view else { return };
        let Some(d) = &mut self.shell.dialog else { return };
        d.awaiting = true;
        v.previewing = false;
        self.calls.push(call.clone());
        if let Some(l) = &mut v.live {
            l.call(vec![call]);
        }
    }

    fn param_names(&self) -> Vec<String> {
        self.shell.doc.as_ref().map_or(Vec::new(), |d| {
            d.items
                .iter()
                .filter_map(|i| match &i.body {
                    forge_doc::Body::Param { name, .. } => Some(name.clone()),
                    _ => None,
                })
                .collect()
        })
    }

    /// The part reads in this unit: a comment on its first line, which nothing builds from.
    fn set_units(&mut self, unit: &'static str) {
        let Some(mut doc) = self.shell.doc.clone() else {
            return;
        };
        let was = doc.items.iter().find(|i| {
            matches!(i.body, forge_doc::Body::Blank)
                && i.note
                    .as_deref()
                    .is_some_and(|n| n.starts_with(forge_doc::units::LINE))
        });
        let was = was.map(|i| i.id);
        let first = doc.items.first().map(|i| i.id);
        let items = doc.read_items(&format!("# {}{unit}", forge_doc::units::LINE));
        let call = match (was, items.into_iter().next()) {
            (Some(id), Some(mut item)) => {
                item.id = id;
                Call::Edit { item }
            }
            (None, Some(item)) => Call::Add {
                before: first,
                items: vec![item],
            },
            _ => return,
        };
        self.shell.units = unit;
        self.tool(call);
    }

    /// A tool call for the worker, kept for replay.
    fn tool(&mut self, call: Call) {
        if let Some(v) = &mut self.view {
            self.calls.push(call.clone());
            if let Some(l) = &mut v.live {
                l.call(vec![call]);
            }
        }
    }

    fn dim_drag(&mut self, value: f64) {
        let (Some(v), Some(e)) = (&mut self.view, &mut self.shell.dim) else {
            return;
        };
        e.value = value;
        v.live_drag = true;
        if let Some(l) = &mut v.live {
            l.drag(e.handle.at.op(short(value)));
        }
    }

    /// The tool call that sets this number: the same one an agent would send.
    fn call_of(at: Target, value: String) -> Call {
        match at {
            Target::Param(item) => Call::SetParam { item, expr: value },
            Target::Arg(item, arg) => Call::SetArg {
                item,
                arg,
                text: value,
            },
            Target::Row(item, row) => Call::SetDim { item, row, value },
        }
    }

    fn dim_commit(&mut self, value: f64) {
        let (Some(v), Some(e)) = (&mut self.view, &mut self.shell.dim) else {
            return;
        };
        e.dragging = false;
        if let Some(l) = &mut v.live {
            l.commit(e.handle.at.op(short(value)));
        }
        self.calls.push(Self::call_of(e.handle.at, short(value)));
    }

    fn dim_cancel(&mut self) {
        let (Some(v), Some(e)) = (&mut self.view, &mut self.shell.dim) else {
            return;
        };
        e.dragging = false;
        e.value = e.handle.value;
        e.error = None;
        if let Some(l) = &mut v.live {
            l.cancel();
        }
    }

    fn dim_type(&mut self, text: String) {
        let (Some(v), Some(e)) = (&mut self.view, &mut self.shell.dim) else {
            return;
        };
        let call = Self::call_of(e.handle.at, text);
        self.calls.push(call.clone());
        if let Some(l) = &mut v.live {
            l.call(vec![call]);
        }
    }

    /// Home and the cube's names, beside the part in `PART.fc.view`: they're no part of what builds.
    fn save_view(&self) {
        let (Some(path), Some(v)) = (&self.view_file, &self.view) else {
            return;
        };
        let mut out = format!("orient {}\n", v.nav.orient.words());
        if let Some(h) = &v.nav.home_view {
            out.push_str(&format!(
                "home {} {} {} {} {} {} {} {} {}\n",
                h.target[0], h.target[1], h.target[2], h.yaw, h.pitch, h.roll, h.dist, h.fovy, h.parallel
            ));
        }
        let mut eyes: Vec<_> = self.shell.sketch_eye.iter().collect();
        eyes.sort();
        for (name, on) in eyes {
            out.push_str(&format!("eye {name} {on}\n"));
        }
        if let Err(why) = std::fs::write(path, out) {
            eprintln!("forge-app: can't keep the view beside the part: {why}");
        }
    }

    /// What's kept beside the part, if anything is: a file it can't read is one it doesn't have.
    fn load_view(
        path: &str,
    ) -> (
        crate::viewcube::Orient,
        Option<Camera>,
        std::collections::HashMap<String, bool>,
    ) {
        let mut orient = crate::viewcube::Orient::default();
        let mut home = None;
        let mut eyes = std::collections::HashMap::new();
        for line in std::fs::read_to_string(path).unwrap_or_default().lines() {
            match line.split_once(' ') {
                Some(("orient", w)) => orient = crate::viewcube::Orient::from_words(w).unwrap_or(orient),
                Some(("home", w)) => {
                    let words: Vec<&str> = w.split(' ').collect();
                    let nums: Vec<f32> = words.iter().take(8).filter_map(|t| t.parse().ok()).collect();
                    if let ([x, y, z, yaw, pitch, roll, dist, fovy], Some(par)) = (&nums[..], words.get(8)) {
                        home = Some(Camera {
                            target: [*x, *y, *z],
                            yaw: *yaw,
                            pitch: *pitch,
                            roll: *roll,
                            dist: *dist,
                            fovy: *fovy,
                            parallel: *par == "true",
                        });
                    }
                }
                Some(("eye", w)) => {
                    if let Some((name, on)) = w.split_once(' ') {
                        eyes.insert(name.to_string(), on == "true");
                    }
                }
                _ => {}
            }
        }
        (orient, home, eyes)
    }

    fn apply_view(&mut self, a: Action) {
        if let Action::Tool(t) = a {
            self.shell.view.tool = t;
        }
        if let Action::Parallel(on) = a {
            self.shell.view.parallel = on;
        }
        let Some(v) = &mut self.view else { return };
        match a {
            Action::FitAll => v.nav.fit_all(&v.scene),
            Action::Home => v.nav.home(&v.scene),
            Action::SetHome => {
                v.nav.home_view = Some(v.nav.camera);
            }
            Action::SetHomeAt(r) => {
                let (c, radius) = v.scene.bounds();
                let mut to = crate::nav::region_view_as(v.nav.camera, r, &v.nav.orient);
                to.target = c;
                to.fit_to(radius * 1.1, v.size.0, v.size.1);
                v.nav.home_view = Some(to);
            }
            Action::SetFaceAs(face, name) => {
                v.nav.set_face_as(face, name);
            }
            Action::ResetOrientation => {
                v.nav.orient = Default::default();
            }
            Action::Parallel(on) => v.nav.set_parallel(on, &v.scene),
            Action::Lines(mode) => {
                v.renderer.line_mode = mode;
                self.shell.view.lines = mode;
            }
            Action::Section => {
                v.nav.key(winit::keyboard::KeyCode::KeyS, &v.scene);
                self.shell.view.section = v.nav.section.is_some();
            }
            Action::Tool(t) => v.nav.tool = t,
            _ => {}
        }
        v.dirty = true;
        if matches!(
            a,
            Action::SetHome | Action::SetHomeAt(_) | Action::SetFaceAs(..) | Action::ResetOrientation
        ) {
            self.save_view();
        }
    }

    /// One whole frame into `target`, `size` device pixels.
    pub fn frame(&mut self, raw: egui::RawInput, target: &wgpu::TextureView, size: (u32, u32)) -> FrameOut {
        let t0 = now_ms();
        let mut times = Times::default();
        self.target_px = size;
        if let Some(why) = self.gpu_failed.lock().expect("no panics while held").as_ref() {
            self.shell.status = format!("GPU error: {}", why.lines().next().unwrap_or(""));
            self.view = None;
            self.shell.view_tex = None;
        }
        self.take_loaded();
        self.take_live();
        self.take_fg();
        self.take_extras(false);
        times.events = now_ms() - t0;
        let mut animating = false;
        if let Some(v) = &mut self.view {
            if v.nav.tick(now_ms()) {
                v.dirty = true;
                animating = true;
            }
            if let Some(t) = v.nav.explode_tick(now_ms()) {
                v.scene.set_explode(t);
                v.dirty = true;
                animating = true;
            }
            let place = v.nav.cube_place();
            self.shell.cube_labels = viewcube::labels(&v.nav.camera, &place, v.nav.cube_hover, &v.nav.orient);
            self.shell.cube_place = Some(place);
        }
        self.sync_selection();
        if let Some(v) = &self.view {
            self.shell.camera = Some((v.nav.camera, v.size));
        }
        let was_selected = self.shell.selected;
        let shell = &mut self.shell;
        let mut output = self.ctx.run_ui(raw, |ui| shell.ui(ui));
        if self.shell.selected != was_selected
            && let Some(v) = &mut self.view
        {
            // a row of the tree was chosen: the view's click no longer says
            v.nav.selected.clear();
            v.clicked.clear();
            v.dirty = true;
        }
        self.sync_selection();
        self.pixels_per_point = output.pixels_per_point;
        let host = self.apply();
        self.apply_fg();
        self.save_settings();
        self.inspect_tick();
        self.sheet_frame();
        self.fg_preview();
        if self.explode_tick() {
            animating = true;
        }
        self.body_eye();
        times.ui = now_ms() - t0;

        if self.shell.tree_hover != self.tree_hover_was {
            self.tree_hover_was = self.shell.tree_hover;
            if let Some(v) = &mut self.view {
                v.dirty = true;
            }
        }
        self.ask_extras();
        if self.extras_changed()
            && let Some(v) = &mut self.view
        {
            v.dirty = true;
        }
        let drawn = self.shell.origin_drawn();
        if let Some(v) = &mut self.view
            && v.origin_was != drawn
        {
            v.origin_was = drawn;
            v.dirty = true;
        }
        if let Some(v) = &self.view {
            self.shell.part_middle = v.scene.bounds().0;
            let (at, p) = (self.shell.view_rect.min, self.pixels_per_point);
            let on_screen = |q: (f32, f32)| at + egui::vec2(q.0, q.1) / p;
            let free = self
                .shell
                .fg
                .as_ref()
                .and_then(|p| p.asm.as_ref())
                .is_some_and(|a| matches!(a.tool, Some(crate::shell::asmtools::AsmTool::Free(_))));
            self.shell.window_box = v
                .nav
                .window_box()
                .filter(|_| !free)
                .map(|(a, b)| (on_screen(a), on_screen(b)));
            // held still, the menu shows; a flick needn't wait for it
            match v.nav.marking_from() {
                Some(from) => {
                    let t = now_ms();
                    let since = *self.mark_held.get_or_insert(t);
                    if t - since > MARK_SHOW_MS && self.shell.marking.is_none() {
                        self.shell.marking = Some(on_screen(from));
                    }
                }
                None => self.mark_held = None,
            }
        }
        let sketches = self.shell.sketches_drawn();
        if let Some(v) = &mut self.view
            && v.sketches_was != sketches
        {
            v.sketches_was = sketches;
            v.dirty = true;
        }
        let want_view = self.view_px();
        if let Some(v) = &mut self.view {
            if want_view != v.size {
                eprintln!(
                    "forge-app: view {}x{} in a window of {}x{}",
                    want_view.0, want_view.1, size.0, size.1
                );
                v.size = want_view;
                v.renderer.resize(want_view);
                v.target = Offscreen::new(&self.device, want_view);
                let raw = raw_view(&v.target);
                self.egui.update_egui_texture_from_wgpu_texture(
                    &self.device,
                    &raw,
                    wgpu::FilterMode::Nearest,
                    v.tex,
                );
                v.nav.size = want_view;
                v.dirty = true;
            }
            v.nav.scale = self.pixels_per_point;
        }
        if let Some(mut v) = self.view.take() {
            self.theme_view(&mut v.renderer);
            self.view = Some(v);
        }
        let t1 = now_ms();
        if self.view.as_ref().is_some_and(|v| v.dirty) {
            self.inspect_section();
        }
        let (extra, extra_behind) = if self.view.as_ref().is_some_and(|v| v.dirty) {
            self.overlay_world()
        } else {
            (Vec::new(), Vec::new())
        };
        if let Some(v) = &mut self.view
            && v.dirty
        {
            let mut world = extra;
            let (c, r) = v.scene.bounds();
            for (item, strong) in &v.origin_was {
                crate::origin::tris(
                    *item,
                    crate::origin::reach(c, r),
                    &v.nav.camera,
                    v.size,
                    *strong,
                    &mut world,
                );
            }
            // a subassembly's origin, at its pose, as far as its parts reach
            if let Some(a) = self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()) {
                for (sub, k) in &a.sub_origins {
                    let Some(pose) = a.pose_of(sub) else { continue };
                    let rows = crate::fg::rows_of(&pose);
                    let at = [rows[0][3], rows[1][3], rows[2][3]];
                    let far = (0..a.occ.len())
                        .filter(|&o| crate::shell::fgasm::under(&a.occ[o].path, sub))
                        .filter_map(|o| self.fg_asm.as_ref()?.0.get(o)?.placement)
                        .filter_map(|pl| self.fg_asm.as_ref()?.1.get(pl))
                        .map(|(_, q)| {
                            let d = [q[0][3] - at[0], q[1][3] - at[1], q[2][3] - at[2]];
                            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                        })
                        .fold(20.0f32, f32::max);
                    let mut local = vec![];
                    crate::origin::tris(
                        crate::origin::ITEMS[*k],
                        far * 0.6,
                        &v.nav.camera,
                        v.size,
                        false,
                        &mut local,
                    );
                    for w in &mut local {
                        let p = w.at;
                        w.at = [0, 1, 2]
                            .map(|i| rows[i][0] * p[0] + rows[i][1] * p[1] + rows[i][2] * p[2] + rows[i][3]);
                    }
                    world.extend(local);
                }
            }
            let mut behind = extra_behind;
            {
                let (t, px) = (self.shell.theme, self.pixels_per_point);
                let rgba = |c: egui::Color32, a: f32| {
                    let hex = (c.r() as u32) << 16 | (c.g() as u32) << 8 | c.b() as u32;
                    let rgb = viewcube::rgb(hex);
                    [rgb[0], rgb[1], rgb[2], a]
                };
                let (cam, size) = (&v.nav.camera, v.size);
                for &(k, lit) in &v.sketches_was {
                    let Some(s) = self.shell.sketch_shapes.get(k) else {
                        continue;
                    };
                    let ink = if lit { t.focus } else { t.sketch };
                    let wide = px * if lit { 3.0 } else { 2.25 };
                    let dim = if lit { 0.5 } else { 0.18 };
                    for (curve, faint) in &s.curves {
                        let pts: Vec<[f32; 3]> = curve
                            .iter()
                            .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
                            .collect();
                        let a = if *faint { 0.5 } else { 1.0 };
                        crate::origin::polyline(&pts, wide, rgba(ink, a), cam, size, &mut world);
                        crate::origin::polyline(&pts, wide, rgba(ink, a * dim), cam, size, &mut behind);
                    }
                    for p in &s.points {
                        let at = [p[0] as f32, p[1] as f32, p[2] as f32];
                        crate::origin::disc(at, px * 4.5, rgba(ink, 1.0), cam, size, &mut world);
                        crate::origin::disc(at, px * 4.5, rgba(ink, dim), cam, size, &mut behind);
                    }
                }
            }
            let marks = if v.live_drag {
                Vec::new()
            } else {
                let quiet = self
                    .shell
                    .sketch
                    .as_ref()
                    .is_some_and(|m| m.tool != Some(SkTool::Project))
                    || self.shell.sketch_hover.is_some();
                let mut m: Vec<Mark> = v
                    .nav
                    .marks()
                    .into_iter()
                    .filter(|m| !m.selected && !quiet)
                    .collect();
                // with a dialog up the picks are what's marked
                if self.shell.dialog.is_some() {
                    m.extend(v.nav.selected.iter().map(|&(placement, entity)| Mark {
                        placement,
                        entity,
                        selected: true,
                    }));
                }
                let of = self
                    .shell
                    .selected
                    .filter(|_| self.shell.dialog.is_none())
                    .map_or(Vec::new(), |id| v.facts.faces_of(id));
                m.extend(of.into_iter().map(|f| Mark {
                    placement: 0,
                    entity: Entity::Face(f),
                    selected: true,
                }));
                // what's at fault on a chosen feature: as it's named on the part shown
                let at_fault: Vec<Entity> = self
                    .shell
                    .flags
                    .iter()
                    .filter(|f| Some(f.item) == self.shell.selected)
                    .filter_map(|f| f.entity)
                    .flat_map(|e| fault_marks(&v.facts, e))
                    .collect();
                m.extend(at_fault.into_iter().map(|entity| Mark {
                    placement: 0,
                    entity,
                    selected: false,
                }));
                let over = self.shell.tree_hover.filter(|h| Some(*h) != self.shell.selected);
                m.extend(
                    over.map_or(Vec::new(), |id| v.facts.faces_of(id))
                        .into_iter()
                        .map(|f| Mark {
                            placement: 0,
                            entity: Entity::Face(f),
                            selected: false,
                        }),
                );
                m
            };
            let overlay = Overlay {
                hud: None,
                cube: Some((v.nav.cube_place(), v.nav.cube_hover)),
                marks: &marks,
                world: &world,
                behind: &behind,
            };
            v.renderer
                .frame(&v.target.view, &mut v.scene, &v.nav.camera, overlay);
            v.dirty = false;
            times.view_drawn = true;
        }
        times.view = now_ms() - t1;

        let t2 = now_ms();
        let prims = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        times.tess = now_ms() - t2;
        let t3 = now_ms();
        if let Some(mut w) = self.warm_textures.take() {
            for (id, deltas) in &w.set {
                for d in deltas.iter() {
                    self.egui.update_texture(&self.device, &self.queue, *id, d);
                }
            }
            w.clear();
        }
        for (id, deltas) in &output.textures_delta.set {
            for d in deltas.iter() {
                self.egui.update_texture(&self.device, &self.queue, *id, d);
            }
        }
        let sd = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.0, size.1],
            pixels_per_point: output.pixels_per_point,
        };
        let mut enc = self.device.create_command_encoder(&Default::default());
        let more = self
            .egui
            .update_buffers(&self.device, &self.queue, &mut enc, &prims, &sd);
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("window"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui.render(&mut pass, &prims, &sd);
        }
        self.queue.submit(more.into_iter().chain([enc.finish()]));
        for id in &output.textures_delta.free {
            self.egui.free_texture(id);
        }
        output.textures_delta.clear();
        times.draw = now_ms() - t3;

        let delay = output
            .viewport_output
            .values()
            .next()
            .map(|v| v.repaint_delay)
            .unwrap_or(std::time::Duration::MAX);
        let again_in = if animating || self.mark_held.is_some() || self.view.as_ref().is_some_and(|v| v.dirty)
        {
            Some(std::time::Duration::ZERO)
        } else if self.shell.checking_since.is_some() {
            // to say so if it goes on
            Some(delay.min(std::time::Duration::from_millis(200)))
        } else if delay == std::time::Duration::MAX {
            None
        } else {
            Some(delay)
        };
        FrameOut {
            platform: output.platform_output,
            host,
            again_in,
            times,
        }
    }

    pub fn device(&self) -> wgpu::Device {
        self.device.clone()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn view_dirty(&self) -> bool {
        self.view.as_ref().is_some_and(|v| v.dirty)
    }

    /// Is a load or a worker job still out? Only tests and shots need to ask.
    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }
}

fn raw_view(t: &Offscreen) -> wgpu::TextureView {
    t.texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(wgpu::TextureFormat::Rgba8Unorm),
        ..Default::default()
    })
}

/// Square on to a plane, as if drawing on a sheet.
fn look_at(v: &mut View, plane: forge_math::Frame) {
    let z: forge_math::Vec3 = plane.z().into();
    let to = crate::nav::sheet_view(v.nav.camera, [z.x as f32, z.y as f32, z.z as f32], &v.nav.orient);
    v.nav.go_to(to);
    v.dirty = true;
}

/// How long the right button stays down before its menu shows, in ms.
const MARK_SHOW_MS: f64 = 220.0;

/// The entities of the part shown with the name a check gave: none if a later feature took it.
fn fault_marks(facts: &Facts, e: forge_topo::Entity) -> Vec<Entity> {
    use forge_topo::Entity as T;
    let find = |names: &[u64], n: u64| -> Vec<u32> {
        names
            .iter()
            .enumerate()
            .filter(|x| *x.1 == n)
            .map(|x| x.0 as u32)
            .collect()
    };
    match e {
        T::Face(n) => find(&facts.face_names, n.0)
            .into_iter()
            .map(Entity::Face)
            .collect(),
        T::Edge(n) => find(&facts.edge_names, n.0)
            .into_iter()
            .map(Entity::Edge)
            .collect(),
        T::Vertex(n) => find(&facts.vertex_names, n.0)
            .into_iter()
            .map(Entity::Vertex)
            .collect(),
        T::Named(n) => {
            let mut all: Vec<Entity> = find(&facts.face_names, n.0)
                .into_iter()
                .map(Entity::Face)
                .collect();
            all.extend(find(&facts.edge_names, n.0).into_iter().map(Entity::Edge));
            all.extend(find(&facts.vertex_names, n.0).into_iter().map(Entity::Vertex));
            all
        }
    }
}

/// A picture read back from the target.
pub struct Pixels {
    width: usize,
    bytes: Vec<u8>,
}

impl Pixels {
    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let k = (y * self.width + x) * 3;
        [self.bytes[k], self.bytes[k + 1], self.bytes[k + 2]]
    }
}

/// The app with no window: input by hand, a texture for a target. What `--shot`, `--bench` and the
/// tests run. Clicks go by the shell's own names for what it drew.
pub struct Headless {
    pub gui: Gui,
    size: (f32, f32),
    scale: f32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    device: wgpu::Device,
    queue: wgpu::Queue,
    time: f64,
    pending: Vec<egui::Event>,
    pointer: egui::Pos2,
}

impl Headless {
    pub fn new(size: (f32, f32), scale: f32) -> Result<Headless, String> {
        let instance = render::native_instance();
        let (device, queue, adapter) = render::block_on(render::open_device(&instance, None))?;
        let px = ((size.0 * scale).round() as u32, (size.1 * scale).round() as u32);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("window"),
            size: wgpu::Extent3d {
                width: px.0,
                height: px.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HEADLESS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let gui = Gui::new(
            device.clone(),
            queue.clone(),
            adapter.get_info(),
            HEADLESS_FORMAT,
            Arc::new(|| {}),
            scale,
        );
        Ok(Headless {
            gui,
            size,
            scale,
            texture,
            view,
            device,
            queue,
            time: 0.0,
            pending: Vec::new(),
            pointer: egui::Pos2::ZERO,
        })
    }

    fn px(&self) -> (u32, u32) {
        (
            (self.size.0 * self.scale).round() as u32,
            (self.size.1 * self.scale).round() as u32,
        )
    }

    /// One frame with what's been queued; waits for the GPU, so a time is a real one.
    pub fn step(&mut self) -> FrameOut {
        self.gui.ctx.set_pixels_per_point(self.scale);
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.size.0, self.size.1),
            )),
            time: Some(self.time),
            focused: true,
            events: std::mem::take(&mut self.pending),
            ..Default::default()
        };
        self.time += 1.0 / 60.0;
        let view = self.view.clone();
        let px = self.px();
        let out = self.gui.frame(raw, &view, px);
        self.gui.picks();
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        self.gui.picks();
        out
    }

    /// Frames until the view's part is in and the worker has said what it is.
    pub fn settle(&mut self) {
        for _ in 0..20_000 {
            self.step();
            if !self.gui.loading() && self.gui.shell.doc.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        let p = egui::pos2(x, y);
        self.pointer = p;
        self.pending.push(egui::Event::PointerMoved(p));
        let px = PhysicalPosition::new((x * self.scale) as f64, (y * self.scale) as f64);
        self.gui.view_event(&WindowEvent::CursorMoved {
            device_id: winit::event::DeviceId::dummy(),
            position: px,
        });
    }

    pub fn button(&mut self, down: bool) {
        self.pending.push(egui::Event::PointerButton {
            pos: self.pointer,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: Default::default(),
        });
        self.gui.view_event(&WindowEvent::MouseInput {
            device_id: winit::event::DeviceId::dummy(),
            state: if down {
                ElementState::Pressed
            } else {
                ElementState::Released
            },
            button: MouseButton::Left,
        });
    }

    /// A right click, or the right button pressed here and let go there.
    pub fn right_drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.move_to(from.0, from.1);
        self.step();
        self.other_button(true);
        self.step();
        self.move_to(to.0, to.1);
        self.step();
        self.other_button(false);
        self.step();
    }

    /// The right button, held or let go, where the pointer is.
    pub fn other_button_for_test(&mut self, down: bool) {
        self.other_button(down);
    }

    fn other_button(&mut self, down: bool) {
        self.pending.push(egui::Event::PointerButton {
            pos: self.pointer,
            button: egui::PointerButton::Secondary,
            pressed: down,
            modifiers: Default::default(),
        });
        self.gui.view_event(&WindowEvent::MouseInput {
            device_id: winit::event::DeviceId::dummy(),
            state: if down {
                ElementState::Pressed
            } else {
                ElementState::Released
            },
            button: MouseButton::Right,
        });
    }

    /// A left click here.
    pub fn click_at(&mut self, x: f32, y: f32) {
        self.move_to(x, y);
        self.step();
        self.button(true);
        self.step();
        self.button(false);
        self.step();
    }

    /// A right click where the pointer is, with no move first.
    pub fn right_click_here(&mut self) {
        self.other_button(true);
        self.step();
        self.other_button(false);
        self.step();
    }

    /// A key press and release.
    pub fn key(&mut self, key: egui::Key) {
        self.key_with(key, Default::default());
    }

    pub fn key_with(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        for pressed in [true, false] {
            self.pending.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            });
        }
    }

    pub fn text(&mut self, s: &str) {
        self.pending.push(egui::Event::Text(s.to_string()));
    }

    /// Frames until this holds, or a good while.
    pub fn until(&mut self, what: impl Fn(&Gui) -> bool) -> bool {
        for _ in 0..5000 {
            self.step();
            if what(&self.gui) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        false
    }

    /// Queues a click at the middle of what the shell last drew under this name.
    pub fn click(&mut self, name: &str) -> bool {
        let Some(r) = self.gui.shell.place(name) else {
            return false;
        };
        self.move_to(r.center().x, r.center().y);
        // egui takes hover first, as a real pointer gives it
        self.step();
        self.button(true);
        self.step();
        self.button(false);
        true
    }

    /// The frame as PPM bytes.
    /// What's on the target now, to read pixels of.
    pub fn gui_pixels(&self) -> Pixels {
        let ppm = self.ppm();
        // P6, width and height, 255, then the bytes
        let mut parts = ppm.splitn(4, |b| *b == b'\n');
        let _ = parts.next();
        let dims = parts.next().unwrap_or_default();
        let width = String::from_utf8_lossy(dims)
            .split(' ')
            .next()
            .and_then(|w| w.parse().ok())
            .unwrap_or(1);
        let _ = parts.next();
        Pixels {
            width,
            bytes: parts.next().unwrap_or_default().to_vec(),
        }
    }

    pub fn ppm(&self) -> Vec<u8> {
        let (w, h) = self.px();
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut e = self.device.create_command_encoder(&Default::default());
        e.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([e.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let v = buffer.slice(..).get_mapped_range().expect("mapped");
        let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let k = y * row as usize + x * 4;
                out.extend_from_slice(&v[k..k + 3]);
            }
        }
        out
    }
}

/// What only an .fc part's worker can do (its dialogs, its tree's items, its dimensions on the
/// model): a Forge part's edits are its panel's typed ops, its tools' forms, its sketches.
fn fc_only(a: Action) -> bool {
    matches!(
        a,
        Action::EditInline(_)
            | Action::DialogOk
            | Action::DialogCancel
            | Action::Edit(_)
            | Action::Delete(_)
            | Action::Suppress(_)
            | Action::Unsuppress(_)
            | Action::Reorder(..)
            | Action::EndAt(_)
            | Action::StartRename(_)
            | Action::DimDrag(_)
            | Action::DimCommit(_)
            | Action::DimCancel
    )
}

impl Gui {
    /// Opens a Forge (.fg) part: its worker builds it at once, and keeps its stages warm after.
    fn open_fg(&mut self, path: &str) {
        let gpu = (self.device.clone(), self.queue.clone(), self.adapter.clone());
        let waker = self.waker.clone();
        let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(move || waker());
        match crate::fg::Session::open(path, Some(gpu), waker) {
            Ok(s) => {
                let name = name_of(path);
                self.shell.fg = Some(crate::shell::fg::FgPanel {
                    file: name.clone(),
                    assembly: crate::fg::is_assembly(&s.src),
                    source: s.src.clone(),
                    ..Default::default()
                });
                self.fg = Some(s);
                self.fg_view = None;
                self.fg_folded = None;
                self.fg_flat = None;
                self.fg_sheets.clear();
                self.fg_asm = None;
                self.explode = Default::default();
                self.fg_keep_camera = None;
                self.loading = None;
                self.view = None;
                self.shell.view_tex = None;
                self.shell.doc = None;
                self.shell.rows.clear();
                self.shell.selected = None;
                self.shell.dialog = None;
                self.shell.dim = None;
                self.shell.sketch = None;
                self.shell.sketch_shapes.clear();
                self.shell.flags.clear();
                self.shell.sheet = crate::shell::sheet::SheetUi::new();
                if self.shell.tab > 4 {
                    self.shell.tab = 0;
                }
                self.shell.name = name.clone();
                self.shell.status = format!("Opening {name}");
            }
            Err(why) => self.shell.status = why,
        }
    }

    /// The panel's text and dirty mark, after the session's text changed.
    fn fg_text(&mut self) {
        if let (Some(s), Some(p)) = (&self.fg, &mut self.shell.fg) {
            p.source = s.src.clone();
            p.dirty = s.dirty();
        }
    }

    /// What the Forge worker has built: the panel's rows, the part's mesh.
    fn take_fg(&mut self) {
        // a part's anchors, found off the frame's thread
        if let Some(r) = &self.fg_asm_anchors
            && let Ok(got) = r.try_recv()
        {
            self.fg_asm_anchors = None;
            if let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
                a.anchors = Some(got);
            }
        }
        let mut asm_moved = false;
        let Some(s) = &mut self.fg else { return };
        let mut changed = false;
        let flat_shown = self.shell.fg.as_ref().is_some_and(|p| p.flat);
        let mut inspect = None;
        for m in s.poll() {
            let b = match m {
                crate::fg::Msg::Built(b) => *b,
                crate::fg::Msg::Warm(d) => {
                    // the faces as they are: the live set's start, not shown
                    if let Some(r) = self
                        .view
                        .as_mut()
                        .map(|v| &mut v.renderer)
                        .or(self.fg_view.as_mut().map(|l| &mut l.renderer))
                    {
                        r.live_apply(0, &d);
                        r.live_show(false);
                    }
                    continue;
                }
                // an older shape's, a newer one's faces already on screen
                crate::fg::Msg::Final { seq, .. } if seq != s.changed => continue,
                crate::fg::Msg::Final { mesh, flat, .. } => {
                    self.fg_folded = Some(mesh);
                    self.fg_flat = flat;
                    if let Some(v) = &mut self.view {
                        v.renderer.live_show(false);
                    }
                    changed = true;
                    continue;
                }
            };
            if b.asm.is_none() {
                self.extras.frames.remove(gui_overlay::SELF);
                self.extras.asked.remove(gui_overlay::SELF);
            }
            if let Some(p) = &mut self.shell.fg {
                p.summary = b.summary();
                p.feats = b.feats;
                p.params = b.params;
                p.issues = b.issues;
                if p.selected.as_ref().is_some_and(|k| {
                    !p.feats.iter().any(|f| {
                        f.id == *k || f.id.starts_with(&format!("{k}.")) || f.id.starts_with(&format!("{k}["))
                    })
                }) {
                    p.selected = None;
                }
                self.shell.status = p.summary.clone();
            }
            if let Some(a) = &b.asm
                && let Some(p) = &mut self.shell.fg
            {
                let ap = p.asm.get_or_insert_with(Default::default);
                let dir = s
                    .path
                    .as_deref()
                    .and_then(|f| std::fs::canonicalize(f).ok())
                    .and_then(|f| f.parent().map(|d| d.to_path_buf()));
                let flags = crate::shell::fgasm::issue_flags(&p.issues);
                ap.set_structure(
                    a.occ
                        .iter()
                        .map(|o| crate::shell::fgasm::OccRow {
                            path: o.path.clone(),
                            file: o.file.clone(),
                            drawn: o.placement.is_some(),
                        })
                        .collect(),
                    dir,
                    a.insts.clone(),
                    a.links.clone(),
                    flags,
                );
                ap.poses = a.poses.clone();
                ap.grounded = a.grounded.clone();
                ap.joints = a.joints.clone();
                ap.mobility = a.mobility;
                ap.bom = a.bom.clone();
                ap.expects = a.expects.clone();
                if a.overlap.is_some() {
                    ap.overlap = a.overlap;
                    ap.checking = false;
                }
                let tm = &a.timings;
                ap.timings = format!(
                    "load {:.0} ms, solve {:.0} ms{}, {} parts meshed ({} new) in {:.0} ms",
                    tm.load_ms,
                    tm.solve_ms,
                    if a.overlap.is_some() {
                        format!(", overlap {:.0} ms", tm.overlap_ms)
                    } else {
                        String::new()
                    },
                    a.parts,
                    a.meshed,
                    b.mesh_ms
                );
                // an assembly's own line says it all: no part summary ("nothing built") above it
                p.summary.clear();
                self.shell.status = format!(
                    "{} occurrences · {} parts · DOF {}",
                    ap.occ.len(),
                    ap.bom.len(),
                    ap.mobility
                );
                self.fg_asm = Some((a.occ.clone(), a.placed.clone()));
                self.fg_asm_boxes = a.boxes.clone();
                self.explode.stale();
                asm_moved = true;
                // which part is placed where changed: the view made again, the camera kept
                if let Some(all) = &a.restage {
                    let parts: Vec<crate::mesh::PartMesh> = if all.is_empty() {
                        vec![crate::mesh::PartMesh::empty()]
                    } else {
                        all.clone()
                    };
                    let placements: Vec<Placement> = if a.placed.is_empty() {
                        vec![Placement::at(0, [0.0; 3])]
                    } else {
                        a.placed
                            .iter()
                            .map(|(m, rows)| Placement {
                                part: *m as u32,
                                rows: *rows,
                                tint: crate::fg::PART_TINT,
                            })
                            .collect()
                    };
                    let mut scene = Scene::new(&part_info(&parts), placements);
                    scene.set_explode(0.0);
                    scene.set_part_boxes(a.boxes.clone());
                    let renderer = Renderer::new(
                        self.device.clone(),
                        self.queue.clone(),
                        self.adapter.clone(),
                        render::OFFSCREEN,
                        (16, 16),
                        &parts,
                        &scene,
                    );
                    if let Some(v) = &self.view {
                        self.fg_keep_camera = Some(v.nav.camera);
                    }
                    self.fg_view = Some(Loaded {
                        file: s.path.clone(),
                        name: name_of(s.path.as_deref().unwrap_or("part.fg")),
                        part: None,
                        empty: false,
                        scene,
                        renderer,
                        unshown: None,
                    });
                }
            }
            if let Some((renderer, scene)) = b.view {
                if let Some(v) = &self.view
                    && b.asm.is_some()
                {
                    self.fg_keep_camera = Some(v.nav.camera);
                }
                self.fg_view = Some(Loaded {
                    file: s.path.clone(),
                    name: name_of(s.path.as_deref().unwrap_or("part.fg")),
                    part: None,
                    empty: b.measure.is_none(),
                    scene,
                    renderer,
                    unshown: None,
                });
            }
            if let Some(p) = &mut self.shell.fg {
                p.sheets = b
                    .sheets
                    .iter()
                    .map(|s| crate::shell::fg::SheetRow {
                        id: s.id.clone(),
                        bends: s.bends,
                        refused: s.dxf.as_ref().err().cloned(),
                        facts: s.facts.clone(),
                        lists: s.lists.clone(),
                    })
                    .collect();
            }
            self.shell.sheet.flats = b.sheets.iter().filter_map(|s| s.view.clone()).collect();
            self.fg_sheets = b.sheets;
            if b.inspect.is_some() {
                inspect = b.inspect;
            }
            let (rows, walls): (Vec<_>, Vec<_>) = b.thermos.into_iter().unzip();
            if let Some(p) = &mut self.shell.fg {
                p.thermos = rows;
            }
            self.fg_walls = walls;
            if let Some(a) = b.anchors {
                if let Some(p) = &mut self.shell.fg {
                    p.sketches = a.sketches.clone();
                }
                self.shell.sheet.edges = a.sheet_edges.iter().map(|e| e.0.clone()).collect();
                self.fg_anchors = Some(a);
            }
            if let Some(d) = &b.delta
                && let Some(v) = &mut self.view
            {
                let part = v.scene.placements[0].part;
                v.renderer.live_apply(part, d);
                if !flat_shown {
                    v.renderer.live_show(true);
                    v.scene.set_part(part, d.center, d.radius, vec![d.chord]);
                    v.dirty = true;
                }
            }
            if let Some(m) = b.mesh {
                self.fg_folded = Some(m);
                self.fg_flat = b.flat;
                changed = true;
            }
        }
        self.inspect_built(inspect);
        if changed {
            self.fg_show();
        }
        self.fg_text();
        // the view takes its size from the layout
        if self.shell.view_rect.area() > 0.0
            && let Some(l) = self.fg_view.take()
        {
            let status = std::mem::take(&mut self.shell.status);
            self.show_loaded(l);
            self.shell.status = status;
            if let (Some(c), Some(v)) = (self.fg_keep_camera.take(), &mut self.view) {
                v.nav.camera = c;
            }
            asm_moved = true;
        }
        if asm_moved {
            self.asm_show();
        }
    }

    /// The Forge panel's asks: typed ops, through forge-lang; saving.
    fn apply_fg(&mut self) {
        self.thermo_host();
        let acts = self
            .shell
            .fg
            .as_mut()
            .map(|p| std::mem::take(&mut p.out))
            .unwrap_or_default();
        for a in acts {
            match a {
                crate::shell::fg::FgAct::Apply(op) => {
                    let _ = self.fg_apply(op);
                }
                crate::shell::fg::FgAct::Save => self.fg_save(),
                crate::shell::fg::FgAct::ExportDxf(id) => {
                    let _ = self.fg_export_dxf(&id);
                }
                crate::shell::fg::FgAct::Tool(op) => {
                    if self.fg_apply(op).is_ok()
                        && let Some(p) = &mut self.shell.fg
                        && let Some(t) = p.tool.take()
                        && t.again
                    {
                        self.fg_open_tool(t.kind);
                    }
                }
                crate::shell::fg::FgAct::Refuse(e) => self.fg_refuse(e),
                crate::shell::fg::FgAct::AsmShow => self.asm_show(),
                crate::shell::fg::FgAct::AsmCheck => {
                    if let Some(s) = &mut self.fg {
                        s.check_overlap();
                    }
                }
                crate::shell::fg::FgAct::AsmAnchors(file) => self.asm_anchors(file),
                a @ (crate::shell::fg::FgAct::AsmOps(_)
                | crate::shell::fg::FgAct::AsmPreview(_)
                | crate::shell::fg::FgAct::AsmPlace { .. }
                | crate::shell::fg::FgAct::AsmBrowse
                | crate::shell::fg::FgAct::AsmStandardPart
                | crate::shell::fg::FgAct::AsmPattern { .. }
                | crate::shell::fg::FgAct::AsmApplyMoves
                | crate::shell::fg::FgAct::AsmMirror { .. }
                | crate::shell::fg::FgAct::AsmSnaps(_)) => self.asm_act(a),
                crate::shell::fg::FgAct::EditSheet(id) => self.shell.sheet.edit_req = Some(id),
                crate::shell::fg::FgAct::Flat(on) => {
                    if let Some(p) = &mut self.shell.fg {
                        p.flat = on && self.fg_flat.is_some();
                    }
                    self.fg_show();
                }
            }
        }
    }

    /// Applies a typed op to the Forge part, as the panel does: refused with its typed error, or
    /// applied and rebuilt.
    pub fn fg_apply(&mut self, op: forge_lang::error::Op) -> Result<(), String> {
        let Some(s) = &mut self.fg else {
            return Err("no Forge part open".into());
        };
        let label = crate::shell::fg::op_label(&op);
        let got = s.apply(op);
        let p = self.shell.fg.as_mut();
        match got {
            Ok(reach) => {
                self.shell.status = match reach {
                    Some(r) => format!(
                        "{label}: {} values in {}{}",
                        r.values,
                        r.features.join(", "),
                        if r.structural {
                            " (structural: re-runs)"
                        } else {
                            ""
                        }
                    ),
                    None => label,
                };
                if let Some(p) = p {
                    p.refused = None;
                }
                self.fg_text();
                Ok(())
            }
            Err(e) => {
                let why = format!("{}: {}", e.kind(), e.text());
                if let Some(p) = p {
                    p.refused = Some(why.clone());
                }
                self.shell.status = format!("{label} refused: {why}");
                Err(why)
            }
        }
    }

    /// An assembly's placements as the panel wants them: hidden ones out of sight, the chosen
    /// and the highlighted tinted.
    pub fn asm_show(&mut self) {
        let (Some((occ, placed)), Some(a)) =
            (&self.fg_asm, self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()))
        else {
            return;
        };
        const NOWHERE: crate::scene::Rows = [[0.0; 4]; 3];
        let target = match &mut self.view {
            Some(v) => Some((&mut v.scene, &mut v.renderer, &mut v.dirty)),
            None => None,
        };
        let Some((scene, renderer, dirty)) = target else {
            return;
        };
        if scene.placements.len() != placed.len() {
            return;
        }
        let (shown, picked) = (a.shown_mask(), a.picked_mask());
        let pastel = self.shell.view.pastel;
        let flagged = !a.highlight.is_empty();
        for (i, o) in occ.iter().enumerate() {
            let Some(k) = o.placement else { continue };
            let top = o.path.split('.').next().unwrap_or(&o.path);
            let rows = if !shown.get(i).copied().unwrap_or(true) {
                NOWHERE
            } else if let Some(m) = a.moved.get(top) {
                gui_asm::moved_rows(m, &placed[k].1)
            } else {
                placed[k].1
            };
            if scene.rows_of(k) != rows {
                scene.set_rows(k, &rows);
            }
            scene.placements[k].tint = if picked.get(i).copied().unwrap_or(false) {
                crate::fg::PICKED_TINT
            } else if flagged && a.flagged(&o.path) {
                crate::fg::CLASH_TINT
            } else {
                crate::palette::part_colour(&o.file, pastel)
            };
        }
        renderer.set_tints(scene);
        *dirty = true;
    }

    /// A part file's anchors, found on a thread: the instance chosen shows them.
    fn asm_anchors(&mut self, file: String) {
        let dir = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        let (send, recv) = channel();
        self.fg_asm_anchors = Some(recv);
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let path = dir
                .as_ref()
                .map(|d| d.join(&file))
                .filter(|p| p.exists())
                .unwrap_or_else(|| std::path::PathBuf::from(&file));
            let got = std::fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .map(|text| {
                    let opts = forge_lang::Options {
                        anchors: true,
                        dir: path.parent().map(|d| d.to_path_buf()),
                        ..forge_lang::Options::default()
                    };
                    let (run, _) = forge_lang::run(&text, opts, Default::default());
                    let mut out: Vec<String> = vec![];
                    if let Some(a) = &run.anchors {
                        let mut faces: Vec<String> = a.faces.values().cloned().collect();
                        faces.sort();
                        faces.dedup();
                        out.extend(faces);
                        for s in &a.sketches {
                            out.extend(s.points.iter().cloned());
                            out.extend(s.lines.iter().cloned());
                            out.extend(s.closed.iter().cloned());
                        }
                    }
                    out
                });
            let _ = send.send((file, got));
            waker();
        });
    }

    /// What the open tool would make, built and shown; nothing once it's closed or can't be
    /// written yet.
    fn fg_preview(&mut self) {
        let want = self
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.tool.as_ref())
            .and_then(|t| t.text().ok());
        let Some(s) = &mut self.fg else { return };
        let Some(text) = want else {
            // only a preview this made is this one's to end: an assembly's dialogs have their own
            if self.fg_previewed.take().is_some() && s.previewing {
                s.preview(None);
            }
            if let Some(t) = self.shell.fg.as_mut().and_then(|p| p.tool.as_mut()) {
                t.note = None;
            }
            return;
        };
        if self.fg_previewed.as_deref() == Some(text.as_str()) {
            // what it made fails in the kernel: the part as it was back, to pick on, and why
            let built = s.previewing && s.done == s.seq;
            if built
                && let Some(p) = &mut self.shell.fg
                && let Some(t) = &mut p.tool
                && let Some(i) = p.issues.iter().find(|i| {
                    i.severity == crate::shell::fg::Severity::Error
                        // its own statement, or one it writes after it (a tapped hole's thread)
                        && i.feature.as_deref().is_some_and(|f| {
                            f == t.name || f.strip_prefix(t.name.as_str()).is_some_and(|r| r.starts_with('_'))
                        })
                })
            {
                t.note = Some(format!("{}: {}", i.kind, i.text));
                s.preview(None);
            }
            return;
        }
        let op = forge_lang::error::Op::AddFeature {
            before: None,
            text: text.clone(),
        };
        // what wouldn't check is said, and the part shown as it is
        let ops = [op];
        let note = match forge_lang::edit::apply_all_checked(&s.src, &ops) {
            Ok(_) => {
                s.preview(Some(&ops));
                None
            }
            Err(e) => {
                s.preview(None);
                Some(format!("{}: {}", e.kind(), e.text()))
            }
        };
        if let Some(t) = self.shell.fg.as_mut().and_then(|p| p.tool.as_mut()) {
            t.note = note;
        }
        self.fg_previewed = Some(text);
        self.fg_text();
    }

    /// A tool's form for a ribbon kind; one with no Forge form is refused, typed.
    pub fn fg_open_tool(&mut self, kind: &'static str) {
        let Some(p) = &mut self.shell.fg else { return };
        let mut taken = crate::shell::fg::statements(&p.feats);
        taken.extend(
            p.params
                .iter()
                .filter_map(|x| x.key.owner.first().map(|s| s.name.clone())),
        );
        let dir = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .and_then(|f| std::path::Path::new(f).parent().map(|d| d.to_path_buf()));
        match crate::shell::fgtool::FgTool::new(kind, &taken) {
            Some(mut t) => {
                crate::shell::fgdialog::offer_lists(&mut t, dir.as_deref());
                p.tool = Some(t);
                p.refused = None;
                self.shell.status = format!("{}: pick what it takes", crate::shell::panel::kind_word(kind));
            }
            None => self.fg_refuse(crate::shell::fgtool::ToolError::NoForgeForm {
                tool: kind,
                why: "no Forge form in the app yet".into(),
            }),
        }
    }

    /// Something asked of a Forge part that can't be: said in the panel and the status bar.
    pub fn fg_refuse(&mut self, e: crate::shell::fgtool::ToolError) {
        let why = format!("{}: {}", e.kind(), e.text());
        if let Some(p) = &mut self.shell.fg {
            p.refused = Some(why.clone());
        }
        self.shell.status = why;
    }

    /// A click in the view on a Forge part: what Forge names it, for the tool waiting, the sketch
    /// being started, or just shown.
    fn fg_picks(&mut self, under: Option<crate::origin::Item>) {
        use crate::origin::Item;
        use crate::shell::fgtool::{SlotKind as K, ToolError};
        if self.fg_asm.is_some() && self.asm_tools_frame() {
            return;
        }
        let Some(v) = &mut self.view else { return };
        if v.nav.clicks == v.clicks {
            return;
        }
        v.clicks = v.nav.clicks;
        // an assembly: a click chooses the occurrence under it
        if let Some((occ, _)) = &self.fg_asm {
            let at = v.nav.selected.last().map(|x| x.0 as usize);
            let k = at.and_then(|p| occ.iter().position(|o| o.placement == Some(p)));
            v.nav.selected.clear();
            v.clicked.clear();
            if let Some(a) = self.shell.fg.as_mut().and_then(|p| p.asm.as_mut()) {
                a.reveal_occ(k);
                a.anchors = None;
            }
            self.asm_show();
            return;
        }
        let ent = v.nav.selected.last().map(|x| x.1);
        let a = self.fg_anchors.clone();
        let named = |list: Option<&Vec<Option<String>>>, k: u32, what: &'static str| {
            list.and_then(|l| l.get(k as usize).cloned().flatten())
                .ok_or(ToolError::NoStableName { what })
        };
        let origin = under.and_then(|i| match i {
            Item::XyPlane => Some("xy"),
            Item::XzPlane => Some("xz"),
            Item::YzPlane => Some("yz"),
            _ => None,
        });
        let (what, text, edge): (&'static str, Result<String, ToolError>, Option<u32>) = match ent {
            Some(Entity::Face(f)) => ("face", named(a.as_ref().map(|a| &a.faces), f, "face"), None),
            Some(Entity::Edge(e)) => ("edge", named(a.as_ref().map(|a| &a.edges), e, "edge"), Some(e)),
            Some(Entity::Vertex(_)) => ("vertex", Err(ToolError::NoStableName { what: "vertex" }), None),
            None => match origin {
                Some(o) => ("origin plane", Ok(o.to_string()), None),
                None => return,
            },
        };
        if self.shell.sketch_pick {
            match (what, &text) {
                ("face" | "origin plane", Ok(t)) => {
                    let frame = if what == "origin plane" {
                        match forge_lang::builtins::constant(t) {
                            Some(forge_lang::eval::Value::Plane(p)) => Some(p.frame),
                            _ => None,
                        }
                    } else {
                        a.as_ref()
                            .and_then(|a| a.planes.get(&format!("{t}.plane")).copied())
                    };
                    match frame {
                        Some(plane) => {
                            let mut m = SketchMode::new(SketchFor::New, t.clone(), plane, Vec::new(), None);
                            m.units = "mm";
                            self.shell.sketch = Some(m);
                            self.shell.sketch_pick = false;
                            self.shell.tab = 1;
                            self.shell.status.clear();
                            v.nav.selected.clear();
                            v.clicked.clear();
                            look_at(v, plane);
                        }
                        None => self.shell.status = format!("{t} is not a plane to sketch on"),
                    }
                }
                (_, Err(e)) => self.shell.status = format!("{}: {}", e.kind(), e.text()),
                _ => {
                    self.shell.status =
                        format!("a sketch goes on a planar face or an origin plane, not a {what}")
                }
            }
            return;
        }
        if let Some(d) = &mut self.shell.sheet.dlg {
            let sheet_edge = edge.and_then(|k| {
                a.as_ref()?
                    .sheet_edges
                    .iter()
                    .find(|s| s.1.contains(&(k as usize)))
                    .map(|s| s.0.clone())
            });
            let got = match (sheet_edge, &text) {
                (Some(e), _) => d.pick_for_slot("sheet edge", &e),
                (None, Ok(t)) => d.pick_for_slot(what, t),
                (None, Err(e)) => Err(e.text()),
            };
            d.take(got);
            v.nav.selected.clear();
            v.clicked.clear();
            v.dirty = true;
            return;
        }
        let Some(p) = &mut self.shell.fg else { return };
        let Some(tool) = &mut p.tool else {
            p.picked = Some(text);
            return;
        };
        let Some(slot) = tool.slots.get(tool.active) else {
            return;
        };
        let (label, kind) = (slot.label, slot.kind);
        let got = match (kind, what) {
            (K::Edges | K::Edge, "edge")
            | (K::Faces | K::Face, "face")
            | (K::Plane, "face" | "origin plane") => text,
            (K::Dir, "face") => text.map(|t| format!("{t}.normal")),
            (K::Axis, "face") => text.map(|t| format!("{t}.axis")),
            (K::EdgeOrFace, "edge" | "face") => text,
            (K::SheetEdge, "edge") => {
                let k = edge.expect("an edge") as usize;
                a.as_ref()
                    .and_then(|a| {
                        a.sheet_edges
                            .iter()
                            .find(|s| s.1.contains(&k))
                            .map(|s| s.0.clone())
                    })
                    .ok_or(ToolError::NoStableName { what: "sheet edge" })
            }
            _ => Err(ToolError::WrongPick { slot: label, what }),
        };
        tool.take(got);
        v.nav.selected.clear();
        v.clicked.clear();
        v.dirty = true;
    }

    /// The sketch drawn on a Forge part, written as a Forge sketch block and added.
    fn fg_finish_sketch(&mut self) {
        let Some(m) = self.shell.sketch.take() else { return };
        self.shell.tab = 0;
        if m.rows.is_empty() {
            return;
        }
        let taken = self
            .shell
            .fg
            .as_ref()
            .map(|p| crate::shell::fg::statements(&p.feats))
            .unwrap_or_default();
        let name = (1..)
            .map(|k| format!("sk{k}"))
            .find(|n| !taken.contains(n))
            .expect("a free name");
        match crate::shell::fgsketch::block(&name, &m.head, &m.rows) {
            Ok(text) => {
                let added = self
                    .fg_apply(forge_lang::error::Op::AddFeature { before: None, text })
                    .is_ok();
                // text placed in the sketch goes on to Emboss, as Inventor's does
                if added && let Some((at, words)) = m.more.texts.first().cloned() {
                    self.fg_open_tool("emboss");
                    if let Some(t) = self.shell.fg.as_mut().and_then(|p| p.tool.as_mut()) {
                        t.slots[0].picks = vec![name.clone()];
                        t.slots[1].picks = vec![format!("{name}.{at}")];
                        if let Some(w) = t.words.iter_mut().find(|w| w.0 == "text") {
                            w.1 = words;
                        }
                        t.active = 1;
                    }
                }
            }
            Err(e) => {
                self.fg_refuse(e);
                // the drawing stays, to be changed
                self.shell.sketch = Some(m);
                self.shell.tab = 1;
            }
        }
    }

    /// The Forge part's folded mesh, or its sheet's flat one when that's asked for.
    fn fg_show(&mut self) {
        let flat = self.shell.fg.as_ref().is_some_and(|p| p.flat);
        let Some(m) = (if flat { self.fg_flat.as_ref() } else { None }).or(self.fg_folded.as_ref()) else {
            return;
        };
        let chords: Vec<f32> = m.lods.iter().map(|x| x.chord).collect();
        if let Some(l) = &mut self.fg_view {
            l.renderer.set_parts(std::slice::from_ref(m));
            l.scene.set_part(0, m.center, m.radius, chords);
        } else if let Some(v) = &mut self.view {
            let part = v.scene.placements[0].part;
            v.renderer.set_parts(std::slice::from_ref(m));
            v.scene.set_part(part, m.center, m.radius, chords);
            v.dirty = true;
        }
    }

    /// A sheet's flat pattern as DXF beside the part's file: `<stem>.dxf`, or `<stem>-<id>.dxf`
    /// when the part has several sheets. Returns where it went.
    pub fn fg_export_dxf(&mut self, id: &str) -> Result<String, String> {
        let got = (|| -> Result<String, String> {
            let path = self
                .fg
                .as_ref()
                .and_then(|s| s.path.clone())
                .ok_or("no file beside which to write")?;
            let s = self
                .fg_sheets
                .iter()
                .find(|s| s.id == id)
                .ok_or(format!("no sheet {id}"))?;
            let text = s
                .dxf
                .clone()
                .map_err(|e| format!("{id} has no flat pattern: {e}"))?;
            let stem = path.strip_suffix(".fg").unwrap_or(&path);
            let out = if self.fg_sheets.len() > 1 {
                format!("{stem}-{id}.dxf")
            } else {
                format!("{stem}.dxf")
            };
            std::fs::write(&out, text).map_err(|e| format!("{out}: {e}"))?;
            Ok(out)
        })();
        self.shell.status = match &got {
            Ok(out) => format!("Wrote {out}"),
            Err(why) => why.clone(),
        };
        got
    }

    /// Writes the Forge part's text to its file.
    pub fn fg_save(&mut self) {
        let Some(s) = &mut self.fg else { return };
        self.shell.status = match s.save() {
            Ok(()) => format!("Saved {}", s.path.as_deref().unwrap_or("")),
            Err(why) => why,
        };
        self.fg_text();
    }
}
