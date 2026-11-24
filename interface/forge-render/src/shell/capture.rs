//! Capture, on every environment's ribbon and the View tab (research/render/record.md): Photo
//! (the view as a PNG at the size set), Record (the environment's timeline stepped frame by frame
//! in its own time, or, with no timeline, the camera once round the model) to MP4 or PNG frames,
//! and their settings. The GUI does the work (`gui_record.rs`); this holds what was chosen, what
//! was asked, and the progress it reports.

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
}

/// What a recording writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// H.264 in MP4 (ffmpeg natively, WebCodecs in the browser); PNG frames when neither is there.
    Mp4,
    /// One PNG a frame, in a folder (natively) or a zip (the browser).
    Frames,
}

#[derive(Clone, Debug)]
pub struct CaptureUi {
    pub photo_size: (u32, u32),
    pub size: (u32, u32),
    pub fps: u32,
    /// With no timeline: the turntable's length, seconds.
    pub orbit_seconds: f32,
    pub format: Format,
    /// The folder photos and recordings go to; empty: the document's folder, else the current one.
    pub folder: String,
    pub settings_open: bool,
    /// Asked, for the GUI.
    pub asks: Vec<Capture>,
    /// A recording under way: frame, of.
    pub recording: Option<(u32, u32)>,
    /// What the last capture did.
    pub last: String,
}

impl Default for CaptureUi {
    fn default() -> CaptureUi {
        CaptureUi {
            photo_size: (1920, 1080),
            size: (1280, 720),
            fps: 30,
            orbit_seconds: 6.0,
            format: Format::Mp4,
            folder: String::new(),
            settings_open: false,
            asks: Vec::new(),
            recording: None,
            last: String::new(),
        }
    }
}

/// The group: Photo and Record large, Cancel and the settings small. Record is off with nothing
/// in view; it records the timeline when the environment has one, else a turntable.
pub(in crate::shell) fn group(s: &Shell) -> Option<Group> {
    let c = &s.capture;
    let open = s.fg.is_some();
    let busy = c.recording.is_some();
    let record_name = if s.env_timeline().is_some() {
        "Record"
    } else {
        "Record Turntable"
    };
    Some(Group {
        name: "Capture",
        big: vec![
            Tool {
                name: "Photo",
                icon: "export",
                action: (open && !busy).then_some(Action::Capture(Capture::Photo)),
                on: false,
            },
            Tool {
                name: record_name,
                icon: "rotate",
                action: (open && !busy).then_some(Action::Capture(Capture::Record)),
                on: busy,
            },
        ],
        small: vec![
            Tool {
                name: "Stop Recording",
                icon: "close",
                action: busy.then_some(Action::Capture(Capture::Cancel)),
                on: false,
            },
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
        c => s.capture.asks.push(c),
    }
}

fn size_row(ui: &mut egui::Ui, label: &str, size: &mut (u32, u32)) {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 18.0], egui::Label::new(label));
        ui.add(egui::DragValue::new(&mut size.0).range(16..=8192));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut size.1).range(16..=8192));
    });
}

/// The settings window and, while recording, its progress over the view with Stop.
pub(super) fn panels(s: &mut Shell, ctx: &egui::Context) {
    let t = s.theme;
    if s.capture.settings_open {
        let mut open = true;
        let mut c = std::mem::take(&mut s.capture);
        egui::Window::new("Capture Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(s.view_rect.right() - 340.0, s.view_rect.top() + 60.0))
            .show(ctx, |ui| {
                size_row(ui, "Photo size", &mut c.photo_size);
                ui.separator();
                size_row(ui, "Video size", &mut c.size);
                ui.horizontal(|ui| {
                    ui.add_sized([110.0, 18.0], egui::Label::new(format!("{} frames/s", c.fps)));
                    ui.add(egui::Slider::new(&mut c.fps, 10..=60).show_value(false));
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [110.0, 18.0],
                        egui::Label::new(format!("Turntable {:.0} s", c.orbit_seconds)),
                    );
                    ui.add(egui::Slider::new(&mut c.orbit_seconds, 1.0..=60.0).show_value(false));
                });
                ui.horizontal(|ui| {
                    ui.add_sized([110.0, 18.0], egui::Label::new("Record as"));
                    ui.radio_value(&mut c.format, Format::Mp4, "MP4 video");
                    ui.radio_value(&mut c.format, Format::Frames, "PNG frames");
                });
                ui.horizontal(|ui| {
                    ui.add_sized([110.0, 18.0], egui::Label::new("Folder"));
                    ui.text_edit_singleline(&mut c.folder)
                        .on_hover_text("empty: beside the open file");
                });
                if !c.last.is_empty() {
                    ui.label(RichText::new(&c.last).small().color(t.ink2));
                }
            });
        c.settings_open = open;
        s.capture = c;
    }
    let Some((k, n)) = s.capture.recording else {
        return;
    };
    let v = s.view_rect;
    let mut stop = false;
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
                            RichText::new(format!("Recording frame {} of {n}", (k + 1).min(n)))
                                .font(super::theme::sans(13.0))
                                .color(t.ink),
                        );
                        ui.add(egui::ProgressBar::new(k as f32 / n.max(1) as f32).desired_width(120.0));
                        let r = ui.button(RichText::new("Stop").font(super::theme::sans(13.0)));
                        s.places.push(("capture:stop".into(), r.rect));
                        stop = r.clicked();
                    });
                });
        });
    if stop {
        s.capture.asks.push(Capture::Cancel);
    }
}
