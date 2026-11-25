//! The Render Studio environment's content (RENDER-STUDIO's; `env/render.rs` only delegates):
//! its ribbon, its browser (each part's appearance, the lighting, the camera, the output), and
//! what it writes over the view. The picture itself is the GUI's (`gui_studio.rs`): this holds
//! what the user chose, and the GUI reads it each frame.

use super::env::EnvId;
use super::ribbon::{Group, Tool};
use super::{Action, Shell};
use crate::studio::appearance::Appearances;
use egui::{Rect, RichText, Ui};

/// Samples before the traced picture is readable: the raster view, in the same looks, stands in
/// until then (gui_studio.rs); then the traced one fades in over as many again.
pub const READABLE: u32 = 8;

/// The ribbon's commands (`Action::Env(RenderStudio, c)`).
pub const RENDER_IMAGE: u16 = 1;
pub const TURNTABLE: u16 = 2;
pub const CANCEL: u16 = 3;
pub const DENOISE: u16 = 4;
pub const FLOOR: u16 = 5;
pub const DEPTH: u16 = 6;
pub const RESTART: u16 = 7;

/// The lighting's choices: the drawn studio (0), the manifest's HDRIs (1..=n, `studio::hdri`),
/// a Radiance file the user names (n + 1).
pub fn lights() -> Vec<String> {
    let mut v = vec!["Studio softbox (drawn)".to_string()];
    for h in crate::studio::hdri::list() {
        v.push(if h.bundled {
            format!("{} (bundled)", h.name)
        } else {
            h.name
        });
    }
    v.push("HDRI file".into());
    v
}

/// A render the user asked for, for the GUI to queue; and what the result's card asks.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Image,
    Turntable,
    Cancel,
    /// Show the render's file in its folder (native).
    Reveal(String),
}

/// A final render under way, as the view's card shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rendering {
    /// "Rendering image", "Rendering turntable · frame 3 of 36".
    pub what: String,
    /// Samples done and to do, over the whole job (every frame of a turntable).
    pub done: u64,
    pub of: u64,
    /// When it started and now, ms.
    pub started_ms: f64,
    pub now_ms: f64,
}

impl Rendering {
    pub fn fraction(&self) -> f32 {
        (self.done as f64 / self.of.max(1) as f64).clamp(0.0, 1.0) as f32
    }

    /// Seconds so far, and an estimate of the seconds left (once there is a rate to go by).
    pub fn times(&self) -> (f64, Option<f64>) {
        let elapsed = ((self.now_ms - self.started_ms) / 1000.0).max(0.0);
        let left = (self.done > 0 && elapsed > 0.2)
            .then(|| elapsed * (self.of.saturating_sub(self.done)) as f64 / self.done as f64);
        (elapsed, left)
    }
}

/// How the last render ended: its file, or why not; a texture of the picture to show.
#[derive(Clone, Debug)]
pub struct RenderResult {
    pub said: Result<String, String>,
    pub thumb: Option<(egui::TextureId, (u32, u32))>,
}

#[derive(Clone, Debug)]
pub struct StudioUi {
    /// Each part's appearance by its index in the view, none: its material's default.
    pub looks: Vec<Option<String>>,
    /// The parts' names, as the GUI found them.
    pub names: Vec<String>,
    /// Each part's group (its file's folder; empty: none), its occurrences, its own material.
    pub groups: Vec<String>,
    pub counts: Vec<usize>,
    pub own: Vec<Option<String>>,
    /// The appearance brush, and whether a click in the view applies it.
    pub brush: Option<String>,
    pub painting: bool,
    /// The palette's search, the parts' filter, the parts chosen (and the last clicked).
    pub find: String,
    pub filter: String,
    pub selected: Vec<usize>,
    pub anchor: Option<usize>,
    /// The groups open in the tree; the palette's other families shown.
    pub open: std::collections::BTreeSet<String>,
    pub more: bool,
    /// What the brush last did, for the view's line.
    pub said: String,
    /// The parts of the row under the pointer (lit in the view); the part under the pointer in
    /// the view (its row lit); a row to scroll into view.
    pub hover_row: Vec<usize>,
    /// Each part's file (an assembly's), for its row's hover.
    pub files: Vec<String>,
    /// The looks before each change (a click, a drag, an Apply), newest last: Undo takes one.
    pub history: Vec<Vec<Option<String>>>,
    pub hover_view: Option<usize>,
    pub reveal: Option<usize>,
    /// A final render under way; how the last one ended (until closed).
    pub rendering: Option<Rendering>,
    pub result: Option<RenderResult>,
    pub light: usize,
    pub hdri: String,
    pub turn_deg: f32,
    pub strength: f32,
    pub exposure: f32,
    pub floor: bool,
    pub denoise: bool,
    pub depth_of_field: bool,
    /// The view's focal length, mm (35 mm frame), as the GUI found it: shown, not set; 0 for a
    /// parallel view.
    pub view_focal_mm: f32,
    pub f_number: f32,
    pub size: (u32, u32),
    pub spp: u32,
    pub frames: u32,
    /// The file a render writes; empty: a fresh `<document>-render-<time>.png` where captures go.
    pub out: String,
    /// Bumped when anything the GUI must rebuild for changes (appearances, the light).
    pub version: u64,
    pub asks: Vec<Ask>,
    /// What the GUI says: samples in the view, the queue's state.
    pub samples: u32,
    /// Samples traced (the picture in the view is shown at the denoise's steps), and how many
    /// the live view refines to.
    pub traced: u32,
    pub target: u32,
    pub queue: String,
    pub library: Option<std::sync::Arc<Appearances>>,
}

impl Default for StudioUi {
    fn default() -> StudioUi {
        StudioUi {
            looks: Vec::new(),
            names: Vec::new(),
            groups: Vec::new(),
            counts: Vec::new(),
            own: Vec::new(),
            brush: None,
            painting: false,
            find: String::new(),
            filter: String::new(),
            selected: Vec::new(),
            anchor: None,
            open: Default::default(),
            more: false,
            said: String::new(),
            hover_row: Vec::new(),
            files: Vec::new(),
            history: Vec::new(),
            hover_view: None,
            reveal: None,
            rendering: None,
            result: None,
            // the bundled HDRI
            light: 1,
            hdri: String::new(),
            turn_deg: 0.0,
            strength: 1.0,
            exposure: -0.5,
            floor: true,
            denoise: true,
            depth_of_field: false,
            view_focal_mm: 50.0,
            f_number: 4.0,
            size: (1920, 1080),
            spp: 512,
            frames: 36,
            out: String::new(),
            version: 0,
            asks: Vec::new(),
            samples: 0,
            traced: 0,
            target: 0,
            queue: String::new(),
            library: None,
        }
    }
}

impl StudioUi {
    pub fn library(&mut self) -> std::sync::Arc<Appearances> {
        self.library
            .get_or_insert_with(|| std::sync::Arc::new(Appearances::builtin()))
            .clone()
    }
}

fn tool(name: &'static str, icon: &'static str, c: u16, on: bool) -> Tool {
    Tool {
        name,
        icon,
        action: Some(Action::Env(EnvId::RenderStudio, c)),
        on,
    }
}

pub(super) fn groups(s: &Shell) -> Vec<Group> {
    let st = &s.studio;
    let busy = st.rendering.is_some();
    let g = |name, big, small| Group {
        name,
        big,
        small,
        icons: vec![],
    };
    vec![
        g(
            "Render",
            vec![
                // while one renders, the commands say so and wait (Cancel stops it)
                Tool {
                    name: if busy { "Rendering…" } else { "Render Image" },
                    icon: "file",
                    action: (!busy).then_some(Action::Env(EnvId::RenderStudio, RENDER_IMAGE)),
                    on: busy,
                },
                Tool {
                    name: "Render Turntable",
                    icon: "rotate",
                    action: (!busy).then_some(Action::Env(EnvId::RenderStudio, TURNTABLE)),
                    on: false,
                },
            ],
            vec![Tool {
                name: "Cancel",
                icon: "close",
                action: busy.then_some(Action::Env(EnvId::RenderStudio, CANCEL)),
                on: false,
            }],
        ),
        g(
            "Scene",
            vec![tool("Floor", "groundplane", FLOOR, st.floor)],
            vec![
                tool("Denoise", "smooth", DENOISE, st.denoise),
                tool("Start Again", "redo", RESTART, false),
            ],
        ),
        g(
            "Camera",
            vec![tool("Depth of Field", "eye", DEPTH, st.depth_of_field)],
            vec![
                Tool {
                    name: "Perspective",
                    icon: "lookat",
                    action: Some(Action::Parallel(!s.view.parallel)),
                    on: !s.view.parallel,
                },
                Tool {
                    name: "Fit All",
                    icon: "fit",
                    action: Some(Action::FitAll),
                    on: false,
                },
            ],
        ),
    ]
}

pub(super) fn command(s: &mut Shell, c: u16) {
    let st = &mut s.studio;
    match c {
        RENDER_IMAGE => st.asks.push(Ask::Image),
        TURNTABLE => st.asks.push(Ask::Turntable),
        CANCEL => st.asks.push(Ask::Cancel),
        DENOISE => st.denoise = !st.denoise,
        FLOOR => st.floor = !st.floor,
        DEPTH => st.depth_of_field = !st.depth_of_field,
        RESTART => st.version += 1,
        _ => {}
    }
}

/// A setting's row: its words with its value and unit, then its control.
fn row(ui: &mut Ui, label: String, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(128.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(128.0);
                ui.add(egui::Label::new(label).truncate());
            },
        );
        control(ui);
    });
}

/// A number with its sign, a true minus.
fn signed(v: f32) -> String {
    if v < -0.005 {
        format!("\u{2212}{:.1}", -v)
    } else if v > 0.005 {
        format!("+{v:.1}")
    } else {
        "0.0".into()
    }
}

/// A section folded to its one-line summary, open on a click.
fn fold(ui: &mut Ui, s: &Shell, id: &str, summary: &str, body: impl FnOnce(&mut Ui)) -> Rect {
    ui.add_space(2.0);
    egui::CollapsingHeader::new(
        RichText::new(summary)
            .font(super::theme::semi(13.0))
            .color(s.theme.ink),
    )
    .id_salt(id)
    .default_open(false)
    .show(ui, body)
    .header_response
    .rect
}

/// A heading in the browser.
pub(super) fn head(ui: &mut Ui, s: &Shell, text: &str) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(text)
            .font(super::theme::semi(13.0))
            .color(s.theme.ink),
    );
    ui.add_space(2.0);
}

pub(super) fn browser(s: &mut Shell, ui: &mut Ui, r: Rect) -> bool {
    let lib = s.studio.library();
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(egui::vec2(10.0, 8.0))));
    child.set_clip_rect(r);
    let mut st = std::mem::take(&mut s.studio);
    let before = (st.looks.clone(), st.light, st.hdri.clone());
    // the folds' headers, for the tests' clicks (`studio:fold:light` ..)
    let mut heads: Vec<(String, Rect)> = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("env-render-browser")
        .show(&mut child, |ui| {
            if st.looks.len() < st.names.len() {
                st.looks.resize(st.names.len(), None);
            }
            super::studio_model::ui(ui, s, &mut st, &lib);
            ui.add_space(6.0);

            let lights = lights();
            let light_name = lights[st.light.min(lights.len() - 1)].clone();
            let head = fold(
                ui,
                s,
                "studio-fold-light",
                &format!("Lighting · {light_name}"),
                |ui| {
                    egui::ComboBox::from_id_salt("studio-light")
                        .selected_text(&lights[st.light.min(lights.len() - 1)])
                        .show_ui(ui, |ui| {
                            for (k, l) in lights.iter().enumerate() {
                                ui.selectable_value(&mut st.light, k, l);
                            }
                        })
                        .response
                        .on_hover_text(
                            "Poly Haven HDRIs, CC0; all but the bundled one come from tools/fetch-hdri.sh",
                        );
                    if st.light == lights.len() - 1 {
                        ui.horizontal(|ui| {
                            ui.label("File");
                            ui.text_edit_singleline(&mut st.hdri)
                                .on_hover_text("a Radiance .hdr, equirectangular");
                        });
                    }
                    row(ui, format!("Turn {:.0}°", st.turn_deg), |ui| {
                        ui.add(egui::Slider::new(&mut st.turn_deg, -180.0..=180.0).show_value(false));
                    });
                    row(ui, format!("Strength {:.2}×", st.strength), |ui| {
                        ui.add(
                            egui::Slider::new(&mut st.strength, 0.05..=8.0)
                                .logarithmic(true)
                                .show_value(false),
                        );
                    });
                    row(ui, format!("Exposure {} EV", signed(st.exposure)), |ui| {
                        ui.add(egui::Slider::new(&mut st.exposure, -4.0..=4.0).show_value(false));
                    });
                },
            );
            heads.push(("studio:fold:light".into(), head));

            let lens = if st.view_focal_mm > 0.0 {
                format!("{:.0} mm", st.view_focal_mm)
            } else {
                "parallel".into()
            };
            let head = fold(ui, s, "studio-fold-camera", &format!("Camera · {lens}"), |ui| {
                ui.label(
                    RichText::new(if st.view_focal_mm > 0.0 {
                        format!("Lens {:.0} mm (the view's)", st.view_focal_mm)
                    } else {
                        "Parallel view: no lens (Camera › Perspective)".into()
                    })
                    .color(s.theme.ink2),
                );
                row(ui, format!("Aperture f/{:.1}", st.f_number), |ui| {
                    ui.add_enabled(
                        st.depth_of_field,
                        egui::Slider::new(&mut st.f_number, 1.0..=22.0)
                            .logarithmic(true)
                            .show_value(false),
                    );
                });
                if !st.depth_of_field {
                    ui.label(
                        RichText::new("Depth of field is off: Camera › Depth of Field")
                            .small()
                            .color(s.theme.ink2),
                    );
                }
            });
            heads.push(("studio:fold:camera".into(), head));

            let out = format!("Output · {} × {}, {} samples", st.size.0, st.size.1, st.spp);
            let head = fold(ui, s, "studio-fold-output", &out, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(egui::DragValue::new(&mut st.size.0).range(16..=8192));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut st.size.1).range(16..=8192));
                });
                row(ui, format!("Samples {}", st.spp), |ui| {
                    ui.add(
                        egui::Slider::new(&mut st.spp, 16..=8192)
                            .logarithmic(true)
                            .show_value(false),
                    );
                });
                row(ui, format!("Turntable {} frames", st.frames), |ui| {
                    ui.add(egui::Slider::new(&mut st.frames, 4..=360).show_value(false));
                });
                ui.horizontal(|ui| {
                    ui.label("File");
                    ui.add(
                        egui::TextEdit::singleline(&mut st.out).hint_text("a new file in the Capture folder"),
                    )
                    .on_hover_text(
                        "Empty: <document>-render-<time>.png in the Capture settings' folder (else the \
                         document's). A name alone goes there too.",
                    );
                });
            });
            heads.push(("studio:fold:output".into(), head));
            if !st.queue.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&st.queue).small());
            }
        });
    if (st.looks.clone(), st.light, st.hdri.clone()) != before {
        st.version += 1;
    }
    s.studio = st;
    s.places.extend(heads);
    s.places.push(("env:render:browser".into(), r));
    true
}

/// The live picture's state, the same in the view's badge and the status bar: preparing; starting
/// again while the raster view stands in; else its samples (one: "1 sample").
pub fn live_words(st: &StudioUi) -> String {
    match st.samples {
        0 => "Preparing".into(),
        s if s < READABLE => "Starting again…".into(),
        _ if refining(st) => format!("Tracing · {} samples", st.traced),
        s => format!(
            "{s} sample{}{}",
            if s == 1 { "" } else { "s" },
            if st.denoise { " · denoised" } else { "" }
        ),
    }
}

/// The live view is still adding samples to a picture already shown (it shows them at the
/// denoise's steps, so the picture may sit unchanged a while on a slow GPU).
pub fn refining(st: &StudioUi) -> bool {
    st.samples >= READABLE && st.traced < st.target && st.rendering.is_none()
}

/// The status bar's line: the live picture's samples, then what a render would make.
pub(super) fn status(s: &Shell) -> String {
    let st = &s.studio;
    let live = live_words(st);
    let queue = if st.queue.is_empty() {
        String::new()
    } else {
        format!(" · {}", st.queue)
    };
    format!("{live} · {} × {} output{queue}", st.size.0, st.size.1)
}

/// m:ss, or h:mm:ss.
fn clock(secs: f64) -> String {
    let s = secs.round() as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// The words under a render's bar: percent, samples, elapsed, left.
pub fn progress_words(r: &Rendering) -> String {
    let (elapsed, left) = r.times();
    let mut w = format!(
        "{:.0}% · {} of {} samples · {} elapsed",
        r.fraction() * 100.0,
        r.done,
        r.of,
        clock(elapsed)
    );
    if let Some(l) = left {
        w.push_str(&format!(" · about {} left", clock(l)));
    }
    w
}

/// The render's cards over the view: its progress with Cancel while it runs (from the first
/// frame); then its result, the picture with its file (Open folder, Copy path) or why it failed,
/// until closed.
fn render_cards(s: &mut Shell, ui: &mut Ui, view: Rect) {
    let t = s.theme;
    if let Some(r) = s.studio.rendering.clone() {
        let mut cancel = false;
        // in the view's own layer, over the picture (as the sample count is)
        let at = Rect::from_center_size(
            view.center_top() + egui::vec2(0.0, 12.0 + 36.0),
            egui::vec2(404.0, 72.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(at), |ui| {
            super::theme::panel_frame(&t)
                .inner_margin(egui::vec2(12.0, 8.0))
                .show(ui, |ui| {
                    ui.set_width(380.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&r.what).font(super::theme::semi(13.0)).color(t.ink));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let b = ui.button("Cancel");
                            s.places.push(("studio:cancel".into(), b.rect));
                            cancel = b.clicked();
                        });
                    });
                    let (bar, _) = ui.allocate_exact_size(egui::vec2(380.0, 6.0), egui::Sense::hover());
                    ui.painter().rect_filled(bar, 3.0, t.line);
                    let mut fill = bar;
                    fill.set_width(bar.width() * r.fraction());
                    ui.painter().rect_filled(fill, 3.0, t.accent);
                    ui.label(RichText::new(progress_words(&r)).small().color(t.ink2));
                });
        });
        if cancel {
            s.studio.asks.push(Ask::Cancel);
        }
        return;
    }
    let Some(res) = s.studio.result.clone() else {
        return;
    };
    let mut close = false;
    let mut reveal = None;
    let at = Rect::from_center_size(
        view.center_bottom() + egui::vec2(0.0, -14.0 - 76.0),
        egui::vec2(520.0, 152.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(at), |ui| {
        super::theme::panel_frame(&t)
            .inner_margin(egui::vec2(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some((tex, (w, h))) = res.thumb {
                        let k = (220.0 / w as f32).min(124.0 / h as f32);
                        ui.image(egui::load::SizedTexture::new(
                            tex,
                            egui::vec2(w as f32 * k, h as f32 * k),
                        ));
                    }
                    ui.vertical(|ui| match &res.said {
                        Ok(path) => {
                            ui.label(
                                RichText::new("Render saved")
                                    .font(super::theme::semi(13.0))
                                    .color(t.ink),
                            );
                            ui.add(egui::Label::new(RichText::new(path).small().color(t.ink2)).truncate());
                            ui.horizontal(|ui| {
                                if !cfg!(target_arch = "wasm32") && ui.button("Open folder").clicked() {
                                    reveal = Some(path.clone());
                                }
                                if ui.button("Copy path").clicked() {
                                    ui.ctx().copy_text(path.clone());
                                }
                                close = ui.button("Close").clicked();
                            });
                        }
                        Err(why) => {
                            ui.label(
                                RichText::new("Render failed")
                                    .font(super::theme::semi(13.0))
                                    .color(t.error),
                            );
                            ui.label(RichText::new(why).small().color(t.ink));
                            close = ui.button("Close").clicked();
                        }
                    });
                });
            });
    });
    if let Some(p) = reveal {
        s.studio.asks.push(Ask::Reveal(p));
    }
    if close {
        s.studio.result = None;
    }
}

/// The sample count over the view's lower left corner; while the picture still refines, with a
/// spinner (§7: between the denoise's steps the picture itself doesn't say so).
pub(super) fn draw(s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
    let t = s.theme;
    let st = &s.studio;
    let text = live_words(st);
    let busy = refining(st);
    // its own layer over the view, so the pointer reaches it (the view's takes it otherwise)
    let target = st.target;
    egui::Area::new(egui::Id::unique("studio-live-badge"))
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(view.left_bottom() + egui::vec2(10.0, -10.0))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ui.ctx(), |ui| {
            let r = super::theme::panel_frame(&t)
                .inner_margin(egui::vec2(8.0, 4.0))
                .show(ui, |ui| {
                    if busy {
                        super::ux::loading(ui, &t, &text);
                    } else {
                        ui.label(RichText::new(&text).font(super::theme::small()).color(t.ink2));
                    }
                })
                .response;
            if busy {
                // (a frame's own response doesn't sense the pointer)
                ui.interact(
                    r.rect,
                    egui::Id::unique("studio-live-badge-hover"),
                    egui::Sense::hover(),
                )
                .on_hover_text(format!("Refines until {target} samples; edits restart it"));
                ui.ctx().request_repaint();
            }
        });
    render_cards(s, ui, view);
    let st = &s.studio;
    let p = ui.painter();
    // the brush at work: what it applies, how to stop, what it did last
    if st.painting
        && let Some(b) = &st.brush
    {
        let mut text = format!("Painting {b} · click parts · Esc to stop");
        if !st.said.is_empty() {
            text = format!("{text}   ({})", st.said);
        }
        let g = p.layout_no_wrap(text, super::theme::sans(12.0), egui::Color32::WHITE);
        let at = view.center_top() + egui::vec2(-g.size().x / 2.0, 12.0);
        let r = Rect::from_min_size(at, g.size()).expand2(egui::vec2(10.0, 5.0));
        p.rect_filled(r, 4.0, egui::Color32::from_black_alpha(160));
        p.galley(at, g, egui::Color32::WHITE);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_badge_and_the_bar_say_the_same() {
        let mut st = StudioUi::default();
        assert_eq!(live_words(&st), "Preparing");
        st.samples = 1;
        assert_eq!(live_words(&st), "Starting again…");
        st.samples = READABLE;
        st.denoise = true;
        assert_eq!(live_words(&st), format!("{READABLE} samples · denoised"));
        st.samples = 1;
        st.denoise = false;
        st.samples = READABLE + 1;
        assert!(!live_words(&st).contains("denoised"));
        // a picture shown, more samples going in: it says it's still refining, with the count
        // traced (the picture shows fewer until the next denoise)
        st.samples = 16;
        st.traced = 24;
        st.target = 1024;
        assert!(refining(&st));
        assert_eq!(live_words(&st), "Tracing · 24 samples");
        // done: the picture's own count
        st.samples = 1024;
        st.traced = 1024;
        st.denoise = true;
        assert!(!refining(&st));
        assert_eq!(live_words(&st), "1024 samples · denoised");
    }
}
