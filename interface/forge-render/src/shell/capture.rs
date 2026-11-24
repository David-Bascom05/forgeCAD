//! Capture, on every environment's ribbon and the View tab (research/render/record.md): Photo
//! (the view as a PNG at the size set), Record (the environment's timeline stepped frame by frame
//! in its own time, or, with no timeline, the camera once round the model) to MP4 or PNG frames,
//! and their settings. The GUI does the work (`gui_record.rs`); this holds what was chosen, what
//! was asked, and the progress and results it reports.

use super::ribbon::{Group, Tool};
use super::{Action, Shell};
use crate::shell::placed::Placed;
use egui::{RichText, pos2, vec2};

/// A capture command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capture {
    Photo,
    Record,
    Cancel,
    Settings,
    /// Show the last file in its folder (native).
    Reveal,
}

/// What a recording writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// H.264 in MP4 (ffmpeg natively, WebCodecs in the browser); PNG frames when neither is there.
    Mp4,
    /// One PNG a frame, in a folder (natively) or a zip (the browser).
    Frames,
}

/// A picture's size: the view's own, a standard one, or the person's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Window,
    Hd,
    Uhd,
    Square,
    Custom(u32, u32),
}

impl Size {
    pub const PRESETS: [Size; 4] = [Size::Window, Size::Hd, Size::Uhd, Size::Square];

    pub fn words(self) -> String {
        match self {
            Size::Window => "Window".into(),
            Size::Hd => "1920 × 1080".into(),
            Size::Uhd => "3840 × 2160".into(),
            Size::Square => "Square 2048".into(),
            Size::Custom(w, h) => format!("Custom {w} × {h}"),
        }
    }

    /// Its pixels, the view being `window` pixels.
    pub fn pixels(self, window: (u32, u32)) -> (u32, u32) {
        match self {
            Size::Window => window,
            Size::Hd => (1920, 1080),
            Size::Uhd => (3840, 2160),
            Size::Square => (2048, 2048),
            Size::Custom(w, h) => (w, h),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaptureUi {
    pub photo_size: Size,
    pub size: Size,
    pub fps: u32,
    /// With no timeline: the turntable's length, seconds.
    pub orbit_seconds: f32,
    pub format: Format,
    /// The view cube in the picture too (off: the picture is the model alone).
    pub overlays: bool,
    /// The view's camera exactly; off: the subject framed to fill the picture (70 %) from the
    /// view's direction, whatever the picture's shape (photos and turntables).
    pub exact_view: bool,
    /// The folder photos and recordings go to; empty: the document's folder.
    pub folder: String,
    /// The exact file (a photo's PNG, a video's MP4, or the frames' folder) the next capture
    /// writes instead of a fresh name in the folder: `forge-app --photo/--record`, `forgecad record`.
    pub out: Option<String>,
    pub settings_open: bool,
    /// Asked, for the GUI.
    pub asks: Vec<Capture>,
    /// A recording under way: frame, of.
    pub recording: Option<(u32, u32)>,
    /// What the last capture did, the file it wrote (natively, for Open folder), and when (ms).
    pub last: String,
    pub last_file: Option<String>,
    pub last_at: f64,
}

impl Default for CaptureUi {
    fn default() -> CaptureUi {
        CaptureUi {
            photo_size: Size::Hd,
            size: Size::Hd,
            fps: 30,
            orbit_seconds: 12.0,
            format: Format::Mp4,
            overlays: false,
            exact_view: false,
            folder: String::new(),
            out: None,
            settings_open: false,
            asks: Vec::new(),
            recording: None,
            last: String::new(),
            last_file: None,
            last_at: 0.0,
        }
    }
}

/// The group: Photo large; Record (or, while recording, Stop Recording) and the settings small.
/// Record records the timeline when the environment has one, else a turntable.
pub(in crate::shell) fn group(s: &Shell) -> Option<Group> {
    let c = &s.capture;
    let open = s.fg.is_some();
    let busy = c.recording.is_some();
    let record = if busy {
        Tool {
            name: "Stop Recording",
            icon: "close",
            action: Some(Action::Capture(Capture::Cancel)),
            on: true,
        }
    } else {
        Tool {
            name: if s.env_timeline().is_some() {
                "Record"
            } else {
                "Record Turntable"
            },
            icon: "rotate",
            action: open.then_some(Action::Capture(Capture::Record)),
            on: false,
        }
    };
    Some(Group {
        name: "Capture",
        big: vec![Tool {
            name: "Photo",
            icon: "export",
            action: (open && !busy).then_some(Action::Capture(Capture::Photo)),
            on: false,
        }],
        small: vec![
            record,
            Tool {
                name: "Capture Settings",
                icon: "options",
                action: Some(Action::Capture(Capture::Settings)),
                on: c.settings_open,
            },
        ],
        icons: vec![],
    })
}

/// Sets the framework's slot once: every environment's ribbon gets the group.
pub(super) fn install() {
    let _ = super::env::CAPTURE.set(group);
}

pub fn command_of(s: &mut Shell, c: Capture) {
    match c {
        Capture::Settings => s.capture.settings_open = !s.capture.settings_open,
        // the shortcut's Record stops a recording under way
        Capture::Record if s.capture.recording.is_some() => s.capture.asks.push(Capture::Cancel),
        c => s.capture.asks.push(c),
    }
}

const LABEL_W: f32 = 130.0;

fn row(ui: &mut egui::Ui, label: String, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            vec2(LABEL_W, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(LABEL_W);
                ui.label(label);
            },
        );
        control(ui);
    });
}

/// Radio buttons in a row with room between them, so each reads as its own.
fn radios<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) {
    for (k, (v, text)) in options.iter().enumerate() {
        if k > 0 {
            ui.add_space(10.0);
        }
        ui.radio_value(value, *v, *text);
    }
}

/// The folder captures go to: its last part (`…/name`), the whole path on hover, Choose… for the
/// system's picker, and Reset back to the document's folder.
fn folder_row(ui: &mut egui::Ui, folder: &mut String) {
    let f = folder.trim();
    let shown = if f.is_empty() {
        "the document's folder".to_string()
    } else {
        let name = std::path::Path::new(f)
            .file_name()
            .map_or_else(|| f.to_string(), |n| n.to_string_lossy().into_owned());
        format!("…/{name}")
    };
    let r = ui.add(egui::Label::new(shown).truncate());
    if !f.is_empty() {
        r.on_hover_text(f);
    }
    if ui.button("Choose…").clicked()
        && let Some(p) = crate::plat::pick_folder("Folder for photos and recordings")
    {
        *folder = p;
    }
    if !folder.trim().is_empty() && ui.small_button("Reset").clicked() {
        folder.clear();
    }
}

fn size_row(ui: &mut egui::Ui, id: &str, label: &str, size: &mut Size, window: (u32, u32)) {
    row(ui, label.into(), |ui| {
        let (w, h) = size.pixels(window);
        egui::ComboBox::from_id_salt(id)
            .selected_text(match *size {
                Size::Window => format!("Window ({w} × {h})"),
                s => s.words(),
            })
            .show_ui(ui, |ui| {
                for p in Size::PRESETS {
                    let text = match p {
                        Size::Window => format!("Window ({} × {})", window.0, window.1),
                        p => p.words(),
                    };
                    ui.selectable_value(size, p, text);
                }
                let custom = matches!(*size, Size::Custom(..));
                if ui.selectable_label(custom, "Custom").clicked() && !custom {
                    *size = Size::Custom(w, h);
                }
            });
    });
    if let Size::Custom(w, h) = size {
        row(ui, String::new(), |ui| {
            ui.add(egui::DragValue::new(w).range(16..=8192).suffix(" px"));
            ui.label("×");
            ui.add(egui::DragValue::new(h).range(16..=8192).suffix(" px"));
        });
    }
}

/// The settings window, the recording's pill (Stop, Esc), and the last result's line.
pub(super) fn panels(s: &mut Shell, ctx: &egui::Context) {
    let t = s.theme;
    let ppp = ctx.pixels_per_point();
    let window = (
        (s.view_rect.width() * ppp).round() as u32,
        (s.view_rect.height() * ppp).round() as u32,
    );
    if s.capture.settings_open {
        let mut open = true;
        let mut c = std::mem::take(&mut s.capture);
        let mut places = std::mem::take(&mut s.places);
        let v = s.view_rect;
        let r = egui::Window::new("Capture Settings")
            .id(egui::Id::unique("capture-settings"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(super::fgasm::dialog_frame(&t))
            .default_pos(pos2(v.right() - 380.0, v.top() + 60.0))
            .constrain_to(v)
            .show_placed(ctx, &mut places, |ui, places| {
                if super::fgasm::dialog_head(
                    ui,
                    &t,
                    "options",
                    "Capture Settings",
                    "capture:settings",
                    places,
                ) {
                    open = false;
                }
                ui.add_space(4.0);
                ui.spacing_mut().slider_width = 170.0;
                ui.spacing_mut().item_spacing.y = 8.0;
                size_row(ui, "capture-photo-size", "Photo size", &mut c.photo_size, window);
                ui.separator();
                size_row(ui, "capture-video-size", "Video size", &mut c.size, window);
                row(ui, "Frame rate".into(), |ui| {
                    radios(ui, &mut c.fps, &[(24, "24 fps"), (30, "30 fps"), (60, "60 fps")]);
                });
                row(ui, "Turntable".into(), |ui| {
                    ui.add(egui::Slider::new(&mut c.orbit_seconds, 2.0..=60.0).show_value(false));
                    ui.label(format!("{:.0} s", c.orbit_seconds));
                });
                row(ui, "Record as".into(), |ui| {
                    radios(
                        ui,
                        &mut c.format,
                        &[(Format::Mp4, "MP4 video"), (Format::Frames, "PNG frames")],
                    );
                });
                ui.separator();
                row(ui, "Framing".into(), |ui| {
                    ui.checkbox(&mut c.exact_view, "Use the view exactly");
                });
                row(ui, "View overlays".into(), |ui| {
                    ui.checkbox(&mut c.overlays, "Include the view cube");
                });
                // the browser's downloads have no folder to choose
                if !cfg!(target_arch = "wasm32") {
                    row(ui, "Folder".into(), |ui| folder_row(ui, &mut c.folder));
                }
            });
        // Enter and Esc close it, as a dialog's keys do
        let keys = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
        if keys && c.recording.is_none() {
            open = false;
        }
        if let Some(r) = r {
            s.view_blocks.push(r.response.rect);
            places.push(("capture:settings".into(), r.response.rect));
        }
        s.places = places;
        c.settings_open = open;
        s.capture = c;
    }
    if let Some((k, n)) = s.capture.recording {
        let v = s.view_rect;
        // the view takes no input while it's being recorded; Esc stops
        s.view_blocks.push(v);
        let mut stop = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::Area::new(egui::Id::unique("capture-progress"))
            .pivot(egui::Align2::CENTER_TOP)
            .fixed_pos(pos2(v.center().x, v.top() + 10.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                super::theme::panel_frame(&t)
                    .inner_margin(vec2(super::theme::S3, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Recording · frame {} of {n}", (k + 1).min(n)))
                                    .font(super::theme::sans(13.0))
                                    .color(t.ink),
                            );
                            // a plain bar: a track and its fill, no knob
                            ui.add_space(10.0);
                            let (bar, _) = ui.allocate_exact_size(vec2(120.0, 6.0), egui::Sense::hover());
                            let p = ui.painter();
                            p.rect_filled(bar, 3.0, t.line);
                            let done = (k as f32 / n.max(1) as f32).clamp(0.0, 1.0);
                            if done > 0.0 {
                                let mut fill = bar;
                                fill.set_width(bar.width() * done);
                                p.rect_filled(fill, 3.0, t.accent);
                            }
                            ui.add_space(10.0);
                            let r = ui.button(RichText::new("Stop").font(super::theme::sans(13.0)));
                            s.places.push(("capture:stop".into(), r.rect));
                            stop |= r.clicked();
                        });
                    });
            });
        if stop {
            s.capture.asks.push(Capture::Cancel);
        }
        ctx.request_repaint();
        return;
    }
    // the last result, a few seconds over the view's foot, with Open folder natively
    let now = ctx.input(|i| i.time) * 1000.0;
    if s.capture.last.is_empty() || now - s.capture.last_at > 8000.0 {
        return;
    }
    let v = s.view_rect;
    let mut reveal = false;
    egui::Area::new(egui::Id::unique("capture-result"))
        .pivot(egui::Align2::CENTER_BOTTOM)
        .fixed_pos(pos2(v.center().x, v.bottom() - 14.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            super::theme::panel_frame(&t)
                .inner_margin(vec2(super::theme::S3, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&s.capture.last)
                                .font(super::theme::sans(13.0))
                                .color(t.ink),
                        );
                        if !cfg!(target_arch = "wasm32") && s.capture.last_file.is_some() {
                            ui.add_space(12.0);
                            let r = ui.link(RichText::new("Open folder").font(super::theme::sans(13.0)));
                            s.places.push(("capture:reveal".into(), r.rect));
                            reveal = r.clicked();
                        }
                    });
                });
        });
    if reveal {
        s.capture.asks.push(Capture::Reveal);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(500));
}
