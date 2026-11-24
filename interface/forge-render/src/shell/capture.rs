//! Capture, on every environment's ribbon and the View tab (research/render/record.md): Photo
//! (the view as a PNG at the size set), Record (the environment's timeline stepped frame by frame
//! in its own time, or, with no timeline, the camera once round the model) to MP4 or PNG frames,
//! and their settings. The GUI does the work (`gui_record.rs`); this holds what was chosen, what
//! was asked, and the progress and results it reports.

use super::ribbon::{Group, Tool};
use super::{Action, Shell};
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
    /// The folder photos and recordings go to; empty: the document's folder.
    pub folder: String,
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
            folder: String::new(),
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
        let v = s.view_rect;
        let r = egui::Window::new("Capture Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(v.right() - 360.0, v.top() + 60.0))
            .constrain_to(v)
            .show(ctx, |ui| {
                size_row(ui, "capture-photo-size", "Photo size", &mut c.photo_size, window);
                ui.separator();
                size_row(ui, "capture-video-size", "Video size", &mut c.size, window);
                row(ui, "Frame rate".into(), |ui| {
                    for f in [24, 30, 60] {
                        ui.radio_value(&mut c.fps, f, format!("{f} fps"));
                    }
                });
                row(ui, format!("Turntable {:.0} s", c.orbit_seconds), |ui| {
                    ui.add(egui::Slider::new(&mut c.orbit_seconds, 2.0..=60.0).show_value(false));
                });
                row(ui, "Record as".into(), |ui| {
                    ui.radio_value(&mut c.format, Format::Mp4, "MP4 video");
                    ui.radio_value(&mut c.format, Format::Frames, "PNG frames");
                });
                row(ui, "View overlays".into(), |ui| {
                    ui.checkbox(&mut c.overlays, "Include the view cube");
                });
                row(ui, "Folder".into(), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut c.folder)
                            .hint_text("the document's folder")
                            .desired_width(180.0),
                    );
                });
            });
        // Enter and Esc close it, as a dialog's keys do
        let keys = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
        if keys && c.recording.is_none() {
            open = false;
        }
        if let Some(r) = r {
            s.view_blocks.push(r.response.rect);
            s.places.push(("capture:settings".into(), r.response.rect));
        }
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
                            ui.add(egui::ProgressBar::new(k as f32 / n.max(1) as f32).desired_width(120.0));
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
