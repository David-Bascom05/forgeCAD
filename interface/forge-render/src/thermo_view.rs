//! A thermoform made into what the app draws: its row (numbers, the section, the faces' drafts,
//! the sheet's points for probing), the wall-thickness map at each step of the forming, and the
//! draft analysis's triangles, all lifted off the mould by the sheet's thickness so they lie just
//! over the formed part's face.
use crate::mesh::{Lod, PartMesh, SEGMENT, VERTEX};
use crate::origin::WorldVert;
use crate::shell::thermo::{BANDS, DRAFT, Masses, ThermoRow, band};
use crate::shell::thermo_ui::draft_class;
use forge_lang::thermo::ThermoOut;
use forge_math::{Point2, Point3, Vec3};

/// What the overlay draws of a thermoform.
#[derive(Clone, Debug, Default)]
pub struct Gfx {
    /// The thickness map at each step.
    pub steps: Vec<Vec<WorldVert>>,
    /// The mould's surface's triangles, lifted, and each one's draft (degrees).
    pub draft: Vec<([[f32; 3]; 3], f32)>,
    /// For export: the formed part, the mould, the trim line (world), and the process sheet's lines.
    pub part: forge_topo::Body,
    pub mould: forge_topo::Body,
    pub trim: Vec<[f64; 3]>,
    pub sheet: Vec<String>,
    /// The vents drilled in the mould, at its surface.
    pub vents: Vec<[f32; 3]>,
    /// The sheet's points on the part and the wall there, mm.
    pub points: Vec<([f32; 3], f32)>,
    /// The simulated part (a mesh fitted from the model) and the blank, as the view draws them,
    /// and the simulated part's wall map, just off its face.
    /// (Made when first shown, off the window's thread: `want_sim`.)
    pub sim: Option<Lod>,
    pub has_sim: bool,
    pub sim_mesh: Option<forge_mesh::TriMesh>,
    pub sim_modelled: Option<std::sync::Arc<Vec<Point3>>>,
    pub sim_feet: Option<Vec<Point3>>,
    pub sim_heat: Vec<WorldVert>,
    pub blank: Lod,
    /// The formed part's and the mould's faces and edges in the part's body.
    pub part_span: Option<Span>,
    pub mould_span: Option<Span>,
    /// The model's wall about the part, and the formed part's own faces as the view meshes them,
    /// each point coloured by the wall there (`paint`), for the mesh it was painted on; and where
    /// MIN and MAX go on them, and on the simulated part.
    pub field: Option<std::sync::Arc<forge_thermo::WallField>>,
    pub tool: Option<std::sync::Arc<forge_thermo::ToolSide>>,
    pub heat: Vec<WorldVert>,
    pub heat_for: usize,
    pub extremes: Option<([f32; 3], [f32; 3])>,
    pub sim_extremes: Option<([f32; 3], [f32; 3])>,
    /// The wall's range the colours span, mm, and where the model's thinnest and thickest are.
    pub range: (f64, f64),
    pub at: (Point3, Point3),
    /// The sheet's thickness, mm.
    pub t: f64,
    /// The principal stretch over the sheet on the part, banded (most stretch red), and its range;
    /// the gathered triangles and the folds, marked.
    pub stretch: Vec<WorldVert>,
    pub stretch_range: (f64, f64),
    pub web: Vec<WorldVert>,
    /// What a distortion print is made from: the forming, its trim plane and the trim's box.
    pub print_src: Option<std::sync::Arc<PrintSrc>>,
    /// What the workers have made and are making.
    pub made: std::sync::Arc<std::sync::Mutex<Made>>,
}

/// A forming, its trim plane and its trim's box: what a distortion print is made from.
pub type PrintSrc = (forge_thermo::Stretch, forge_math::Frame, (Point2, Point2));

/// Where MIN and MAX go.
type Pins = ([f32; 3], [f32; 3]);

/// The simulated part's drawing and the colours, as the workers finish them.
#[derive(Debug, Default)]
pub struct Made {
    sim: Option<(Lod, Vec<WorldVert>)>,
    sim_asked: bool,
    heat: Option<(usize, Vec<WorldVert>, Option<Pins>)>,
    heat_asked: Option<usize>,
}

/// A solid's faces and edges in the part's body.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub faces: std::ops::Range<u32>,
    pub edges: std::ops::Range<u32>,
}

/// A legend colour (sRGB, as the legend paints it) for the view's linear target: the same colour
/// on the screen, full strength, unlit.
pub fn unlit(c: [f32; 3]) -> [f32; 4] {
    let lin = |x: f32| {
        if x <= 0.04045 {
            x / 12.92
        } else {
            forge_math::dmath::powf(((x + 0.055) / 1.055) as f64, 2.4) as f32
        }
    };
    [lin(c[0]), lin(c[1]), lin(c[2]), 1.0]
}

/// How far a wall map lies off the face it's painted on, mm: clear of it in depth, no gap seen.
const LIFT: f64 = 0.08;

/// Faces numbered from here are no body's: a pick passes them by.
pub const UNPICKED: u32 = 0x7000_0000;

pub fn unpicked(h: &crate::render::PickHit) -> bool {
    matches!(h.entity, crate::render::Entity::Face(f) if f >= UNPICKED)
}

impl Gfx {
    /// The draft analysis's triangles at least draft `min`.
    pub fn draft_tris(&self, min: f32) -> Vec<WorldVert> {
        let mut out = Vec::with_capacity(self.draft.len() * 3);
        for (tri, a) in &self.draft {
            let c = DRAFT[draft_class(*a, min)];
            for p in tri {
                out.push(WorldVert {
                    at: *p,
                    colour: unlit(c),
                });
            }
        }
        out
    }
}

fn f3(p: Point3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

fn axis_word(d: Vec3) -> String {
    for (v, w) in [(Vec3::X, "X"), (Vec3::Y, "Y"), (Vec3::Z, "Z")] {
        let c = d.dot(v);
        if c.abs() > 1.0 - 1e-9 {
            return format!("{}{w}", if c > 0.0 { "+" } else { "-" });
        }
    }
    format!("({:.2}, {:.2}, {:.2})", d.x, d.y, d.z)
}

/// The row and the overlay of a thermoform; `limit` the material's least wall (a fraction of t)
/// and `draft_min` its least draft for this mould, degrees.
pub fn view(o: &ThermoOut, spans: &[forge_lang::build::Span], draft_min: f32) -> (ThermoRow, Gfx) {
    let limit = o.min_wall;
    let f = &o.formed;
    let r = &f.report;
    let male = o.tool == forge_thermo::Tool::Male;
    let d = f.frame.z().get();
    let lift = (o.t * 1.05 + 0.1) as f32;
    // the mould's surface's triangles: each one's draft, and lifted off the mould
    let mut gfx = Gfx {
        part: f.part.clone(),
        mould: f.mould.clone(),
        trim: forge_thermo::build::polygon(&f.trim)
            .iter()
            .map(|q| {
                let w = f.frame.point_to_world(Point3::new(q.x, q.y, 0.0));
                [w.x, w.y, w.z]
            })
            .collect(),
        // shown through the formed part: lifted off the mould past the sheet
        vents: r.vents.iter().map(|v| f3(*v + d * (o.t * 1.2 + 0.3))).collect(),
        blank: crate::tess::lod_of_mesh(&o.blank(), UNPICKED + 8),
        has_sim: o.sim.is_some(),
        sim_mesh: o.sim.as_ref().map(|s| s.mesh.clone()),
        sim_modelled: o.sim.as_ref().map(|s| std::sync::Arc::new(s.modelled.clone())),
        sim_feet: o.sim.as_ref().map(|s| s.foot.clone()),
        part_span: span_of(spans, o.part_solid),
        mould_span: span_of(spans, o.mould_solid),
        field: o.field.as_ref().map(|f| std::sync::Arc::new((**f).clone())),
        tool: Some(std::sync::Arc::new(forge_thermo::ToolSide::new(
            &f.frame,
            &forge_thermo::OnTool {
                mesh: &f.mesh,
                contact: &f.contact,
            },
            0.1 * o.t,
        ))),
        ..Gfx::default()
    };
    let kg = o.density.map(|d| d * 1e-9);
    let masses = |volume: f64, area: f64, c: Point3, i: [[f64; 3]; 3]| Masses {
        volume,
        area,
        mass: kg.map(|k| k * volume),
        cog: [c.x, c.y, c.z],
        inertia: kg.map(|k| i.map(|r| r.map(|x| x * k))),
    };
    let mut face_min: Vec<f32> = vec![f32::INFINITY; f.open.len()];
    for &k in &f.contact {
        let [a, b, c] = f.mesh.triangles[k].map(|i| f.mesh.positions[i as usize]);
        let Some(n) = (b - a).cross(c - a).normalize() else {
            continue;
        };
        let n = if male { n.get() } else { -n.get() };
        let deg = forge_math::dmath::asin(n.dot(d).clamp(-1.0, 1.0)).to_degrees() as f32;
        let up = n * lift as f64;
        gfx.draft.push(([a + up, b + up, c + up].map(f3), deg));
        let face = f.mesh.face_of[k] as usize;
        face_min[face] = face_min[face].min(deg);
    }
    let face_drafts: Vec<f32> = face_min.into_iter().filter(|a| a.is_finite()).collect();
    // the section through the middle along the trim plane's x
    let fd = &f.field;
    let j = fd.ny / 2;
    let mut section = vec![];
    for i in 0..fd.nx {
        let z = fd.z[j * fd.nx + i];
        if !z.is_nan() {
            let c = fd.centre(i, j);
            section.push([c.x as f32, z as f32]);
        }
    }
    let side = if male { 1.0 } else { -1.0 };
    let section_part = section
        .iter()
        .map(|q| [q[0], q[1] + side * o.t as f32 * 0.5])
        .collect();
    let along = |p: Point3| f.frame.point_to_local(p).z;
    let base = f
        .mould
        .vertex_ids()
        .map(|v| along(f.mould.vertex(v).point))
        .fold(0.0f64, f64::min)
        .abs() as f32;
    let (lo, hi) = r.trim_box;
    let mut facts = vec![
        (
            "material".to_string(),
            format!("{} at {:.0} °C", o.material, o.temperature),
        ),
        ("sheet".into(), format!("{:.2} mm", o.t)),
        ("depth".into(), format!("{:.1} mm", r.depth)),
        ("areal draw ratio".into(), format!("{:.2}", r.adr)),
        ("mean wall".into(), format!("{:.2} mm", r.mean_wall)),
        (
            "vents".into(),
            format!("{} × Ø{:.1}", r.vents.len(), 0.0f64.max(o.t * 0.4).min(1.0)),
        ),
        (
            "mould scale".into(),
            format!("{:.4} (+{:.2} %)", r.scale, (r.scale - 1.0) * 100.0),
        ),
        (
            "sheet used".into(),
            format!(
                "{:.0} × {:.0} × {:.1}",
                hi.x - lo.x + 50.0,
                hi.y - lo.y + 50.0,
                o.t
            ),
        ),
    ];
    let mut row = ThermoRow {
        id: o.id.clone(),
        material: o.material.clone(),
        female: !male,
        from: match o.from {
            forge_thermo::Given::Outer => "outer",
            forge_thermo::Given::Wall => "wall",
            forge_thermo::Given::Inner => "inner",
        }
        .into(),
        pull: axis_word(d),
        t: o.t,
        shrink: r.scale - 1.0,
        draft_min,
        face_drafts,
        section,
        section_part,
        base,
        limit: limit * o.t,
        modelled: o
            .modelled
            .as_ref()
            .map(|m| masses(m.volume, m.area, m.centroid, m.inertia)),
        simulated: o
            .sim
            .as_ref()
            .map(|s| masses(s.mass.volume, s.mesh.area(), s.mass.centroid, s.mass.inertia)),
        density: o.density,
        passes: o.passes(),
        unformed: o.walls.as_ref().map_or(0, |w| w.unformed.len()),
        blank: o.blank_size(),
        shape: o.shape.clone(),
        ..ThermoRow::default()
    };
    // the shrink: the vendor's figure and its source, and free contraction's band
    if let Some(m) = forge_thermo::placeholder().material(&o.material) {
        row.shrink_band = m.shrink_band();
        if let (Some(off), Some(tg)) = (m.demould, m.tg) {
            row.shrink_band_from = format!("free contraction from {off:.0}-{tg:.0} °C to 23 °C");
        }
    }
    row.shrink_source = forge_material::library()
        .iter()
        .find(|r| r.name.eq_ignore_ascii_case(&o.material))
        .map_or(String::new(), |r| r.source_of("thermo.shrink").to_string());
    let Some(w) = &o.walls else {
        row.facts = facts;
        return (row, gfx);
    };
    gfx.print_src = Some(std::sync::Arc::new(((**w).clone(), f.frame, r.trim_box)));
    let poly = forge_thermo::build::polygon(&f.trim);
    let inside = |p: [f32; 3]| {
        let l = f
            .frame
            .point_to_local(Point3::new(p[0] as f64, p[1] as f64, p[2] as f64));
        forge_thermo::build::inside(&poly, Point2::new(l.x, l.y)).0
    };
    // the triangles over the part: inside the trim where the sheet ended
    let last = w.steps.last();
    let shown: Vec<usize> = (0..w.tris.len())
        .filter(|&k| {
            let v = w.tris[k];
            let n = last.map_or(&w.nodes[..0], |_| &w.nodes[..]);
            n.is_empty() || {
                let m = v.map(|i| f3(w.nodes[i as usize]));
                inside([
                    (m[0][0] + m[1][0] + m[2][0]) / 3.0,
                    (m[0][1] + m[1][1] + m[2][1]) / 3.0,
                    (m[0][2] + m[1][2] + m[2][2]) / 3.0,
                ])
            }
        })
        .collect();
    let (wlo, whi) = (w.min.0 * o.t, w.max.0 * o.t);
    gfx.range = (wlo, whi);
    gfx.t = o.t;
    gfx.at = (w.min.1, w.max.1);
    for s in &w.steps {
        let mut n = vec![[0.0f32; 3]; s.nodes.len()];
        for v in &w.tris {
            let [a, b, c] = v.map(|i| s.nodes[i as usize]);
            let (e1, e2) = (
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
            );
            let x = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            for &i in v {
                let q = &mut n[i as usize];
                q[0] += x[0];
                q[1] += x[1];
                q[2] += x[2];
            }
        }
        let mut tris = Vec::with_capacity(shown.len() * 3);
        for &k in &shown {
            let v = w.tris[k];
            for &i in &v {
                // each point its band, blended across the triangle
                let h = s.ratio[i as usize] as f64 * o.t;
                let c = band(if whi > wlo {
                    ((h - wlo) / (whi - wlo)) as f32
                } else {
                    0.5
                });
                let q = n[i as usize];
                let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt().max(1e-12);
                let p = s.nodes[i as usize];
                tris.push(WorldVert {
                    at: [
                        p[0] + q[0] / l * lift,
                        p[1] + q[1] / l * lift,
                        p[2] + q[2] / l * lift,
                    ],
                    colour: unlit(c),
                });
            }
        }
        gfx.steps.push(tris);
    }
    // the sheet's points on the part, for probing
    if let Some(s) = last {
        row.points = (0..s.nodes.len())
            .filter(|&k| w.touched[k] && inside(s.nodes[k]))
            .map(|k| (s.nodes[k], s.ratio[k] * o.t as f32))
            .collect();
    }
    gfx.points = row.points.clone();
    // MIN and MAX on the simulated part: its outside's points whose sheet points are the model's
    // thinnest and thickest (the modelled part's are found the same way when it is painted)
    if let Some(sim) = &o.sim {
        let m = &sim.mesh;
        let outside = |i: usize| (m.positions[i] - sim.foot[i]).length() > 0.25 * o.t;
        let at = |target: Point3| {
            (0..m.positions.len())
                .filter(|&i| outside(i))
                .min_by(|a, b| {
                    (sim.sheet[*a] - target)
                        .length_sq()
                        .total_cmp(&(sim.sheet[*b] - target).length_sq())
                })
                .map(|i| f3(m.positions[i]))
        };
        if let (Some(a), Some(b)) = (at(w.min.1), at(w.max.1)) {
            gfx.sim_extremes = Some((a, b));
        }
    }
    let below = row.points.iter().filter(|p| (p.1 as f64) < limit * o.t).count();
    facts.push((
        "below limit".into(),
        format!(
            "{:.1} % of points",
            100.0 * below as f64 / row.points.len().max(1) as f64
        ),
    ));
    facts.push(("unformed".into(), format!("{}", w.unformed.len())));
    row.facts = facts;
    // the surface: each triangle over the part coloured by its larger principal stretch, the
    // gathered ones and the folds marked, lifted as the thickness map is
    let sv = forge_thermo::surface::stretches(w, &f.frame);
    let on = |k: &usize| w.tris[*k].iter().all(|&i| w.touched[i as usize]);
    let top = shown
        .iter()
        .filter(|k| on(k))
        .map(|&k| sv.l1[k])
        .fold(1.0f64, f64::max);
    let area = shown
        .iter()
        .filter(|k| on(k))
        .map(|&k| sv.l1[k] * sv.l2[k])
        .fold(1.0f64, f64::max);
    let uneven = shown
        .iter()
        .filter(|k| on(k))
        .map(|&k| sv.l1[k] / sv.l2[k].max(1e-9))
        .fold(1.0f64, f64::max);
    row.stretch = Some([top, area, uneven]);
    gfx.stretch_range = (1.0, top);
    let web = forge_thermo::surface::webbing(w, &f.frame, o.t, 0.05);
    row.web = Some((web.squeezed_area, web.folds.len()));
    if let Some(s) = last {
        let mut n = vec![Vec3::ZERO; s.nodes.len()];
        let at = |i: u32| {
            let p = s.nodes[i as usize];
            Vec3::new(p[0] as f64, p[1] as f64, p[2] as f64)
        };
        for v in &w.tris {
            let [a, b, c] = v.map(at);
            let x = (b - a).cross(c - a);
            v.iter().for_each(|&i| n[i as usize] += x);
        }
        let lifted = |i: u32| {
            let u = n[i as usize].normalize().map_or(Vec3::ZERO, |u| u.get());
            f3(Point3::ORIGIN + at(i) + u * lift as f64)
        };
        let squeezed: std::collections::HashSet<usize> = web.squeezed.iter().copied().collect();
        let warn = unlit([0.95, 0.55, 0.10]);
        for &k in &shown {
            let span = (top - 1.0).max(1e-9);
            let c = unlit(band(1.0 - ((sv.l1[k] - 1.0) / span) as f32));
            for &i in &w.tris[k] {
                gfx.stretch.push(WorldVert {
                    at: lifted(i),
                    colour: c,
                });
            }
            if squeezed.contains(&k) {
                for &i in &w.tris[k] {
                    gfx.web.push(WorldVert {
                        at: lifted(i),
                        colour: warn,
                    });
                }
            }
        }
        // each fold a small three-way cross, red
        let red = unlit([0.86, 0.16, 0.16]);
        let r = (2.0 * o.t).max(1.5);
        for p in &web.folds {
            for (u, v) in [(Vec3::X, Vec3::Y), (Vec3::Y, Vec3::Z), (Vec3::Z, Vec3::X)] {
                let q = |a: f64, b: f64| f3(*p + u * (a * r) + v * (b * r));
                gfx.web.extend(
                    [
                        q(-1.0, -1.0),
                        q(1.0, -1.0),
                        q(1.0, 1.0),
                        q(-1.0, -1.0),
                        q(1.0, 1.0),
                        q(-1.0, 1.0),
                    ]
                    .map(|at| WorldVert { at, colour: red }),
                );
            }
        }
    }
    gfx.sheet = process_sheet(o, &row);
    row.min = (wlo, f3(w.min.1));
    row.max = (whi, f3(w.max.1));
    row.stages = w.steps.iter().map(|s| s.stage).collect();
    row.model = format!(
        "{} · {:.0} °C. Membrane, sticking contact, isothermal: an estimate. {}",
        short_law(&o.law.law),
        o.temperature,
        cite(&o.law.source)
    );
    (row, gfx)
}

/// The process sheet's lines: what the shop needs to form the part.
fn process_sheet(o: &ThermoOut, row: &ThermoRow) -> Vec<String> {
    let r = &o.formed.report;
    let mut out = vec![
        format!("Thermoform process sheet: {}", o.id),
        String::new(),
        format!("Material        {}", o.material),
        format!("Sheet gauge     {:.2} mm", o.t),
        format!("Sheet temp      {:.0} C", o.temperature),
        format!(
            "Mould           {}, pull {}",
            if row.female { "female" } else { "male" },
            row.pull
        ),
        format!(
            "Mould scale     {:.4} (shrink {:.2} %)",
            r.scale,
            (r.scale - 1.0) * 100.0
        ),
        format!("Depth of draw   {:.1} mm", r.depth),
        format!("Areal draw      {:.2}", r.adr),
        format!("Vents           {}", r.vents.len()),
    ];
    for (k, v) in &row.facts {
        if k == "sheet used" {
            out.push(format!("Blank           {v}"));
        }
    }
    if row.max.0 > 0.0 {
        out.push(format!(
            "Wall (model)    {:.2} to {:.2} mm, mean {:.2}",
            row.min.0, row.max.0, r.mean_wall
        ));
        out.push(format!("Wall limit      {:.2} mm", row.limit));
    }
    out.push(String::new());
    out.push("Vent positions (mm):".into());
    out.push(format!("  {:>8} {:>8} {:>8}", "x", "y", "z"));
    // to the hundredth, a -0.004 is 0.00
    let mm = |x: f64| {
        let r = (x * 100.0).round() / 100.0;
        if r == 0.0 { 0.0 } else { r }
    };
    for v in r.vents.iter().take(40) {
        out.push(format!("  {:8.2} {:8.2} {:8.2}", mm(v.x), mm(v.y), mm(v.z)));
    }
    if !row.model.is_empty() {
        out.push(String::new());
        out.push(row.model.clone());
    }
    out
}

/// The process sheet as a one-page PDF in Courier, its text in WinAnsi (so `×` and `°` print).
pub fn process_pdf(lines: &[String]) -> Vec<u8> {
    let mut content = String::from("BT /F1 10 Tf 50 800 Td 13 TL\n");
    for l in lines.iter().take(58) {
        content += &format!("{} '\n", forge_draw::pdf::pdf_str(l));
    }
    content += "ET";
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut at = vec![];
    for (k, o) in objs.iter().enumerate() {
        at.push(out.len());
        out += &format!("{} 0 obj\n{o}\nendobj\n", k + 1);
    }
    let xref = out.len();
    out += &format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1);
    for a in at {
        out += &format!("{a:010} 00000 n \n");
    }
    out += &format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objs.len() + 1
    );
    out.into_bytes()
}

/// A source as the legend cites it: its authors and year, then its table.
fn cite(s: &str) -> String {
    let who = s.split(',').next().unwrap_or(s).trim();
    let year = s
        .as_bytes()
        .windows(4)
        .find(|w| w.iter().all(u8::is_ascii_digit) && (w[0] == b'1' || w[0] == b'2'))
        .map(|w| String::from_utf8_lossy(w).to_string())
        .unwrap_or_default();
    let tail = s
        .rsplit("; ")
        .next()
        .filter(|t| t.len() < 60 && *t != s)
        .unwrap_or("");
    format!("{who} {year}{}{tail}", if tail.is_empty() { "" } else { ", " })
        .trim()
        .to_string()
}

fn short_law(l: &forge_thermo::Law) -> String {
    match l.terms.len() {
        1 if l.terms[0].1 == 2.0 => format!("neo-Hookean G {:.2} MPa", l.terms[0].0),
        n => format!("Ogden {n}-term"),
    }
}

fn span_of(spans: &[forge_lang::build::Span], solid: usize) -> Option<Span> {
    spans.iter().find(|s| s.solid == solid).map(|s| Span {
        faces: s.faces.clone(),
        edges: s.edges.clone(),
    })
}

fn f32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

/// The part's mesh with some of its solids' faces and edges left out, and meshes no body has
/// (the simulated part, the blank) put in: what the thermoform's eyes and state ask for. The faces
/// kept keep their numbers, so a pick still finds the body's.
pub fn compose(full: &PartMesh, hide: &[&Span], extra: &[&Lod]) -> PartMesh {
    let gone_face = |f: u32| hide.iter().any(|s| s.faces.contains(&f));
    let gone_edge = |e: u32| hide.iter().any(|s| s.edges.contains(&e));
    let lods = full
        .lods
        .iter()
        .map(|l| {
            let mut out = Lod {
                chord: l.chord,
                vertices: l.vertices.clone(),
                ..Lod::default()
            };
            let mut kept = std::collections::HashSet::new();
            for f in l.faces.iter().filter(|f| !gone_face(f.face)) {
                let first = (out.indices.len() / 12) as u32;
                let (a, b) = (f.first_tri as usize * 12, (f.first_tri + f.tris) as usize * 12);
                for k in (a..b).step_by(4) {
                    let v = f32_at(&l.indices, k) as usize * VERTEX;
                    kept.insert([0, 4, 8].map(|o| f32_at(&l.vertices, v + o)));
                }
                out.indices.extend_from_slice(&l.indices[a..b]);
                out.faces.push(crate::mesh::FaceRange {
                    first_tri: first,
                    ..*f
                });
            }
            for e in l.edges.iter().filter(|e| !gone_edge(e.edge)) {
                let first = (out.segments.len() / SEGMENT) as u32;
                out.segments.extend_from_slice(
                    &l.segments[e.first as usize * SEGMENT..(e.first + e.count) as usize * SEGMENT],
                );
                out.edges.push(crate::mesh::EdgeRange { first, ..*e });
            }
            out.fins = (out.segments.len() / SEGMENT) as u32;
            // a silhouette's line stays where both its ends are on a face kept
            for k in l.fins as usize..l.segments.len() / SEGMENT {
                let s = &l.segments[k * SEGMENT..(k + 1) * SEGMENT];
                let end = |o: usize| [0, 4, 8].map(|j| f32_at(s, o + j));
                if kept.contains(&end(0)) && kept.contains(&end(16)) {
                    out.segments.extend_from_slice(s);
                }
            }
            for x in extra {
                let base = (out.vertices.len() / VERTEX) as u32;
                let first = (out.indices.len() / 12) as u32;
                out.vertices.extend_from_slice(&x.vertices);
                for k in (0..x.indices.len()).step_by(4) {
                    out.indices
                        .extend_from_slice(&(f32_at(&x.indices, k) + base).to_le_bytes());
                }
                for f in &x.faces {
                    out.faces.push(crate::mesh::FaceRange {
                        first_tri: f.first_tri + first,
                        first_vertex: f.first_vertex + base,
                        ..*f
                    });
                }
                out.segments.extend_from_slice(&x.segments);
            }
            out
        })
        .collect();
    PartMesh {
        center: full.center,
        radius: full.radius,
        lods,
    }
}

/// The body under the pointer in the browser, lit in the view.
pub fn marks(u: &crate::shell::thermo_ui::ThermoUi, g: &Gfx) -> Vec<crate::render::Mark> {
    use crate::shell::thermo_ui::Body;
    let faces = |b: Body| -> std::ops::Range<u32> {
        match b {
            Body::Part if !u.shows(Body::Part) => 0..0,
            Body::Part if u.simulated && g.has_sim => UNPICKED..UNPICKED + 3,
            Body::Part => g.part_span.as_ref().map_or(0..0, |s| s.faces.clone()),
            Body::Mould if !u.shows(Body::Mould) => 0..0,
            Body::Mould => g.mould_span.as_ref().map_or(0..0, |s| s.faces.clone()),
            Body::Blank if !u.shows(Body::Blank) => 0..0,
            Body::Blank => UNPICKED + 8..UNPICKED + 14,
        }
    };
    // the body looked at shows alone: only the one under the pointer is lit
    let over = u.hover.filter(|h| Some(*h) != u.picked);
    [(over, false)]
        .into_iter()
        .filter_map(|(b, selected)| b.map(|b| (faces(b), selected)))
        .flat_map(|(r, selected)| {
            r.map(move |f| crate::render::Mark {
                placement: 0,
                entity: crate::render::Entity::Face(f),
                selected,
                second: false,
            })
        })
        .collect()
}

fn vertex_at(b: &[u8], i: usize) -> Point3 {
    let f = |k: usize| {
        f32::from_le_bytes(b[i * VERTEX + k..i * VERTEX + k + 4].try_into().expect("4 bytes")) as f64
    };
    Point3::new(f(0), f(4), f(8))
}

impl Gfx {
    /// Takes in what the workers finished; true when something came (the view is drawn again).
    pub fn take_made(&mut self) -> bool {
        let mut m = self.made.lock().unwrap_or_else(|e| e.into_inner());
        let mut got = false;
        if let Some((lod, heat)) = m.sim.take() {
            self.sim = Some(lod);
            self.sim_heat = heat;
            got = true;
        }
        if let Some((key, heat, ext)) = m.heat.take() {
            self.heat_for = key;
            self.heat = heat;
            if ext.is_some() {
                self.extremes = ext;
            }
            got = true;
        }
        got
    }

    /// The simulated part's drawing and its colours, made on a worker the first time they are
    /// wanted.
    pub fn want_sim(&mut self, waker: &crate::gui::Waker) {
        if !self.has_sim || self.sim.is_some() {
            return;
        }
        let mut m = self.made.lock().unwrap_or_else(|e| e.into_inner());
        if m.sim_asked {
            return;
        }
        m.sim_asked = true;
        let (Some(mesh), Some(modelled)) = (self.sim_mesh.clone(), self.sim_modelled.clone()) else {
            return;
        };
        let (field, range, made, waker) = (self.field.clone(), self.range, self.made.clone(), waker.clone());
        std::thread::spawn(move || {
            let lod = crate::tess::lod_of_mesh(&mesh, UNPICKED);
            // the simulated part's outside, banded as the legend is, by the model's own wall at
            // each point of the modelled part it was moved from: the modelled part's colours
            let mut heat = vec![];
            if let Some(field) = &field {
                let tris: Vec<[(Point3, Point3); 3]> = mesh
                    .triangles
                    .iter()
                    .zip(&mesh.face_of)
                    .filter(|x| *x.1 != forge_thermo::simulated::TOOL_SIDE)
                    .map(|x| x.0.map(|i| (mesh.positions[i as usize], modelled[i as usize])))
                    .collect();
                paint_mesh(&tris, field, range, &mut heat, &mut |_, _| {});
            }
            made.lock().unwrap_or_else(|e| e.into_inner()).sim = Some((lod, heat));
            waker();
        });
    }

    /// The formed part's faces of `mesh` (the view's own, finest level) coloured by the model's
    /// wall at each point (at its foot on the sheet), banded as the legend is; and the points on
    /// the part's outside over the model's thinnest and thickest: on a worker, once per mesh.
    pub fn want_heat(&mut self, mesh: &PartMesh, key: usize, waker: &crate::gui::Waker) {
        if self.heat_for == key {
            return;
        }
        let mut m = self.made.lock().unwrap_or_else(|e| e.into_inner());
        if m.heat_asked == Some(key) {
            return;
        }
        let (Some(field), Some(span), Some(tool)) =
            (self.field.clone(), self.part_span.clone(), self.tool.clone())
        else {
            return;
        };
        // the level the view's mesh has at a twentieth of a millimetre (its finest is a
        // hundredth: far more triangles than a map needs), the map lifted clear of its chord
        let Some(l) = mesh
            .lods
            .iter()
            .find(|l| l.chord >= 0.04)
            .or(mesh.lods.first())
            .cloned()
        else {
            return;
        };
        m.heat_asked = Some(key);
        let (range, at, t, made, waker) = (self.range, self.at, self.t, self.made.clone(), waker.clone());
        std::thread::spawn(move || {
            let (heat, ext) = paint(&l, &span, &field, &tool, range, at, t);
            let mut m = made.lock().unwrap_or_else(|e| e.into_inner());
            // a newer mesh asked meanwhile: this one is not kept
            if m.heat_asked == Some(key) {
                m.heat = Some((key, heat, ext));
            }
            drop(m);
            waker();
        });
    }

    /// Whether a worker is still making something asked for.
    pub fn working(&self) -> bool {
        let m = self.made.lock().unwrap_or_else(|e| e.into_inner());
        (m.sim_asked && self.has_sim && self.sim.is_none())
            || m.heat_asked.is_some_and(|k| k != self.heat_for)
    }

    /// `want_heat` and `want_sim` done here and now (for tests and shots).
    pub fn made_now(&mut self, mesh: Option<&PartMesh>) {
        if let (Some(mesh), Some(field), Some(span), Some(tool)) =
            (mesh, &self.field, &self.part_span, &self.tool)
            && let Some(l) = mesh.lods.iter().find(|l| l.chord >= 0.04).or(mesh.lods.first())
        {
            let (heat, ext) = paint(l, span, field, tool, self.range, self.at, self.t);
            self.heat = heat;
            self.extremes = ext.or(self.extremes);
        }
        if self.has_sim && self.sim.is_none() {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let waker: crate::gui::Waker = std::sync::Arc::new(move || {
                let _ = tx.send(());
            });
            self.want_sim(&waker);
            let _ = rx.recv();
            self.take_made();
        }
    }
}

/// `Gfx::want_heat`'s painting of level `l`.
#[allow(clippy::too_many_arguments)]
fn paint(
    l: &Lod,
    span: &Span,
    field: &forge_thermo::WallField,
    tool: &forge_thermo::ToolSide,
    range: (f64, f64),
    at: (Point3, Point3),
    t: f64,
) -> (Vec<WorldVert>, Option<Pins>) {
    // MIN and MAX: the outside's points whose sheet points are the model's thinnest and thickest
    let (mut best_lo, mut best_hi) = ((f64::INFINITY, [0.0f32; 3]), (f64::INFINITY, [0.0f32; 3]));
    let (target_lo, target_hi) = at;
    // on the part's outside: well off the sheet (its side on the tool is within a quarter of t)
    let off = 0.6 * t;
    let mut mark = |p: Point3, foot: Point3| {
        if (p - foot).length() > off {
            for (best, target) in [(&mut best_lo, target_lo), (&mut best_hi, target_hi)] {
                let d = (foot - target).length_sq();
                if d < best.0 {
                    *best = (d, f3(p));
                }
            }
        }
    };
    let idx = |k: usize| u32::from_le_bytes(l.indices[k * 4..k * 4 + 4].try_into().expect("4 bytes"));
    // a face on the tool's side (most of its area lies on the tool, or flat on the trim plane) is
    // left bare, whole: each face is painted all over or not at all, so no edge of the map is
    // ragged
    let faces: Vec<&crate::mesh::FaceRange> =
        l.faces.iter().filter(|f| span.faces.contains(&f.face)).collect();
    let tris_of = |f: &crate::mesh::FaceRange| -> Vec<[Point3; 3]> {
        (f.first_tri..f.first_tri + f.tris)
            .map(|k| [0, 1, 2].map(|c| vertex_at(&l.vertices, idx(k as usize * 3 + c) as usize)))
            .collect()
    };
    let bare = forge_math::par::map(faces.len(), |k| {
        let (mut on, mut all) = (0.0, 0.0);
        for p in tris_of(faces[k]) {
            let a = (p[1] - p[0]).cross(p[2] - p[0]).length();
            all += a;
            if tool.has(p) {
                on += a;
            }
        }
        on > 0.5 * all
    });
    let tris: Vec<[(Point3, Point3); 3]> = faces
        .iter()
        .zip(&bare)
        .filter(|x| !*x.1)
        .flat_map(|x| tris_of(x.0))
        .map(|p| p.map(|x| (x, x)))
        .collect();
    let mut heat = vec![];
    paint_mesh(&tris, field, range, &mut heat, &mut mark);
    let ext = (best_lo.0.is_finite() && best_hi.0.is_finite()).then_some((best_lo.1, best_hi.1));
    (heat, ext)
}

/// How long a side of a painted triangle is at most, mm: the wall is looked up at every corner, so a
/// big flat face of the part shows the model's wall across it, its extremes too.
const PAINT: f64 = 2.0;

/// A part's triangles, drawn at `p`, their colours looked up at `q` (their points on the modelled
/// part): one mesh, its sides split till none is over `PAINT` (a side split in both its triangles,
/// so no piece leaves a crack beside its neighbour), the wall looked up once at every corner, then
/// each triangle banded as the legend is. `mark` sees each corner and its sheet point.
pub fn paint_mesh(
    tris: &[[(Point3, Point3); 3]],
    field: &forge_thermo::WallField,
    range: (f64, f64),
    out: &mut Vec<WorldVert>,
    mark: &mut dyn FnMut(Point3, Point3),
) {
    // one corner where triangles meet
    let mut pts: Vec<(Point3, Point3)> = vec![];
    let mut index: std::collections::HashMap<(i64, i64, i64), u32> = Default::default();
    let key = |p: Point3| {
        (
            (p.x * 1e4).round() as i64,
            (p.y * 1e4).round() as i64,
            (p.z * 1e4).round() as i64,
        )
    };
    let mut mesh: Vec<[u32; 3]> = tris
        .iter()
        .map(|t| {
            t.map(|c| {
                *index.entry(key(c.0)).or_insert_with(|| {
                    pts.push(c);
                    (pts.len() - 1) as u32
                })
            })
        })
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
        .collect();
    split_mesh(&mut pts, &mut mesh);
    let wall: Vec<Option<(f64, Point3)>> = forge_math::par::map(pts.len(), |i| field.raw_at(pts[i].1));
    let lift = lifts(&pts, &mesh);
    for (c, w) in pts.iter().zip(&wall) {
        if let Some((_, foot)) = w {
            mark(c.0, *foot);
        }
    }
    for t in &mesh {
        let w = t.map(|i| wall[i as usize]);
        let known: Vec<f64> = w.iter().flatten().map(|x| x.0).collect();
        if known.is_empty() {
            continue;
        }
        let fill = known.iter().sum::<f64>() / known.len() as f64;
        banded(
            t.map(|i| pts[i as usize].0 + lift[i as usize]),
            w.map(|x| x.map_or(fill, |x| x.0)),
            range,
            out,
        );
    }
}

/// Each corner's lift off the part, the same for every triangle at it so no piece leaves a crack:
/// at least `LIFT` off every face that meets there. On a smooth patch that is its normal; at a hard
/// edge or corner it leans out along the faces' bisector, so neither face's paint sinks into it.
pub fn lifts(pts: &[(Point3, Point3)], mesh: &[[u32; 3]]) -> Vec<Vec3> {
    // the faces' directions at each corner, those within CREASE of each other one direction
    const CREASE: f64 = 0.94;
    let mut dirs: Vec<Vec<Vec3>> = vec![vec![]; pts.len()];
    for t in mesh {
        let [a, b, c] = t.map(|i| pts[i as usize].0);
        let n = (b - a).cross(c - a);
        let Some(u) = n.normalize().map(|u| u.get()) else {
            continue;
        };
        if n.length() < 1e-10 {
            continue;
        }
        for &i in t {
            let d = &mut dirs[i as usize];
            match d
                .iter_mut()
                .find(|m| m.normalize().is_some_and(|m| m.get().dot(u) > CREASE))
            {
                Some(m) => *m += n,
                None => d.push(n),
            }
        }
    }
    dirs.iter()
        .map(|d| {
            let n: Vec<Vec3> = d.iter().filter_map(|m| m.normalize().map(|m| m.get())).collect();
            let Some(&first) = n.first() else {
                return Vec3::ZERO;
            };
            let mut v = n
                .iter()
                .fold(Vec3::ZERO, |a, &m| a + m)
                .normalize()
                .map_or(first, |u| u.get())
                * LIFT;
            // pushed out till it is LIFT off each face (a fold back on itself is held at 3 LIFT)
            for _ in 0..32 {
                let mut moved = false;
                for m in &n {
                    let short = LIFT - v.dot(*m);
                    if short > 1e-6 * LIFT {
                        v += *m * short;
                        moved = true;
                    }
                }
                if !moved {
                    break;
                }
            }
            if v.length() > 3.0 * LIFT {
                v = v * (3.0 * LIFT / v.length());
            }
            v
        })
        .collect()
}

/// Longest sides halved till none is over `PAINT` (looked at on the modelled points), each split
/// side split in both its triangles.
fn split_mesh(pts: &mut Vec<(Point3, Point3)>, tris: &mut Vec<[u32; 3]>) {
    for _ in 0..16 {
        let len = |a: u32, b: u32, pts: &[(Point3, Point3)]| (pts[a as usize].1 - pts[b as usize].1).length();
        let key = |a: u32, b: u32| (a.min(b), a.max(b));
        let mut split: std::collections::HashMap<(u32, u32), u32> = Default::default();
        for &[a, b, c] in tris.iter() {
            let (x, y) = [(a, b), (b, c), (c, a)]
                .into_iter()
                .max_by(|p, q| len(p.0, p.1, pts).total_cmp(&len(q.0, q.1, pts)))
                .expect("three sides");
            if len(x, y, pts) > PAINT && !split.contains_key(&key(x, y)) {
                let (u, v) = (pts[x as usize], pts[y as usize]);
                pts.push((u.0 + (v.0 - u.0) * 0.5, u.1 + (v.1 - u.1) * 0.5));
                split.insert(key(x, y), (pts.len() - 1) as u32);
            }
        }
        if split.is_empty() {
            return;
        }
        let mut out = Vec::with_capacity(tris.len() * 2);
        for &[a, b, c] in tris.iter() {
            let m = |x: u32, y: u32| split.get(&key(x, y)).copied();
            match (m(a, b), m(b, c), m(c, a)) {
                (None, None, None) => out.push([a, b, c]),
                (Some(p), None, None) => out.extend([[a, p, c], [p, b, c]]),
                (None, Some(p), None) => out.extend([[a, b, p], [a, p, c]]),
                (None, None, Some(p)) => out.extend([[a, b, p], [p, b, c]]),
                (Some(p), Some(q), None) => out.extend([[a, p, c], [p, b, q], [p, q, c]]),
                (None, Some(p), Some(q)) => out.extend([[a, b, q], [q, b, p], [q, p, c]]),
                (Some(p), None, Some(q)) => out.extend([[a, p, q], [p, b, c], [q, p, c]]),
                (Some(p), Some(q), Some(r)) => out.extend([[a, p, r], [p, b, q], [r, q, c], [p, q, r]]),
            }
        }
        *tris = out;
    }
}

/// A triangle (already lifted off its face) cut along the legend's band edges (ten bands from
/// `range`'s thinnest to its thickest), each piece filled with its band's colour: a contour plot,
/// the value linear across the triangle, no colour blended across a band edge.
pub fn banded(p: [Point3; 3], v: [f64; 3], range: (f64, f64), out: &mut Vec<WorldVert>) {
    let (lo, hi) = range;
    let span = (hi - lo).max(1e-12);
    let poly: Vec<(Point3, f64)> = (0..3).map(|k| (p[k], v[k])).collect();
    let (vmin, vmax) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| {
        (a.0.min(x), a.1.max(x))
    });
    // the band of a value: 0 the thickest's (as `band` reads it), 9 the thinnest's
    let index = |x: f64| (((1.0 - ((x - lo) / span).clamp(0.0, 1.0)) * 10.0) as usize).min(9);
    let (first, last) = (index(vmax), index(vmin));
    for (k, c) in BANDS.iter().enumerate().take(last + 1).skip(first) {
        // band k holds the values from lo + (9 - k) span / 10 to lo + (10 - k) span / 10
        let (a, b) = (
            lo + span * (9 - k) as f64 / 10.0,
            lo + span * (10 - k) as f64 / 10.0,
        );
        let below = if k == 9 { f64::NEG_INFINITY } else { a };
        let above = if k == 0 { f64::INFINITY } else { b };
        let piece = clip_value(&clip_value(&poly, below, true), above, false);
        if piece.len() < 3 {
            continue;
        }
        for j in 1..piece.len() - 1 {
            for q in [piece[0].0, piece[j].0, piece[j + 1].0] {
                out.push(WorldVert {
                    at: f3(q),
                    colour: unlit(*c),
                });
            }
        }
    }
}

/// The part of a polygon whose value is at least `edge` (`keep_above`) or at most it.
fn clip_value(poly: &[(Point3, f64)], edge: f64, keep_above: bool) -> Vec<(Point3, f64)> {
    if edge.is_infinite() {
        return poly.to_vec();
    }
    let inside = |x: f64| if keep_above { x >= edge } else { x <= edge };
    let mut out = vec![];
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        if inside(a.1) {
            out.push(a);
        }
        if inside(a.1) != inside(b.1) {
            let f = (edge - a.1) / (b.1 - a.1);
            out.push((a.0 + (b.0 - a.0) * f, edge));
        }
    }
    out
}
