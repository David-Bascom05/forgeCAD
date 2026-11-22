//! Random part programs over every feature kind, written as Forge (.fg) source. Each statement is
//! made for the part as it stands: placed on its faces (through small helper sketches that hold
//! the points and directions a placement needs), its edges and faces picked by selections (`of` a
//! feature, `between` two, by crease and shape, and `extreme` along a world axis to single one
//! out), never by a name read off the part, so a program keeps its meaning when the shrinker drops
//! statements above. A statement the kernel refuses with a clear error is dropped and the program
//! goes on; a panic, an invalid body, an open mesh, an n-ary regen that disagrees with the
//! sequential one, or a statement the language cannot read (the generator's bug) is a failure.
//!
//! The kernel's robustness gates drive it: `run` and `run_kinds` grow a history from a seed,
//! `judge` builds a whole program and says how it fared, `shrink` cuts a failing one down.

use crate::Options;
use crate::build::Cache;
use crate::error::Error;
use crate::eval::{Level, Rules};
use forge_agent::select::{self, Ent};
use forge_geom::CurveGeom;
use forge_math::rng::Rng;
use forge_math::{Box3, Point3, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_model::{Feature, Part, Solid};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::collections::{BTreeMap, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// One feature kind's generator: statements (helper sketches first) for the part as it stands,
/// or none where the kind has nothing to work on.
pub struct Maker {
    pub name: &'static str,
    weight: usize,
    make: fn(&mut G) -> Option<Vec<String>>,
}

macro_rules! makers {
    ($($name:literal $w:literal $f:ident),* $(,)?) => {
        &[$(Maker { name: $name, weight: $w, make: $f }),*]
    };
}

/// Every feature kind the language has (an import needs a file: its round trips are forge-step's
/// and the language's own tests), each with how often it is drawn.
pub const MAKERS: &[Maker] = makers![
    "extrude" 7 extrude,
    "revolve" 3 revolve,
    "ball" 2 ball,
    "fillet" 7 fillet,
    "chamfer" 4 chamfer,
    "shell" 1 shell,
    "draft" 2 draft,
    "hole" 5 hole,
    "pattern" 3 pattern,
    "sweep" 2 sweep,
    "coil" 1 coil,
    "loft" 2 loft,
    "workplane" 1 workplane,
    "facefillet" 2 facefillet,
    "fullround" 1 fullround,
    "param" 1 param,
    "sketch" 2 sketch,
    "rib" 2 rib,
    "offset_faces" 1 offset_faces,
    "thicken" 1 thicken,
    "delete_faces" 1 delete_faces,
    "replace_face" 1 replace_face,
    "move_faces" 1 move_faces,
    "workaxis" 1 workaxis,
    "workpoint" 1 workpoint,
    "ucs" 1 ucs,
    "combine" 1 combine,
    "split" 1 split,
    "splitface" 1 splitface,
    "movebody" 1 movebody,
    "copybody" 1 copybody,
    "thread" 1 thread,
    "text" 1 text,
];

/// The seeds the tests run. (Not 0: its program has a thread that takes the kernel four minutes,
/// kept as `kernel_history_repros::slow_thread_after_a_moved_body`.)
pub const SEEDS: [u64; 34] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33, 34,
];

/// Mesh chord for the volume checks.
const CHORD: f64 = 0.004;

/// The font text statements name: a small test font (the suite's text part's glyphs), written
/// beside every program `options` builds.
pub const FONT: &str = "forgecad-test.ttf";

/// The test font's bytes: I, O and D on a grid of 1000 (cap height 700), D and O kerned.
pub fn font_bytes() -> Vec<u8> {
    use forge_font::Seg;
    use forge_font::testing::{TestGlyph, build, square_ring};
    use forge_math::Point2;
    let p = Point2::new;
    let q = |a: (f64, f64), c: (f64, f64), b: (f64, f64)| Seg::Quad(p(a.0, a.1), p(c.0, c.1), p(b.0, b.1));
    let d = |s: f64, o: f64| {
        vec![
            Seg::Line(p(o, o), p(o, 700.0 - o)),
            q((o, 700.0 - o), (600.0 - o * s, 700.0 - o), (600.0 - o * s, 350.0)),
            q((600.0 - o * s, 350.0), (600.0 - o * s, o), (o, o)),
        ]
    };
    let mut hole = d(1.5, 150.0);
    hole.reverse();
    let hole: Vec<Seg> = hole
        .into_iter()
        .map(|s| match s {
            Seg::Line(a, b) => Seg::Line(b, a),
            Seg::Quad(a, c, b) => Seg::Quad(b, c, a),
        })
        .collect();
    let glyphs = [
        ('I', TestGlyph::bar(100.0, 0.0, 200.0, 700.0, 300.0)),
        ('O', square_ring()),
        (
            'D',
            TestGlyph {
                contours: vec![d(1.0, 0.0), hole],
                advance: 700,
            },
        ),
    ];
    build(1000, &glyphs, &[('D', 'O', -40)])
}

/// A folder holding the test font, made once per process. Where no file can be written (a
/// sandbox without a temp folder), text statements are refused for want of the font.
pub fn font_dir() -> std::path::PathBuf {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("forge-fuzz-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(FONT), font_bytes()));
        dir
    })
    .clone()
}

/// How the generated programs are built: the test font beside them, the style rules that a
/// generated placement breaks on purpose (numbers outside params, world planes and directions
/// after the first feature, selections by where things lie) off, no measures.
pub fn options() -> Options {
    Options {
        dir: Some(font_dir()),
        rules: Rules {
            literals_in_params: Level::Off,
            scoped_params: Level::Off,
            world_only_first: Level::Off,
            no_positional_selection: Level::Off,
            expect_copied: Level::Off,
            ..Rules::default()
        },
        checks: false,
        overlap: false,
        expects: false,
        meshes: false,
        ..Options::default()
    }
}

/// What a feature statement made, for the statements after it.
#[derive(Clone, Debug)]
struct Label {
    name: String,
    kind: &'static str,
    /// A solid feature a pattern may copy and a selection may name.
    solid: bool,
    /// It made a body (the first, `new: true`, a split keeping both, a copy).
    body: bool,
    /// A hole drilled with a thread recorded.
    thread_hole: bool,
}

/// The program so far: its features, params, and which made each face of the part.
#[derive(Default)]
struct State {
    labels: Vec<Label>,
    params: Vec<(String, f64)>,
    /// Each face name of the part and the labels (by index) that made it.
    made: HashMap<Name, Vec<usize>>,
    /// Helper sketches named so far.
    helpers: usize,
}

/// What a generator sees: the part so far and the dice.
pub struct G<'a> {
    r: &'a mut Rng,
    st: &'a State,
    body: &'a Body,
    bx: Box3,
    label: &'a mut usize,
    helpers: usize,
    /// Points the statements need, by the height of their helper sketch: (name, x, y).
    points: BTreeMap<String, Vec<(String, String, String)>>,
    new_labels: Vec<Label>,
    new_params: Vec<(String, f64)>,
}

/// The world axis a direction is, if it is one.
fn world(v: Vec3) -> Option<&'static str> {
    let w = [
        (Vec3::X, "X"),
        (-Vec3::X, "-X"),
        (Vec3::Y, "Y"),
        (-Vec3::Y, "-Y"),
        (Vec3::Z, "Z"),
        (-Vec3::Z, "-Z"),
    ];
    w.iter().find(|(d, _)| (*d - v).length() < 1e-12).map(|x| x.1)
}

const DIRS: [(Vec3, &str); 6] = [
    (Vec3::X, "X"),
    (Vec3::new(-1.0, 0.0, 0.0), "-X"),
    (Vec3::Y, "Y"),
    (Vec3::new(0.0, -1.0, 0.0), "-Y"),
    (Vec3::Z, "Z"),
    (Vec3::new(0.0, 0.0, -1.0), "-Z"),
];

impl G<'_> {
    /// A new feature's name.
    fn label(&mut self) -> String {
        *self.label += 1;
        format!("L{}", *self.label)
    }
    /// Records what the statement named `name` makes.
    fn made(&mut self, name: &str, kind: &'static str, solid: bool, body: bool) {
        self.new_labels.push(Label {
            name: name.into(),
            kind,
            solid,
            body,
            thread_hole: false,
        });
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        self.r.range(lo, hi)
    }
    fn one_in(&mut self, n: usize) -> bool {
        self.r.below(n) == 0
    }
    fn pick<T: Clone>(&mut self, xs: &[T]) -> Option<T> {
        (!xs.is_empty()).then(|| xs[self.r.below(xs.len())].clone())
    }
    /// One of these faces or edges, by name: a seed makes the same part whatever order the
    /// arenas are in.
    fn pick_named<T: Copy + Named>(&mut self, xs: &[T]) -> Option<T> {
        let mut v = xs.to_vec();
        v.sort_by_key(|x| x.name_in(self.body));
        self.pick(&v)
    }
    /// A size on the part's scale.
    fn size(&mut self, lo: f64, hi: f64) -> f64 {
        let d = self.bx.diagonal();
        self.range(lo * d, hi * d)
    }
    fn labels(&self, keep: impl Fn(&Label) -> bool) -> Vec<String> {
        self.st
            .labels
            .iter()
            .filter(|l| keep(l))
            .map(|l| l.name.clone())
            .collect()
    }
    fn planes(&self) -> Vec<String> {
        self.labels(|l| l.kind == "workplane")
    }
    /// Solid features a pattern may copy, a selection name.
    fn solids(&self) -> Vec<String> {
        self.labels(|l| l.solid)
    }
    fn op(&mut self) -> &'static str {
        match self.r.below(40) {
            0 => "keep",
            k if k < 20 => "cut",
            _ => "add",
        }
    }
    /// The flag a solid's op is written with.
    fn op_arg(op: &str) -> &'static str {
        match op {
            "cut" => ", cut: true",
            "keep" => ", keep: true",
            "new" => ", new: true",
            _ => "",
        }
    }

    // ---- placement: points and directions through helper sketches ----

    /// A point, written as a helper sketch's fixed point (to two decimals).
    fn pt(&mut self, p: Point3) -> String {
        self.pt_at(n(p.x), n(p.y), n(p.z))
    }
    /// A point exactly where a derived position is.
    fn pt_exact(&mut self, p: Point3) -> String {
        self.pt_at(x(p.x), x(p.y), x(p.z))
    }
    fn pt_at(&mut self, px: String, py: String, pz: String) -> String {
        let pts = self.points.entry(pz.clone()).or_default();
        let k = match pts.iter().position(|q| q.1 == px && q.2 == py) {
            Some(k) => k,
            None => {
                pts.push((format!("p{}", pts.len()), px, py));
                pts.len() - 1
            }
        };
        // the sketches are named once all points are in: a placeholder for now
        format!("@{pz}@.p{k}")
    }
    /// A direction: a world axis, or the axis through two helper points.
    fn dir(&mut self, v: Vec3) -> String {
        if let Some(w) = world(v) {
            return w.into();
        }
        let v = Vec3::new(r3(v.x), r3(v.y), r3(v.z));
        if let Some(w) = world(v) {
            return w.into();
        }
        let a = self.pt_at("0".into(), "0".into(), "0".into());
        let b = self.pt_at(x(100.0 * v.x), x(100.0 * v.y), x(100.0 * v.z));
        format!("axis({a}, {b})")
    }
    /// A plane at a point with a normal (and an x along a direction).
    fn plane(&mut self, at: Point3, z: Vec3, xd: Option<Vec3>) -> String {
        let p = self.pt(at);
        let nz = self.dir(z);
        match xd {
            Some(xd) => {
                let nx = self.dir(xd);
                format!("plane(at: {p}, normal: {nz}, x: {nx})")
            }
            None => format!("plane(at: {p}, normal: {nz})"),
        }
    }
    /// The helper sketches the points live in, and the statements with their names filled in.
    fn emit(&mut self, lines: Vec<String>) -> Vec<String> {
        let mut out = Vec::new();
        let mut names: Vec<(String, String)> = Vec::new();
        for (z, pts) in &self.points {
            let name = format!("H{}", self.st.helpers + self.helpers);
            self.helpers += 1;
            let mut s = format!("sketch {name}(on: offset(xy, {z})) {{\n");
            for (p, px, py) in pts {
                s.push_str(&format!("  {p} = pt({px}, {py})\n"));
            }
            s.push('}');
            out.push(s);
            names.push((format!("@{z}@"), name));
        }
        for l in lines {
            let mut l = l;
            for (z, nm) in &names {
                l = l.replace(z, nm);
            }
            out.push(l);
        }
        out
    }

    /// A point on a random planar face and the face's outward normal (the top of the box
    /// sometimes, or when there is no planar face).
    fn spot(&mut self) -> (Point3, Vec3) {
        let planar: Vec<FaceId> = self
            .body
            .face_ids()
            .filter(|&f| select::plane_normal(self.body, f).is_some())
            .collect();
        match self.pick_named(&planar) {
            Some(f) if !self.one_in(5) => {
                let nm = select::plane_normal(self.body, f).expect("planar");
                let fb = self.body.face_box(f);
                let edges = select::edges_of_face(self.body, f);
                let at = match self.pick_named(&edges) {
                    Some(e) => {
                        let m = select::centre(self.body, Ent::Edge(e));
                        let t = self.range(0.15, 0.6);
                        m + (fb.center() - m) * t
                    }
                    None => fb.center(),
                };
                (at, nm)
            }
            _ => {
                let (px, py) = (
                    self.range(self.bx.x.lo, self.bx.x.hi),
                    self.range(self.bx.y.lo, self.bx.y.hi),
                );
                (Point3::new(px, py, self.bx.z.hi), Vec3::Z)
            }
        }
    }
    /// `n`, tipped a little now and then.
    fn tilt(&mut self, nv: Vec3) -> Vec3 {
        if self.one_in(4) {
            let t = Vec3::new(
                self.range(-0.3, 0.3),
                self.range(-0.3, 0.3),
                self.range(-0.3, 0.3),
            );
            (nv + t).normalize().map_or(nv, |u| u.get())
        } else {
            nv
        }
    }

    // ---- selections, worked out here as the language works them out ----

    /// The labels (by index) that made face `f`.
    fn makers_of(&self, f: FaceId) -> Vec<usize> {
        self.st
            .made
            .get(&self.body.face(f).name)
            .cloned()
            .unwrap_or_default()
    }
    fn label_index(&self, l: &str) -> Option<usize> {
        self.st.labels.iter().position(|x| x.name == l)
    }
    /// Edges as `edges(of:, between:, crease:, shape:, extreme:)` finds them.
    fn edges_sel(
        &self,
        of: Option<usize>,
        between: Option<(usize, usize)>,
        crease: Option<&str>,
        shape: Option<&str>,
        extreme: Option<Vec3>,
    ) -> Vec<EdgeId> {
        let b = self.body;
        let by = |f: FaceId, l: usize| self.makers_of(f).contains(&l);
        let mut es: Vec<EdgeId> = b.edge_ids().collect();
        if let Some(l) = of {
            es.retain(|&e| select::faces_of_edge(b, e).into_iter().any(|f| by(f, l)));
        }
        if let Some((p, q)) = between {
            es.retain(|&e| match &select::faces_of_edge(b, e)[..] {
                [f, g] => (by(*f, p) && by(*g, q)) || (by(*f, q) && by(*g, p)),
                _ => false,
            });
        }
        if let Some(c) = crease {
            es.retain(|&e| select::crease(b, e) == Some(c));
        }
        if let Some(s) = shape {
            es.retain(|&e| select::curve_kind(&b.edge(e).curve) == s);
        }
        if let Some(d) = extreme {
            let at = |e: EdgeId| (select::centre(b, Ent::Edge(e)) - Point3::ORIGIN).dot(d);
            let top = es.iter().map(|&e| at(e)).fold(f64::NEG_INFINITY, f64::max);
            es.retain(|&e| at(e) >= top - 1e-6 * top.abs().max(1.0));
        }
        es
    }
    /// Faces as `faces(of:, shape:, normal:, extreme:)` finds them.
    fn faces_sel(
        &self,
        of: Option<usize>,
        shape: Option<&str>,
        normal: Option<Vec3>,
        extreme: Option<Vec3>,
    ) -> Vec<FaceId> {
        let b = self.body;
        let mut fs: Vec<FaceId> = b.face_ids().collect();
        if let Some(l) = of {
            fs.retain(|&f| self.makers_of(f).contains(&l));
        }
        if let Some(s) = shape {
            fs.retain(|&f| select::surface_kind(&b.face(f).surface) == s);
        }
        if let Some(nv) = normal {
            fs.retain(|&f| select::plane_normal(b, f).is_some_and(|m| m.dot(nv) > 1.0 - 1e-9));
        }
        if let Some(d) = extreme {
            let at = |f: FaceId| (select::centre(b, Ent::Face(f)) - Point3::ORIGIN).dot(d);
            let top = fs.iter().map(|&f| at(f)).fold(f64::NEG_INFINITY, f64::max);
            fs.retain(|&f| at(f) >= top - 1e-6 * top.abs().max(1.0));
        }
        fs
    }
    /// A selection that finds exactly edge `e` (`crease`: its crease named), if one does.
    fn sel_edge(&mut self, e: EdgeId, crease: bool) -> Option<String> {
        let b = self.body;
        let c = select::crease(b, e);
        let k = select::curve_kind(&b.edge(e).curve);
        let mut ofs: Vec<Option<usize>> = vec![None];
        let mut mk: Vec<usize> = select::faces_of_edge(b, e)
            .into_iter()
            .flat_map(|f| self.makers_of(f))
            .collect();
        mk.sort_unstable();
        mk.dedup();
        ofs.extend(mk.into_iter().map(Some));
        let creases: Vec<Option<&str>> = if crease { vec![c] } else { vec![None, c] };
        let mut tries = Vec::new();
        for &of in &ofs {
            for &cr in &creases {
                for sh in [None, Some(k)] {
                    for (d, dn) in DIRS {
                        tries.push((of, cr, sh, d, dn));
                    }
                }
            }
        }
        let start = self.r.below(tries.len());
        tries.rotate_left(start);
        for (of, cr, sh, d, dn) in tries {
            if self.edges_sel(of, None, cr, sh, Some(d)) == [e] {
                let mut args = Vec::new();
                if let Some(l) = of {
                    args.push(format!("of: {}", self.st.labels[l].name));
                }
                if let Some(cr) = cr {
                    args.push(format!("crease: \"{cr}\""));
                }
                if let Some(sh) = sh {
                    args.push(format!("shape: \"{sh}\""));
                }
                args.push(format!("extreme: {dn}"));
                return Some(format!("edges({})", args.join(", ")));
            }
        }
        // no world axis singles it out: the way from the middle of its like out to it
        for &of in &ofs {
            for &cr in &creases {
                for sh in [None, Some(k)] {
                    let like = self.edges_sel(of, None, cr, sh, None);
                    let at = |e: EdgeId| select::centre(b, Ent::Edge(e));
                    let Some(d) = outward(&like.iter().map(|&x| at(x)).collect::<Vec<_>>(), at(e)) else {
                        continue;
                    };
                    if self.edges_sel(of, None, cr, sh, Some(d)) == [e] {
                        let mut args = Vec::new();
                        if let Some(l) = of {
                            args.push(format!("of: {}", self.st.labels[l].name));
                        }
                        if let Some(cr) = cr {
                            args.push(format!("crease: \"{cr}\""));
                        }
                        if let Some(sh) = sh {
                            args.push(format!("shape: \"{sh}\""));
                        }
                        args.push(format!("extreme: {}", self.dir(d)));
                        return Some(format!("edges({})", args.join(", ")));
                    }
                }
            }
        }
        None
    }
    /// A selection that finds exactly face `f`: of label `of` if given, its shape named if `shape`.
    fn sel_face(&mut self, f: FaceId, shape: bool, of: Option<usize>) -> Option<String> {
        let b = self.body;
        let k = select::surface_kind(&b.face(f).surface);
        let nm = select::plane_normal(b, f).filter(|v| world(*v).is_some());
        let ofs: Vec<Option<usize>> = match of {
            Some(l) => vec![Some(l)],
            None => {
                let mut v = vec![None];
                v.extend(self.makers_of(f).into_iter().map(Some));
                v
            }
        };
        let shapes: Vec<Option<&str>> = if shape { vec![Some(k)] } else { vec![None, Some(k)] };
        let mut tries = Vec::new();
        for &o in &ofs {
            for &sh in &shapes {
                for nv in [None, nm] {
                    for (d, dn) in DIRS {
                        tries.push((o, sh, nv, d, dn));
                    }
                }
            }
        }
        let start = self.r.below(tries.len());
        tries.rotate_left(start);
        for (o, sh, nv, d, dn) in tries {
            if self.faces_sel(o, sh, nv, Some(d)) == [f] {
                let mut args = Vec::new();
                if let Some(l) = o {
                    args.push(format!("of: {}", self.st.labels[l].name));
                }
                if let Some(sh) = sh {
                    args.push(format!("shape: \"{sh}\""));
                }
                if let Some(v) = nv {
                    args.push(format!("normal: {}", world(v).expect("a world axis")));
                }
                args.push(format!("extreme: {dn}"));
                return Some(format!("faces({})", args.join(", ")));
            }
        }
        // no world axis singles it out: the way from the middle of its like out to it
        for &o in &ofs {
            for &sh in &shapes {
                let like = self.faces_sel(o, sh, None, None);
                let at = |f: FaceId| select::centre(b, Ent::Face(f));
                let Some(d) = outward(&like.iter().map(|&x| at(x)).collect::<Vec<_>>(), at(f)) else {
                    continue;
                };
                if self.faces_sel(o, sh, None, Some(d)) == [f] {
                    let mut args = Vec::new();
                    if let Some(l) = o {
                        args.push(format!("of: {}", self.st.labels[l].name));
                    }
                    if let Some(sh) = sh {
                        args.push(format!("shape: \"{sh}\""));
                    }
                    args.push(format!("extreme: {}", self.dir(d)));
                    return Some(format!("faces({})", args.join(", ")));
                }
            }
        }
        None
    }
    /// The labels (by index) that made a face round vertex `v`.
    fn makers_round(&self, v: VertexId) -> Vec<usize> {
        let b = self.body;
        let mut out: Vec<usize> = b
            .edge_ids()
            .filter(|&e| b.edge(e).start == v || b.edge(e).end == v)
            .flat_map(|e| select::faces_of_edge(b, e))
            .flat_map(|f| self.makers_of(f))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
    /// Vertices as `verts(of:, extreme:)` finds them.
    fn verts_sel(&self, of: Option<usize>, extreme: Option<Vec3>) -> Vec<VertexId> {
        let b = self.body;
        let mut vs = select::verts(b);
        if let Some(l) = of {
            vs.retain(|&v| self.makers_round(v).contains(&l));
        }
        if let Some(d) = extreme {
            let at = |v: VertexId| (b.vertex(v).point - Point3::ORIGIN).dot(d);
            let top = vs.iter().map(|&v| at(v)).fold(f64::NEG_INFINITY, f64::max);
            vs.retain(|&v| at(v) >= top - 1e-6 * top.abs().max(1.0));
        }
        vs
    }
    /// A selection that finds exactly vertex `v` (the kernel follows it through edits), if one
    /// does.
    fn sel_vert(&mut self, v: VertexId) -> Option<String> {
        let b = self.body;
        let mut ofs: Vec<Option<usize>> = vec![None];
        ofs.extend(self.makers_round(v).into_iter().map(Some));
        let mut tries = Vec::new();
        for &of in &ofs {
            for (d, dn) in DIRS {
                tries.push((of, d, dn.to_string()));
            }
        }
        let start = self.r.below(tries.len());
        tries.rotate_left(start);
        let write = |g: &Self, of: Option<usize>, dn: String| match of {
            Some(l) => format!("verts(of: {}, extreme: {dn})", g.st.labels[l].name),
            None => format!("verts(extreme: {dn})"),
        };
        for (of, d, dn) in tries {
            if self.verts_sel(of, Some(d)) == [v] {
                return Some(write(self, of, dn));
            }
        }
        // no world axis singles it out: the way from the middle of its like out to it
        for &of in &ofs {
            let like = self.verts_sel(of, None);
            let pts: Vec<Point3> = like.iter().map(|&x| b.vertex(x).point).collect();
            let Some(d) = outward(&pts, b.vertex(v).point) else {
                continue;
            };
            if self.verts_sel(of, Some(d)) == [v] {
                let dn = self.dir(d);
                return Some(write(self, of, dn));
            }
        }
        None
    }
    /// A vertex of the part, as a selection the kernel follows (a helper point exactly on it
    /// where no selection singles it out).
    fn vert(&mut self) -> Option<String> {
        let vs = select::verts(self.body);
        let v = self.pick_named(&vs)?;
        match self.sel_vert(v) {
            Some(s) => Some(s),
            None => {
                let p = self.body.vertex(v).point;
                Some(self.pt_exact(p))
            }
        }
    }
    /// A selection over the whole part that finds between 1 and `most` edges.
    fn found_edges(&self, es: &[EdgeId], most: usize) -> bool {
        !es.is_empty() && es.len() <= most
    }
    /// Edges where two faces meet at an angle.
    fn creases(&self) -> Vec<EdgeId> {
        self.body
            .edge_ids()
            .filter(|&e| matches!(select::crease(self.body, e), Some("convex" | "concave")))
            .collect()
    }
    /// Up to `k` edges (by selections) and the length of the shortest.
    fn edges(&mut self, k: usize) -> Option<(Vec<String>, f64)> {
        let creases = self.creases();
        // a whole feature's rims or seams at once, sometimes
        if self.one_in(3) {
            let ls = self.solids();
            let a = self.pick(&ls)?;
            let b = self.pick(&ls)?;
            let (ia, ib) = (self.label_index(&a)?, self.label_index(&b)?);
            let words: [(Option<&str>, Option<&str>); 6] = [
                (Some("convex"), None),
                (Some("concave"), None),
                (None, Some("circle")),
                (None, Some("line")),
                (Some("convex"), Some("circle")),
                (Some("concave"), Some("line")),
            ];
            let (cr, sh) = self.pick(&words)?;
            let (s, hits) = if a != b && self.one_in(2) {
                (
                    format!("edges(between: ({a}, {b}))"),
                    self.edges_sel(None, Some((ia, ib)), None, None, None),
                )
            } else {
                let mut args = vec![format!("of: {a}")];
                if let Some(c) = cr {
                    args.push(format!("crease: \"{c}\""));
                }
                if let Some(s) = sh {
                    args.push(format!("shape: \"{s}\""));
                }
                (
                    format!("edges({})", args.join(", ")),
                    self.edges_sel(Some(ia), None, cr, sh, None),
                )
            };
            if self.found_edges(&hits, 12) {
                let len = hits
                    .iter()
                    .map(|&e| select::length(self.body, e))
                    .fold(f64::INFINITY, f64::min);
                return Some((vec![s], len));
            }
        }
        let mut out = Vec::new();
        let mut len = f64::INFINITY;
        for _ in 0..1 + self.r.below(k) {
            let e = self.pick_named(&creases)?;
            let named = self.r.below(3) == 0;
            if let Some(s) = self.sel_edge(e, named)
                && !out.contains(&s)
            {
                len = len.min(select::length(self.body, e));
                out.push(s);
            }
        }
        (!out.is_empty()).then_some((out, len))
    }
    /// A face of the part: planar if `planar`, and passing `keep` on its normal.
    fn face(&mut self, planar: bool, keep: impl Fn(Option<Vec3>) -> bool) -> Option<(FaceId, String)> {
        let fs: Vec<FaceId> = self
            .body
            .face_ids()
            .filter(|&f| {
                let nv = select::plane_normal(self.body, f);
                (!planar || nv.is_some()) && keep(nv)
            })
            .collect();
        for _ in 0..4 {
            let f = self.pick_named(&fs)?;
            if let Some(s) = self.sel_face(f, planar, None) {
                return Some((f, s));
            }
        }
        None
    }
    /// An end for a solid travelling from `at` along `dir`: a work plane ahead (made here if
    /// there is none), or (a third of the time) a face of an earlier feature; and an offset past
    /// it.
    fn upto(&mut self, at: Point3, dir: Vec3, out: &mut Vec<String>) -> Option<(String, String)> {
        if self.one_in(3) {
            let ls = self.solids();
            let l = self.pick(&ls)?;
            let il = self.label_index(&l)?;
            let hits = self.faces_sel(Some(il), None, None, None);
            let f = self.pick_named(&hits)?;
            let face = self.sel_face(f, false, Some(il))?;
            return Some((face, self.offset()));
        }
        let planes = self.planes();
        let p = match self.pick(&planes) {
            Some(p) if self.one_in(2) => p,
            _ => {
                let d = self.size(0.05, 0.4);
                let z = self.tilt(dir);
                let l = self.label();
                let pl = self.plane(at + dir * d, z, None);
                out.push(format!("{l} = workplane(on: {pl})"));
                self.made(&l, "workplane", false, false);
                l
            }
        };
        Some((p, self.offset()))
    }
    fn offset(&mut self) -> String {
        if self.one_in(3) {
            n(self.range(-2.0, 2.0))
        } else {
            String::new()
        }
    }
    /// Profile rows round the sketch origin, about `s` across: an outline `o`, a hole `c` now
    /// and then.
    fn loops(&mut self, s: f64) -> Vec<String> {
        let h = s / 2.0;
        let mut out = match self.r.below(6) {
            0 | 1 => vec![format!(
                "o = rect(corner: ({}, {}), size: ({}, {}))",
                n(-h),
                n(-h * 0.6),
                n(s),
                n(s * 0.6)
            )],
            2 => {
                let rr = rd(s * self.range(0.05, 0.29));
                vec![rounded_rect("o", rd(-h), rd(-h * 0.6), rd(s), rd(s * 0.6), rr)]
            }
            3 => vec![format!("o = circle((0, 0), {})", n(h))],
            4 => {
                let k = 3 + self.r.below(4);
                let mut pts = Vec::new();
                for i in 0..k {
                    let a = dmath::TAU * (i as f64 + self.range(-0.3, 0.3)) / k as f64;
                    let (sa, ca) = dmath::sin_cos(a);
                    let rr = h * self.range(0.6, 1.0);
                    pts.push(format!("({}, {})", n(rr * ca), n(rr * sa)));
                }
                vec![format!("o = poly([{}])", pts.join(", "))]
            }
            _ => vec![
                format!("l = line(({}, 0), ({}, 0))", n(-h), n(h)),
                format!("a = arc(center: (0, 0), from: ({}, 0), to: ({}, 0))", n(h), n(-h)),
            ],
        };
        if self.one_in(5) {
            out.push(format!("c = circle((0, {}), {})", n(s * 0.05), n(s * 0.12)));
        }
        out
    }
}

/// The direction from the middle of `pts` out to `p`, as `dir` writes it (to a thousandth),
/// unit: none where `p` is the middle.
fn outward(pts: &[Point3], p: Point3) -> Option<Vec3> {
    if pts.len() < 2 {
        return None;
    }
    let k = pts.len() as f64;
    let m = pts
        .iter()
        .fold(Vec3::new(0.0, 0.0, 0.0), |a, q| a + (*q - Point3::ORIGIN))
        / k;
    let d = (p - Point3::ORIGIN - m).normalize()?.get();
    let d = Vec3::new(r3(d.x), r3(d.y), r3(d.z));
    // as the language reads it: the axis between two points 100 apart along it
    Some(d.normalize()?.get())
}

/// A rounded rectangle: the hull of four discs of radius `r` in its corners.
fn rounded_rect(name: &str, x0: f64, y0: f64, w: f64, h: f64, r: f64) -> String {
    let (x1, y1) = (x0 + w - r, y0 + h - r);
    let (x0, y0) = (x0 + r, y0 + r);
    format!(
        "{name} = hull(disc(({}, {}), {r}), disc(({}, {}), {r}), disc(({}, {}), {r}), disc(({}, {}), {r}))",
        x(x0),
        x(y0),
        x(x1),
        x(y0),
        x(x1),
        x(y1),
        x(x0),
        x(y1),
        r = x(r)
    )
}

/// A sketch statement: its name, plane and rows.
fn sketch_block(name: &str, on: &str, rows: &[String]) -> String {
    let mut s = format!("sketch {name}(on: {on}) {{\n");
    for r in rows {
        s.push_str("  ");
        s.push_str(r);
        s.push('\n');
    }
    s.push('}');
    s
}

/// What orders an entity among its like, whatever the arena.
trait Named {
    fn name_in(&self, b: &Body) -> Name;
}

impl Named for FaceId {
    fn name_in(&self, b: &Body) -> Name {
        b.face(*self).name
    }
}

impl Named for EdgeId {
    fn name_in(&self, b: &Body) -> Name {
        b.edge(*self).name
    }
}

impl Named for forge_topo::VertexId {
    fn name_in(&self, b: &Body) -> Name {
        b.vertex(*self).name
    }
}

/// A number as Rust writes it: the shortest that reads back the same.
fn fmt_num(v: f64) -> String {
    format!("{v}")
}

/// A generated value, to two decimals.
fn rd(v: f64) -> f64 {
    (v * 100.0).round() / 100.0 + 0.0
}

/// A direction's component, to a thousandth.
fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0 + 0.0
}

fn rp(p: Point3) -> Point3 {
    Point3::new(rd(p.x), rd(p.y), rd(p.z))
}

fn n(v: f64) -> String {
    fmt_num(rd(v))
}

/// A derived number, exactly.
fn x(v: f64) -> String {
    fmt_num(v + 0.0)
}

/// A derived sketch coordinate, to a thousandth.
fn x_(v: f64) -> String {
    fmt_num(r3(v))
}

fn deg(v: f64) -> String {
    format!("{}deg", n(v))
}

/// The first feature: a block, a rounded or round plate, or a turned stepped shaft.
fn base(r: &mut Rng) -> String {
    let (w, d, t) = (r.range(40.0, 150.0), r.range(30.0, 120.0), r.range(8.0, 40.0));
    match r.below(10) {
        0 => format!(
            "{}\nL0 = extrude(S0, {})",
            sketch_block(
                "S0",
                "xy",
                &[format!("o = circle((0, 0), {})", n(w.min(d) / 2.0))]
            ),
            n(t)
        ),
        1 => {
            let rr = rd(r.range(1.0, 0.45 * w.min(d)));
            format!(
                "{}\nL0 = extrude(S0, {})",
                sketch_block("S0", "xy", &[rounded_rect("o", 0.0, 0.0, rd(w), rd(d), rr)]),
                n(t)
            )
        }
        2 => {
            let (r0, r1) = (r.range(6.0, 20.0), r.range(10.0, 30.0));
            let pts = [
                (0.0, 0.0),
                (r0, 0.0),
                (r0, t),
                (r1, t),
                (r1, t + w / 3.0),
                (0.0, t + w / 3.0),
            ];
            let pts: Vec<String> = pts
                .iter()
                .map(|(a, b)| format!("({}, {})", n(*a), n(*b)))
                .collect();
            format!(
                "{}\nL0 = revolve(S0.o, axis: S0.ax)",
                sketch_block(
                    "S0",
                    "xz",
                    &[
                        format!("o = poly([{}])", pts.join(", ")),
                        "ax = line((0, 0), (0, 1), construction: true)".into(),
                    ]
                )
            )
        }
        _ => format!(
            "{}\nL0 = extrude(S0, {})",
            sketch_block(
                "S0",
                "xy",
                &[format!("o = rect(corner: (0, 0), size: ({}, {}))", n(w), n(d))]
            ),
            n(t)
        ),
    }
}

fn extrude(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    // now and then another body
    let op = if g.one_in(15) { "new" } else { g.op() };
    let (at, nrm) = g.spot();
    let s = g.size(0.04, 0.3);
    let depth = g.size(0.03, 0.25);
    // a cut sketched on the face going in, an add sunk a little going out
    let (start, dir) = if op == "add" || op == "new" {
        (at - nrm * g.range(0.0, 1.0), nrm)
    } else {
        (at + nrm * g.range(0.0, 1.0), -nrm)
    };
    let z = g.tilt(dir);
    let ext = match g.r.below(12) {
        0 => format!("sym({})", n(depth)),
        1 => format!("two({}, {})", n(depth), n(g.range(0.5, 3.0))),
        // thru, thru back (the sketch turned over below), thru both
        2 | 3 if op == "cut" => ["thru", "thru back", "thru_all"][g.r.below(3)].to_string(),
        // next, or next the other way (`flip`: the kernel's ToNext reversed)
        4 => ["next", "next back"][g.r.below(2)].to_string(),
        5 | 6 => {
            let (to, off) = g.upto(start, z, &mut out)?;
            if off.is_empty() {
                format!("upto({to})")
            } else {
                format!("upto({to}, {off})")
            }
        }
        _ => n(depth),
    };
    // a cut runs against its sketch's normal; through the other way, the sketch turned over
    let back = ext == "thru back";
    let ext = if back { "thru".to_string() } else { ext };
    let (ext, flip) = match ext.as_str() {
        "next back" => ("next".to_string(), ", flip: true"),
        _ => (ext, ""),
    };
    let sz = if op == "cut" && !back { -z } else { z };
    let on = match g.planes().last() {
        Some(p) if g.one_in(8) => {
            let p = p.clone();
            let a = g.pt(start);
            format!("{p}, at: {a}")
        }
        _ => g.plane(start, sz, None),
    };
    let l = g.label();
    let sk = format!("S{}", &l[1..]);
    out.push(sketch_block(&sk, &on, &g.loops(s)));
    let call = match op {
        "cut" => format!("cut({sk}, {ext}{flip})"),
        "keep" => format!("intersect({sk}, {ext}{flip})"),
        "new" => format!("extrude({sk}, {ext}, new: true{flip})"),
        _ => format!("extrude({sk}, {ext}{flip})"),
    };
    out.push(format!("{l} = {call}"));
    g.made(&l, "extrude", true, op == "new");
    Some(out)
}

fn revolve(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let op = g.op();
    let (at, _) = g.spot();
    let rad = g.size(0.04, 0.2);
    let tube = rad * g.range(0.1, 0.5);
    // sketch (u along the axis, v out from it)
    let (u0, v0) = (g.range(-tube, tube), rad);
    let which = g.r.below(3);
    let axis = [
        "ax = line((0, 0), (1, 0), construction: true)",
        "ax = line((0, 0), (0, -1), construction: true)",
        "ax = line((0, 0), (0, 1), construction: true)",
    ][which];
    let place = |u: f64, v: f64| match which {
        0 => (u, v),
        1 => (v, -u),
        _ => (v, u),
    };
    let prof = match g.r.below(3) {
        0 => {
            let (cx, cy) = place(u0, v0);
            format!("o = circle(({}, {}), {})", n(cx), n(cy), n(tube))
        }
        k => {
            let pts: Vec<(f64, f64)> = if k == 1 {
                vec![
                    (-tube, v0 - tube),
                    (tube, v0 - tube),
                    (tube, v0 + tube),
                    (-tube, v0 + tube),
                ]
            } else {
                vec![(-tube, v0 - tube), (tube, v0 - tube), (0.0, v0 + tube)]
            };
            let pts: Vec<String> = pts
                .into_iter()
                .map(|(u, v)| {
                    let (a, b) = place(u, v);
                    format!("({}, {})", n(a), n(b))
                })
                .collect();
            format!("o = poly([{}])", pts.join(", "))
        }
    };
    let turn = match g.r.below(8) {
        0 => format!(", angle: {}", deg(g.range(30.0, 300.0))),
        1 => format!(", angle: sym({})", deg(g.range(20.0, 150.0))),
        2 => format!(
            ", angle: two({}, {})",
            deg(g.range(20.0, 150.0)),
            deg(g.range(10.0, 90.0))
        ),
        3 => {
            // a plane through the standing axis (the sketch's y), turned about it
            let a = g.range(0.3, 2.8);
            let (s, c) = dmath::sin_cos(a);
            let w = g.label();
            let pl = g.plane(at, Vec3::new(rd(-s), rd(c), 0.0), None);
            out.push(format!("{w} = workplane(on: {pl})"));
            g.made(&w, "workplane", false, false);
            let off = if g.one_in(3) { ", 5deg" } else { "" };
            format!(", angle: upto({w}{off})")
        }
        _ => String::new(),
    };
    let lift = at.z + g.range(-tube, tube);
    let on = g.plane(Point3::new(at.x, at.y, lift), -Vec3::Y, Some(Vec3::X));
    let l = g.label();
    let sk = format!("S{}", &l[1..]);
    out.push(sketch_block(&sk, &on, &[prof, axis.to_string()]));
    out.push(format!(
        "{l} = revolve({sk}.o, axis: {sk}.ax{turn}{})",
        G::op_arg(op)
    ));
    g.made(&l, "revolve", true, false);
    Some(out)
}

fn ball(g: &mut G) -> Option<Vec<String>> {
    let op = g.op();
    let (at, nrm) = g.spot();
    let r = g.size(0.02, 0.15);
    let c = at + nrm * (r * g.range(-0.8, 0.8));
    let p = g.pt(c);
    let l = g.label();
    g.made(&l, "ball", true, false);
    Some(vec![format!("{l} = ball({p}, {}{})", n(r), G::op_arg(op))])
}

/// A fillet radius law no bigger than `most`.
fn radius(g: &mut G, most: f64) -> String {
    let r = |g: &mut G| n((most * g.range(0.1, 1.0)).max(0.01));
    match g.r.below(6) {
        0 => format!("vary({}, {})", r(g), r(g)),
        1 => format!(
            "vary([(0, {}), ({}, {}), (1, {})])",
            r(g),
            n(g.range(0.2, 0.8)),
            r(g),
            r(g)
        ),
        _ => r(g),
    }
}

fn fillet(g: &mut G) -> Option<Vec<String>> {
    let (edges, len) = g.edges(3)?;
    let most = (len * 0.4).min(g.bx.diagonal() * 0.05);
    let first = radius(g, most);
    let mut groups = vec![(edges[0].clone(), first)];
    for e in &edges[1..] {
        if g.one_in(3) {
            let r = radius(g, most);
            groups.push((e.clone(), r));
        } else {
            let last = groups.last_mut().expect("one");
            last.0 = format!("{} + {e}", last.0);
        }
    }
    // a law's start, or a corner's setback, at a vertex of the part
    let vert = |g: &mut G| g.vert();
    let mut opts = String::new();
    // (a law's start belongs to the one-group form; a group carries its own)
    if g.one_in(6)
        && let Some(v) = vert(g)
    {
        if groups.len() == 1 {
            opts.push_str(&format!(", from: {v}"));
        } else {
            let last = groups.last_mut().expect("one");
            last.1 = format!("{}, {v}", last.1);
        }
    }
    if g.one_in(6)
        && let Some(v) = vert(g)
    {
        opts.push_str(&format!(", setbacks: [({v}, {})]", n(most * g.range(0.5, 2.0))));
    }
    let l = g.label();
    let line = if groups.len() == 1 {
        format!("{l} = fillet({}, {}{opts})", groups[0].0, groups[0].1)
    } else {
        let gs: Vec<String> = groups.iter().map(|(e, r)| format!("({e}, {r})")).collect();
        format!("{l} = fillet(groups: [{}]{opts})", gs.join(", "))
    };
    g.made(&l, "fillet", false, false);
    Some(vec![line])
}

fn chamfer(g: &mut G) -> Option<Vec<String>> {
    // chamfers run along lines and circles
    let creases: Vec<_> = g
        .creases()
        .into_iter()
        .filter(|&e| select::curve_kind(&g.body.edge(e).curve) != "curve")
        .collect();
    let e = g.pick_named(&creases)?;
    let len = select::length(g.body, e);
    let d = |g: &mut G| n((len * 0.3).min(g.bx.diagonal() * 0.04) * g.range(0.1, 1.0) + 0.01);
    let mut sel = g.sel_edge(e, false)?;
    let size = match g.r.below(4) {
        k @ (0 | 1) => {
            // the face the first distance is measured on: one of the edge's own
            let fs = select::faces_of_edge(g.body, e);
            let f = g.pick_named(&fs)?;
            let face = g.sel_face(f, false, None)?;
            if k == 0 {
                format!("{}, d2: {}, face: {face}", d(g), d(g))
            } else {
                format!("{}, angle: {}, face: {face}", d(g), deg(g.range(20.0, 70.0)))
            }
        }
        _ => d(g),
    };
    if g.one_in(2)
        && let Some((more, _)) = g.edges(2)
    {
        for m in more {
            sel = format!("{sel} + {m}");
        }
    }
    let l = g.label();
    g.made(&l, "chamfer", false, false);
    Some(vec![format!("{l} = chamfer({sel}, {size})")])
}

fn shell(g: &mut G) -> Option<Vec<String>> {
    let t = g.size(0.003, 0.02);
    let out = if g.one_in(4) { ", out: true" } else { "" };
    let mut open: Vec<String> = Vec::new();
    for _ in 0..g.r.below(3) {
        if g.one_in(2) {
            let s = "faces(shape: \"plane\", normal: Z, extreme: Z)".to_string();
            if !open.contains(&s) {
                open.push(s);
            }
        } else if let Some((_, s)) = g.face(true, |_| true)
            && !open.contains(&s)
        {
            open.push(s);
        }
    }
    let open = if open.is_empty() {
        String::new()
    } else {
        format!(", open: {}", open.join(" + "))
    };
    let l = g.label();
    g.made(&l, "shell", false, false);
    Some(vec![format!("{l} = shell({}{open}{out})", n(t + 0.1))])
}

fn draft(g: &mut G) -> Option<Vec<String>> {
    let a = g.range(1.0, 8.0) * if g.one_in(2) { -1.0 } else { 1.0 };
    let z = g.range(g.bx.z.lo, g.bx.z.hi);
    let mut faces: Vec<String> = Vec::new();
    for _ in 0..1 + g.r.below(4) {
        // walls: planes square to the pull, or round walls along it
        let wall = |nm: Option<Vec3>| nm.is_none_or(|v| v.z.abs() < 1e-9);
        let planar = !g.one_in(4);
        if let Some((_, s)) = g.face(planar, wall)
            && !faces.contains(&s)
        {
            faces.push(s);
        }
    }
    if faces.is_empty() {
        return None;
    }
    let l = g.label();
    g.made(&l, "draft", false, false);
    Some(vec![format!(
        "{l} = draft({}, angle: {}, neutral: offset(xy, {}))",
        faces.join(" + "),
        deg(a),
        n(z)
    )])
}

fn hole(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let (at, nrm) = g.spot();
    let d = g.size(0.01, 0.08) + 0.5;
    let head = match g.r.below(4) {
        0 => format!(
            ", cbore: ({}, {})",
            n(d * g.range(1.4, 2.0)),
            n(d * g.range(0.3, 1.0))
        ),
        1 => format!(
            ", csink: ({}, {})",
            n(d * g.range(1.4, 2.2)),
            ["90deg", "82deg", "100deg", "120deg"][g.r.below(4)]
        ),
        _ => String::new(),
    };
    let (end, thru) = match g.r.below(6) {
        0 | 1 => ("thru".to_string(), true),
        2 => ("next".to_string(), false),
        3 => {
            let (to, off) = g.upto(at, -nrm, &mut out)?;
            let off = if off.is_empty() { off } else { format!(", {off}") };
            (format!("upto({to}{off})"), false)
        }
        _ => (n(g.size(0.03, 0.2)), false),
    };
    // a blind hole's drill point
    let pt = if !thru && g.one_in(3) {
        ", point: 118deg"
    } else {
        ""
    };
    let mut threaded = false;
    // a standard size, its fit and seat, now and then
    let size = if g.one_in(3) {
        let m = ["M3", "M4", "M5", "M6", "M8", "M10", "M8x1", "1/4-20", "10-32"][g.r.below(9)];
        let std = ["", ", convention: \"iso\"", ", convention: \"ansi\""][g.r.below(3)];
        let fit = [
            "",
            ", fit: \"close\"",
            ", fit: \"normal\"",
            ", fit: \"loose\"",
            ", fit: \"tap\"",
        ][g.r.below(5)];
        let seat = ["", "", ", seat: \"cbore\"", ", seat: \"csink\""][g.r.below(4)];
        threaded = fit.contains("tap");
        format!("screw: \"{m}\"{std}{fit}{seat}")
    } else {
        let thread = if g.one_in(4) {
            threaded = true;
            format!(", thread: ({}, {})", n(d * 1.2), n(d * 0.15 + 0.2))
        } else {
            String::new()
        };
        format!("d: {}{head}{thread}", n(d))
    };
    let p = g.pt(at);
    let on = g.plane(at, nrm, None);
    let l = g.label();
    out.push(format!("{l} = hole(at: {p}, on: {on}, {size}, depth: {end}{pt})"));
    g.made(&l, "hole", true, false);
    g.new_labels.last_mut().expect("just made").thread_hole = threaded;
    Some(out)
}

fn pattern(g: &mut G) -> Option<Vec<String>> {
    let whole = g.one_in(4);
    let src = if whole {
        "whole: true".to_string()
    } else {
        let ls = g.solids();
        let s = g.pick(&ls[1.min(ls.len())..])?;
        if g.one_in(3)
            && let Some(o) = g.pick(&ls[1.min(ls.len())..])
            && o != s
        {
            format!("of: [{s}, {o}]")
        } else {
            format!("of: {s}")
        }
    };
    let c = g.bx.center();
    let step = g.size(0.05, 0.3);
    // copies of the whole part are many n-ary booleans at once: a few
    let most = if whole { 1 } else { 3 };
    let axes = g.labels(|l| l.kind == "workaxis");
    let points = g.labels(|l| l.kind == "workpoint");
    let layout = match g.r.below(5) {
        3 => {
            let a = g.pick(&axes)?;
            let how = if g.one_in(2) {
                String::new()
            } else {
                format!(", step: {}", n(step))
            };
            let opts = format!(
                "{}{}",
                if g.one_in(3) { ", fixed: true" } else { "" },
                if g.one_in(3) { ", reverse: true" } else { "" }
            );
            format!("follow(curve: {a}, n: {}{how}{opts})", 2 + g.r.below(3))
        }
        4 => {
            let mut l = Vec::new();
            for _ in 0..1 + g.r.below(3) {
                match g.pick(&points) {
                    Some(p) if g.one_in(2) => l.push(p),
                    _ => {
                        let q = Point3::new(
                            g.range(g.bx.x.lo, g.bx.x.hi),
                            g.range(g.bx.y.lo, g.bx.y.hi),
                            g.range(g.bx.z.lo, g.bx.z.hi),
                        );
                        l.push(g.pt(q));
                    }
                }
            }
            format!("places(at: [{}])", l.join(", "))
        }
        0 => {
            let a = g.range(0.0, dmath::TAU);
            let (s, co) = dmath::sin_cos(a);
            let k = 2 + g.r.below(most);
            let along = g.dir(Vec3::new(rd(co), rd(s), 0.0));
            let mut l = format!("lin(along: {along}, n: {k}, step: {}", n(step));
            if most > 1 && g.one_in(3) {
                let along2 = g.dir(Vec3::new(rd(-s), rd(co), 0.0));
                l.push_str(&format!(
                    ", along2: {along2}, n2: {}, step2: {}",
                    2 + g.r.below(2),
                    n(step)
                ));
            }
            l + ")"
        }
        1 => {
            let o = Point3::new(c.x + g.range(-step, step), c.y + g.range(-step, step), 0.0);
            let (pa, pb) = (g.pt(o), g.pt(Point3::new(o.x, o.y, 10.0)));
            let k = 2 + g.r.below(2 * most);
            let over = match g.r.below(3) {
                0 => format!(", over: {}", deg(g.range(30.0, 300.0))),
                1 => format!(", each: {}", deg(g.range(10.0, 90.0))),
                _ => String::new(),
            };
            format!("circ(about: axis({pa}, {pb}), n: {k}{over})")
        }
        _ => {
            let a = g.range(0.0, dmath::PI);
            let (s, co) = dmath::sin_cos(a);
            let off = g.range(-0.6, 0.6) * g.bx.diagonal() / 2.0;
            let o = Point3::new(c.x + co * off, c.y + s * off, 0.0);
            let pl = g.plane(o, Vec3::new(rd(co), rd(s), 0.0), None);
            format!("mirror(on: {pl})")
        }
    };
    let l = g.label();
    g.made(&l, "pattern", !whole, false);
    Some(vec![format!("{l} = pattern({src}, layout: {layout})")])
}

fn sweep(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let op = g.op();
    let (at, nrm) = g.spot();
    let r = g.size(0.01, 0.04);
    // along the face, half sunk, or diving into it
    let dirs = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    let t = g.pick(&dirs)?;
    let bends: Vec<Vec3> = dirs.iter().copied().filter(|d| d.dot(t) == 0.0).collect();
    let b = g.pick(&bends)?;
    let a = rp(at - t * g.size(0.05, 0.2) + nrm * (r * 0.5));
    let rad = rd(r * g.range(1.5, 5.0));
    let turn: f64 = [90.0, 45.0, 60.0, 120.0, 30.0][g.r.below(5)];
    let (l1, l2) = (rd(g.size(0.05, 0.3)), rd(g.size(0.0, 0.3)).max(0.5));
    let s = 2.0 * r;
    let prof = match g.r.below(3) {
        0 => format!(
            "o = rect(corner: ({}, {}), size: ({}, {}))",
            n(-r),
            n(-r),
            n(s),
            n(s)
        ),
        1 => rounded_rect("o", rd(-r), rd(-r), rd(s), rd(s), rd(r * 0.3)),
        _ => format!("o = circle((0, 0), {})", n(r)),
    };
    let ends = match g.r.below(4) {
        0 => format!(", extend: ({}, {})", n(g.range(0.0, 5.0)), n(g.range(0.0, 5.0))),
        1 => {
            let (to, off) = g.upto(a + t * l1, t, &mut out)?;
            if off.is_empty() {
                format!(", upto: {to}")
            } else {
                format!(", upto: {to}, offset: {off}")
            }
        }
        _ => String::new(),
    };
    // the path in the plane of the run and the bend: x along the run, y towards the bend
    let (st, ct) = dmath::sin_cos(turn / 180.0 * dmath::PI);
    let (ex, ey) = (l1 + rad * st, rad * (1.0 - ct));
    let (fx, fy) = (ex + l2 * ct, ey + l2 * st);
    let path_on = g.plane(a, t.cross(b), Some(t));
    let prof_on = g.plane(a, t, Some(b));
    let l = g.label();
    let k = &l[1..];
    out.push(sketch_block(
        &format!("P{k}"),
        &path_on,
        &[
            "path route(start: (0, 0)) {".into(),
            format!("  line(to: ({}, 0))", x(l1)),
            format!("  tarc(to: ({}, {}))", x(ex), x(ey)),
            format!("  line(to: ({}, {}))", x(fx), x(fy)),
            "}".into(),
        ],
    ));
    out.push(sketch_block(&format!("S{k}"), &prof_on, &[prof]));
    out.push(format!(
        "{l} = sweep(S{k}.o, along: P{k}.route{ends}{})",
        G::op_arg(op)
    ));
    g.made(&l, "sweep", true, false);
    Some(out)
}

fn coil(g: &mut G) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let op = g.op();
    let (at, _) = g.spot();
    let rad = g.size(0.03, 0.15);
    let r = rad * g.range(0.08, 0.25);
    let prof = if g.one_in(2) {
        format!("o = circle(({}, 0), {})", n(rad), n(r))
    } else {
        format!(
            "o = rect(corner: ({}, {}), size: ({}, {}))",
            n(rad - r),
            n(-r),
            n(2.0 * r),
            n(2.0 * r)
        )
    };
    let pitch = 2.0 * r * g.range(1.3, 3.0);
    let turns = g.range(1.0, 5.0);
    let down = g.one_in(3);
    let size = match g.r.below(4) {
        0 => format!("pitch: {}, turns: {}", n(pitch), n(turns)),
        1 => format!("turns: {}, height: {}", n(turns), n(pitch * turns)),
        2 => format!("pitch: {}, height: {}", n(pitch), n(pitch * turns)),
        _ => {
            let dir = if down { -Vec3::Z } else { Vec3::Z };
            let (to, off) = g.upto(at, dir, &mut out)?;
            let off = if off.is_empty() {
                off
            } else {
                format!(", offset: {off}")
            };
            format!("pitch: {}, upto: {to}{off}", n(pitch))
        }
    };
    let taper = if g.one_in(4) {
        format!(", taper: {}", deg(g.range(-10.0, 10.0)))
    } else {
        String::new()
    };
    let left = if g.one_in(3) { ", left: true" } else { "" };
    let axis = if down {
        "ax = line((0, 0), (0, -1), construction: true)"
    } else {
        "ax = line((0, 0), (0, 1), construction: true)"
    };
    let z0 = at.z + g.range(-2.0 * r, 2.0 * r);
    let on = g.plane(Point3::new(at.x, at.y, z0), -Vec3::Y, Some(Vec3::X));
    let l = g.label();
    let sk = format!("S{}", &l[1..]);
    out.push(sketch_block(&sk, &on, &[prof, axis.into()]));
    out.push(format!(
        "{l} = coil({sk}.o, axis: {sk}.ax, {size}{taper}{left}{})",
        G::op_arg(op)
    ));
    g.made(&l, "coil", true, false);
    Some(out)
}

fn loft(g: &mut G) -> Option<Vec<String>> {
    let op = g.op();
    let (at, _) = g.spot();
    let k = 2 + g.r.below(2);
    let round = g.one_in(3);
    let smooth = k == 3 || g.one_in(2);
    let mut z = at.z - g.range(0.0, 2.0);
    let mut opts = String::new();
    if smooth {
        opts.push_str(", smooth: true");
        if g.one_in(3) {
            opts.push_str(&format!(", start: {}", n(g.range(0.2, 2.0))));
        }
        if g.one_in(3) {
            opts.push_str(&format!(", end: {}", n(g.range(0.2, 2.0))));
        }
    }
    let l = g.label();
    let num = &l[1..];
    let mut out = Vec::new();
    let mut names = Vec::new();
    let mut corners = Vec::new();
    for i in 0..k {
        let s = g.size(0.03, 0.15);
        let twist = !smooth && g.one_in(3);
        let xd = if twist {
            let (sa, ca) = dmath::sin_cos(g.range(-0.6, 0.6));
            Some(Vec3::new(rd(ca), rd(sa), 0.0))
        } else {
            None
        };
        let on = g.plane(Point3::new(at.x, at.y, z), Vec3::Z, xd);
        let sk = format!("S{num}_{i}");
        let row = if round {
            format!("o = circle((0, 0), {})", n(s / 2.0))
        } else {
            let h = rd(s / 2.0);
            corners.push(Point3::new(rd(at.x) - h, rd(at.y) - h, rd(z)));
            format!(
                "o = rect(corner: ({}, {}), size: ({}, {}))",
                x(-h),
                x(-h),
                x(2.0 * h),
                x(2.0 * h)
            )
        };
        out.push(sketch_block(&sk, &on, &[row]));
        names.push(format!("{sk}.o"));
        if i + 1 < k {
            z += g.size(0.05, 0.2);
        }
    }
    // a straight line in 3D from `a` up to `b` (straight above it), in a sketch of its own
    let upright = |g: &mut G, name: &str, a: Point3, b: Point3| {
        let on = g.plane(a, Vec3::X, Some(Vec3::Z));
        sketch_block(name, &on, &[format!("l = line((0, 0), ({}, 0))", x(b.z - a.z))])
    };
    if smooth && k == 2 && !round && g.one_in(2) {
        // a straight rail through each section's first corner
        let rail = format!("R{num}");
        let (a, b) = (corners[0], corners[1]);
        let s = upright(g, &rail, Point3::new(a.x, a.y, a.z), Point3::new(a.x, a.y, b.z));
        out.push(s);
        opts.push_str(&format!(", rails: [({}[0].a, {rail}.l)]", names[0]));
    } else if smooth && g.one_in(3) {
        let a = rp(Point3::new(at.x, at.y, at.z - 3.0));
        let c = format!("C{num}");
        let s = upright(g, &c, a, Point3::new(a.x, a.y, rd(z + 3.0)));
        out.push(s);
        opts.push_str(&format!(", centre: {c}.l"));
    }
    out.push(format!(
        "{l} = loft([{}]{opts}{})",
        names.join(", "),
        G::op_arg(op)
    ));
    g.made(&l, "loft", true, false);
    Some(out)
}

fn workplane(g: &mut G) -> Option<Vec<String>> {
    let c = g.bx.center();
    let z = g.tilt(Vec3::Z);
    let h = match g.r.below(3) {
        0 => g.bx.z.hi + g.size(0.02, 0.3),
        1 => g.bx.z.lo - g.size(0.02, 0.3),
        _ => g.range(g.bx.z.lo, g.bx.z.hi),
    };
    let on = g.plane(Point3::new(c.x, c.y, h), z, None);
    let l = g.label();
    g.made(&l, "workplane", false, false);
    Some(vec![format!("{l} = workplane(on: {on})")])
}

fn facefillet(g: &mut G) -> Option<Vec<String>> {
    // the two faces of a crease
    let creases = g.creases();
    let e = g.pick_named(&creases)?;
    let mut fs = select::faces_of_edge(g.body, e);
    fs.sort_by_key(|f| f.name_in(g.body));
    let (a, b) = (*fs.first()?, *fs.get(1)?);
    let (a, b) = (g.sel_face(a, false, None)?, g.sel_face(b, false, None)?);
    let r = (select::length(g.body, e) * 0.2).min(g.bx.diagonal() * 0.03) * g.range(0.1, 1.0);
    let l = g.label();
    g.made(&l, "facefillet", false, false);
    Some(vec![format!("{l} = facefillet({a}, {b}, {})", n(r.max(0.05)))])
}

fn fullround(g: &mut G) -> Option<Vec<String>> {
    // a planar face between two planar neighbours facing apart, square to it
    let b = g.body;
    let planar: Vec<FaceId> = b
        .face_ids()
        .filter(|&f| select::plane_normal(b, f).is_some())
        .collect();
    let mut found = Vec::new();
    for &m in &planar {
        let nm = select::plane_normal(b, m)?;
        let near: Vec<FaceId> = select::edges_of_face(b, m)
            .into_iter()
            .flat_map(|e| select::faces_of_edge(b, e))
            .filter(|&f| f != m && select::plane_normal(b, f).is_some_and(|v| v.dot(nm).abs() < 1e-9))
            .collect();
        for &s1 in &near {
            for &s2 in &near {
                let (n1, n2) = (select::plane_normal(b, s1)?, select::plane_normal(b, s2)?);
                if n1.dot(n2) < -1.0 + 1e-9 {
                    found.push((s1, m, s2));
                }
            }
        }
    }
    found.sort_by_key(|v: &(FaceId, FaceId, FaceId)| (v.0.name_in(b), v.1.name_in(b), v.2.name_in(b)));
    let (s1, m, s2) = g.pick(&found)?;
    let (a, mid, c) = (
        g.sel_face(s1, false, None)?,
        g.sel_face(m, false, None)?,
        g.sel_face(s2, false, None)?,
    );
    let l = g.label();
    g.made(&l, "fullround", false, false);
    Some(vec![format!(
        "{l} = fullround(side: {a}, middle: {mid}, other: {c})"
    )])
}

fn param(g: &mut G) -> Option<Vec<String>> {
    let k = g.st.params.len() + g.new_params.len();
    let v = g.size(0.02, 0.1);
    let unit = if g.one_in(3) { "mm" } else { "" };
    let (expr, value) = match (k, g.r.below(3)) {
        (0, _) | (_, 0) => (format!("{}{unit}", n(v)), rd(v)),
        (_, 1) => {
            let f = rd(g.range(0.5, 1.5));
            let prev = g.st.params.last().map_or(0.0, |p| p.1);
            (format!("len{} * {} + 1{unit}", k - 1, fmt_num(f)), prev * f + 1.0)
        }
        _ => {
            let prev = g.st.params.last().map_or(0.0, |p| p.1);
            (
                format!("max(len{}, {}{unit}) / 2", k - 1, n(v)),
                prev.max(rd(v)) / 2.0,
            )
        }
    };
    let name = format!("len{k}");
    g.new_params.push((name.clone(), value));
    Some(vec![format!("shared {name} = {expr}")])
}

/// A dimension: a number, or now and then an earlier parameter (sized as numbers here are).
fn dim(g: &mut G, v: f64) -> String {
    let ps: Vec<String> =
        g.st.params
            .iter()
            .filter(|p| p.1 > 0.0)
            .map(|p| p.0.clone())
            .collect();
    match g.pick(&ps) {
        Some(p) if g.one_in(3) => format!("{p} * {}", n(v / g.bx.diagonal() * 20.0 + 0.1)),
        _ => n(v),
    }
}

fn sketch(g: &mut G) -> Option<Vec<String>> {
    let (at, nrm) = g.spot();
    let s = g.size(0.04, 0.2);
    let on = match g.face(true, |_| true) {
        Some((_, f)) if g.one_in(3) => f,
        _ => g.plane(at, nrm, None),
    };
    let w = dim(g, s);
    let (x0, y0, x1, y1) = (rd(-s / 2.0), rd(-s / 3.0), rd(s / 2.0), rd(s / 3.0));
    let mut rows = vec![
        format!(
            "o = poly([({}, {}), near({}, {}), near({}, {}), near({}, {})])",
            x(x0),
            x(y0),
            x(x1),
            x(y0),
            x(x1),
            x(y1),
            x(x0),
            x(y1)
        ),
        "horizontal(o[0])".into(),
        "vertical(o[1])".into(),
        "horizontal(o[2])".into(),
        "vertical(o[3])".into(),
        format!("distance(o[1], {})", n(y1 - y0)),
        format!("distance(o[0], {w})"),
    ];
    if g.one_in(2) {
        rows.push(format!(
            "c = circle(near(0, 0), {})",
            n((s * g.range(0.05, 0.15)).max(0.05))
        ));
    }
    let op = g.op();
    let depth = g.size(0.02, 0.15);
    let d = dim(g, depth);
    let l = g.label();
    let sk = format!("S{}", &l[1..]);
    let call = match op {
        "cut" => format!("cut({sk}, {d})"),
        "keep" => format!("intersect({sk}, {d})"),
        _ => format!("extrude({sk}, {d})"),
    };
    g.made(&l, "extrude", true, false);
    Some(vec![sketch_block(&sk, &on, &rows), format!("{l} = {call}")])
}

fn rib(g: &mut G) -> Option<Vec<String>> {
    // a gusset across a concave straight edge: in the plane square to it, from one face to the
    // other, filling the corner between
    let b = g.body;
    let edges: Vec<EdgeId> = g
        .creases()
        .into_iter()
        .filter(|&e| {
            select::crease(b, e) == Some("concave") && select::curve_kind(&b.edge(e).curve) == "line"
        })
        .collect();
    let e = g.pick_named(&edges)?;
    let mut fs = select::faces_of_edge(b, e);
    fs.sort_by_key(|f| f.name_in(b));
    let (na, nb) = (
        select::plane_normal(b, *fs.first()?)?,
        select::plane_normal(b, *fs.get(1)?)?,
    );
    let m = select::centre(b, Ent::Edge(e));
    let dir = b.edge(e).curve.deriv(b.edge(e).range.mid()).d1.normalize()?.get();
    let xd = (nb - dir * nb.dot(dir)).normalize()?.get();
    let y = dir.cross(xd);
    let reach = (select::length(b, e) * 0.5).min(g.size(0.03, 0.15));
    let (p, q) = ((reach, 0.0), (reach * na.dot(xd), reach * na.dot(y)));
    // the corner (the sketch origin) on the path's left
    let left = (q.0 - p.0) * (0.0 - p.1) - (q.1 - p.1) * (0.0 - p.0) > 0.0;
    let (p, q) = if left { (p, q) } else { (q, p) };
    let thick = (g.size(0.005, 0.02) + 0.2).min(select::length(b, e) * 0.3);
    let draft = if g.one_in(4) {
        format!(", draft: {}", deg(g.range(0.5, 5.0)))
    } else {
        String::new()
    };
    let side = ["", "", ", side: \"pos\"", ", side: \"neg\""][g.r.below(4)];
    let on = {
        let at = g.pt_exact(m);
        let nz = g.dir(dir);
        let nx = g.dir(xd);
        format!("plane(at: {at}, normal: {nz}, x: {nx})")
    };
    let l = g.label();
    let sk = format!("S{}", &l[1..]);
    g.made(&l, "rib", true, false);
    Some(vec![
        sketch_block(
            &sk,
            &on,
            &[format!(
                "l = line(({}, {}), ({}, {}))",
                x_(p.0),
                x_(p.1),
                x_(q.0),
                x_(q.1)
            )],
        ),
        format!("{l} = rib({sk}.l, {}{side}{draft})", n(thick)),
    ])
}

fn offset_faces(g: &mut G) -> Option<Vec<String>> {
    let planar = !g.one_in(4);
    let (_, f) = g.face(planar, |_| true)?;
    let d = g.size(0.002, 0.02) * if g.one_in(2) { -1.0 } else { 1.0 };
    let l = g.label();
    g.made(&l, "offset_faces", false, false);
    Some(vec![format!("{l} = offset_faces({f}, {})", n(d))])
}

fn thicken(g: &mut G) -> Option<Vec<String>> {
    let planar = !g.one_in(4);
    let (_, f) = g.face(planar, |_| true)?;
    let op = ["add", "cut", "add", "new"][g.r.below(4)];
    let side = [", side: \"in\"", ", side: \"both\"", "", ""][g.r.below(4)];
    let t = g.size(0.005, 0.04) + 0.2;
    let l = g.label();
    g.made(&l, "thicken", true, op == "new");
    Some(vec![format!(
        "{l} = thicken({f}, {}{side}{})",
        n(t),
        G::op_arg(op)
    )])
}

fn delete_faces(g: &mut G) -> Option<Vec<String>> {
    // a hole, boss or pocket made by one earlier statement, or a blend strip
    let ls = g.solids();
    let l = g.pick(&ls[1.min(ls.len())..])?;
    let s = if g.one_in(3) {
        let blends: Vec<FaceId> = g
            .body
            .face_ids()
            .filter(|&f| select::surface_kind(&g.body.face(f).surface) == "blend")
            .collect();
        let f = g.pick_named(&blends)?;
        g.sel_face(f, true, None)?
    } else {
        let il = g.label_index(&l)?;
        let hits = g.faces_sel(Some(il), None, None, None);
        if hits.is_empty() || hits.len() > 40 {
            return None;
        }
        format!("faces(of: {l})")
    };
    let k = g.label();
    g.made(&k, "delete_faces", false, false);
    Some(vec![format!("{k} = delete_faces({s})")])
}

fn replace_face(g: &mut G) -> Option<Vec<String>> {
    // a planar face put on a work plane near and almost parallel to it
    let (f, s) = g.face(true, |_| true)?;
    let nrm = select::plane_normal(g.body, f)?;
    let at = select::centre(g.body, Ent::Face(f)) + nrm * g.size(-0.03, 0.03);
    let z = g.tilt(nrm);
    let w = g.label();
    let on = g.plane(at, z, None);
    let l = g.label();
    g.made(&w, "workplane", false, false);
    g.made(&l, "replace_face", false, false);
    Some(vec![
        format!("{w} = workplane(on: {on})"),
        format!("{l} = replace_face({s}, with: {w})"),
    ])
}

fn move_faces(g: &mut G) -> Option<Vec<String>> {
    let ls = g.solids();
    let l = g.pick(&ls[1.min(ls.len())..])?;
    let il = g.label_index(&l)?;
    let hits = g.faces_sel(Some(il), None, None, None);
    if hits.is_empty() || hits.len() > 40 {
        return None;
    }
    let d = g.size(0.005, 0.05);
    let how = if g.one_in(3) {
        let c = g.bx.center();
        let (pa, pb) = (g.pt(c), g.pt(Point3::new(c.x, c.y, c.z + 10.0)));
        format!("about: axis({pa}, {pb}), by: {}", deg(g.range(-5.0, 5.0)))
    } else {
        let v = Vec3::new(rd(g.range(-d, d)), rd(g.range(-d, d)), rd(g.range(-d, d) * 0.3));
        let len = v.length();
        let u = v.normalize()?.get();
        format!("along: {}, d: {}", g.dir(u), n(len))
    };
    let k = g.label();
    g.made(&k, "move_faces", false, false);
    Some(vec![format!("{k} = move_faces(faces(of: {l}), {how})")])
}

/// A face, edge or vertex an earlier statement made: `all` faces, edges or a vertex (a
/// `verts(..)` selection, the kernel following it through edits), `shape` its kind.
fn made_ref(g: &mut G, all: &str, shape: &str) -> Option<String> {
    if all == "verts" {
        let vs = select::verts(g.body);
        let v = g.pick_named(&vs)?;
        return g.sel_vert(v);
    }
    let ls = g.solids();
    let l = g.pick(&ls)?;
    let il = g.label_index(&l)?;
    if all == "edges" {
        let hits = g.edges_sel(Some(il), None, None, Some(shape), None);
        let e = g.pick_named(&hits)?;
        g.sel_edge(e, false)
    } else {
        let hits = g.faces_sel(Some(il), Some(shape), None, None);
        let f = g.pick_named(&hits)?;
        g.sel_face(f, true, Some(il))
    }
}

fn workaxis(g: &mut G) -> Option<Vec<String>> {
    let by = match g.r.below(3) {
        0 => format!("along: {}", made_ref(g, "edges", "line")?),
        1 => format!("along: {}", made_ref(g, "faces", "cyl")?),
        _ => format!(
            "through: ({}, {})",
            made_ref(g, "verts", "")?,
            made_ref(g, "verts", "")?
        ),
    };
    let l = g.label();
    g.made(&l, "workaxis", false, false);
    Some(vec![format!("{l} = workaxis({by})")])
}

fn workpoint(g: &mut G) -> Option<Vec<String>> {
    let by = match g.r.below(3) {
        0 => format!("at: {}", made_ref(g, "verts", "")?),
        // a circle edge's centre
        1 => format!("at: {}", made_ref(g, "edges", "circle")?),
        _ => format!("mid: {}", made_ref(g, "edges", "line")?),
    };
    let l = g.label();
    g.made(&l, "workpoint", false, false);
    Some(vec![format!("{l} = workpoint({by})")])
}

fn ucs(g: &mut G) -> Option<Vec<String>> {
    let (at, nrm) = g.spot();
    let z = g.tilt(nrm);
    let on = g.plane(at, z, None);
    let l = g.label();
    g.made(&l, "ucs", false, false);
    Some(vec![format!("{l} = ucs(on: {on})")])
}

/// A cutting plane through the part, tipped a little.
fn cutter(g: &mut G) -> String {
    let c = g.bx.center();
    let axis = [Vec3::X, Vec3::Y, Vec3::Z][g.r.below(3)];
    let z = g.tilt(axis);
    let at = c + z * g.size(-0.2, 0.2);
    g.plane(at, z, None)
}

/// A rigid motion: moved, turned about an upright line, or both.
fn motion(g: &mut G) -> Option<String> {
    let d = g.size(0.05, 0.6);
    let v = Vec3::new(rd(g.range(-d, d)), rd(g.range(-d, d)), rd(g.range(-d, d) * 0.3));
    let by = format!(
        "slide(along: {}, by: {})",
        g.dir(v.normalize()?.get()),
        x(v.length())
    );
    let c = g.bx.center();
    let (pa, pb) = (g.pt(c), g.pt(Point3::new(c.x, c.y, c.z + 10.0)));
    let rot = format!(
        "spin(about: axis({pa}, {pb}), by: {})",
        deg(g.range(-180.0, 180.0))
    );
    Some(match g.r.below(3) {
        0 => by,
        1 => rot,
        _ => format!("{by}, {rot}"),
    })
}

/// The statements that made bodies (the first, a later `new`, a split or a copy).
fn bodies(g: &G) -> Vec<String> {
    g.labels(|l| l.body)
}

fn combine(g: &mut G) -> Option<Vec<String>> {
    let bs = bodies(g);
    let (t, u) = (g.pick(&bs)?, g.pick(&bs)?);
    if t == u {
        return None;
    }
    let op = ["add", "cut", "keep"][g.r.below(3)];
    let keep = if g.one_in(3) { ", keep: true" } else { "" };
    let l = g.label();
    g.made(&l, "combine", false, false);
    Some(vec![format!(
        "{l} = combine({t}, tools: {u}, op: \"{op}\"{keep})"
    )])
}

fn split(g: &mut G) -> Option<Vec<String>> {
    let (side, both) = [
        ("", true),
        (", keep: \"plus\"", false),
        (", keep: \"minus\"", false),
    ][g.r.below(3)];
    let by = cutter(g);
    let l = g.label();
    g.made(&l, "split", false, both);
    Some(vec![format!("{l} = split(by: {by}{side})")])
}

fn splitface(g: &mut G) -> Option<Vec<String>> {
    let (f, s) = g.face(true, |_| true)?;
    let mut out = Vec::new();
    let l = g.label();
    let nrm = select::plane_normal(g.body, f)?;
    let at = select::centre(g.body, Ent::Face(f));
    let across = g.body.face_box(f).diagonal();
    let by = if g.one_in(2) {
        // a plane through the face's middle, square to it but for a tip
        let a = g.range(0.0, dmath::TAU);
        let u = nrm.normalize()?.any_perpendicular().get();
        let v = nrm.cross(u);
        let (sa, ca) = dmath::sin_cos(a);
        let z = g.tilt(u * ca + v * sa);
        g.plane(at, z, None)
    } else {
        // a line across the face, drawn on its plane
        let r = across * g.range(0.3, 1.0);
        let on = g.plane(at, nrm, None);
        let sk = format!("S{}", &l[1..]);
        out.push(sketch_block(
            &sk,
            &on,
            &[format!("l = line(({}, 0), ({}, 0))", n(-r), n(r))],
        ));
        format!("{sk}.l")
    };
    g.made(&l, "splitface", false, false);
    out.push(format!("{l} = splitface(faces: {s}, by: {by})"));
    Some(out)
}

fn movebody(g: &mut G) -> Option<Vec<String>> {
    let bs = bodies(g);
    let b = g.pick(&bs)?;
    let m = motion(g)?;
    let l = g.label();
    g.made(&l, "movebody", false, false);
    Some(vec![format!("{l} = movebody(of: {b}, moves: [{m}])")])
}

fn copybody(g: &mut G) -> Option<Vec<String>> {
    let bs = bodies(g);
    let b = g.pick(&bs)?;
    let m = motion(g)?;
    let l = g.label();
    g.made(&l, "copybody", false, true);
    Some(vec![format!("{l} = copybody(of: {b}, moves: [{m}])")])
}

fn thread(g: &mut G) -> Option<Vec<String>> {
    let end = match g.r.below(3) {
        0 => "\"full\"".to_string(),
        _ => n(g.size(0.03, 0.2)),
    };
    let opts = format!(
        "{}{}{}",
        if g.one_in(4) { ", left: true" } else { "" },
        if g.one_in(3) { ", chamfer: true" } else { "" },
        if g.one_in(4) { ", runout: true" } else { "" }
    );
    // the ISO coarse size nearest, now and then a fine pitch
    let fine = g.one_in(4);
    let spec = |d: f64| {
        let sizes = [
            2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0, 24.0, 30.0,
        ];
        let m = sizes
            .iter()
            .copied()
            .min_by(|a, b| (a - d).abs().total_cmp(&(b - d).abs()))
            .unwrap_or(6.0);
        if fine {
            format!("\"M{}x{}\"", fmt_num(m), fmt_num(rd(m * 0.1)))
        } else {
            format!("\"M{}\"", fmt_num(m))
        }
    };
    let call = match g.r.below(3) {
        // the thread of a hole drilled with one
        0 => {
            let holes = g.labels(|l| l.thread_hole);
            let h = g.pick(&holes)?;
            format!("thread(hole: {h}, length: {end}{opts})")
        }
        // round a cylinder face of an earlier statement
        1 => {
            let ls = g.solids();
            let l = g.pick(&ls)?;
            let il = g.label_index(&l)?;
            let hits = g.faces_sel(Some(il), Some("cyl"), None, None);
            let f = g.pick_named(&hits)?;
            let r = select::radius(g.body, Ent::Face(f))?;
            let face = g.sel_face(f, true, Some(il))?;
            let flip = if g.one_in(2) { ", flip: true" } else { "" };
            format!(
                "thread(face: {face}, spec: {}, length: {end}{flip}{opts})",
                spec(2.0 * r)
            )
        }
        // into the part at a spot, inside or outside
        _ => {
            let (at, nrm) = g.spot();
            let d = g.size(0.02, 0.08) + 2.0;
            let int = if g.one_in(2) { ", internal: true" } else { "" };
            let on = g.plane(at, nrm, None);
            let p = g.pt(at);
            format!(
                "thread(on: {on}, at: {p}, spec: {}{int}, length: {end}{opts})",
                spec(d)
            )
        }
    };
    let l = g.label();
    g.made(&l, "thread", true, false);
    Some(vec![format!("{l} = {call}")])
}

fn text(g: &mut G) -> Option<Vec<String>> {
    let (at, nrm) = g.spot();
    let words = ["DO", "OD", "D", "ODD"];
    let w = words[g.r.below(words.len())];
    let size = g.size(0.03, 0.15) + 1.0;
    let depth = g.size(0.005, 0.03) + 0.2;
    let cut = if g.one_in(2) { "" } else { ", cut: true" };
    let wrap = if g.one_in(4) {
        format!(", wrap: {}", n(g.size(0.05, 0.3) + 5.0))
    } else {
        String::new()
    };
    let on = g.plane(at, nrm, None);
    let p = g.pt(at);
    let l = g.label();
    g.made(&l, "text", true, false);
    Some(vec![format!(
        "{l} = text(\"{w}\", font: \"{FONT}\", size: {}, depth: {}, on: {on}, at: {p}{cut}{wrap})",
        n(size),
        n(depth)
    )])
}

/// A program's result.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Valid, closed, and the n-ary regen agrees.
    Ok,
    /// A statement refused with a clear error (its kind, the message).
    Refused(String),
    /// A kernel failure, or a statement the language cannot read (the generator's bug): what
    /// kind (stable across instances, for grouping).
    Fail(String),
}

/// A generated program: its source, each tried statement group's fate, and each failure with
/// the program that shows it.
pub struct Run {
    /// How many times the finished part's n-ary regen took the sequential one for a reference it
    /// names otherwise.
    pub name_fallbacks: usize,
    pub src: String,
    /// Per tried maker: its kind, and None if it built, else the refusal or failure.
    pub tried: Vec<(&'static str, Option<String>)>,
    pub fails: Vec<(String, String)>,
    /// Each refusal with the program it ends (the census of what the kernel will not build).
    pub refused: Vec<(String, String)>,
    /// The slowest statement group's seconds and the program it ends.
    pub slowest: (f64, String),
}

/// The message with names, handles and numbers blanked (to `*` and `9`), so one kind of
/// failure groups however many places it shows up.
fn kind_of_msg(msg: &str) -> String {
    let mut out = String::new();
    let mut cs = msg.chars().peekable();
    while let Some(c) = cs.next() {
        if c == '#' || c == '@' {
            while cs.peek().is_some_and(|d| d.is_ascii_hexdigit()) {
                cs.next();
            }
            out.push('*');
        } else if c.is_ascii_digit() || c == '-' && cs.peek().is_some_and(|d| d.is_ascii_digit()) {
            while cs
                .peek()
                .is_some_and(|d| d.is_ascii_digit() || *d == '.' || *d == 'e' || *d == '-')
            {
                cs.next();
            }
            out.push('9');
        } else {
            out.push(c);
        }
    }
    out.chars().take(100).collect()
}

/// What an error says, for its verdict: the kind, and the kernel's or the solver's own words.
pub fn message(e: &Error) -> String {
    match e {
        Error::Build { error, .. } => format!("Build {error:?}"),
        Error::SketchKernel { error, .. } => format!("SketchKernel {error:?}"),
        Error::SketchProfile { error, .. } => format!("SketchProfile {error:?}"),
        Error::Feat(f) => format!("{} {}", f.kind(), f.text()),
        Error::Sheet(s) => format!("Sheet {s:?}"),
        o => o.kind().to_string(),
    }
}

/// Error kinds that say the program is not one the language reads: the generator wrote it wrong.
const UNREAD: &[&str] = &[
    "Syntax",
    "ReservedWord",
    "UnknownName",
    "UnknownMember",
    "UnknownArg",
    "MissingArg",
    "TooManyArgs",
    "NotCallable",
    "WrongType",
    "WrongDim",
    "DimMismatch",
    "NotWhole",
    "Rebind",
    "ShadowsBuiltin",
    "ShadowsTopLevel",
    "ParamTwice",
    "DuplicateId",
    "IndexOut",
    "WrongPlace",
    "OtherSketch",
    "WrongEntity",
    "NotWith",
    "UnnamedFeature",
    "PositionalId",
    "PositionalSelection",
    "LooseLiteral",
    "WorldAfterFirst",
    "ParamOutOfScope",
    "Forms",
    "Conflict",
    "NotApplicable",
    "UnknownWord",
    "UnknownSize",
    "NotARef",
];

/// A refusal names what is wrong with the request. A kernel bug says Internal (a boolean lost
/// its way), Invalid (an operation made a bad body and caught it) or NotClosed (a boolean lost a
/// face); a statement the language cannot read is the generator's bug.
pub fn classify(kind: &str, e: &Error) -> Verdict {
    let msg = message(e);
    let bug = ["Internal", "internal error", "Invalid(", "NotClosed"]
        .iter()
        .any(|w| msg.contains(w));
    let unread = UNREAD.contains(&e.kind()) || matches!(e, Error::Feat(f) if UNREAD.contains(&f.kind()));
    let what = format!("{kind}: {}", kind_of_msg(&msg));
    if bug || unread {
        Verdict::Fail(what)
    } else {
        Verdict::Refused(what)
    }
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    let m = p
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "?".into());
    kind_of_msg(&m)
}

fn bbox(b: &Body) -> Box3 {
    let mut fs = b.face_ids();
    let first = fs
        .next()
        .map(|f| b.face_box(f))
        .unwrap_or(Box3::from_point(Point3::ORIGIN));
    fs.fold(first, |a, f| a.hull(&b.face_box(f)))
}

/// Nothing wrong with the body but vertices where its surface pinches.
fn only_pinched(b: &Body) -> bool {
    catch_unwind(AssertUnwindSafe(|| forge_topo::check(b))).is_ok_and(|p| {
        p.iter()
            .all(|v| matches!(v, forge_topo::Problem::VertexFans { .. }))
    })
}

/// A body's validity (kind `kind` names what made it).
fn valid(kind: &str, b: &Body) -> Verdict {
    match catch_unwind(AssertUnwindSafe(|| forge_topo::check(b))) {
        Err(p) => Verdict::Fail(format!("{kind}: check panics: {}", panic_text(p))),
        Ok(p) if !p.is_empty() => {
            let s = format!("{:?}", p[0]);
            Verdict::Fail(format!(
                "{kind}: invalid {}",
                s.split([' ', '{', '(']).next().unwrap_or("")
            ))
        }
        Ok(_) => Verdict::Ok,
    }
}

/// Each body of a stage valid on its own (separate bodies may overlap).
fn valid_solids(kind: &str, solids: &[Solid]) -> Verdict {
    solids
        .iter()
        .map(|s| valid(kind, &s.body))
        .find(|v| *v != Verdict::Ok)
        .unwrap_or(Verdict::Ok)
}

/// The finished part closes up and the n-ary regen makes the same solid.
fn finish(features: &[Feature], ids: &[u64], seq: &[Solid]) -> Verdict {
    if seq.is_empty() {
        return Verdict::Ok;
    }
    let q = Quality {
        chord: CHORD,
        ..Quality::default()
    };
    let measure = |b: &Body, path: &str| -> Result<(f64, f64), Verdict> {
        let b = b.compact();
        match catch_unwind(AssertUnwindSafe(|| tessellate(&b, &q))) {
            Err(p) => Err(Verdict::Fail(format!("{path} mesh: panic {}", panic_text(p)))),
            Ok(Err(e)) => Err(Verdict::Fail(format!(
                "{path} mesh: {}",
                kind_of_msg(&format!("{e:?}"))
            ))),
            Ok(Ok(t)) if !t.open_edges().is_empty() => Err(Verdict::Fail(format!("{path} mesh: open edges"))),
            Ok(Ok(t)) if t.volume() <= 0.0 => Err(Verdict::Fail(format!("{path} mesh: volume <= 0"))),
            Ok(Ok(t)) => Ok((t.volume(), t.area())),
        }
    };
    // every body, on both regens
    let sum = |bs: Vec<&Body>, path: &str| -> Result<(f64, f64), Verdict> {
        let mut t = (0.0, 0.0);
        for b in bs {
            let (v, a) = measure(b, path)?;
            t = (t.0 + v, t.1 + a);
        }
        Ok(t)
    };
    let (v, a) = match sum(seq.iter().map(|s| &s.body).collect(), "seq") {
        Ok(v) => v,
        Err(f) => return f,
    };
    let part = Part {
        features: features.to_vec(),
    };
    let nary = match catch_unwind(AssertUnwindSafe(|| part.regenerate_nary_solids_ids(1, Some(ids)))) {
        Err(p) => return Verdict::Fail(format!("nary: panic {}", panic_text(p))),
        Ok(Err(e)) => return Verdict::Fail(format!("nary: {}", kind_of_msg(&format!("{e:?}")))),
        Ok(Ok(b)) => b,
    };
    if nary.len() != seq.len() {
        return Verdict::Fail(format!("nary: {} bodies, sequential {}", nary.len(), seq.len()));
    }
    for s in &nary {
        if let f @ Verdict::Fail(_) = valid("nary", &s.body) {
            return f;
        }
    }
    match sum(nary.iter().map(|s| &s.body).collect(), "nary") {
        Err(f) => f,
        // both meshes sag inside curved faces by under a chord: they differ by less than that
        Ok((vn, _)) if (vn - v).abs() > CHORD * a + 1e-9 * v.abs() => {
            Verdict::Fail("nary: volume differs".into())
        }
        Ok(_) => Verdict::Ok,
    }
}

/// A program as the kernel saw it: its features and their ids, the solids after each feature
/// built, and the first error (with the statement it came from).
pub struct Built {
    pub run: crate::Run,
    pub features: Vec<Feature>,
    pub ids: Vec<u64>,
    /// The solids after each feature built (one stage per feature, up to a refusal): the
    /// cache's own stages, shared, not copied (a part of many features and faces would hold
    /// every stage's bodies twice over).
    pub stages: Vec<std::rc::Rc<crate::build::Stage>>,
}

impl Built {
    /// How many features built.
    pub fn built(&self) -> usize {
        self.stages.len()
    }
    /// The solids after `k` features (none before the first).
    pub fn solids_after(&self, k: usize) -> &[Solid] {
        k.checked_sub(1)
            .and_then(|i| self.stages.get(i))
            .map_or(&[], |s| &s.solids[..])
    }
    /// The bodies after `k` features as one (each finished, as selections see them).
    pub fn body_after(&self, k: usize) -> Option<Body> {
        let solids = self.solids_after(k);
        match solids {
            [] => None,
            [one] => Some(forge_model::finish(one.body.clone())),
            many => {
                let mut all = Body::new();
                for s in many {
                    all.append(&forge_model::finish(s.body.clone()));
                }
                Some(all)
            }
        }
    }
    /// The part as it stands after the last feature built.
    pub fn body(&self) -> Option<Body> {
        self.body_after(self.built())
    }
    /// The first error.
    pub fn error(&self) -> Option<&Error> {
        self.run.errors.first()
    }
    /// The feature the kernel refused, if it refused one.
    pub fn refused(&self) -> Option<&Feature> {
        match self.run.errors.first() {
            Some(Error::Build { .. }) => self.features.get(self.stages.len()),
            _ => None,
        }
    }
    /// The 0-based source line an error is on.
    pub fn line_of(&self, src: &str, e: &Error) -> usize {
        e.span()
            .map_or(0, |s| src[..(s.lo as usize).min(src.len())].matches('\n').count())
    }
    /// The kernel features as a part (with a refused one last, when there is one).
    pub fn part(&self) -> Part {
        Part {
            features: self.features.clone(),
        }
    }
}

/// Builds `src` as the generated programs are built (`options`).
pub fn build(src: &str) -> Built {
    build_with(src, options())
}

/// Builds `src` with `opts`, keeping every stage.
pub fn build_with(src: &str, opts: Options) -> Built {
    build_cached(src, opts, Cache::default()).0
}

/// Builds `src` with `opts` from the stages `cache` keeps (a longer source rebuilds only its
/// tail), keeping every stage; and the cache for the next.
pub fn build_cached(src: &str, opts: Options, cache: Cache) -> (Built, Cache) {
    let (run, cache) = crate::run(src, opts, cache);
    let (f, ids) = cache.history();
    let b = Built {
        features: f.to_vec(),
        ids: ids.to_vec(),
        stages: cache.stage_rcs().to_vec(),
        run,
    };
    (b, cache)
}

/// The kind of statement at line `line` of `src`: the call it makes (`extrude`, `fillet`, ..).
fn stmt_kind(src: &str, line: usize) -> String {
    let lines: Vec<&str> = src.lines().collect();
    // the statement a line is in: back to its top-level start
    let mut i = line.min(lines.len().saturating_sub(1));
    while i > 0 && (lines[i].starts_with(' ') || lines[i].starts_with('}')) {
        i -= 1;
    }
    let l = lines.get(i).copied().unwrap_or("");
    if l.starts_with("sketch ") {
        return "sketch".into();
    }
    if l.starts_with("shared ") {
        return "param".into();
    }
    let rhs = l.split_once(" = ").map_or(l, |x| x.1);
    rhs.split('(').next().unwrap_or("").trim().to_string()
}

/// Builds a program, checking every stage, then the finished part.
pub fn judge(src: &str) -> Verdict {
    judge_built(src).0
}

/// `judge`, and what it built (none if it panicked).
pub fn judge_built(src: &str) -> (Verdict, Option<Built>) {
    let run = catch_unwind(AssertUnwindSafe(|| {
        let b = build(src);
        let n = b.built();
        for i in 0..n {
            let kind = b.features.get(i).map_or("?", feature_kind);
            // a pinch on the way is the part's business if a later feature clears it: the
            // finished part is held to the full check
            let finished = i + 1 == n;
            if !finished && b.stages[i].solids.iter().all(|s| only_pinched(&s.body)) {
                continue;
            }
            if let f @ Verdict::Fail(_) = valid_solids(kind, &b.stages[i].solids) {
                return (f, b);
            }
        }
        if let Some(e) = b.run.errors.first() {
            let kind = stmt_kind(src, b.line_of(src, e));
            let v = classify(&kind, e);
            return (v, b);
        }
        let v = finish(&b.features, &b.ids, b.solids_after(n));
        (v, b)
    }));
    match run {
        Ok((v, b)) => (v, Some(b)),
        Err(p) => (Verdict::Fail(format!("panic: {}", panic_text(p))), None),
    }
}

/// A kernel feature's kind, for verdicts.
pub fn feature_kind(f: &Feature) -> &'static str {
    match f {
        Feature::Extrude { .. } => "extrude",
        Feature::Revolve { .. } => "revolve",
        Feature::Ball { .. } => "ball",
        Feature::Fillet { .. } => "fillet",
        Feature::FaceFillet { .. } => "facefillet",
        Feature::FullRound { .. } => "fullround",
        Feature::Chamfer { .. } => "chamfer",
        Feature::Shell { .. } => "shell",
        Feature::Draft { .. } => "draft",
        Feature::Hole(_) => "hole",
        Feature::Pattern { .. } => "pattern",
        Feature::Sweep(_) => "sweep",
        Feature::Coil(_) => "coil",
        Feature::Loft(_) => "loft",
        Feature::Plane(_) => "workplane",
        Feature::Param(_) => "param",
        Feature::Sketch(_) => "sketch",
        Feature::Work(_) => "work",
        Feature::Edit(_) => "edit",
        Feature::Thread(_) => "thread",
        Feature::Text(_) => "text",
        Feature::Import(_) => "import",
        Feature::Body(_) => "body",
    }
}

/// The asks that say which faces each feature made.
fn lineage_asks(labels: &[Label]) -> Vec<String> {
    labels.iter().map(|l| format!("faces(of: {})", l.name)).collect()
}

/// Which labels made each face, from the run's answers to `lineage_asks`.
fn lineage(asked: &[(String, crate::anchors::Asked)]) -> HashMap<Name, Vec<usize>> {
    let mut made: HashMap<Name, Vec<usize>> = HashMap::new();
    for (i, (_, a)) in asked.iter().enumerate() {
        if let crate::anchors::Asked::Faces(names, _) = a {
            for nm in names {
                made.entry(*nm).or_default().push(i);
            }
        }
    }
    made
}

/// Grows a program of up to `len` statement groups from `seed`. A group the kernel refuses, or
/// fails on, is dropped and the program goes on from the part before it.
pub fn run(seed: u64, len: usize) -> Run {
    run_kinds(seed, len, u64::MAX)
}

/// `run` drawing only the kinds whose bit in `kinds` is set (of MAKERS, in order; empty counts as
/// all). A history with few kinds piles them on each other, which all kinds at once rarely do
/// (swarm testing: Groce et al., ISSTA 2012).
pub fn run_kinds(seed: u64, len: usize, kinds: u64) -> Run {
    let kinds = if kinds == 0 { u64::MAX } else { kinds };
    let makers: Vec<&Maker> = MAKERS
        .iter()
        .enumerate()
        .filter(|(i, _)| kinds >> (i % 64) & 1 == 1)
        .map(|(_, m)| m)
        .collect();
    let mut r = Rng::new(seed.wrapping_mul(0x2545_f491_4f6c_dd1d).wrapping_add(0x7e57));
    let mut src = base(&mut r) + "\n";
    let mut st = State::default();
    st.labels.push(Label {
        name: "L0".into(),
        kind: "extrude",
        solid: true,
        body: true,
        thread_hole: false,
    });
    let mut tried = Vec::new();
    let mut fails = Vec::new();
    let mut refused = Vec::new();
    let mut slowest = (0.0, String::new());
    let mut label = 0;
    let total: usize = makers.iter().map(|k| k.weight).sum();
    let mut built = 0;
    // the part as it stands: its body, and the cache its stages are kept in
    let opts = |labels: &[Label]| Options {
        asks: lineage_asks(labels),
        ..options()
    };
    let (first, mut cache) = match catch_unwind(AssertUnwindSafe(|| {
        crate::run(&src, opts(&st.labels), Cache::default())
    })) {
        Ok(x) => x,
        Err(p) => {
            fails.push((format!("base: panic: {}", panic_text(p)), src.clone()));
            return Run {
                name_fallbacks: 0,
                src,
                tried,
                fails,
                refused,
                slowest,
            };
        }
    };
    if let Some(e) = first.errors.first() {
        fails.push((format!("base: {}", kind_of_msg(&message(e))), src.clone()));
    }
    st.made = lineage(&first.asked);
    let mut body = first.body;
    for _ in 0..len * 3 {
        if built >= len {
            break;
        }
        let Some(b) = &body else { break };
        // nothing left to place features on
        let d = bbox(b).diagonal();
        if d.is_nan() || d <= 1e-6 {
            break;
        }
        let mut pick = r.below(total);
        let maker = makers
            .iter()
            .find(|k| {
                let hit = pick < k.weight;
                pick = pick.saturating_sub(k.weight);
                hit
            })
            .expect("weights cover the draw");
        let (lines, new_labels, new_params, helpers) = {
            let mut g = G {
                r: &mut r,
                st: &st,
                bx: bbox(b),
                body: b,
                label: &mut label,
                helpers: 0,
                points: BTreeMap::new(),
                new_labels: vec![],
                new_params: vec![],
            };
            let lines = (maker.make)(&mut g).map(|ls| g.emit(ls));
            (lines, g.new_labels, g.new_params, g.helpers)
        };
        let Some(lines) = lines else { continue };
        let cand = format!("{src}{}\n", lines.join("\n"));
        // HISTORY_LAST=F: each program tried is written to F first, so one that never ends is there
        if let Some(f) = std::env::var_os("HISTORY_LAST") {
            let _ = std::fs::write(f, &cand);
        }
        let mut labels = st.labels.clone();
        labels.extend(new_labels.iter().cloned());
        let before = cache.stages().count();
        let t = std::time::Instant::now();
        let taken = std::mem::take(&mut cache);
        let (verdict, out) = match catch_unwind(AssertUnwindSafe(|| crate::run(&cand, opts(&labels), taken)))
        {
            Err(p) => (
                Verdict::Fail(format!("{}: panic: {}", maker.name, panic_text(p))),
                None,
            ),
            Ok((run, c)) => {
                let v = match run.errors.first() {
                    Some(e) => classify(maker.name, e),
                    None => match c.stages().last() {
                        Some(s) if c.stages().count() > before || before == 0 => valid_solids(maker.name, s),
                        _ => Verdict::Ok,
                    },
                };
                cache = c;
                (v, Some(run))
            }
        };
        let secs = t.elapsed().as_secs_f64();
        if secs > slowest.0 {
            slowest = (secs, cand.clone());
        }
        match verdict {
            Verdict::Ok => {
                tried.push((maker.name, None));
                src = cand;
                built += 1;
                st.labels = labels;
                st.params.extend(new_params);
                st.helpers += helpers;
                let run = out.expect("it ran");
                st.made = lineage(&run.asked);
                body = run.body;
            }
            Verdict::Refused(why) => {
                tried.push((maker.name, Some(why.clone())));
                refused.push((why, cand));
            }
            Verdict::Fail(why) => {
                tried.push((maker.name, Some(why.clone())));
                fails.push((why, cand));
            }
        }
        // a refused or failed label number is not used again either way (the next is new)
    }
    let before = forge_model::nary_name_fallbacks();
    let last = catch_unwind(AssertUnwindSafe(|| {
        crate::run(&src, options(), std::mem::take(&mut cache))
    }));
    match last {
        Ok((_, c)) => {
            let (f, ids) = c.history();
            let seq: Vec<Solid> = c.stages().last().map_or(vec![], |s| s.to_vec());
            if let Verdict::Fail(what) = finish(f, ids, &seq) {
                fails.push((what, src.clone()));
            }
        }
        Err(p) => fails.push((format!("panic: {}", panic_text(p)), src.clone())),
    }
    let name_fallbacks = forge_model::nary_name_fallbacks() - before;
    Run {
        name_fallbacks,
        src,
        tried,
        fails,
        refused,
        slowest,
    }
}

/// A program's top-level statements: each a line, or a block's lines to its closing brace.
pub fn statements(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut depth = 0i32;
    for l in src.lines() {
        if depth > 0 {
            let last = out.last_mut().expect("a block open");
            last.push('\n');
            last.push_str(l);
        } else if !l.trim().is_empty() {
            out.push(l.to_string());
        }
        depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
    }
    out
}

/// The fewest statements of `src` that still fail as `want` does.
pub fn shrink(src: &str, want: &Verdict) -> String {
    let mut stmts = statements(src);
    let same = |ss: &[String]| judge(&(ss.join("\n") + "\n")) == *want;
    // chunks from big to single statements, from the end (the cheapest to rebuild)
    let mut chunk = stmts.len() / 2;
    while chunk >= 1 {
        let mut end = stmts.len();
        while end > 1 {
            let start = end.saturating_sub(chunk).max(1);
            let mut t = stmts.clone();
            t.drain(start..end);
            if same(&t) {
                stmts = t;
                end = start.min(stmts.len());
            } else {
                end = start;
            }
        }
        chunk /= 2;
    }
    // the first too, when another can start the part
    if stmts.len() > 1 && same(&stmts[1..]) {
        stmts.remove(0);
    }
    // then each statement's optional arguments and summed selections, one at a time
    for i in 0..stmts.len() {
        let mut k = 0;
        loop {
            let cuts = optional(&stmts[i]);
            if k >= cuts.len() {
                break;
            }
            let (a, b) = cuts[k];
            let mut t = stmts.clone();
            t[i] = format!("{}{}", &stmts[i][..a], &stmts[i][b..]);
            if same(&t) {
                stmts = t;
            } else {
                k += 1;
            }
        }
    }
    stmts.join("\n") + "\n"
}

/// Spans of a statement that may go: `, key: value` arguments (at the call's own depth) and the
/// later terms of a `A + B` selection.
fn optional(s: &str) -> Vec<(usize, usize)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    // ", key: value" at depth 1 (inside the statement's outer call)
    let mut depth = 0i32;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b',' if depth == 1 => {
                // to the next comma or closing bracket at this depth
                let mut j = i + 1;
                let mut d = depth;
                while j < b.len() {
                    match b[j] {
                        b'(' | b'[' => d += 1,
                        b')' | b']' if d == depth => break,
                        b')' | b']' => d -= 1,
                        b',' if d == depth => break,
                        _ => {}
                    }
                    j += 1;
                }
                let arg = &s[i + 1..j];
                if arg.trim_start().split(':').next().is_some_and(|k| {
                    let k = k.trim();
                    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                }) && arg.contains(':')
                {
                    out.push((i, j));
                }
            }
            b'+' if i > 0 && b[i - 1] == b' ' => {
                // " + term" up to the next comma, close or plus at this depth
                let mut j = i + 1;
                let mut d = depth;
                while j < b.len() {
                    match b[j] {
                        b'(' | b'[' => d += 1,
                        b')' | b']' if d == depth => break,
                        b')' | b']' => d -= 1,
                        b',' | b'+' if d == depth => break,
                        _ => {}
                    }
                    j += 1;
                }
                let end = if j < b.len() && b[j] == b'+' { j - 1 } else { j };
                out.push((i - 1, end));
            }
            _ => {}
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements_keep_blocks_whole() {
        let src = "sketch S0(on: xy) {\n  o = circle((0, 0), 5)\n}\nL0 = extrude(S0, 3)\nshared a = 2\n";
        let s = statements(src);
        assert_eq!(s.len(), 3);
        assert!(s[0].ends_with('}'));
    }

    #[test]
    fn optional_arguments_and_terms() {
        let s = "L3 = fillet(edges(of: L1, extreme: X) + edges(of: L2), 2, from: H0.p1)";
        let cuts: Vec<&str> = optional(s).into_iter().map(|(a, b)| &s[a..b]).collect();
        assert_eq!(cuts, vec![" + edges(of: L2)", ", from: H0.p1"]);
    }

    #[test]
    fn the_font_parses() {
        assert!(forge_font::Font::parse(font_bytes()).is_ok());
    }
}
