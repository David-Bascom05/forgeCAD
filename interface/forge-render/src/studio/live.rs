//! The Studio's view and its render queue: the tracer kept going a few samples a frame while
//! nothing changes, started again when anything does, denoised as it goes; and final renders (an
//! image, a turntable's frames) run one after another the same way, so the window never waits.

use super::bsdf::{Bsdf, Lut};
use super::env::Env;
use super::gpu::Tracer;
use super::scene::{Built, Instance};
use super::{Settings, View};
use crate::mesh::PartMesh;

/// How many samples a frame may take: grown while frames come quickly, shrunk when they don't.
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    pub per_frame: u32,
    /// A frame's time the tracer may fill, ms.
    pub budget_ms: f64,
}

impl Default for Pace {
    fn default() -> Pace {
        Pace {
            per_frame: 1,
            budget_ms: 16.0,
        }
    }
}

impl Pace {
    /// A frame took `ms` with `per_frame` samples in it: the next frame's count.
    pub fn after(&mut self, ms: f64) {
        if ms < self.budget_ms * 0.6 && self.per_frame < 64 {
            self.per_frame += 1.max(self.per_frame / 4);
        } else if ms > self.budget_ms * 1.5 && self.per_frame > 1 {
            self.per_frame = (self.per_frame * 2 / 3).max(1);
        }
    }
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
        }
    }

    pub fn set_scene(&mut self, parts: &[PartMesh], placed: &[Instance], materials: &[Bsdf]) -> Result<(), String> {
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

    pub fn samples(&self) -> u32 {
        self.tracer.samples
    }

    /// One frame's worth: started again if the camera, the settings or the size changed, then
    /// `pace.per_frame` more samples (none past `target`), shown denoised. True: the picture
    /// changed.
    pub fn frame(&mut self, view: View, settings: Settings, size: (u32, u32)) -> bool {
        if !self.ready() || size.0 == 0 || size.1 == 0 {
            return false;
        }
        if size != self.tracer.size {
            self.tracer.resize(size);
            self.last = None;
        }
        if self.last != Some((view, settings)) {
            self.tracer.reset();
            self.last = Some((view, settings));
        }
        if self.tracer.samples >= self.target {
            return false;
        }
        let n = self.pace.per_frame.min(self.target - self.tracer.samples);
        self.tracer.step(&view, &settings, n);
        self.tracer.show(settings.exposure, true);
        true
    }

    /// Something not in the view or the settings changed (a light's map, appearances): again.
    pub fn restart(&mut self) {
        self.tracer.reset();
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
    Running { job: usize, frame: u32, done: u32, of: u32 },
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
        match (&self.tracer, self.jobs.get(self.at.0)) {
            (Some(t), Some(j)) => Progress::Running {
                job: self.at.0,
                frame: self.at.1,
                done: t.samples,
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
            self.log.push(got.and_then(|rgba| save(&path, job.size, rgba)).map(|_| path));
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
        let mut p = Pace::default();
        for _ in 0..40 {
            p.after(4.0);
        }
        assert!(p.per_frame > 10, "{}", p.per_frame);
        let was = p.per_frame;
        p.after(40.0);
        assert!(p.per_frame < was);
        for _ in 0..40 {
            p.after(100.0);
        }
        assert_eq!(p.per_frame, 1);
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
        assert!((v.eye[0]).abs() < 1e-4 && (v.eye[1] - 10.0).abs() < 1e-4 && v.eye[2] == 5.0, "{:?}", v.eye);
        assert!((v.forward[1] + 1.0).abs() < 1e-5);
    }
}
