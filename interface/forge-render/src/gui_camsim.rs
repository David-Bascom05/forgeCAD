//! A machining document in the app: opened into the toolpath and stock simulation (the CAM run,
//! a view of its own with no part scene: everything is drawn from the simulation), played on
//! each frame, its geometry made for the view. The chrome is `gui_camsim_panel.rs`.

use super::Gui;
use crate::camsim::Sim;
use crate::camsim::draw::{self, Eye, PathShow, StockColour};
use crate::gui_camsim_panel::{CamSimUi, PartShow};
use crate::origin::WorldVert;

impl Gui {
    /// The workspace's document simulated off the window's thread whenever its run is new; a
    /// finished one taken in (keeping the time shown), the workspace's focus followed.
    fn camsim_follow(&mut self) {
        let Some(w) = &self.shell.cam else {
            self.shell.camsim = None;
            return;
        };
        let job = &mut self.shell.camsim_job;
        // a finished simulation
        let done = crate::plat::lock(&job.slot).take();
        if let Some((text, sim)) = done {
            job.running = None;
            let mut ui = CamSimUi::new(sim, &w.path);
            ui.src = text;
            if let Some(old) = self.shell.camsim.as_deref() {
                // the user's choices and the time carry over a regeneration
                ui.rapids = old.rapids;
                ui.played_only = old.played_only;
                ui.stock = old.stock;
                ui.colour = old.colour;
                ui.part = old.part;
                ui.tool = old.tool;
                ui.fixture = old.fixture;
                ui.follow = old.follow;
                ui.path_width = old.path_width;
                ui.tab = old.tab;
                ui.folded = old.folded;
                ui.speed = old.speed;
                ui.jump(old.t);
            } else {
                // the workspace shows the part itself
                ui.part = if self.fg.is_some() {
                    PartShow::Hidden
                } else {
                    PartShow::Ghost
                };
            }
            let first = self.shell.camsim.is_none();
            self.shell.camsim = Some(Box::new(ui));
            self.shell.camsim_job.focus = None;
            // no view to draw in (the workspace's part isn't open): one of the simulation's own
            if self.view.is_none() && self.fg.is_none() {
                self.camsim_view();
            } else if first && self.fg.is_none() {
                self.camsim_fit();
            }
            if let Some(v) = &mut self.view {
                v.dirty = true;
            }
        }
        let Some(w) = &self.shell.cam else { return };
        // the workspace has a run of text the simulation is not of: simulate it
        // a turning document is not a milling run (its removal comes later, through this
        // interface): no simulation of it
        if forge_turn_lang::is_turning(&w.text) {
            self.shell.camsim = None;
            return;
        }
        if let Some(run) = &w.run {
            // keyed by the text and what its run came to: the same text run again with the
            // files it reads now there (a browser page's arrive after it opens) is simulated again
            let key = format!("{}\u{1}{}\u{1}{}", run.text, run.certification, run.ops.len());
            let have = self.shell.camsim.as_deref().map(|s| s.src.as_str());
            let job = &mut self.shell.camsim_job;
            if have != Some(key.as_str()) && job.running.as_deref() != Some(key.as_str()) {
                let (text, dir) = (run.text.clone(), w.dir.clone());
                job.running = Some(key.clone());
                let slot = job.slot.clone();
                let waker = self.waker.clone();
                crate::plat::spawn("forge-camsim", move || {
                    let sim = Sim::open(&text, &dir);
                    *crate::plat::lock(&slot) = Some((key, sim));
                    waker();
                });
            }
        }
        // the workspace's focus, when it changes
        let f = w.focus().clone();
        if self.shell.camsim_job.focus.as_ref() != Some(&f)
            && let Some(st) = self.shell.camsim.as_deref_mut()
        {
            st.take_focus(&f);
            self.shell.camsim_job.focus = Some(f);
            if let Some(v) = &mut self.view {
                v.dirty = true;
            }
        }
    }

    /// A view with no part in it: the simulation draws everything.
    fn camsim_view(&mut self) {
        use crate::render::{self, Renderer, part_info};
        use crate::scene::{Placement, Scene};
        let parts = vec![crate::mesh::PartMesh::empty()];
        let scene = Scene::new(&part_info(&parts), vec![Placement::at(0, [0.0; 3])]);
        let renderer = Renderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.adapter.clone(),
            render::OFFSCREEN,
            (16, 16),
            &parts,
            &scene,
        );
        let name = self.shell.name.clone();
        self.show_loaded(super::Loaded {
            file: None,
            name: name.clone(),
            empty: false,
            scene,
            renderer,
        });
        self.shell.name = name;
        self.camsim_fit();
    }

    /// The view fitted to what the setups' stocks hold.
    fn camsim_fit(&mut self) {
        let Some(st) = self.shell.camsim.as_deref() else {
            return;
        };
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for s in &st.sim.setups {
            for k in 0..3 {
                lo[k] = lo[k].min(s.stock.min[k] as f32);
                hi[k] = hi[k].max(s.stock.max[k] as f32);
            }
        }
        if lo[0].is_finite()
            && let Some(v) = &mut self.view
        {
            let c = [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]));
            let r =
                0.5 * ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt();
            v.nav.camera = super::view_camera(c, r * 1.25, self.shell.view.parallel, v.size);
            v.nav.home_view = Some(v.nav.camera);
            v.dirty = true;
        }
    }

    /// Simulates the document again (its files changed), keeping the time shown.
    pub fn camsim_reload(&mut self) {
        if let Some(st) = self.shell.camsim.as_deref_mut() {
            st.src.clear();
        }
        // the workspace's run read the files as they were: generate again with what arrived
        if let Some(w) = &mut self.shell.cam {
            w.want_run = true;
        }
        // and a frame to start it in
        (self.waker)();
    }

    /// Plays the simulation on; true while it moves.
    pub(super) fn camsim_tick(&mut self) -> bool {
        self.camsim_follow();
        let Some(st) = self.shell.camsim.as_deref_mut() else {
            return false;
        };
        let now = crate::viewer::now_ms();
        let dt = st.last_ms.map_or(0.0, |t| (now - t) / 1000.0).min(0.1);
        st.last_ms = Some(now);
        if std::mem::take(&mut st.removal_asked) {
            if let Err(e) = st.sim.check_removal(0.5) {
                self.shell.status = e;
            }
            st.drawn_key = None;
        }
        let moved = st.tick(dt);
        if let Some(v) = &mut self.view {
            if st.follow
                && let Some((_, p, _)) = st.tool_now()
            {
                v.nav.camera.target = [p[0] as f32, p[1] as f32, p[2] as f32];
            }
            // the picture follows the time shown
            let key = (
                st.t.to_bits(),
                st.sim.cells(),
                st.colour == StockColour::Compare,
                st.stock,
            );
            if st.drawn_key != Some(key) || moved {
                v.dirty = true;
            }
        }
        if !st.playing {
            st.last_ms = None;
        }
        moved
    }
}

/// The simulation's triangles for this frame: opaque ones returned, translucent ones added to
/// `world`.
pub fn geometry(
    st: &mut CamSimUi,
    cam: &crate::camera::Camera,
    size: (u32, u32),
    world: &mut Vec<WorldVert>,
) -> Vec<WorldVert> {
    let mut solid = Vec::new();
    let (forward, _, _) = cam.axes();
    let px = cam.dist * (cam.fovy * 0.5).tan() * 2.0 / size.1.max(1) as f32;
    let eye = Eye {
        at: cam.eye(),
        forward,
        parallel: cam.parallel,
        mm_px: if cam.parallel { px } else { px / cam.dist.max(1e-6) },
    };
    let setup = st.setup_now();
    // the stock, as played to now (its mesh kept until the time or the colouring changes)
    if st.stock {
        let key = (
            st.t.to_bits(),
            st.sim.cells(),
            st.colour == StockColour::Compare,
            true,
        );
        if st.drawn_key != Some(key) {
            let colour = st.colour;
            let tol = st.sim.setups.get(setup).map_or(0.01, |s| s.tol) as f32;
            let t = st.t;
            let mut mesh = std::mem::take(&mut st.stock_mesh);
            mesh.clear();
            if st.sim.stock_at(t).is_some()
                && let Some((z, s)) = st.sim.stock_shown()
            {
                draw::stock(
                    z,
                    st.sim.part_maps.get(s).and_then(|m| m.as_ref()),
                    colour,
                    tol,
                    &mut mesh,
                );
            }
            st.stock_mesh = mesh;
            st.drawn_key = Some(key);
        }
        solid.extend_from_slice(&st.stock_mesh);
    } else {
        st.drawn_key = None;
    }
    // the part
    if let Some(tris) = st.sim.part_tris.get(setup) {
        match st.part {
            PartShow::Hidden => {}
            PartShow::Ghost => draw::part(tris, 0.22, world),
            PartShow::Solid => draw::part(tris, 1.0, &mut solid),
        }
    }
    // the workholding
    if st.fixture
        && let Some(s) = st.sim.setups.get(setup)
    {
        for (_, lo, hi) in &s.fixture {
            draw::solid_box(*lo, *hi, draw::look::FIXTURE, &mut solid);
        }
    }
    // the tool at the time shown, red while a failed check's move plays
    if st.tool
        && let Some((tool, p, i)) = st.tool_now()
        && let Some(tu) = st.sim.cam.sim_tools().get(tool)
    {
        let g = st.sim.tl.segs[i];
        let fail = st
            .sim
            .markers
            .iter()
            .any(|m| m.severe() && m.op == Some(g.op as usize) && (m.t - st.t).abs() <= g.dt.max(1e-6));
        draw::tool(&tu.shape, p, fail, &mut solid);
    }
    // the machine about the part, when its run plays; a body in a collision now, red
    if st.machine
        && let Some(mv) = st.sim.machine()
    {
        let hit: Vec<String> = mv
            .run
            .contacts
            .contacts
            .iter()
            .filter(|c| {
                let g = &mv.run.tl.segments[c.segment];
                st.t >= g.t0 && st.t <= g.t0 + g.t_stop.max(g.t_min)
            })
            .flat_map(|c| [c.a.clone(), c.b.clone()])
            .collect();
        mv.draw(st.t, &hit, &mut solid);
    }
    // the toolpath
    let show = PathShow {
        ops: &st.show_ops,
        rapids: st.rapids,
        upto: Some(st.t),
        played_only: st.played_only,
        width: st.path_width,
        picked: st.picked,
        values: st.values.as_ref(),
    };
    draw::toolpath(&st.sim.tl, &eye, &show, world);
    // the leftover regions (G4, exact), once their setup is done
    for m in &st.sim.markers {
        if let Some((b, z0, z1)) = m.region
            && m.setup == setup
            && st.t + 1e-9 >= m.t
        {
            draw::box_edges(
                &eye,
                [b[0], b[1], z0],
                [b[2], b[3], z1],
                1.5,
                draw::lin(0xF08A24, 0.9),
                world,
            );
        }
    }
    solid
}

impl Gui {
    /// Drives the simulation by words (the page's `app_camsim`, tests, a host): `t=SECONDS`,
    /// `at=FRACTION`, `play`, `pause`, `colour=material|compare`, `part=hidden|ghost|solid`,
    /// `tab=program|display|checks|moves`, `follow=0|1`, `rapids=0|1`, `played=0|1`,
    /// `stock=0|1`, `cells=N`, `speed=X`, `op=N` (to op N's start), `marker=N` (to marker N),
    /// `locate=OP,PATH,MOVE,S`; several separated by `;`.
    pub fn camsim_command(&mut self, cmds: &str) -> Result<(), String> {
        let st = self
            .shell
            .camsim
            .as_deref_mut()
            .ok_or("no machining document is open")?;
        for cmd in cmds.split(';').map(str::trim).filter(|c| !c.is_empty()) {
            let (k, v) = cmd.split_once('=').unwrap_or((cmd, ""));
            let num = || {
                v.parse::<f64>()
                    .map_err(|_| format!("`{cmd}`: a number after `=`"))
            };
            let on = || Ok::<bool, String>(matches!(v, "1" | "true" | "on"));
            match k {
                "t" => st.jump(num()?),
                "at" => {
                    let total = st.total();
                    st.jump(num()? * total);
                }
                "play" => st.playing = true,
                "pause" => st.playing = false,
                "colour" | "color" => {
                    st.colour = match v {
                        "compare" => StockColour::Compare,
                        "material" => StockColour::Material,
                        _ => return Err(format!("`{cmd}`: material or compare")),
                    }
                }
                "part" => {
                    st.part = match v {
                        "hidden" => PartShow::Hidden,
                        "ghost" => PartShow::Ghost,
                        "solid" => PartShow::Solid,
                        _ => return Err(format!("`{cmd}`: hidden, ghost or solid")),
                    }
                }
                "tab" => {
                    use crate::gui_camsim_panel::Tab;
                    st.tab = match v {
                        "program" => Tab::Program,
                        "display" => Tab::Display,
                        "checks" => Tab::Checks,
                        "moves" => Tab::Moves,
                        _ => return Err(format!("`{cmd}`: program, display, checks or moves")),
                    }
                }
                "follow" => st.follow = on()?,
                "source" => {
                    let want = match v {
                        "toolpath" => None,
                        "machine" => {
                            Some(st.sim.machines.iter().position(|m| m.is_ok()).ok_or_else(|| {
                                st.sim
                                    .machines
                                    .iter()
                                    .find_map(|m| m.as_ref().err().cloned())
                                    .unwrap_or_else(|| "no machine run".into())
                            })?)
                        }
                        _ => return Err(format!("`{cmd}`: toolpath or machine")),
                    };
                    let f = st.t / st.total().max(1e-9);
                    st.sim.set_source(want)?;
                    st.drawn_key = None;
                    let total = st.total();
                    st.jump(f * total);
                }
                "machine" => st.machine = on()?,
                "removal" => {
                    let step = if v.is_empty() { 0.5 } else { num()? };
                    st.sim.check_removal(step)?;
                    st.drawn_key = None;
                }
                "fit" => {
                    // the view on the stock, or on the whole machine about it
                    let mut tris = vec![];
                    match v {
                        "machine" => {
                            let mv = st.sim.machine().ok_or("`fit=machine`: no machine run plays")?;
                            mv.draw(st.t, &[], &mut tris);
                        }
                        _ => {
                            let s = st.sim.setups.get(st.setup_now()).ok_or("no setup")?;
                            draw::solid_box(s.stock.min, s.stock.max, [1.0; 3], &mut tris);
                        }
                    }
                    let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
                    for w in &tris {
                        for k in 0..3 {
                            lo[k] = lo[k].min(w.at[k]);
                            hi[k] = hi[k].max(w.at[k]);
                        }
                    }
                    if let Some(view) = &mut self.view
                        && lo[0].is_finite()
                    {
                        let c = [0, 1, 2].map(|k| 0.5 * (lo[k] + hi[k]));
                        let r = 0.5
                            * ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2))
                                .sqrt();
                        view.nav.camera = super::view_camera(c, r, self.shell.view.parallel, view.size);
                    }
                }
                "rapids" => st.rapids = on()?,
                "played" => st.played_only = on()?,
                "stock" => st.stock = on()?,
                "tool" => st.tool = on()?,
                "fold" => st.folded = on()?,
                "cells" => {
                    st.sim.set_cells(num()? as usize);
                    st.drawn_key = None;
                }
                "speed" => st.speed = num()?,
                "op" => st.select_op(num()? as usize),
                "marker" => {
                    let m = st
                        .sim
                        .markers
                        .get(num()? as usize)
                        .ok_or(format!("`{cmd}`: no such marker"))?;
                    let t = m.t;
                    st.jump(t);
                }
                "locate" => {
                    let n: Vec<f64> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if n.len() != 4 || !st.locate(n[0] as usize, n[1] as usize, n[2] as usize, n[3]) {
                        return Err(format!(
                            "`{cmd}`: locate=OP,PATH,MOVE,S of a move the program has"
                        ));
                    }
                }
                _ => return Err(format!("`{cmd}`: not a simulation word")),
            }
        }
        if let Some(v) = &mut self.view {
            v.dirty = true;
        }
        Ok(())
    }

    /// The simulation's state for a host: the time, the op, the tool's place, the verdict.
    pub fn camsim_state(&self) -> Option<forge_agent::json::Json> {
        use forge_agent::json::Json;
        let st = self.shell.camsim.as_deref()?;
        let n = |x: f64| Json::Num(x);
        let now = st.tool_now();
        let refused = st.sim.cam.ops.iter().filter(|o| o.status != "built").count();
        Some(Json::obj(vec![
            ("t", n(st.t)),
            ("total", n(st.total())),
            ("playing", st.playing.into()),
            ("ops", n(st.sim.tl.ops.len() as f64)),
            ("moves", n(st.sim.tl.segs.len() as f64)),
            ("op", st.sim.tl.op_at(st.t).map_or(Json::Null, |o| n(o as f64))),
            ("piece", now.map_or(Json::Null, |(_, _, i)| n(i as f64))),
            (
                "tip",
                now.map_or(Json::Null, |(_, p, _)| {
                    Json::Arr(p.iter().map(|x| n(*x)).collect())
                }),
            ),
            ("markers", n(st.sim.markers.len() as f64)),
            ("refused", n(refused as f64)),
            ("ok", st.sim.cam.ok.into()),
            ("certification", st.sim.cam.certification().into()),
            (
                "source",
                st.sim
                    .machine()
                    .map_or(Json::from("toolpath"), |m| Json::from(m.run.def.name.clone())),
            ),
            (
                "machine_errors",
                Json::Arr(
                    st.sim
                        .machines
                        .iter()
                        .filter_map(|m| m.as_ref().err().map(|e| Json::from(e.clone())))
                        .collect(),
                ),
            ),
        ]))
    }
}
