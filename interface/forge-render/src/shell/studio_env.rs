//! The Render Studio environment's content (RENDER-STUDIO's; `env/render.rs` only delegates):
//! its ribbon, its browser (each part's appearance, the lighting, the camera, the output), and
//! what it writes over the view. The picture itself is the GUI's (`gui_studio.rs`): this holds
//! what the user chose, and the GUI reads it each frame.

use super::env::EnvId;
use super::ribbon::{Group, Tool};
use super::{Action, Shell};
use crate::studio::appearance::Appearances;
use egui::{Rect, RichText, Ui};

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

/// A render the user asked for, for the GUI to queue.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Image,
    Turntable,
    Cancel,
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
    pub out: String,
    /// Bumped when anything the GUI must rebuild for changes (appearances, the light).
    pub version: u64,
    pub asks: Vec<Ask>,
    /// What the GUI says: samples in the view, the queue's state.
    pub samples: u32,
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
            out: "render.png".into(),
            version: 0,
            asks: Vec::new(),
            samples: 0,
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
    let busy = !st.queue.is_empty();
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
                tool("Render Image", "file", RENDER_IMAGE, false),
                tool("Render Turntable", "rotate", TURNTABLE, false),
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
fn fold(ui: &mut Ui, s: &Shell, id: &str, summary: &str, body: impl FnOnce(&mut Ui)) {
    ui.add_space(2.0);
    egui::CollapsingHeader::new(
        RichText::new(summary)
            .font(super::theme::semi(13.0))
            .color(s.theme.ink),
    )
    .id_salt(id)
    .default_open(false)
    .show(ui, body);
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
            fold(
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

            let lens = if st.view_focal_mm > 0.0 {
                format!("{:.0} mm", st.view_focal_mm)
            } else {
                "parallel".into()
            };
            fold(ui, s, "studio-fold-camera", &format!("Camera · {lens}"), |ui| {
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

            let out = format!("Output · {} × {}, {} samples", st.size.0, st.size.1, st.spp);
            fold(ui, s, "studio-fold-output", &out, |ui| {
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
                    ui.text_edit_singleline(&mut st.out);
                });
            });
            if !st.queue.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&st.queue).small());
            }
        });
    if (st.looks.clone(), st.light, st.hdri.clone()) != before {
        st.version += 1;
    }
    s.studio = st;
    s.places.push(("env:render:browser".into(), r));
    true
}

/// The status bar's line: the live picture's samples, then what a render would make.
pub(super) fn status(s: &Shell) -> String {
    let st = &s.studio;
    let live = if st.samples == 0 {
        "preparing".to_string()
    } else {
        format!(
            "{} samples{}",
            st.samples,
            if st.denoise { " · denoised" } else { "" }
        )
    };
    let queue = if st.queue.is_empty() {
        String::new()
    } else {
        format!(" · {}", st.queue)
    };
    format!("{live} · {} × {} output{queue}", st.size.0, st.size.1)
}

/// The sample count over the view's lower left corner.
pub(super) fn draw(s: &mut Shell, ui: &mut Ui, view: Rect) -> bool {
    let st = &s.studio;
    let text = if st.samples == 0 {
        "Render Studio: preparing".to_string()
    } else {
        format!(
            "{} samples{}",
            st.samples,
            if st.denoise { " · denoised" } else { "" }
        )
    };
    let p = ui.painter();
    let g = p.layout_no_wrap(text, super::theme::sans(12.0), egui::Color32::WHITE);
    let at = view.left_bottom() + egui::vec2(10.0, -10.0 - g.size().y);
    let r = Rect::from_min_size(at, g.size()).expand2(egui::vec2(8.0, 4.0));
    p.rect_filled(r, 4.0, egui::Color32::from_black_alpha(140));
    p.galley(at, g, egui::Color32::WHITE);
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
