//! Photos and recordings of the view (research/render/record.md, `shell/capture.rs`): the view
//! drawn offscreen at the size asked from the same camera and read back without blocking (the
//! browser can't wait). A recording steps the environment's timeline in its own time, frame k at
//! t = k / fps (or, with no timeline, turns the camera once round the model), waits a frame for the
//! window to show that state, draws it, and hands it to ffmpeg (natively) or keeps PNG frames.

use super::Gui;
use crate::render::{Offscreen, Overlay};
use crate::shell::capture::{Capture, Format};
use crate::studio::gpu::{Pending, read_texture};

/// A photo under way: the picture coming back, where it goes.
pub struct PhotoJob {
    pending: Pending,
    path: String,
    size: (u32, u32),
}

/// What the last photo did: the file written, or why not.
pub type PhotoDone = Result<String, String>;

/// Where a recording's frames go.
enum Sink {
    #[cfg(not(target_arch = "wasm32"))]
    Video(crate::record::ffmpeg::Video),
    /// PNG files in a folder (natively), or collected for a zip (the browser).
    Frames {
        dir: String,
        zip: Option<Vec<(String, Vec<u8>)>>,
        note: String,
    },
}

enum Stage {
    /// Put the state at frame k's time.
    Seek,
    /// It's in the window's state: draw it.
    Draw,
    Reading(Pending),
}

/// A recording under way.
pub struct RecordJob {
    k: u32,
    n: u32,
    fps: u32,
    size: (u32, u32),
    /// None: the environment's timeline; Some: the camera it started from, turned once round.
    orbit: Option<crate::camera::Camera>,
    stage: Stage,
    sink: Option<Sink>,
}

/// Opaque: the view's texture holds the scheme's alpha.
fn opaque(mut rgba: Vec<u8>) -> Vec<u8> {
    for a in rgba.iter_mut().skip(3).step_by(4) {
        *a = 255;
    }
    rgba
}

fn png_bytes(size: (u32, u32), rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, size.0, size.1);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .and_then(|mut w| w.write_image_data(rgba))
        .map_err(|e| e.to_string())?;
    Ok(out)
}

impl Gui {
    /// The view drawn offscreen at `size`, coming back: the model alone, or with the view cube
    /// when the capture settings ask for overlays.
    fn draw_view_at(&mut self, size: (u32, u32)) -> Result<Pending, String> {
        let overlays = self.shell.capture.overlays;
        if size.0 == 0 || size.1 == 0 || size.0 > 8192 || size.1 > 8192 {
            return Err(format!(
                "a picture is 1 to 8192 pixels a side, not {} x {}",
                size.0, size.1
            ));
        }
        let Some(v) = &mut self.view else {
            return Err("nothing in view".into());
        };
        let was = v.size;
        let target = Offscreen::new(&self.device, size);
        v.renderer.resize(size);
        let overlay = Overlay {
            cube: overlays.then(|| (v.nav.cube_place(), None)),
            ..Overlay::default()
        };
        v.renderer
            .frame(&target.view, &mut v.scene, &v.nav.camera, overlay);
        v.renderer.resize(was);
        v.dirty = true;
        Ok(read_texture(&self.device, &self.queue, &target.texture))
    }

    /// Draws the view at `size` and starts reading it back; `photo_poll` writes it to `path`.
    pub fn photo_start(&mut self, size: (u32, u32), path: &str) -> Result<(), String> {
        let pending = self.draw_view_at(size)?;
        self.photo = Some(PhotoJob {
            pending,
            path: path.to_string(),
            size,
        });
        Ok(())
    }

    /// A frame's look at the photo under way: Some when it finished (written, or why not).
    pub fn photo_poll(&mut self) -> Option<PhotoDone> {
        let job = self.photo.as_ref()?;
        let got = job.pending.take(&self.device)?;
        let job = self.photo.take()?;
        Some(got.and_then(|rgba| {
            let bytes = png_bytes(job.size, &opaque(rgba))?;
            deliver(&job.path, &bytes);
            Ok(job.path.clone())
        }))
    }

    /// Whether a photo or a recording is under way (the window draws again until it's done).
    pub fn capture_busy(&self) -> bool {
        self.photo.is_some() || self.recording.is_some()
    }

    /// Where captures go, and the name they start with: the folder set, else the open file's;
    /// `<document>-<environment>` (the View tab's captures say `view`).
    fn capture_place(&self) -> (std::path::PathBuf, String) {
        let file = self.shell.fg.as_ref().map(|f| f.path.clone()).unwrap_or_default();
        let p = std::path::Path::new(&file);
        let stem = p
            .file_stem()
            .map_or_else(|| "forgecad".to_string(), |s| s.to_string_lossy().into_owned());
        let env = self.shell.env.map_or("view", |e| e.key());
        let dir = if !self.shell.capture.folder.trim().is_empty() {
            std::path::PathBuf::from(self.shell.capture.folder.trim())
        } else {
            p.parent().map(std::path::Path::to_path_buf).unwrap_or_default()
        };
        (dir, format!("{stem}-{env}"))
    }

    /// `name-yyyymmdd-hhmmss.ext` (UTC), with `-2`, `-3` after it if that's taken.
    fn fresh_name(dir: &std::path::Path, name: &str, ext: &str) -> String {
        let secs = forge_math::clock::system_now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let stamp = utc_stamp(secs);
        (1..)
            .map(|k| {
                let n = if k == 1 {
                    format!("{name}-{stamp}{ext}")
                } else {
                    format!("{name}-{stamp}-{k}{ext}")
                };
                dir.join(n)
            })
            .find(|p| !forge_dirs::fs::exists(p))
            .expect("some number is free")
            .to_string_lossy()
            .into_owned()
    }

    /// The window's view in device pixels: what Size::Window means.
    fn window_px(&self) -> (u32, u32) {
        self.view.as_ref().map_or((1280, 720), |v| v.size)
    }

    /// A capture's result, said over the view and in the status bar.
    fn capture_said(&mut self, said: String, file: Option<String>) {
        self.shell.capture.last = said.clone();
        self.shell.capture.last_file = file;
        self.shell.capture.last_at = self.ctx.input(|i| i.time) * 1000.0;
        self.shell.status = said;
    }

    /// Starts a recording at the capture settings' size and rate.
    pub fn record_start(&mut self) -> Result<(), String> {
        if self.view.is_none() {
            return Err("nothing in view".into());
        }
        let c = &self.shell.capture;
        let (fps, size, format) = (c.fps.max(1), c.size.pixels(self.window_px()), c.format);
        let (duration, orbit, what) = match self.shell.env_timeline() {
            Some(t) => (
                t.duration(&self.shell),
                None,
                self.shell.env.map_or("timeline", |e| e.key()),
            ),
            None => (
                c.orbit_seconds as f64,
                self.view.as_ref().map(|v| v.nav.camera),
                "turntable",
            ),
        };
        let _ = what;
        let n = ((duration * fps as f64).round() as u32).max(1);
        let (dir, stem) = self.capture_place();
        let frames_sink = |note: String| {
            let d = Gui::fresh_name(&dir, &format!("{stem}-frames"), "");
            Sink::Frames {
                dir: d,
                zip: cfg!(target_arch = "wasm32").then(Vec::new),
                note,
            }
        };
        let sink = match format {
            Format::Frames => frames_sink(String::new()),
            #[cfg(not(target_arch = "wasm32"))]
            Format::Mp4 => match crate::record::ffmpeg::locate() {
                Some(ff) => {
                    let path = Gui::fresh_name(&dir, &stem, ".mp4");
                    Sink::Video(crate::record::ffmpeg::Video::start(&ff, &path, size, fps)?)
                }
                None => frames_sink(format!(
                    "Couldn't record MP4: no ffmpeg (forgecad-update installs it); saved PNG frames instead. {}",
                    crate::record::ffmpeg::no_ffmpeg_note("{frames}", fps)
                )),
            },
            #[cfg(target_arch = "wasm32")]
            Format::Mp4 => frames_sink("MP4 in the browser is coming: PNG frames meanwhile".into()),
        };
        if let Sink::Frames { dir, zip: None, .. } = &sink {
            forge_dirs::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
        }
        self.recording = Some(RecordJob {
            k: 0,
            n,
            fps,
            size,
            orbit,
            stage: Stage::Seek,
            sink: Some(sink),
        });
        self.shell.capture.recording = Some((0, n));
        Ok(())
    }

    fn record_end(&mut self, cancelled: bool) {
        let Some(mut job) = self.recording.take() else {
            return;
        };
        self.shell.capture.recording = None;
        if let (Some(cam), Some(v)) = (job.orbit, &mut self.view) {
            v.nav.camera = cam;
            v.dirty = true;
        }
        let mut file = None;
        let said = match job.sink.take() {
            #[cfg(not(target_arch = "wasm32"))]
            Some(Sink::Video(video)) if cancelled => {
                video.cancel();
                "Recording stopped".to_string()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Some(Sink::Video(video)) => {
                let enc = video.encoder;
                match video.finish() {
                    Ok(p) => {
                        let name = file_name(&p);
                        file = Some(p);
                        format!("Video saved: {name} ({} frames, {enc})", job.n)
                    }
                    Err(e) => format!("Couldn't record: {e}"),
                }
            }
            Some(Sink::Frames { dir, zip, note }) => {
                let base = if cancelled {
                    format!("Recording stopped: {} frames in {dir}", job.k)
                } else {
                    format!("Recorded {} frames", job.n)
                };
                let mut said = match zip {
                    Some(files) => {
                        let path = format!("{dir}.zip");
                        match crate::record::zip::store(&files) {
                            Ok(z) => {
                                let name = file_name(&path);
                                deliver(&path, &z);
                                format!(
                                    "{base}: {name} {}",
                                    if cfg!(target_arch = "wasm32") {
                                        "downloaded"
                                    } else {
                                        "saved"
                                    }
                                )
                            }
                            Err(e) => format!("Couldn't record: {e}"),
                        }
                    }
                    None => {
                        file = Some(format!("{dir}/frame_00000.png"));
                        format!("{base}: {}", file_name(&dir))
                    }
                };
                if !note.is_empty() {
                    said = note.replace("{frames}", &dir);
                }
                said
            }
            None => String::new(),
        };
        self.capture_said(said, file);
    }

    /// A frame's step of whatever capture is under way, and the asks the ribbon left.
    pub(super) fn capture_tick(&mut self) {
        for ask in std::mem::take(&mut self.shell.capture.asks) {
            let r = match ask {
                Capture::Photo => {
                    let (dir, stem) = self.capture_place();
                    let path = Gui::fresh_name(&dir, &stem, ".png");
                    self.photo_start(self.shell.capture.photo_size.pixels(self.window_px()), &path)
                }
                Capture::Reveal => {
                    if let Some(f) = self.shell.capture.last_file.clone() {
                        reveal(&f);
                    }
                    Ok(())
                }
                Capture::Record => self.record_start(),
                Capture::Cancel => {
                    self.record_end(true);
                    Ok(())
                }
                Capture::Settings => Ok(()),
            };
            if let Err(e) = r {
                self.capture_said(format!("Couldn't capture: {e}"), None);
            }
        }
        if let Some(done) = self.photo_poll() {
            match done {
                Ok(p) => {
                    let name = file_name(&p);
                    let how = if cfg!(target_arch = "wasm32") {
                        "downloaded"
                    } else {
                        "saved"
                    };
                    self.capture_said(format!("Photo {how}: {name}"), Some(p));
                }
                Err(e) => self.capture_said(format!("Couldn't save the photo: {e}"), None),
            }
        }
        self.record_step();
    }

    fn record_step(&mut self) {
        let Some(mut job) = self.recording.take() else {
            return;
        };
        let t = job.k as f64 / job.fps as f64;
        match &job.stage {
            Stage::Seek => {
                match job.orbit {
                    Some(cam) => {
                        if let Some(v) = &mut self.view {
                            let turns = t / (job.n as f64 / job.fps as f64);
                            v.nav.camera = cam;
                            v.nav.camera.yaw = cam.yaw + (turns * std::f64::consts::TAU) as f32;
                            v.dirty = true;
                        }
                    }
                    None => {
                        if let Some(tl) = self.shell.env_timeline() {
                            tl.seek(&mut self.shell, t);
                        }
                    }
                }
                job.stage = Stage::Draw;
            }
            Stage::Draw => match self.draw_view_at(job.size) {
                Ok(p) => job.stage = Stage::Reading(p),
                Err(e) => {
                    self.recording = Some(job);
                    self.record_end(true);
                    self.capture_said(format!("Couldn't record: {e}"), None);
                    return;
                }
            },
            Stage::Reading(p) => {
                let Some(got) = p.take(&self.device) else {
                    self.recording = Some(job);
                    return;
                };
                let pushed = got.and_then(|rgba| {
                    let rgba = opaque(rgba);
                    match job.sink.as_mut().expect("a sink while recording") {
                        #[cfg(not(target_arch = "wasm32"))]
                        Sink::Video(v) => v.push(&rgba),
                        Sink::Frames { dir, zip, .. } => {
                            let bytes = png_bytes(job.size, &rgba)?;
                            let name = format!("frame_{:05}.png", job.k);
                            match zip {
                                Some(files) => {
                                    files.push((name, bytes));
                                    Ok(())
                                }
                                None => forge_dirs::fs::write(format!("{dir}/{name}"), &bytes)
                                    .map_err(|e| e.to_string()),
                            }
                        }
                    }
                });
                if let Err(e) = pushed {
                    self.recording = Some(job);
                    self.record_end(true);
                    self.capture_said(format!("Couldn't record: {e}"), None);
                    return;
                }
                job.k += 1;
                job.stage = Stage::Seek;
                if job.k >= job.n {
                    self.shell.capture.recording = Some((job.k, job.n));
                    self.recording = Some(job);
                    self.record_end(false);
                    return;
                }
            }
        }
        self.shell.capture.recording = Some((job.k, job.n));
        self.recording = Some(job);
    }
}

fn file_name(p: &str) -> String {
    std::path::Path::new(p)
        .file_name()
        .map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned())
}

/// Writes a capture: natively a file; in the browser the project's file and a download.
fn deliver(path: &str, bytes: &[u8]) {
    if let Some(d) = std::path::Path::new(path).parent()
        && !d.as_os_str().is_empty()
    {
        let _ = forge_dirs::fs::create_dir_all(d);
    }
    let _ = forge_dirs::fs::write(path, bytes);
    #[cfg(target_arch = "wasm32")]
    crate::webapp::download(&file_name(path), bytes);
}

/// The file's folder in the system's file manager.
fn reveal(file: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = std::path::Path::new(file)
            .parent()
            .map_or_else(|| std::path::PathBuf::from("."), std::path::Path::to_path_buf);
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(windows) {
            "explorer"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(opener).arg(dir).spawn();
    }
    #[cfg(target_arch = "wasm32")]
    let _ = file;
}

/// A UNIX time as `yyyymmdd-hhmmss` (UTC; Hinnant's civil-from-days).
fn utc_stamp(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_unix_time_reads_as_its_utc_date() {
        assert_eq!(super::utc_stamp(0), "19700101-000000");
        // 2026-11-24 04:45:00 UTC
        assert_eq!(super::utc_stamp(1_795_495_500), "20261124-044500");
        assert_eq!(super::utc_stamp(951_782_400), "20000229-000000");
    }
}
