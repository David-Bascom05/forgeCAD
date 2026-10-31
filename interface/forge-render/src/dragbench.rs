//! The 16 ms benchmark: drags a number on each bench part at 60 values a second, headless, and
//! times each value from being set to its frame being done: rebuild, mesh, upload, draw.

use crate::bench::summary;
use crate::camera::Camera;
use crate::live::{self, Out};
use crate::load;
use crate::render::{self, Offscreen, Overlay, Renderer, part_info};
use crate::scene::{Placement, Scene};
use crate::viewer::now_ms;
use forge_agent::live::{Event, Live};
use forge_agent::tools::{Dim, Part};
use std::collections::HashMap;

/// How far a value swings, of itself: a slow drag either way.
const SWING: f64 = 0.02;
const TICK_MS: f64 = 1000.0 / 60.0;
/// A part as slow as this stops short of its ticks, once it has shown this many frames.
const BUDGET_MS: f64 = 25_000.0;
const ENOUGH_FRAMES: usize = 8;
/// Numbers tried for each kind of drag before giving up on it.
const TRIES: usize = 12;

/// What the part's faces are, as one number.
fn shape(p: &Part) -> u64 {
    let Some(b) = p.model.body() else { return 0 };
    b.face_ids()
        .fold(0, |h, f| h ^ forge_agent::facemesh::key(b, f).0)
}

/// Whether a part still builds with this number moved both ways by the swing, and changes shape
/// each way (some numbers round to themselves, or are only a sketch's construction). How many
/// lines it rebuilt: for a parameter, one nothing uses rebuilds only its own.
fn drags(p: &mut Part, d: &Dim) -> Option<usize> {
    let was = shape(p);
    let mut most = 0;
    let mut fine = true;
    for k in [1.0 + SWING, 1.0 - SWING] {
        fine &= p.drag(d.at.op(forge_agent::short(d.value * k))).is_ok()
            && p.model.error.is_none()
            && shape(p) != was;
        most = most.max(p.model.rebuilt);
        if !fine {
            break;
        }
    }
    p.drag_cancel();
    fine.then_some(most)
}

/// The numbers to drag: the first parameter used below, one halfway down the history, the last.
fn targets(part: &mut Part) -> Vec<(&'static str, Dim)> {
    let dims: Vec<Dim> = part.dims().into_iter().filter(|d| d.value != 0.0).collect();
    let (params, features): (Vec<&Dim>, Vec<&Dim>) = dims.iter().partition(|d| !d.name.contains('.'));
    let mut out = Vec::new();
    let found = params
        .iter()
        .take(TRIES)
        .find(|d| drags(part, d).is_some_and(|lines| lines > 1));
    if let Some(d) = found {
        out.push(("param", (*d).clone()));
    }
    let n = features.len();
    if let Some(d) = features
        .iter()
        .skip(n / 2)
        .take(TRIES)
        .find(|d| drags(part, d).is_some())
    {
        out.push(("mid", (*d).clone()));
    }
    if let Some(d) = features
        .iter()
        .rev()
        .take(TRIES)
        .find(|d| drags(part, d).is_some())
    {
        out.push(("last", (*d).clone()));
    }
    out
}

#[derive(Default)]
struct Run {
    latency: Vec<f64>,
    rebuild: Vec<f64>,
    kernel: Vec<f64>,
    lines: Vec<f64>,
    finish: Vec<f64>,
    mesh: Vec<f64>,
    meshed: Vec<f64>,
    upload: Vec<f64>,
    bytes: Vec<f64>,
    draw: Vec<f64>,
    asked: u64,
    failed: u64,
    dropped: u64,
    faces: usize,
    tris: u64,
    chord: f32,
    warm: f64,
}

fn drag_once(
    name: &str,
    d: &Dim,
    ticks: usize,
    r: &mut Renderer,
    scene: &mut Scene,
    target: &Offscreen,
    cam: &Camera,
) -> Result<Run, String> {
    let part = load::tool_part(name)?;
    let mut run = Run::default();
    let t0 = now_ms();
    let mut l: Live<Out> = live::start(part);
    // the warm-up: every face on the GPU, before any value
    loop {
        let events = l.poll();
        if let Some(Event::Frame(f)) = events.into_iter().next() {
            if let Some(Out::Delta(x)) = f.out {
                r.live_apply(0, &x);
                run.faces = x.faces;
                run.chord = x.chord;
                scene.set_part(0, x.center, x.radius, vec![x.chord]);
            }
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    r.live_show(true);
    r.frame(&target.view, scene, cam, Overlay::default());
    r.wait();
    run.warm = now_ms() - t0;
    run.tris = r.live_tris();
    let mut asked_at: HashMap<u64, f64> = HashMap::new();
    let start = now_ms();
    let (mut tick, mut last, mut shown) = (0usize, 0u64, 0usize);
    loop {
        let now = now_ms();
        let out_of_ticks = tick >= ticks;
        let over_budget = now - start > BUDGET_MS && shown >= ENOUGH_FRAMES;
        let hard_stop = now - start > 3.0 * BUDGET_MS;
        if !out_of_ticks && !over_budget && now >= start + tick as f64 * TICK_MS {
            let swing = forge_math::dmath::sin(tick as f64 / 120.0 * core::f64::consts::TAU);
            let v = d.value * (1.0 + SWING * swing);
            last = l.drag(d.at.op(forge_agent::short(v)));
            asked_at.insert(last, now_ms());
            run.asked += 1;
            tick += 1;
        }
        let events = l.poll();
        let mut newest = None;
        for e in events {
            let Event::Frame(f) = e else { continue };
            if f.failure.is_some() {
                run.failed += 1;
                if f.seq == last && (out_of_ticks || over_budget) {
                    newest = Some(last);
                }
                continue;
            }
            run.rebuild.push(f.rebuild_ms);
            run.kernel.push(f.kernel_ms);
            run.lines.push(f.lines as f64);
            run.finish.push(f.finish_ms);
            run.mesh.push(f.mesh_ms);
            if let Some(Out::Delta(x)) = f.out {
                run.meshed.push(x.meshed as f64);
                let t = now_ms();
                run.bytes.push(r.live_apply(0, &x) as f64);
                run.upload.push(now_ms() - t);
                scene.set_part(0, x.center, x.radius, vec![x.chord]);
                newest = Some(f.seq);
            }
        }
        if let Some(seq) = newest {
            let t = now_ms();
            r.frame(&target.view, scene, cam, Overlay::default());
            r.wait();
            let done = now_ms();
            run.draw.push(done - t);
            if let Some(a) = asked_at.get(&seq) {
                run.latency.push(done - a);
            }
            shown += 1;
        }
        if (out_of_ticks || over_budget) && newest == Some(last) || hard_stop {
            break;
        }
        std::thread::sleep(std::time::Duration::from_micros(200));
    }
    run.dropped = l.dropped();
    r.live_show(false);
    Ok(run)
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// Drags each part (all 15 if none are named) and prints a line a drag, then which miss 16 ms.
pub fn run(names: &[String], ticks: usize, size: (u32, u32)) -> Result<(), String> {
    let all: Vec<String> = if names.is_empty() {
        forge_agent::bench::NAMES.iter().map(|n| n.to_string()).collect()
    } else {
        names.to_vec()
    };
    let instance = render::native_instance();
    let (device, queue, adapter) = render::block_on(render::open_device(&instance, None))?;
    let info = adapter.get_info();
    println!(
        "gpu: {} ({:?})  {}x{}, {} values at 60 Hz, each swinging {}%",
        info.name,
        info.device_type,
        size.0,
        size.1,
        ticks,
        SWING * 100.0
    );
    println!(
        "{:<9}{:<6}{:<13}{:>7}{:>8}{:>8}{:>8}{:>8}{:>8}{:>7}{:>7}{:>7}{:>7}{:>7}{:>8}{:>8}{:>7}{:>8}",
        "part",
        "drag",
        "number",
        "frames",
        "mean",
        "p99",
        "worst",
        "rebuild",
        "kernel",
        "lines",
        "finish",
        "mesh",
        "upload",
        "draw",
        "faces",
        "meshed",
        "drop%",
        "KB/frame"
    );
    let mut misses: Vec<String> = Vec::new();
    for name in &all {
        let mut part = load::tool_part(name)?;
        let (mesh, _) = load::open(name, true)?;
        let parts = [mesh];
        let radius = parts[0].radius;
        let center = parts[0].center;
        for (kind, d) in targets(&mut part) {
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
            let run = drag_once(name, &d, ticks, &mut r, &mut scene, &target, &cam)?;
            if run.latency.is_empty() {
                println!(
                    "{name:<9}{kind:<6}{:<13} no frame ({} of {} values failed)",
                    d.name, run.failed, run.asked
                );
                misses.push(format!("{name} {kind}: no frame"));
                continue;
            }
            let s = summary(&run.latency);
            let spent = mean(&run.rebuild)
                + mean(&run.finish)
                + mean(&run.mesh)
                + mean(&run.upload)
                + mean(&run.draw);
            println!(
                "{name:<9}{kind:<6}{:<13}{:>7}{:>8.2}{:>8.2}{:>8.2}{:>8.2}{:>8.2}{:>7.1}{:>7.2}{:>7.2}{:>7.2}{:>7.2}{:>8}{:>8.1}{:>7.0}{:>8.0}",
                d.name,
                run.latency.len(),
                s.mean,
                s.p99,
                s.worst,
                mean(&run.rebuild),
                mean(&run.kernel),
                mean(&run.lines),
                mean(&run.finish),
                mean(&run.mesh),
                mean(&run.upload),
                mean(&run.draw),
                run.faces,
                mean(&run.meshed),
                run.dropped as f64 * 100.0 / run.asked.max(1) as f64,
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
            if s.p99 > TICK_MS {
                let parts = [
                    ("rebuild", mean(&run.rebuild)),
                    ("finish", mean(&run.finish)),
                    ("mesh", mean(&run.mesh)),
                    ("upload", mean(&run.upload)),
                    ("draw", mean(&run.draw)),
                ];
                let worst = parts
                    .iter()
                    .copied()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .expect("five");
                misses.push(format!(
                    "{name} {kind} ({}): mean {:.1} p99 {:.1} ms; {} {:.1} ms is {:.0}% of the {:.1} ms spent",
                    d.name, s.mean, s.p99, worst.0, worst.1, worst.1 * 100.0 / spent, spent
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
