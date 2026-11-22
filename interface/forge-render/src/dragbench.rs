//! The 16 ms benchmark: sets a number of each bench part again and again, as a drag of its field
//! would, headless, and times each value from being set to its frame being done: the Forge edit,
//! the rebuild through the warm stages, the changed faces meshed, uploaded and drawn.

use crate::bench::summary;
use crate::camera::Camera;
use crate::live::Mesher;
use crate::load;
use crate::render::{self, Offscreen, Overlay, Renderer, part_info};
use crate::scene::{Placement, Scene};
use crate::viewer::now_ms;
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;

/// How far a value swings, of itself: a slow drag either way.
const SWING: f64 = 0.02;
/// A part as slow as this stops short of its values, once it has shown this many frames.
const BUDGET_MS: f64 = 25_000.0;
const ENOUGH_FRAMES: usize = 8;
/// Numbers tried for each kind of drag before giving up on it.
const TRIES: usize = 12;

/// A number of the part: a driving param, its value as written and what it measures.
#[derive(Clone, Debug)]
pub struct Num {
    pub key: ParamKey,
    pub value: f64,
    pub dim: Dim,
}

impl Num {
    fn set(&self, value: f64) -> Op {
        Op::SetParam {
            param: self.key.clone(),
            value: Literal { value, dim: self.dim },
        }
    }
}

/// A part's text and how it builds: from its file's place, its shape only, as a drag shows it.
struct Part {
    src: String,
    opts: forge_lang::Options,
    cache: forge_lang::build::Cache,
}

impl Part {
    fn open(name: &str) -> Result<Part, String> {
        let src = load::source(name)?;
        let dir = match load::bench_src(name) {
            Some(_) => Some(load::bench_dir()?),
            None => std::path::Path::new(name).parent().map(|d| d.to_path_buf()),
        };
        let opts = forge_lang::Options {
            dir,
            checks: false,
            ..forge_lang::Options::default()
        };
        Ok(Part {
            src,
            opts,
            cache: Default::default(),
        })
    }

    /// The part with `op` applied to its text, built through the warm stages.
    fn run(&mut self, op: Option<&Op>) -> Result<forge_lang::Run, String> {
        let text = match op {
            Some(op) => forge_lang::edit::apply(&self.src, op).map_err(|e| e.text())?,
            None => self.src.clone(),
        };
        let (run, cache) = forge_lang::run(&text, self.opts.clone(), std::mem::take(&mut self.cache));
        self.cache = cache;
        Ok(run)
    }
}

/// What the part's faces are, as one number.
fn shape(run: &forge_lang::Run) -> u64 {
    let Some(b) = &run.body else { return 0 };
    b.face_ids()
        .fold(0, |h, f| h ^ forge_agent::facemesh::key(b, f).0)
}

/// The part's numbers: each param that is one number written as a literal.
fn nums(part: &mut Part) -> Result<Vec<(String, Num)>, String> {
    let run = part.run(None)?;
    if let Some(e) = run.errors.first() {
        return Err(format!("{}: {}", e.kind(), e.text()));
    }
    let (_, params, _) = crate::fg::rows(&run, &part.src);
    Ok(params
        .into_iter()
        .filter(|p| p.driving())
        .filter_map(|p| {
            let (value, dim) = p.value?;
            let num = Num {
                key: p.key.clone(),
                value,
                dim,
            };
            Some((p.key.to_string(), num))
        })
        .collect())
}

/// A part's numbers as `--dims` lists them: each param's name and its value as written.
pub fn dims(name: &str) -> Result<Vec<(String, String)>, String> {
    let mut part = Part::open(name)?;
    let run = part.run(None)?;
    let (_, params, _) = crate::fg::rows(&run, &part.src);
    Ok(params
        .into_iter()
        .map(|p| {
            let value = match p.value {
                Some((v, _)) if p.driving() => crate::shell::fg::short(v),
                _ => p.text.clone(),
            };
            (p.key.to_string(), value)
        })
        .collect())
}

/// Whether a part still builds with this number moved both ways by the swing, and changes shape
/// each way (some numbers are only a sketch's construction). How many stages it rebuilt: for a
/// shared param, one nothing much uses rebuilds only its own.
fn drags(part: &mut Part, n: &Num) -> Option<usize> {
    let was = shape(&part.run(None).ok()?);
    let mut most = 0;
    for k in [1.0 + SWING, 1.0 - SWING] {
        let run = part.run(Some(&n.set(n.value * k))).ok()?;
        if !run.errors.is_empty() || shape(&run) == was {
            return None;
        }
        most = most.max(run.rebuilt);
    }
    Some(most)
}

/// The numbers to drag: the first shared param that rebuilds more than its own stage, a
/// feature's halfway down the history, the last feature's.
fn targets(part: &mut Part) -> Result<Vec<(&'static str, String, Num)>, String> {
    let all: Vec<(String, Num)> = nums(part)?
        .into_iter()
        .filter(|n| n.1.value != 0.0 && matches!(n.1.dim, Dim::Length | Dim::Angle))
        .collect();
    let (shared, features): (Vec<_>, Vec<_>) = all.iter().partition(|n| n.1.key.is_shared());
    let mut out = Vec::new();
    if let Some(n) = shared
        .iter()
        .take(TRIES)
        .find(|n| drags(part, &n.1).is_some_and(|stages| stages > 1))
    {
        out.push(("param", n.0.clone(), n.1.clone()));
    }
    let k = features.len();
    if let Some(n) = features
        .iter()
        .skip(k / 2)
        .take(TRIES)
        .find(|n| drags(part, &n.1).is_some())
    {
        out.push(("mid", n.0.clone(), n.1.clone()));
    }
    if let Some(n) = features
        .iter()
        .rev()
        .take(TRIES)
        .find(|n| drags(part, &n.1).is_some())
    {
        out.push(("last", n.0.clone(), n.1.clone()));
    }
    Ok(out)
}

#[derive(Default)]
struct Run {
    latency: Vec<f64>,
    rebuild: Vec<f64>,
    kernel: Vec<f64>,
    stages: Vec<f64>,
    mesh: Vec<f64>,
    meshed: Vec<f64>,
    upload: Vec<f64>,
    bytes: Vec<f64>,
    draw: Vec<f64>,
    asked: u64,
    failed: u64,
    faces: usize,
    tris: u64,
    chord: f32,
    warm: f64,
}

fn drag_once(
    name: &str,
    n: &Num,
    ticks: usize,
    r: &mut Renderer,
    scene: &mut Scene,
    target: &Offscreen,
    cam: &Camera,
) -> Result<Run, String> {
    let mut part = Part::open(name)?;
    let mut run = Run::default();
    let t0 = now_ms();
    // the warm-up: the stages built and every face on the GPU, before any value
    let first = part.run(None)?;
    let body = first.body.as_ref().ok_or(format!("{name}: nothing built"))?;
    let mut mesher = Mesher::default();
    let d = mesher.delta(body);
    r.live_apply(0, &d);
    run.faces = d.faces;
    run.chord = d.chord;
    scene.set_part(0, d.center, d.radius, vec![d.chord]);
    r.live_show(true);
    r.frame(&target.view, scene, cam, Overlay::default());
    r.wait();
    run.warm = now_ms() - t0;
    run.tris = r.live_tris();
    let start = now_ms();
    for tick in 0..ticks {
        if now_ms() - start > BUDGET_MS && run.latency.len() >= ENOUGH_FRAMES {
            break;
        }
        let swing = forge_math::dmath::sin(tick as f64 / 120.0 * core::f64::consts::TAU);
        let v = n.value * (1.0 + SWING * swing);
        run.asked += 1;
        let t = now_ms();
        let built = part.run(Some(&n.set(v)));
        let Some(body) = built
            .as_ref()
            .ok()
            .filter(|b| b.errors.is_empty())
            .and_then(|b| b.body.as_ref())
        else {
            run.failed += 1;
            continue;
        };
        let b = built.as_ref().expect("built");
        run.rebuild.push(now_ms() - t);
        run.kernel.push(b.timings.kernel_ms);
        run.stages.push(b.rebuilt as f64);
        let m = now_ms();
        let d = mesher.delta(body);
        run.mesh.push(now_ms() - m);
        run.meshed.push(d.meshed as f64);
        let u = now_ms();
        run.bytes.push(r.live_apply(0, &d) as f64);
        run.upload.push(now_ms() - u);
        scene.set_part(0, d.center, d.radius, vec![d.chord]);
        let f = now_ms();
        r.frame(&target.view, scene, cam, Overlay::default());
        r.wait();
        let done = now_ms();
        run.draw.push(done - f);
        run.latency.push(done - t);
    }
    r.live_show(false);
    Ok(run)
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// Drags each part (all 15 if none are named) and prints a line a drag, then which miss 16 ms.
pub fn run(names: &[String], ticks: usize, size: (u32, u32)) -> Result<(), String> {
    let all: Vec<String> = if names.is_empty() {
        load::BENCH.iter().map(|n| n.to_string()).collect()
    } else {
        names.to_vec()
    };
    let instance = render::native_instance();
    let (device, queue, adapter) = render::block_on(render::open_device(&instance, None))?;
    let info = adapter.get_info();
    println!(
        "gpu: {} ({:?})  {}x{}, {} values one after another, each swinging {}%",
        info.name,
        info.device_type,
        size.0,
        size.1,
        ticks,
        SWING * 100.0
    );
    println!(
        "{:<9}{:<6}{:<16}{:>7}{:>8}{:>8}{:>8}{:>8}{:>8}{:>7}{:>7}{:>7}{:>7}{:>8}{:>8}{:>8}",
        "part",
        "drag",
        "number",
        "frames",
        "mean",
        "p99",
        "worst",
        "rebuild",
        "kernel",
        "stages",
        "mesh",
        "upload",
        "draw",
        "faces",
        "meshed",
        "KB/frame"
    );
    let mut misses: Vec<String> = Vec::new();
    for name in &all {
        let mut part = Part::open(name)?;
        let (mesh, _) = load::open(name, true)?;
        let parts = [mesh];
        let radius = parts[0].radius;
        let center = parts[0].center;
        for (kind, label, n) in targets(&mut part)? {
            let mut scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
            let mut r = Renderer::new(
                device.clone(),
                queue.clone(),
                info.clone(),
                render::OFFSCREEN,
                size,
                &parts,
                &scene,
            );
            let target = Offscreen::new(&r.device, size);
            let cam = Camera::fit(center, radius);
            let run = drag_once(name, &n, ticks, &mut r, &mut scene, &target, &cam)?;
            if run.latency.is_empty() {
                println!(
                    "{name:<9}{kind:<6}{label:<16} no frame ({} of {} values failed)",
                    run.failed, run.asked
                );
                misses.push(format!("{name} {kind}: no frame"));
                continue;
            }
            let s = summary(&run.latency);
            let spent = mean(&run.rebuild) + mean(&run.mesh) + mean(&run.upload) + mean(&run.draw);
            println!(
                "{name:<9}{kind:<6}{label:<16}{:>7}{:>8.2}{:>8.2}{:>8.2}{:>8.2}{:>8.2}{:>7.1}{:>7.2}{:>7.2}{:>7.2}{:>8}{:>8.1}{:>8.0}",
                run.latency.len(),
                s.mean,
                s.p99,
                s.worst,
                mean(&run.rebuild),
                mean(&run.kernel),
                mean(&run.stages),
                mean(&run.mesh),
                mean(&run.upload),
                mean(&run.draw),
                run.faces,
                mean(&run.meshed),
                mean(&run.bytes) / 1024.0,
            );
            if run.failed > 0 {
                println!("  {} of {} values did not build", run.failed, run.asked);
            }
            println!(
                "  warm-up {:.0} ms, {:.0}k tris at chord {}",
                run.warm,
                run.tris as f64 / 1000.0,
                run.chord
            );
            if s.p99 > 1000.0 / 60.0 {
                let parts = [
                    ("rebuild", mean(&run.rebuild)),
                    ("mesh", mean(&run.mesh)),
                    ("upload", mean(&run.upload)),
                    ("draw", mean(&run.draw)),
                ];
                let worst = parts
                    .iter()
                    .copied()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .expect("four");
                misses.push(format!(
                    "{name} {kind} ({label}): mean {:.1} p99 {:.1} ms; {} {:.1} ms is {:.0}% of the {:.1} ms spent",
                    s.mean,
                    s.p99,
                    worst.0,
                    worst.1,
                    worst.1 * 100.0 / spent,
                    spent
                ));
            }
        }
    }
    println!("\nmiss 16 ms at p99 ({}):", misses.len());
    for m in &misses {
        println!("  {m}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bench_parts_numbers_are_its_params_and_a_swing_of_one_changes_its_shape() {
        let mut part = Part::open("bracket").unwrap();
        let all = nums(&mut part).unwrap();
        assert!(!all.is_empty());
        let got = targets(&mut part).unwrap();
        assert!(!got.is_empty(), "something to drag on the bracket");
        for (_, label, n) in &got {
            assert!(drags(&mut part, n).is_some(), "{label}");
        }
        let d = dims("bracket").unwrap();
        assert!(d.iter().any(|(name, _)| *name == got[0].1), "{d:?}");
    }
}
