//! The ForgeCAD app. With `--shot`, `--bench`: no window, input by script.

use forge_render::gui::Headless;

const USAGE: &str = "forge-app [PART | file.fc] [--size WxH] [--times]
  [--shot FILE.ppm [--scale S] [--light] [--tab N] [--select NAME] [--click NAME]..]
  [--bench]
PART: a bench part or a .fc file. --times: input-to-present on exit. --shot: one frame, no window.
--bench: what clicks and frames cost, no window.";

struct Args {
    file: Option<String>,
    size: (f32, f32),
    scale: f32,
    light: bool,
    shot: Option<String>,
    tab: usize,
    select: Option<String>,
    clicks: Vec<String>,
    bench: bool,
    times: bool,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        file: None,
        size: (1600.0, 900.0),
        scale: 1.0,
        light: false,
        shot: None,
        tab: 0,
        select: None,
        clicks: Vec::new(),
        bench: false,
        times: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().ok_or(format!("{k} takes a value\n{USAGE}"));
        match k.as_str() {
            "--size" => {
                let s = v()?;
                let (w, h) = s.split_once('x').ok_or(USAGE)?;
                a.size = (w.parse().map_err(|_| USAGE)?, h.parse().map_err(|_| USAGE)?);
            }
            "--scale" => a.scale = v()?.parse().map_err(|_| USAGE)?,
            "--light" => a.light = true,
            "--shot" => a.shot = Some(v()?),
            "--tab" => a.tab = v()?.parse().map_err(|_| USAGE)?,
            "--select" => a.select = Some(v()?),
            "--click" => a.clicks.push(v()?),
            "--bench" => a.bench = true,
            "--times" => a.times = true,
            s if s.starts_with("--") => return Err(USAGE.into()),
            part => a.file = Some(part.into()),
        }
    }
    Ok(a)
}

fn pct(mut v: Vec<f64>, p: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p) as usize]
}

fn bench(h: &mut Headless) {
    let now = forge_render::viewer::now_ms;
    let names: Vec<String> = h
        .gui
        .shell
        .rows
        .iter()
        .map(|r| format!("tree:{}", r.name))
        .collect();
    let report = |what: &str, v: Vec<f64>| {
        println!(
            "{what}: p50 {:.2} p95 {:.2} max {:.2} ms",
            pct(v.clone(), 0.5),
            pct(v.clone(), 0.95),
            pct(v, 1.0)
        );
    };
    let mut clicks = Vec::new();
    for k in 0..200 {
        let name = &names[k % names.len().max(1)];
        h.click(name);
        let t = now();
        h.step();
        clicks.push(now() - t);
    }
    report("click tree row to GPU done", clicks);
    let mut tabs = Vec::new();
    for k in 0..200 {
        h.click(&format!("tab:{}", forge_render::shell::TABS[k % 5]));
        let t = now();
        h.step();
        tabs.push(now() - t);
    }
    report("click ribbon tab to GPU done", tabs);
    let mut orbit = Vec::new();
    for k in 0..200 {
        if let Some(v) = &mut h.gui.view {
            v.nav.camera.yaw += 0.01;
            v.mark_dirty();
        }
        let t = now();
        let o = h.step();
        assert!(o.times.view_drawn, "frame {k}");
        orbit.push(now() - t);
    }
    report("orbit frame, chrome and viewport, GPU done", orbit);
    let o = h.step();
    let o2 = h.step();
    println!(
        "idle: asks to draw again {:?} then {:?}; viewport drawn {} ",
        o.again_in, o2.again_in, o2.times.view_drawn
    );
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let a = args()?;
    if a.shot.is_none() && !a.bench {
        return forge_render::app::run(a.file, (a.size.0 as f64, a.size.1 as f64), a.times);
    }
    let mut h = Headless::new(a.size, a.scale)?;
    if a.light {
        h.gui.shell.theme = forge_render::shell::theme::LIGHT;
    }
    h.gui.shell.tab = a.tab;
    h.step();
    h.gui.open(a.file.as_deref().unwrap_or("bracket"));
    h.settle();
    for _ in 0..5 {
        h.step();
    }
    if let Some(s) = &a.select
        && !h.click(&format!("tree:{s}"))
    {
        return Err(format!("no tree row {s}"));
    }
    for c in &a.clicks {
        if !h.click(c) {
            return Err(format!("nothing called {c}"));
        }
    }
    for _ in 0..3 {
        h.step();
    }
    if a.bench {
        bench(&mut h);
    }
    if let Some(s) = &a.shot {
        std::fs::write(s, h.ppm()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
