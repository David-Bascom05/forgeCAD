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
    /// The view drawn offscreen at `size`, coming back.
    fn draw_view_at(&mut self, size: (u32, u32)) -> Result<Pending, String> {
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
        v.renderer
            .frame(&target.view, &mut v.scene, &v.nav.camera, Overlay::default());
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
            crate::studio::save_png(&job.path, job.size, &opaque(rgba)).map(|_| job.path.clone())
        }))
    }

    /// Whether a photo or a recording is under way (the window draws again until it's done).
    pub fn capture_busy(&self) -> bool {
        self.photo.is_some() || self.recording.is_some()
    }

    /// Where captures go, and the name they start with: the folder set, else the open file's.
    fn capture_place(&self) -> (std::path::PathBuf, String) {
        let file = self.shell.fg.as_ref().map(|f| f.path.clone()).unwrap_or_default();
        let p = std::path::Path::new(&file);
        let stem = p
            .file_stem()
            .map_or_else(|| "forgecad".to_string(), |s| s.to_string_lossy().into_owned());
        let dir = if !self.shell.capture.folder.trim().is_empty() {
            std::path::PathBuf::from(self.shell.capture.folder.trim())
        } else {
            p.parent().map(std::path::Path::to_path_buf).unwrap_or_default()
        };
        (dir, stem)
    }

    /// The first `stem-what-N.ext` not already there.
    fn fresh_name(dir: &std::path::Path, stem: &str, what: &str, ext: &str) -> String {
        (1..)
            .map(|k| dir.join(format!("{stem}-{what}-{k}{ext}")))
            .find(|p| !forge_dirs::fs::exists(p))
            .expect("some number is free")
            .to_string_lossy()
            .into_owned()
    }

    /// Starts a recording at the capture settings' size and rate.
    pub fn record_start(&mut self) -> Result<(), String> {
        if self.view.is_none() {
            return Err("nothing in view".into());
        }
        let c = &self.shell.capture;
        let (fps, size, format) = (c.fps.max(1), c.size, c.format);
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
        let n = ((duration * fps as f64).round() as u32).max(1);
        let (dir, stem) = self.capture_place();
        let frames_sink = |note: String| {
            let d = Gui::fresh_name(&dir, &stem, &format!("{what}-frames"), "");
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
                    let path = Gui::fresh_name(&dir, &stem, what, ".mp4");
                    Sink::Video(crate::record::ffmpeg::Video::start(&ff, &path, size, fps)?)
                }
                None => frames_sink(crate::record::ffmpeg::no_ffmpeg_note("{frames}", fps)),
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
                    Ok(p) => format!("Recorded {p} ({} frames, {enc})", job.n),
                    Err(e) => format!("Recording failed: {e}"),
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
                        match crate::record::zip::store(&files)
                            .and_then(|z| forge_dirs::fs::write(&path, &z).map_err(|e| e.to_string()))
                        {
                            Ok(()) => format!("{base} to {path}"),
                            Err(e) => format!("Recording failed: {e}"),
                        }
                    }
                    None => format!("{base} in {dir}"),
                };
                if !note.is_empty() {
                    said = format!("{said}; {}", note.replace("{frames}", &dir));
                }
                said
            }
            None => String::new(),
        };
        self.shell.capture.last = said.clone();
        self.shell.status = said;
    }

    /// A frame's step of whatever capture is under way, and the asks the ribbon left.
    pub(super) fn capture_tick(&mut self) {
        for ask in std::mem::take(&mut self.shell.capture.asks) {
            let r = match ask {
                Capture::Photo => {
                    let (dir, stem) = self.capture_place();
                    let path = Gui::fresh_name(&dir, &stem, "photo", ".png");
                    self.photo_start(self.shell.capture.photo_size, &path)
                }
                Capture::Record => self.record_start(),
                Capture::Cancel => {
                    self.record_end(true);
                    Ok(())
                }
                Capture::Settings => Ok(()),
            };
            if let Err(e) = r {
                self.shell.capture.last = e.clone();
                self.shell.status = format!("Capture: {e}");
            }
        }
        if let Some(done) = self.photo_poll() {
            let said = match done {
                Ok(p) => format!("Photo saved: {p}"),
                Err(e) => format!("Photo failed: {e}"),
            };
            self.shell.capture.last = said.clone();
            self.shell.status = said;
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
                    self.shell.status = format!("Recording failed: {e}");
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
                    self.shell.status = format!("Recording failed: {e}");
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
