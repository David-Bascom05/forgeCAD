//! The Machine Builder's host side (research/cam/machine-builder.md §4): the machine's bodies in
//! the view, one placement each, posed by the jogged joint values and tinted by link (red where
//! the live check finds a hit, amber where it is near). The part's renderer and scene are kept
//! while the builder is open and put back when it closes (gui_versions.rs's way).
use super::{Gui, Loaded};
use crate::mesh::{FaceRange, Lod, PartMesh};
use crate::render::Renderer;
use crate::scene::{Placement, Rows, Scene};
use forge_machine::def::{MachineDef, Prim};
use forge_machine::kin;

#[derive(Default)]
pub struct MachHost {
    /// The part's renderer and scene, while the machine is shown in their place.
    taken: Option<(Renderer, Scene)>,
    /// No view was open: the builder made one, and closing drops it.
    made_view: bool,
    geom: u64,
    pose: u64,
    fit: u64,
    /// Each placement's body name and link name.
    bodies: Vec<(String, String)>,
    /// A library preview's placements' tints (its parts' appearances, then its ghosts).
    tints: Vec<[f32; 4]>,
    /// A library preview's ghost labels (world point, words) and its framing.
    labels: Vec<([f64; 3], String)>,
    /// The preview's box ghosts, drawn see-through over the frame (`crate::ghost`).
    ghost_boxes: Vec<crate::frame::Bounds>,
    /// A long tool's section and dimensions, for the inset.
    inset: Option<crate::library_mesh::Inset>,
}

fn oct(n: [f32; 3]) -> [i16; 2] {
    let s = n[0].abs() + n[1].abs() + n[2].abs();
    let (mut x, mut y) = (n[0] / s, n[1] / s);
    if n[2] < 0.0 {
        (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
    }
    [(x * 32767.0).round() as i16, (y * 32767.0).round() as i16]
}

/// A body's triangles (machine_mesh: its part, or its primitives) as a mesh, base frame at home.
pub fn body_mesh(t: &crate::machine_mesh::Tris) -> PartMesh {
    let mut lod = Lod {
        chord: 0.05,
        ..Lod::default()
    };
    let f = |p: [f64; 3]| [p[0] as f32, p[1] as f32, p[2] as f32];
    for (p, n) in t.pos.iter().zip(&t.nrm) {
        for v in f(*p) {
            lod.vertices.extend_from_slice(&v.to_le_bytes());
        }
        let o = oct(f(*n));
        lod.vertices.extend_from_slice(&o[0].to_le_bytes());
        lod.vertices.extend_from_slice(&o[1].to_le_bytes());
    }
    for tri in &t.tri {
        for v in tri {
            lod.indices.extend_from_slice(&v.to_le_bytes());
        }
    }
    lod.faces = t
        .patches
        .iter()
        .enumerate()
        .map(|(k, &(t0, nt, v0, nv))| FaceRange {
            face: k as u32,
            first_tri: t0,
            tris: nt,
            first_vertex: v0,
            vertices: nv,
        })
        .collect();
    for (e, poly) in t.edges.iter().enumerate() {
        let first = (lod.segments.len() / crate::mesh::SEGMENT) as u32;
        for w in poly.windows(2) {
            for (p, n) in [(w[0], [crate::mesh::ALWAYS, 0]), (w[1], [0, 0])] {
                for v in f(p) {
                    lod.segments.extend_from_slice(&v.to_le_bytes());
                }
                lod.segments.extend_from_slice(&n[0].to_le_bytes());
                lod.segments.extend_from_slice(&n[1].to_le_bytes());
            }
        }
        lod.edges.push(crate::mesh::EdgeRange {
            edge: e as u32,
            first,
            count: poly.len() as u32 - 1,
            start: 0,
            end: 0,
            start_at: f(poly[0]),
            end_at: f(poly[poly.len() - 1]),
        });
    }
    lod.fins = (lod.segments.len() / crate::mesh::SEGMENT) as u32;
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &t.pos {
        let p = f(*p);
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let center = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
    let radius = t
        .pos
        .iter()
        .map(|p| {
            let p = f(*p);
            (0..3).map(|k| (p[k] - center[k]).powi(2)).sum::<f32>()
        })
        .fold(0.0f32, f32::max)
        .sqrt();
    PartMesh {
        center,
        radius,
        lods: vec![lod],
    }
}

fn rows_of(r: &kin::Rigid) -> Rows {
    let m = r.r;
    [0, 1, 2].map(|i| [m[i][0] as f32, m[i][1] as f32, m[i][2] as f32, r.p[i] as f32])
}

/// A link's colour: the base grey, each joint's link a pastel by its place in the tree.
fn link_tint(def: &MachineDef, link: &str) -> [f32; 4] {
    match def.joint_index(link) {
        None => crate::palette::GREY,
        Some(i) => {
            let c = crate::palette::PASTEL[i % crate::palette::PASTEL.len()];
            [c[0], c[1], c[2], 0.0]
        }
    }
}

/// A machine's bodies at home (their primitives are in the home pose).
fn machine_bounds(def: &forge_machine::def::MachineDef) -> Option<crate::frame::Bounds> {
    use forge_machine::def::Prim;
    let pts = def
        .bodies
        .iter()
        .flat_map(|b| b.prims.iter())
        .flat_map(|p| match p {
            Prim::Box { min, max } => vec![*min, *max],
            Prim::Cyl { base, axis, r, h } => crate::shell::lib_form::cyl_pts(*base, *axis, *r, *h),
        });
    crate::frame::Bounds::of(pts.map(|p| p.map(|x| x as f32)))
}

/// A library preview framed (docs/ux/STANDARD.md §2, `crate::frame`): its bounds, ghosts
/// included, at the standard fill from the 3/4 view; a long tool (a punch or die set) by its near
/// end's stretch, seen along its length, so its section reads (the inset shows it to scale).
fn frame_preview(p: &crate::library_mesh::Preview, size: (u32, u32)) -> crate::camera::Camera {
    use crate::frame::{Bounds, Framing, ViewRect};
    let view = ViewRect::full(size.0 as f32, size.1 as f32);
    let Some((lo, hi)) = p.bounds() else {
        return crate::frame::frame(Bounds::new([-50.0; 3], [50.0; 3]), view, Framing::default(), &[]).camera;
    };
    let f = |a: [f64; 3]| a.map(|x| x as f32);
    let ext = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let sec = ext[0].max(ext[2]);
    if p.long_axis == Some(1) && ext[1] > 4.0 * sec {
        let near = Bounds::new(f(lo), f([hi[0], lo[1] + 4.0 * sec, hi[2]]));
        return crate::frame::frame_from(near, view, 0.7, -62.0, 22.0);
    }
    let how = if p.operator {
        Framing {
            // front-right, well round to the side and low: a housing's side face, with its C
            // (the throat) cut, is what you see
            dir: crate::frame::Dir::Custom(-25.0, 14.0),
            ..Framing::default()
        }
    } else {
        Framing::default()
    };
    crate::frame::frame(Bounds::new(f(lo), f(hi)), view, how, &[]).camera
}

impl super::Gui {
    /// The Library Builder's box ghosts into the frame's translucent world (gui_overlay).
    pub(super) fn machine_ghost_world(
        &self,
        cam: &crate::camera::Camera,
        size: (u32, u32),
        out: &mut Vec<crate::origin::WorldVert>,
    ) {
        if self.shell.machine.as_ref().is_some_and(|m| m.lib != "machine") {
            for b in &self.mach.ghost_boxes {
                crate::ghost::ghost_box(*b, cam, size, out);
            }
        }
    }
}

const HIT: [f32; 4] = [0.93, 0.25, 0.22, 0.0];
const NEAR: [f32; 4] = [0.98, 0.66, 0.18, 0.0];

impl Gui {
    /// The Manage tab's Machine Builder: opens the workspace on the open part's folder (its
    /// project), or closes it.
    pub(crate) fn machine_toggle(&mut self) {
        if self.shell.machine.take().is_some() {
            return;
        }
        let project = self
            .shell
            .fg
            .as_ref()
            .and_then(|f| std::path::Path::new(&f.path).parent().map(|p| p.to_path_buf()));
        self.shell.machine = Some(crate::shell::machine::MachineUi::new(project));
    }

    /// Each frame: the scene follows the workspace (built when the bodies change, posed and tinted
    /// when the joints move), and the part comes back when it closes.
    pub(crate) fn machine_frame(&mut self) {
        let Some(m) = &self.shell.machine else {
            if self.mach.made_view {
                self.mach.made_view = false;
                self.view = None;
                self.shell.view_tex = None;
            } else if let (Some((r, s)), Some(v)) = (self.mach.taken.take(), &mut self.view) {
                v.renderer = r;
                v.scene = s;
                v.renderer.resize(v.size);
                v.dirty = true;
            }
            self.mach.taken = None;
            self.mach.geom = 0;
            self.mach.pose = 0;
            self.mach.fit = 0;
            return;
        };
        // a library record: its preview (its parts from its dimensions, its ghosts) is the scene
        let tooling = (m.lib != "machine").then_some(());
        let def = match (&tooling, m.def.clone()) {
            (Some(_), _) => forge_machine::library::builtin().remove(0),
            (None, Some(d)) => d,
            (None, None) => return,
        };
        if m.geom != self.mach.geom {
            self.mach.geom = m.geom;
            let preview = if tooling.is_some() {
                m.library_preview()
            } else {
                None
            };
            self.mach.inset = preview.as_ref().and_then(|p| p.section.clone());
            self.mach.pose = 0;
            // a user's or project's machine: its parts are beside its file; a built-in's are embedded
            let dir = match &m.origin {
                Some((forge_machine_builder::catalog::Level::Builtin, _)) => None,
                Some((_, f)) => std::path::Path::new(f).parent().map(|p| p.to_path_buf()),
                None => None,
            };
            let unsaved_parts = m.parts.clone();
            let mut meshes: Vec<PartMesh> = if let Some(prev) = &preview {
                let mut ms: Vec<PartMesh> = prev.parts.iter().map(|(t, _)| body_mesh(t)).collect();
                self.mach.tints = prev.parts.iter().map(|(_, l)| l.tint()).collect();
                let f32s = |p: [f64; 3]| p.map(|x| x as f32);
                self.mach.ghost_boxes = prev
                    .ghosts
                    .iter()
                    .filter_map(|g| g.bounds)
                    .map(|(a, b)| crate::frame::Bounds::new(f32s(a), f32s(b)))
                    .collect();
                // the ghosts that are not boxes: dashed edges, on one empty part
                if prev.ghosts.iter().any(|g| !g.edges.is_empty()) {
                    let mut g = crate::machine_mesh::Tris::default();
                    let at = prev
                        .ghosts
                        .iter()
                        .find_map(|g| g.edges.first().and_then(|e| e.first()))
                        .copied()
                        .unwrap_or([0.0; 3]);
                    g.pos.extend([at; 3]);
                    g.nrm.extend([[0.0, 0.0, 1.0]; 3]);
                    g.tri.push([0, 1, 2]);
                    g.edges = prev.ghosts.iter().flat_map(|x| x.edges.iter().cloned()).collect();
                    g.patches.push((0, 1, 0, 3));
                    ms.push(body_mesh(&g));
                    self.mach.tints.push(crate::palette::GREY);
                }
                self.mach.labels = prev
                    .ghosts
                    .iter()
                    .filter(|g| !g.label.is_empty())
                    .map(|g| (g.at, g.label.clone()))
                    .collect();
                self.mach.bodies = (0..ms.len())
                    .map(|k| (format!("preview{k}"), "base".into()))
                    .collect();
                ms
            } else {
                self.mach.labels.clear();
                self.mach.ghost_boxes.clear();
                def.bodies
                    .iter()
                    .map(|b| {
                        // a made machine's parts not saved yet: from the texts it carries
                        if m.origin.is_none()
                            && let Some(p) = &b.part
                            && let Some((_, src)) = unsaved_parts.iter().find(|(r, _)| r == p)
                            && let Ok(t) = crate::machine_mesh::part_tris(src, None)
                        {
                            return body_mesh(&t);
                        }
                        body_mesh(&crate::machine_mesh::body_tris(&def, b, dir.as_deref()))
                    })
                    .collect()
            };
            if tooling.is_none() {
                self.mach.bodies = def
                    .bodies
                    .iter()
                    .map(|b| (b.name.clone(), b.link.clone()))
                    .collect();
            }
            if meshes.is_empty() {
                // a scene needs a part: an empty mesh at the origin
                meshes.push(body_mesh(&crate::machine_mesh::prims_tris(&[Prim::Box {
                    min: [-0.001; 3],
                    max: [0.001; 3],
                }])));
            }
            let info = crate::render::part_info(&meshes);
            let placements: Vec<Placement> = (0..meshes.len())
                .map(|k| Placement {
                    tint: if tooling.is_some() {
                        self.mach.tints.get(k).copied().unwrap_or(crate::palette::GREY)
                    } else {
                        def.bodies
                            .get(k)
                            .map_or(crate::palette::GREY, |b| link_tint(&def, &b.link))
                    },
                    ..Placement::at(k as u32, [0.0; 3])
                })
                .collect();
            let mut scene = Scene::new(&info, placements);
            scene.set_explode(0.0);

            let mut renderer = Renderer::new(
                self.device.clone(),
                self.queue.clone(),
                self.adapter.clone(),
                crate::render::OFFSCREEN,
                (16, 16),
                &meshes,
                &scene,
            );
            // as a part's view is set up (show_loaded)
            self.theme_view(&mut renderer);
            match &mut self.view {
                Some(v) => {
                    let mut renderer = renderer;
                    renderer.resize(v.size);
                    renderer.style = v.renderer.style;
                    renderer.ao = v.renderer.ao;
                    renderer.set_tints(&scene);
                    let fresh = self.mach.taken.is_none() && !self.mach.made_view;
                    let old_r = std::mem::replace(&mut v.renderer, renderer);
                    let old_s = std::mem::replace(&mut v.scene, scene);
                    if fresh {
                        self.mach.taken = Some((old_r, old_s));
                    }
                    if fresh || m.fit != self.mach.fit {
                        self.mach.fit = m.fit;
                        v.nav.camera = match preview.as_ref() {
                            Some(p) => frame_preview(p, v.size),
                            None => match machine_bounds(&def) {
                                // a machine: its bodies at home, framed from the front 3/4
                                Some(b) => {
                                    crate::frame::frame(
                                        b,
                                        crate::frame::ViewRect::full(v.size.0 as f32, v.size.1 as f32),
                                        crate::frame::Framing::default(),
                                        &[],
                                    )
                                    .camera
                                }
                                None => {
                                    let (c, r) = v.scene.bounds();
                                    super::view_camera(c, r, self.shell.view.parallel, v.size)
                                }
                            },
                        };
                    }
                    v.dirty = true;
                }
                None => {
                    self.mach.made_view = true;
                    let status = std::mem::take(&mut self.shell.status);
                    self.show_loaded(Loaded {
                        file: None,
                        name: def.name.clone(),
                        empty: false,
                        scene,
                        renderer,
                    });
                    self.shell.status = status;
                }
            }
        }
        // the ghosts' labels where they are on the screen, and the inset, for the panel to draw
        if let Some(v) = &self.view {
            let rect = self.shell.view_rect;
            let px: Vec<(egui::Pos2, String)> = self
                .mach
                .labels
                .iter()
                .filter_map(|(p, l)| {
                    crate::gui_camsim_panel::screen_of(&v.nav.camera, v.size, rect, *p)
                        .map(|q| (q, l.clone()))
                })
                .collect();
            let inset = self.mach.inset.clone();
            if let Some(m) = &mut self.shell.machine {
                m.view_labels = px;
                m.inset = inset;
            }
        }
        let Some(m) = &self.shell.machine else { return };
        if m.pose != self.mach.pose
            && let Some(v) = &mut self.view
        {
            self.mach.pose = m.pose;
            let valid = m.problem.is_none() && tooling.is_none();
            let pose = if valid { kin::pose(&def, &m.q) } else { Vec::new() };
            for (i, (name, link)) in self.mach.bodies.iter().enumerate() {
                if i >= v.scene.placements.len() {
                    break;
                }
                let rows = if valid {
                    rows_of(kin::link(&def, &pose, link))
                } else {
                    rows_of(&kin::Rigid::ID)
                };
                v.scene.set_rows(i, &rows);
                let hit = m.live.iter().find(|(a, b, _)| a == name || b == name);
                v.scene.placements[i].tint = match hit {
                    Some((_, _, true)) => HIT,
                    Some((_, _, false)) => NEAR,
                    None if tooling.is_some() => {
                        self.mach.tints.get(i).copied().unwrap_or(crate::palette::GREY)
                    }
                    None => link_tint(&def, link),
                };
            }
            v.renderer.set_tints(&v.scene);
            v.dirty = true;
        }
    }
}

impl Gui {
    /// A Machine Builder ribbon command.
    pub(crate) fn machine_cmd(&mut self, c: crate::shell::machine::MachineCmd) {
        use crate::shell::machine::MachineCmd as C;
        if c == C::Close {
            self.shell.machine = None;
            return;
        }
        if let Some(m) = &mut self.shell.machine {
            m.command(c);
        }
    }

    /// The Machine Builder by words, for the page and tests (`app_machine`): `open`, `close`,
    /// `pick NAME`, `template KIND NAME`, `clone NAME`, `jog JOINT VALUE`, `tab NAME`, `check`,
    /// `save user|project`. The answer goes to the status line as `machine: [CMD] ANSWER`.
    pub fn machine_command(&mut self, cmd: &str) -> Result<String, String> {
        use crate::shell::machine::Tab;
        use forge_machine_builder::catalog::Level;
        use forge_machine_builder::make::Template;
        let w: Vec<&str> = cmd.split_whitespace().collect();
        if w.first() == Some(&"open") {
            if self.shell.machine.is_none() {
                self.machine_toggle();
            }
        } else if w.first() == Some(&"close") {
            self.shell.machine = None;
            self.shell.status = "machine: closed".into();
            return Ok("closed".into());
        }
        let Some(m) = &mut self.shell.machine else {
            return Err("the Machine Builder is not open".into());
        };
        let got: Result<(), String> = match w.as_slice() {
            ["open"] => Ok(()),
            ["pick", name] => match m.cat.get(name) {
                Some(e) => {
                    let (d, l, f) = (e.def.clone(), e.level, e.file.clone());
                    m.load(d, Some((l, f)), Vec::new());
                    Ok(())
                }
                None => Err(format!("no machine `{name}`")),
            },
            ["template", kind, name] => match Template::from_word(kind) {
                Some(t) => {
                    m.template = t;
                    m.new_name = name.to_string();
                    m.make_template();
                    Ok(())
                }
                None => Err(format!("no template `{kind}`")),
            },
            ["clone", name] => {
                m.new_name = name.to_string();
                m.make_clone();
                Ok(())
            }
            ["jog", joint, value] => {
                let i = m.def.as_ref().and_then(|d| d.joint_index(joint));
                match (i, value.parse::<f64>()) {
                    (Some(i), Ok(v)) => {
                        m.q[i] = v;
                        m.edited(false);
                        Ok(())
                    }
                    _ => Err(format!("no joint `{joint}` or not a number `{value}`")),
                }
            }
            ["tab", name] => match Tab::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name)) {
                Some(t) => {
                    m.tab = t;
                    Ok(())
                }
                None => Err(format!("no tab `{name}`")),
            },
            ["range", joint, lo, hi] => {
                let i = m.def.as_ref().and_then(|d| d.joint_index(joint));
                match (i, lo.parse::<f64>(), hi.parse::<f64>()) {
                    (Some(i), Ok(a), Ok(b)) if a < b => {
                        if let Some(d) = &mut m.def {
                            d.joints[i].range = Some([a, b]);
                            let path = format!("joint.{joint}.range");
                            d.cites.retain(|c| c.path != path);
                        }
                        m.changed = true;
                        m.edited(false);
                        Ok(())
                    }
                    _ => Err(format!("no joint `{joint}` or not a range `{lo} {hi}`")),
                }
            }
            ["lib", word] => {
                let all = ["machine", "workholding"]
                    .into_iter()
                    .chain(forge_tooling_builder::KINDS.iter().map(|k| k.0))
                    .chain(forge_brake_builder::KINDS.iter().map(|k| k.0));
                let found = all.into_iter().find(|k| k == word);
                match found {
                    Some(k) => {
                        m.set_lib(k);
                        Ok(())
                    }
                    None => Err(format!("no library `{word}`")),
                }
            }
            ["rec", id] => m.pick_record(id),
            ["check"] => {
                m.command(crate::shell::machine::MachineCmd::Check);
                Ok(())
            }
            ["save", level] => {
                m.level = if *level == "project" {
                    Level::Project
                } else {
                    Level::User
                };
                m.save();
                Ok(())
            }
            _ => Err(format!(
                "`{cmd}`: open | close | lib KIND | rec ID | range JOINT LO HI | pick NAME | template KIND NAME | clone NAME | jog JOINT VALUE | tab NAME | check | save user|project"
            )),
        };
        let say = match got {
            Ok(()) => {
                let live: Vec<String> = m
                    .live
                    .iter()
                    .map(|(a, b, hit)| format!("{a}/{b} {}", if *hit { "HIT" } else { "near" }))
                    .collect();
                format!(
                    "{}{}{}{}",
                    m.record_name().unwrap_or_else(|| "none".to_string()),
                    m.problem
                        .as_ref()
                        .map_or(String::new(), |p| format!(" refused: {p}")),
                    if live.is_empty() {
                        String::new()
                    } else {
                        format!(" contact: {}", live.join(", "))
                    },
                    if m.message.is_empty() {
                        String::new()
                    } else {
                        format!(" | {}", m.message)
                    }
                )
            }
            Err(e) => e,
        };
        self.shell.status = format!("machine: [{cmd}] {say}");
        Ok(say)
    }
}
