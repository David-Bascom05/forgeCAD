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
    let mut v = vec!["Studio (drawn)".to_string()];
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
    pub light: usize,
    pub hdri: String,
    pub turn_deg: f32,
    pub strength: f32,
    pub exposure: f32,
    pub floor: bool,
    pub denoise: bool,
    pub depth_of_field: bool,
    pub focal_mm: f32,
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
            // the bundled HDRI
            light: 1,
            hdri: String::new(),
            turn_deg: 0.0,
            strength: 1.0,
            exposure: -0.5,
            floor: true,
            denoise: true,
            depth_of_field: false,
            focal_mm: 85.0,
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
                tool("Turntable", "rotate", TURNTABLE, false),
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

/// A heading in the browser.
fn head(ui: &mut Ui, s: &Shell, text: &str) {
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
            head(ui, s, "Appearances");
            if st.looks.len() < st.names.len() {
                st.looks.resize(st.names.len(), None);
            }
            for (k, name) in st.names.iter().enumerate() {
                let shown = st.looks[k].clone().unwrap_or_else(|| "Material's own".into());
                egui::ComboBox::from_id_salt(("studio-look", k))
                    .selected_text(&shown)
                    .width(ui.available_width() - 4.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(st.looks[k].is_none(), "Material's own")
                            .clicked()
                        {
                            st.looks[k] = None;
                        }
                        let mut family = "";
                        for a in lib.all() {
                            if a.family != family {
                                family = &a.family;
                                ui.label(RichText::new(family).small().weak());
                            }
                            let on = st.looks[k].as_deref() == Some(a.name.as_str());
                            if ui
                                .selectable_label(on, &a.name)
                                .on_hover_text(&a.source)
                                .clicked()
                            {
                                st.looks[k] = Some(a.name.clone());
                            }
                        }
                    })
                    .response
                    .on_hover_text(name);
                ui.label(RichText::new(name).small().weak());
            }

            head(ui, s, "Lighting");
            let lights = lights();
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
            ui.add(egui::Slider::new(&mut st.turn_deg, -180.0..=180.0).text("Turn °"));
            ui.add(
                egui::Slider::new(&mut st.strength, 0.05..=8.0)
                    .logarithmic(true)
                    .text("Strength"),
            );
            ui.add(egui::Slider::new(&mut st.exposure, -4.0..=4.0).text("Exposure EV"));

            head(ui, s, "Camera");
            ui.add(
                egui::Slider::new(&mut st.focal_mm, 14.0..=300.0)
                    .logarithmic(true)
                    .text("Focal mm"),
            );
            ui.add_enabled(
                st.depth_of_field,
                egui::Slider::new(&mut st.f_number, 1.0..=22.0)
                    .logarithmic(true)
                    .text("f/"),
            );

            head(ui, s, "Output");
            ui.horizontal(|ui| {
                ui.label("Size");
                ui.add(egui::DragValue::new(&mut st.size.0).range(16..=8192));
                ui.label("×");
                ui.add(egui::DragValue::new(&mut st.size.1).range(16..=8192));
            });
            ui.add(
                egui::Slider::new(&mut st.spp, 16..=8192)
                    .logarithmic(true)
                    .text("Samples"),
            );
            ui.add(egui::Slider::new(&mut st.frames, 4..=360).text("Turntable frames"));
            ui.horizontal(|ui| {
                ui.label("File");
                ui.text_edit_singleline(&mut st.out);
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
    false
}
