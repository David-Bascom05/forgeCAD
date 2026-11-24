//! What a run holds, read-only, for the app's toolpath and stock simulation (forge-render's
//! camsim): each setup's part, stock, frame and workholding, and each tool's shape. Nothing here
//! decides anything: the verdicts stay the certified checks' (`OpOut::checks`, `Cam::leftover`).

use crate::{Cam, OpPath};
use forge_cam::ops::{Stock, ToolUse};
use std::path::PathBuf;

/// One setup as the simulation draws it.
#[derive(Clone, Debug)]
pub struct SimSetup {
    pub name: String,
    /// The part's source and its directory (to build and mesh it for display).
    pub part_src: String,
    pub part_dir: PathBuf,
    pub part_file: String,
    /// The part turned over about x for this setup: its frame maps the part's (x, y, z) to
    /// (x, −y, h − z) with this h. None: the part as drawn.
    pub turned: Option<f64>,
    /// The part's box in this setup's frame.
    pub part_box: ([f64; 3], [f64; 3]),
    pub stock: Stock,
    /// The WCS origin in the part's frame (the posted text's zero).
    pub origin: [f64; 3],
    pub tol: f64,
    /// The workholding's pieces as boxes (name, min, max), in this setup's frame.
    pub fixture: Vec<(String, [f64; 3], [f64; 3])>,
    /// Its name, if it has one.
    pub fixture_name: Option<String>,
    /// The setup this one continues, turned over.
    pub from: Option<usize>,
}

impl Cam {
    /// Every setup, in order.
    pub fn sim_setups(&self) -> Vec<SimSetup> {
        self.setups
            .iter()
            .map(|s| {
                let p = &self.parts[s.part];
                let (src_part, turned) = match p.turn_of {
                    Some(k) => (&self.parts[k], Some(self.parts[k].h())),
                    None => (p, None),
                };
                SimSetup {
                    name: s.name.clone(),
                    part_src: src_part.src.clone(),
                    part_dir: src_part.pdir.clone(),
                    part_file: src_part.file.clone(),
                    turned,
                    part_box: p.bbox,
                    stock: s.stock,
                    origin: s.origin,
                    tol: s.tol,
                    fixture: s.fixture.as_ref().map(|f| f.boxes()).unwrap_or_default(),
                    fixture_name: s.fixture.as_ref().map(|f| f.name.clone()),
                    from: s.from,
                }
            })
            .collect()
    }

    /// The tools the document holds, by the index an op's paths name.
    pub fn sim_tools(&self) -> &[ToolUse] {
        &self.tools
    }

    /// The paths of op `i` the simulation plays: as posted where a program posted it (what the
    /// checks were run on and their `move_index` counts), else as planned.
    pub fn sim_paths(&self, i: usize) -> &[OpPath] {
        let op = &self.ops[i];
        if op.posted.len() == op.paths.len() && !op.posted.is_empty() {
            &op.posted
        } else {
            &op.paths
        }
    }
}

/// A setup's posted program run on its machine (forge-machine): the machine as data, the
/// posted text interpreted to joint motion and timed, and every body checked through it. What
/// the machine does is the posted text's, not the toolpath's: the check Fusion leaves out.
pub struct MachineRun {
    pub def: forge_machine::MachineDef,
    /// The program it ran, and its dialect.
    pub program: String,
    pub dialect: String,
    pub text: String,
    /// The part's frame on the machine is the document's moved by `shift`: where the setup's
    /// `at:` (the WCS origin in machine coordinates) puts it; with none, the stock (or the
    /// workholding under it) centred on the part mount's datum, its bottom on it.
    pub shift: [f64; 3],
    /// The work offset used (per control letter), and whether it was the setup's own (`at:`)
    /// or placed here (the WCS origin over the datum, as above).
    pub wcs: Vec<f64>,
    pub wcs_given: bool,
    pub tools: Vec<forge_machine::sim::SimTool>,
    pub h: Vec<(u32, f64)>,
    pub control: forge_machine::gcode::Control,
    pub tl: forge_machine::sim::Timeline,
    pub contacts: forge_machine::collide::Report,
    /// The workholding as the check took it (part frame on the machine); the stock is left
    /// to the certified checker (it changes as it is cut).
    pub work: Vec<forge_machine::collide::CBody>,
}

impl MachineRun {
    /// The forge-machine setup (it borrows the definition).
    pub fn setup(&self) -> forge_machine::sim::Setup<'_> {
        let mut s =
            forge_machine::sim::Setup::new(&self.def, self.control.clone()).expect("made once already");
        s.wcs = vec![(54, self.wcs.clone())];
        s.tools = self.tools.clone();
        s.h = self.h.clone();
        s
    }

    /// The tool tip in the document's part frame at joint values q, tool t.
    pub fn tip(&self, q: &[f64], t: u32) -> Option<[f64; 3]> {
        let (p, _) = self.setup().tip_in_part(q, t)?;
        Some([p[0] - self.shift[0], p[1] - self.shift[1], p[2] - self.shift[2]])
    }
}

fn solve3(m: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(m);
    if d.abs() < 1e-12 {
        return None;
    }
    let mut x = [0.0; 3];
    for (k, xk) in x.iter_mut().enumerate() {
        let mut mk = m;
        for r in 0..3 {
            mk[r][k] = b[r];
        }
        *xk = det(mk) / d;
    }
    Some(x)
}

impl Cam {
    /// Runs setup `si`'s posted program on its machine (a built-in forge-machine definition of
    /// the setup's machine name): Err says why it can't (no program posted, no such machine, a
    /// letter the control lacks, ...).
    pub fn machine_run(&self, si: usize) -> Result<MachineRun, String> {
        use forge_machine::{collide, gcode, sim};
        let s = self.setups.get(si).ok_or("no such setup")?;
        let prog = self
            .programs
            .iter()
            .find(|p| p.setups.contains(&si) && p.text.is_some())
            .ok_or_else(|| format!("setup {}: no program posted for it", s.name))?;
        let text = prog.text.clone().expect("found with text");
        let def = forge_machine::library::builtin()
            .into_iter()
            .find(|d| d.name == s.machine)
            .ok_or_else(|| format!("machine `{}` is not one forge-machine defines", s.machine))?;
        let control = gcode::Control::for_dialect(&def, &prog.post, 1)?;
        // the tools, as stacked cylinders from the tip; their gauge length is the assembly's top
        let mut tools = vec![];
        let mut h = vec![];
        for tu in &self.tools {
            let top = tu.shape.top();
            let mut body: Vec<(f64, f64, f64)> = tu
                .shape
                .bodies
                .iter()
                .map(|b| (b.rmax, b.z0.max(0.0), b.z1))
                .filter(|(r, a, b)| *r > 0.0 && b > a)
                .collect();
            body.sort_by(|a, b| a.1.total_cmp(&b.1));
            let r = 0.5 * tu.tool.dc;
            let corner = match tu.tool.kind {
                forge_tooling::Kind::Ball => r,
                forge_tooling::Kind::Bull => tu.tool.re,
                _ => 0.0,
            };
            let m = def
                .mounts
                .iter()
                .find(|m| m.kind == forge_machine::def::MountKind::Tool)
                .ok_or("the machine has no tool mount")?;
            tools.push(sim::SimTool {
                t: tu.t,
                name: tu.tool.id.clone(),
                tip: [m.dir[0] * top, m.dir[1] * top, m.dir[2] * top],
                body,
                cut: sim::Cut::Mill { r, corner },
            });
            h.push((tu.t, top));
        }
        // the part on the machine: centred on the datum, its lowest (stock or workholding) on it
        let mut lo = s.stock.min;
        let hi = s.stock.max;
        if let Some(f) = &s.fixture {
            for (_, a, _) in f.boxes() {
                lo[2] = lo[2].min(a[2]);
            }
        }
        let mut shift = [-0.5 * (lo[0] + hi[0]), -0.5 * (lo[1] + hi[1]), -lo[2]];
        let mut setup = sim::Setup::new(&def, control.clone()).map_err(|e| e.to_string())?;
        setup.tools = tools.clone();
        setup.h = h.clone();
        // the work offset: the setup's own, else the joint values that put the first tool's tip
        // on the WCS origin, less its length register (the linear X, Y, Z of a 3-axis head)
        let (wcs, given) = match s.at {
            Some(a) => {
                let mut w = vec![0.0; control.letters.len()];
                for (k, l) in control.letters.iter().enumerate() {
                    if let Some(i) = "XYZ".find(*l) {
                        w[k] = a[i];
                    }
                }
                // the part is where the offset puts it, not where a placement would: the WCS
                // origin is the first tool's tip at the offset's joint values, its length (H)
                // added on the length joint, as the control does
                let t0 = tools.first().ok_or("no tools")?;
                let mut q: Vec<f64> = def.joints.iter().map(|j| j.home).collect();
                for (k, wk) in w
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| "XYZ".contains(control.letters[*k]))
                {
                    let j = setup.joint_of[k];
                    q[j] = *wk;
                    if def.length_joint.as_deref() == Some(def.joints[j].name.as_str()) {
                        q[j] += t0.tip.iter().map(|v| v * v).sum::<f64>().sqrt();
                    }
                }
                let o = setup
                    .tip_in_part(&q, t0.t)
                    .ok_or("the tool's tip is not in the part's frame")?
                    .0;
                shift = [o[0] - s.origin[0], o[1] - s.origin[1], o[2] - s.origin[2]];
                (w, true)
            }
            None => {
                let t0 = tools.first().map(|t| t.t).ok_or("no tools")?;
                let n = def.joints.len();
                let home: Vec<f64> = def.joints.iter().map(|j| j.home).collect();
                let tip = |q: &[f64]| setup.tip_in_part(q, t0).map(|x| x.0);
                let base = tip(&home).ok_or("the tool's tip is not in the part's frame")?;
                let lin: Vec<(usize, usize)> = control
                    .letters
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| "XYZ".contains(**l))
                    .map(|(k, _)| (k, setup.joint_of[k]))
                    .collect();
                if lin.len() != 3 {
                    return Err("placing the work offset needs X, Y and Z".into());
                }
                let mut m = [[0.0; 3]; 3];
                for (c, &(_, j)) in lin.iter().enumerate() {
                    let mut q = home.clone();
                    q[j] += 1.0;
                    let p = tip(&q).ok_or("no tip")?;
                    for r in 0..3 {
                        m[r][c] = p[r] - base[r];
                    }
                }
                let want = [
                    s.origin[0] + shift[0],
                    s.origin[1] + shift[1],
                    s.origin[2] + shift[2],
                ];
                let x = solve3(m, [want[0] - base[0], want[1] - base[1], want[2] - base[2]])
                    .ok_or("the machine's X, Y and Z do not span space")?;
                let mut w = vec![0.0; control.letters.len()];
                for (c, &(k, j)) in lin.iter().enumerate() {
                    w[k] = home[j] + x[c];
                    if def.length_joint.as_deref() == Some(def.joints[j].name.as_str()) {
                        w[k] -= tools[0].tip.iter().map(|v| v * v).sum::<f64>().sqrt();
                    }
                }
                let _ = n;
                (w, false)
            }
        };
        setup.wcs = vec![(54, wcs.clone())];
        let mv = |p: [f64; 3]| [p[0] + shift[0], p[1] + shift[1], p[2] + shift[2]];
        let tl = sim::simulate(&text, &setup).map_err(|e| e.to_string())?;
        // the workholding only: the stock changes as it is cut, and its box would call every
        // rapid over a cleared region a hit; the tool against the in-process stock is the
        // certified checker's (G2, G3), not this one's
        let mut work = vec![];
        if let Some(f) = &s.fixture {
            for (name, a, b) in f.boxes() {
                work.push(collide::part_body(
                    &setup,
                    &name,
                    &[forge_machine::def::Prim::Box {
                        min: mv(a),
                        max: mv(b),
                    }],
                    collide::Role::Fixture,
                ));
            }
        }
        let contacts = collide::check(&setup, &tl, &work);

        Ok(MachineRun {
            program: prog.name.clone(),
            dialect: prog.post.clone(),
            text,
            shift,
            wcs,
            wcs_given: given,
            tools,
            h,
            control,
            tl,
            contacts,
            work,
            def,
        })
    }
}
