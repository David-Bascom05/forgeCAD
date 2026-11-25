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
    /// Per piece of `fixture`: a body of the device (its base, a riser, a plate), not a jaw or
    /// a parallel, by the workholding library's role where the device is one of its own (else
    /// an old vise's "bed").
    pub fixture_body: Vec<bool>,
    /// The plane the workholding stands on (the machine table), in this setup's frame, when
    /// known: a workholding library device's mount. None: no workholding, or an old record's
    /// boxes, which stop at the bed and say nothing of the table.
    pub table_z: Option<f64>,
    /// The workholding library's placed device, a body each, for drawing: its real part (a
    /// mesh relative to forge-workholding's data/models, posed into this setup's frame) where
    /// it has one, else its own tight primitives. Empty for an old record's boxes.
    pub holding: Vec<HoldingBody>,
    /// Its name, if it has one.
    pub fixture_name: Option<String>,
    /// The setup this one continues, turned over.
    pub from: Option<usize>,
    /// A setup that cuts soft jaws (its part is the jaws, held in their vise): the setup that
    /// then grips the part in them. The two share one frame (forge_cam::fixture::soft_jaws: the
    /// gripping setup's), so the part as that setup holds it is where it will sit in the jaws:
    /// a reference while they are cut, never material.
    pub jaws_for: Option<usize>,
}

impl Cam {
    /// Every setup, in order.
    /// For a setup that cuts soft jaws (its part is a `soft_jaw`'s jaws: no source of its own,
    /// held in their vise), the later setup held in the same named jaws whose part is a real one.
    fn jaws_for(&self, s: &crate::SetupIn) -> Option<usize> {
        let jaws = &self.parts[s.part];
        if !(jaws.src.is_empty() && jaws.hold.is_some()) {
            return None;
        }
        let name = &s.fixture.as_ref()?.name;
        self.setups.iter().position(|t| {
            t.from.is_some()
                && !std::ptr::eq(t, s)
                && t.fixture.as_ref().is_some_and(|f| &f.name == name)
                && !self.parts[t.part].src.is_empty()
        })
    }

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
                    fixture_body: s.fixture.as_ref().map(body_pieces).unwrap_or_default(),
                    table_z: s.fixture.as_ref().and_then(|f| f.table_z),
                    holding: s.fixture.as_ref().map(holding_bodies).unwrap_or_default(),
                    fixture_name: s.fixture.as_ref().map(|f| f.name.clone()),
                    from: s.from,
                    jaws_for: self.jaws_for(s),
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
    /// The workholding's base above the part mount's datum (the table), mm, in the part's frame
    /// on the machine: 0 where it is seated (as it is placed with no `at:`), positive floating
    /// above the table, negative sunk into it. None: the setup has no workholding, or one whose
    /// table plane is unknown (`Fixture::table_z`: an old record's boxes).
    pub seat: Option<f64>,
    /// The workholding's extent in the document's part frame (lowest corner, highest).
    pub holding: Option<([f64; 3], [f64; 3])>,
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
        // the one machine list's record (read once), not the built-ins parsed again
        let def = crate::machines::record(&s.machine)
            .cloned()
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
                // a touch probe removes nothing: its ball's contact on a skip move is the trigger
                cut: if tu.probe {
                    sim::Cut::Probe { r }
                } else {
                    sim::Cut::Mill { r, corner }
                },
            });
            h.push((tu.t, top));
        }
        // the part on the machine: centred on the datum, its lowest (stock or workholding) on it
        let mut lo = s.stock.min;
        let hi = s.stock.max;
        // (a library device stands on its table plane; an old record's boxes, on their lowest)
        if let Some(f) = &s.fixture {
            match f.table_z {
                Some(z) => lo[2] = z,
                None => {
                    for (_, a, _) in f.boxes() {
                        lo[2] = lo[2].min(a[2]);
                    }
                }
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
        let holding = s.fixture.as_ref().map(|f| {
            f.boxes().iter().fold(
                ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]),
                |(lo, hi), (_, a, b)| {
                    (
                        [lo[0].min(a[0]), lo[1].min(a[1]), lo[2].min(a[2])],
                        [hi[0].max(b[0]), hi[1].max(b[1]), hi[2].max(b[2])],
                    )
                },
            )
        });
        // the table plane where the device gives one: an old record's boxes give none (their
        // lowest is the bed's slab, not the base on the table), so nothing is said of it
        let seat = s.fixture.as_ref().and_then(|f| f.table_z).map(|z| z + shift[2]);

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
            seat,
            holding,
            def,
        })
    }
}

impl Cam {
    /// The rapid rates (mm/min) of setup `si`'s machine along X, Y and Z, from its forge-machine
    /// record (each linear axis's `rapid`): None when the machine is not one forge-machine defines
    /// or a rate is not published, and the simulation plays rapids at its own speed.
    pub fn sim_rapids(&self, si: usize) -> Option<[f64; 3]> {
        let s = self.setups.get(si)?;
        let def = crate::machines::record(&s.machine)?;
        let mut out = [0.0; 3];
        for (k, l) in ["X", "Y", "Z"].iter().enumerate() {
            let j = def
                .joints
                .iter()
                .find(|j| j.letter == *l && j.ty == forge_machine::def::JointType::Linear)?;
            out[k] = j.rapid.filter(|r| *r > 0.0)?;
        }
        Some(out)
    }

    /// Setup `si`'s machine's tool change, chip to chip, seconds, from its forge-machine record
    /// (`changer.change_s`); None where the machine is not one forge-machine defines or the
    /// time is not cited.
    pub fn sim_tool_change(&self, si: usize) -> Option<f64> {
        let s = self.setups.get(si)?;
        crate::machines::record(&s.machine)?.changer.as_ref()?.change_s
    }
}

/// Per piece of a fixture's boxes: a body of the device (base, riser, plate, column, module,
/// a chuck's body), not what holds (jaws, parallels, clamps, studs), by the workholding
/// library's roles where it placed the device, else an old vise's "bed".
fn body_pieces(f: &forge_cam::fixture::Fixture) -> Vec<bool> {
    use forge_workholding::Role;
    f.boxes()
        .iter()
        .map(|(name, _, _)| match &f.placed {
            Some(p) => p.bodies.iter().find(|b| &b.name == name).is_some_and(|b| {
                matches!(
                    b.role,
                    Role::Base | Role::Riser | Role::Plate | Role::Column | Role::Module | Role::ChuckBody
                )
            }),
            None => name == "bed",
        })
        .collect()
}

/// A body of a placed workholding device, as the simulation draws it.
#[derive(Clone, Debug)]
pub struct HoldingBody {
    pub name: String,
    /// The device's body (base, riser, plate..), not what holds (jaws, parallels..).
    pub body: bool,
    /// Its real part: a path relative to forge-workholding's data/models (an STL) or its parts
    /// folder (a `.fg`), and the pose from the part's own frame into the setup's (rows of the
    /// rotation, then the shift).
    pub part: Option<(String, [[f64; 3]; 3], [f64; 3])>,
    /// Its primitives' bounds (the body's own tight boxes): drawn where the part is not there.
    pub boxes: Vec<([f64; 3], [f64; 3])>,
}

fn holding_bodies(f: &forge_cam::fixture::Fixture) -> Vec<HoldingBody> {
    use forge_workholding::Role;
    let Some(p) = &f.placed else { return vec![] };
    p.bodies
        .iter()
        .map(|b| HoldingBody {
            name: b.name.clone(),
            body: matches!(
                b.role,
                Role::Base | Role::Riser | Role::Plate | Role::Column | Role::Module | Role::ChuckBody
            ),
            part: match (&b.part, &b.part_pose) {
                (Some(part), Some(pose)) => Some((part.clone(), pose.m, pose.t)),
                _ => None,
            },
            boxes: b.prims.iter().map(|q| q.grown(b.keepout).bounds()).collect(),
        })
        .collect()
}
