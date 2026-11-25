//! The Studio's view and its render queue: the tracer kept going a few samples a frame while
//! nothing changes, started again when anything does, denoised as it goes; and final renders (an
//! image, a turntable's frames) run one after another the same way, so the window never waits.

use super::bsdf::{Bsdf, Lut};
use super::env::Env;
use super::gpu::{DENOISE_PASSES, Tracer};
use super::scene::{Built, Instance};
use super::{Settings, View};
use crate::mesh::PartMesh;

/// How much the view's trace may do a frame: rows of a sample (a sample is traced a band at a
/// time, so a slow GPU keeps the window answering), fitted to what the GPU took for the last
/// measured band.
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    /// Rows a frame (a multiple of 8; more than the picture's height: whole samples).
    pub rows: u32,
    /// The GPU time a frame's trace may take, ms.
    pub budget_ms: f64,
    /// The least a measured band took: what the measure takes whatever the rows (in the browser
    /// the queue says it's done a frame or so late), not counted against them.
    pub floor_ms: f64,
}

impl Default for Pace {
    fn default() -> Pace {
        Pace {
            rows: 64,
            budget_ms: 8.0,
            floor_ms: f64::INFINITY,
        }
    }
}

/// The most rows a frame: 64 samples of the tallest picture.
const MOST_ROWS: u32 = 64 * 4096;

impl Pace {
    /// The GPU took `ms` for `rows` rows: the rows that fill the budget.
    pub fn fit(&mut self, ms: f64, rows: u32) {
        self.floor_ms = self.floor_ms.min(ms);
        self.fit_cost(ms - self.floor_ms, rows);
    }

    /// `rows` rows cost the frame `ms`: the rows that fill the budget.
    pub fn fit_cost(&mut self, ms: f64, rows: u32) {
        let per_row = (ms / rows.max(1) as f64).max(1e-6);
        let fit = (self.budget_ms / per_row) as u32;
        // no more than doubling at once: a measure taken while the GPU was idle overstates it
        self.rows = (fit.min(self.rows.saturating_mul(2)) / 8 * 8).clamp(8, MOST_ROWS);
    }
}

/// The page's frames, the pace's measure in the browser (whose queue says its work is done only
/// a frame or so late, too coarse to time a band): each 16 frames, two trace nothing, the second's
/// time a bare frame's; the others' times, past a bare frame's, are what their rows cost. Fitted
/// once a 16, growing no more than a quarter, between an eighth of a sample and a sample a frame
/// (a page's frames go out without waiting, so their times answer late: a fit to each overshoots).
#[derive(Default)]
struct Frames {
    /// When the last frame began, and the rows it sent.
    at: Option<(f64, u32)>,
    /// The frame's place in the 16 (0 and 1: no rows).
    k: u32,
    /// A bare frame's time, ms.
    bare: Option<f64>,
    /// This 16's traced frames: their times, rows, how many.
    sum: (f64, f64, u32),
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl Frames {
    /// A frame begins in a picture `h` rows high: the last one's time taken, the pace fitted.
    /// True: send no rows this frame.
    fn tick(&mut self, pace: &mut Pace, h: u32) -> bool {
        let now = crate::viewer::now_ms();
        if let Some((at, sent)) = self.at
            && now - at < 5000.0
        {
            let ms = now - at;
            if self.k == 2 {
                self.bare = Some(self.bare.map_or(ms, |b| b + 0.3 * (ms - b)));
            } else if self.k >= 3 || self.k == 0 {
                self.sum = (self.sum.0 + ms, self.sum.1 + sent as f64, self.sum.2 + 1);
            }
        }
        if self.k == 0 {
            if let (Some(bare), (t, r, n @ 1..)) = (self.bare, self.sum) {
                let (t, r) = (t / n as f64, r / n as f64);
                // a slow page's frames may grow by a quarter for the trace
                pace.budget_ms = 8f64.max(0.25 * bare);
                let per_row = ((t - bare).max(0.0) / r.max(1.0)).max(1e-6);
                let fit = (pace.budget_ms / per_row).min(pace.rows as f64 * 1.25) as u32;
                let lo = (h / 8).max(8);
                pace.rows = (fit / 8 * 8).clamp(lo, h.max(lo));
            }
            self.sum = (0.0, 0.0, 0);
        }
        let probe = self.k < 2;
        self.k = (self.k + 1) % 16;
        self.at = Some((now, 0));
        probe
    }

    fn sent(&mut self, rows: u32) {
        if let Some((_, s)) = &mut self.at {
            *s = rows;
        }
    }
}

/// The sample counts the view's picture is denoised and shown at: from 8 (`READABLE`), each
/// doubling, then every 64, and the last.
fn shown_at(samples: u32, target: u32) -> bool {
    samples == target || (samples >= 8 && (samples.is_power_of_two() || samples.is_multiple_of(64)))
}

/// The view's tracer and what its sum was made of.
pub struct Live {
    pub tracer: Tracer,
    pub lut: Lut,
    pub pace: Pace,
    /// Stop adding samples here: the picture is as good as asked.
    pub target: u32,
    last: Option<(View, Settings)>,
    has_scene: bool,
    has_env: bool,
    /// The next row of the sample under way.
    row: u32,
    /// The samples in the picture shown.
    shown: u32,
    /// The denoiser's next pass and row, while a denoise is spread over frames (the trace
    /// waits for it).
    stage: Option<(u32, u32)>,
    /// A band measured: when it was sent, its rows; `done` says when the GPU had finished it.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    measuring: Option<(f64, u32)>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    done: std::sync::Arc<std::sync::Mutex<Option<f64>>>,
    /// The browser's measure (`Frames`).
    frames: Frames,
}

impl Live {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) -> Live {
        Live {
            tracer: Tracer::new(device, queue, size),
            lut: Lut::new(512),
            pace: Pace::default(),
            target: 1024,
            last: None,
            has_scene: false,
            has_env: false,
            row: 0,
            shown: 0,
            stage: None,
            measuring: None,
            done: Default::default(),
            frames: Frames::default(),
        }
    }

    pub fn set_scene(
        &mut self,
        parts: &[PartMesh],
        placed: &[Instance],
        materials: &[Bsdf],
    ) -> Result<(), String> {
        let built = Built::new(parts, placed);
        self.tracer.set_scene(&built, materials, &self.lut)?;
        self.has_scene = true;
        Ok(())
    }

    /// A scene already built (its hierarchy kept for final renders too).
    pub fn set_built(&mut self, built: &Built, materials: &[Bsdf]) -> Result<(), String> {
        self.tracer.set_scene(built, materials, &self.lut)?;
        self.has_scene = true;
        Ok(())
    }

    pub fn set_materials(&mut self, materials: &[Bsdf]) {
        self.tracer.set_materials(materials, &self.lut);
    }

    pub fn set_env(&mut self, env: &Env) {
        self.tracer.set_env(env);
        self.has_env = true;
    }

    /// Ready to trace: a scene and a light.
    pub fn ready(&self) -> bool {
        self.has_scene && self.has_env
    }

    /// The samples in the picture shown (it's shown at `shown_at`'s counts).
    pub fn samples(&self) -> u32 {
        self.shown
    }

    /// The samples traced so far (the picture shown may have fewer).
    pub fn traced(&self) -> u32 {
        self.tracer.samples
    }

    /// One frame's worth: started again if the camera, the settings or the size changed, then
    /// `pace.rows` more rows of work: samples (none past `target`) and, at `shown_at`'s counts,
    /// the denoise (when asked) that shows them, a band of rows at a time like the samples. True:
    /// work was done.
    pub fn frame(&mut self, view: View, settings: Settings, size: (u32, u32), denoise: bool) -> bool {
        if !self.ready() || size.0 == 0 || size.1 == 0 {
            return false;
        }
        if size != self.tracer.size {
            self.tracer.resize(size);
            self.last = None;
        }
        if self.last != Some((view, settings)) {
            self.restart();
            self.last = Some((view, settings));
        }
        // natively, what the GPU took for the band measured (its callback comes with a poll); in
        // the browser, the page's frames
        #[cfg(not(target_arch = "wasm32"))]
        let probe = {
            let _ = self.tracer.device.poll(wgpu::PollType::Poll);
            if let Some((sent, rows)) = self.measuring
                && let Some(at) = self.done.lock().ok().and_then(|mut d| d.take())
            {
                self.pace.fit(at - sent, rows);
                self.measuring = None;
            }
            false
        };
        #[cfg(target_arch = "wasm32")]
        let probe = self.frames.tick(&mut self.pace, size.1);
        if probe {
            return true;
        }
        let h = size.1;
        // this frame's rows: a denoise's passes under way first (the trace waits for it), then
        // the samples, each a band at a time
        let mut left = self.pace.rows;
        let mut sent = 0;
        while left > 0 {
            if let Some((k, y)) = self.stage {
                let rows = left.min(h - y);
                self.tracer.denoise_rows(k, y, rows);
                sent += rows;
                left -= rows;
                let next = if y + rows >= h { (k + 1, 0) } else { (k, y + rows) };
                if next.0 == DENOISE_PASSES {
                    self.tracer.show_denoised(settings.exposure);
                    self.shown = self.tracer.samples;
                    self.stage = None;
                } else {
                    self.stage = Some(next);
                }
                continue;
            }
            if self.tracer.samples >= self.target {
                break;
            }
            let rows = left.min(h - self.row);
            self.tracer.trace_rows(&view, &settings, self.row, rows);
            sent += rows;
            left -= rows;
            self.row += rows;
            if self.row < h {
                continue;
            }
            self.row = 0;
            self.tracer.sample_done();
            let s = self.tracer.samples;
            if !denoise {
                self.tracer.show(settings.exposure, false);
                self.shown = s;
            } else if shown_at(s, self.target) {
                self.stage = Some((0, 0));
            }
        }
        self.frames.sent(sent);
        #[cfg(not(target_arch = "wasm32"))]
        if self.measuring.is_none() && sent > 0 {
            let done = self.done.clone();
            self.measuring = Some((crate::viewer::now_ms(), sent));
            self.tracer.queue.on_submitted_work_done(move || {
                if let Ok(mut d) = done.lock() {
                    *d = Some(crate::viewer::now_ms());
                }
            });
        }
        sent > 0
    }

    /// Something not in the view or the settings changed (a light's map, appearances): again.
    pub fn restart(&mut self) {
        self.tracer.reset();
        self.row = 0;
        self.shown = 0;
        self.stage = None;
    }
}

/// A final render's kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// One picture to this file.
    Image(String),
    /// `frames` pictures turning about z, to `stem_0000.png`...
    Turntable { stem: String, frames: u32 },
}

/// A render waiting or under way.
#[derive(Clone, Debug)]
pub struct Job {
    pub output: Output,
    pub size: (u32, u32),
    pub spp: u32,
    pub denoise: bool,
    pub view: View,
    pub settings: Settings,
    /// The turntable's centre: frames turn the camera about z through it.
    pub pivot: [f32; 3],
}

impl Job {
    pub fn frames(&self) -> u32 {
        match self.output {
            Output::Image(_) => 1,
            Output::Turntable { frames, .. } => frames.max(1),
        }
    }

    /// A frame's file.
    pub fn path(&self, frame: u32) -> String {
        match &self.output {
            Output::Image(p) => p.clone(),
            Output::Turntable { stem, .. } => format!("{}_{frame:04}.png", stem.trim_end_matches(".png")),
        }
    }

    /// The camera for a frame: turned about z through the pivot.
    pub fn view_at(&self, frame: u32) -> View {
        let a = frame as f32 / self.frames() as f32 * std::f32::consts::TAU;
        if a == 0.0 {
            return self.view;
        }
        let (s, c) = a.sin_cos();
        let p = self.pivot;
        let turn = |v: [f32; 3]| [c * v[0] - s * v[1], s * v[0] + c * v[1], v[2]];
        let e = self.view.eye;
        let rel = turn([e[0] - p[0], e[1] - p[1], e[2]]);
        View {
            eye: [p[0] + rel[0], p[1] + rel[1], rel[2]],
            forward: turn(self.view.forward),
            right: turn(self.view.right),
            up: turn(self.view.up),
            ..self.view
        }
    }
}

/// What the queue is doing.
#[derive(Clone, Debug, PartialEq)]
pub enum Progress {
    Idle,
    /// Job, frame, samples done of all.
    Running {
        job: usize,
        frame: u32,
        done: u32,
        of: u32,
    },
}

/// Final renders, one at a time, a few samples a frame.
pub struct Queue {
    pub jobs: Vec<Job>,
    /// Files written, and failures, newest last.
    pub log: Vec<Result<String, String>>,
    tracer: Option<Tracer>,
    at: (usize, u32),
    /// The frame finished and on its way back from the GPU.
    reading: Option<super::gpu::Pending>,
    pub per_frame: u32,
}

impl Default for Queue {
    fn default() -> Queue {
        Queue {
            jobs: Vec::new(),
            log: Vec::new(),
            tracer: None,
            at: (0, 0),
            reading: None,
            per_frame: 8,
        }
    }
}

impl Queue {
    pub fn push(&mut self, job: Job) {
        self.jobs.push(job);
    }

    pub fn progress(&self) -> Progress {
        // a job asked and not yet begun is under way at none of its samples (its tracer is made
        // on the next tick, after the frame that shows it)
        match (&self.tracer, self.jobs.get(self.at.0)) {
            (t, Some(j)) => Progress::Running {
                job: self.at.0,
                frame: self.at.1,
                done: t.as_ref().map_or(0, |t| t.samples),
                of: j.spp,
            },
            _ => Progress::Idle,
        }
    }

    /// Stops the job under way and drops the rest.
    pub fn cancel(&mut self) {
        self.tracer = None;
        self.reading = None;
        self.jobs.clear();
        self.at = (0, 0);
    }

    /// A frame's step: the job under way gets samples; a finished frame comes back from the GPU
    /// over the next ticks (nothing waits) and is written with
    /// `save(path, size, rgba)` (native: a file; the browser: a download). The scene is the
    /// view's, given each time it's needed: `scene()` -> its triangles and appearances.
    pub fn tick(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: impl FnOnce() -> Option<(Built, Vec<Bsdf>, Lut)>,
        env: &Env,
        mut save: impl FnMut(&str, (u32, u32), Vec<u8>) -> Result<(), String>,
    ) {
        let Some(job) = self.jobs.get(self.at.0).cloned() else {
            self.tracer = None;
            self.jobs.clear();
            self.at = (0, 0);
            return;
        };
        if self.tracer.is_none() {
            let Some((built, mats, lut)) = scene() else {
                self.log.push(Err("nothing to render".into()));
                self.cancel();
                return;
            };
            let mut t = Tracer::new(device, queue, job.size);
            if let Err(e) = t.set_scene(&built, &mats, &lut) {
                self.log.push(Err(e));
                self.cancel();
                return;
            }
            t.set_env(env);
            self.tracer = Some(t);
        }
        if let Some(r) = &self.reading {
            let Some(got) = r.take(device) else {
                return;
            };
            self.reading = None;
            let path = job.path(self.at.1);
            self.log
                .push(got.and_then(|rgba| save(&path, job.size, rgba)).map(|_| path));
            let t = self.tracer.as_mut().expect("it read");
            t.reset();
            self.at.1 += 1;
            if self.at.1 >= job.frames() {
                self.at = (self.at.0 + 1, 0);
                self.tracer = None;
            }
            return;
        }
        let t = self.tracer.as_mut().expect("made above");
        let view = job.view_at(self.at.1);
        let n = self.per_frame.min(job.spp - t.samples.min(job.spp));
        if n > 0 {
            t.step(&view, &job.settings, n);
        }
        if t.samples < job.spp {
            return;
        }
        t.show(job.settings.exposure, job.denoise);
        self.reading = Some(t.start_read_rgba());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pace_fills_the_budget_and_backs_off() {
        // a GPU at 0.01 ms a row, measured 20 ms late whatever the rows (as a page's queue says)
        let took = |rows: u32| 20.0 + 0.01 * rows as f64;
        let mut p = Pace::default();
        // it grows no more than doubling at once, to about the budget's 800 rows past the floor
        p.fit(took(64), 64);
        assert_eq!(p.rows, 128);
        for _ in 0..12 {
            let r = p.rows;
            p.fit(took(r), r);
        }
        assert!((780..=880).contains(&p.rows) && p.rows % 8 == 0, "{}", p.rows);
        // slower: it backs off at once, to the rows the budget holds
        let r = p.rows;
        p.fit(20.0 + 0.1 * r as f64, r);
        assert!((72..=96).contains(&p.rows), "{}", p.rows);
        // however slow, a band of 8 rows a frame still goes
        p.fit(1e4, p.rows);
        assert_eq!(p.rows, 8);
    }

    #[test]
    fn the_picture_is_shown_at_doublings_then_every_64() {
        let at: Vec<u32> = (1..=300).filter(|&s| shown_at(s, 1024)).collect();
        assert_eq!(at, [8, 16, 32, 64, 128, 192, 256]);
        assert!(shown_at(5, 5), "the last sample is shown");
    }

    #[test]
    fn a_turntable_turns_about_its_pivot() {
        let view = View {
            eye: [10.0, 0.0, 5.0],
            forward: [-1.0, 0.0, 0.0],
            right: [0.0, 1.0, 0.0],
            up: [0.0, 0.0, 1.0],
            fovy: 0.5,
            parallel: 0.0,
            aperture: 0.0,
            focus: 10.0,
        };
        let job = Job {
            output: Output::Turntable {
                stem: "out/t.png".into(),
                frames: 4,
            },
            size: (8, 8),
            spp: 1,
            denoise: false,
            view,
            settings: Settings::default(),
            pivot: [0.0; 3],
        };
        assert_eq!(job.path(3), "out/t_0003.png");
        let v = job.view_at(1);
        assert!(
            (v.eye[0]).abs() < 1e-4 && (v.eye[1] - 10.0).abs() < 1e-4 && v.eye[2] == 5.0,
            "{:?}",
            v.eye
        );
        assert!((v.forward[1] + 1.0).abs() < 1e-5);
    }
}
