//! A thermoform made into what the app draws: its row (numbers, the section, the faces' drafts,
//! the sheet's points for probing), the wall-thickness map at each step of the forming, and the
//! draft analysis's triangles, all lifted off the mould by the sheet's thickness so they lie just
//! over the formed part's face.
use crate::origin::WorldVert;
use crate::shell::thermo::{DRAFT, ThermoRow, band};
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
                    colour: [c[0], c[1], c[2], 1.0],
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
pub fn view(o: &ThermoOut, limit: f64, draft_min: f32) -> (ThermoRow, Gfx) {
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
        ..Gfx::default()
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
        ..ThermoRow::default()
    };
    let Some(w) = &o.walls else {
        row.facts = facts;
        return (row, gfx);
    };
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
            let mean = v.iter().map(|&i| s.ratio[i as usize]).sum::<f32>() / 3.0 * o.t as f32;
            let fr = if whi > wlo {
                ((mean as f64 - wlo) / (whi - wlo)) as f32
            } else {
                0.5
            };
            let c = band(fr);
            for &i in &v {
                let q = n[i as usize];
                let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt().max(1e-12);
                let p = s.nodes[i as usize];
                tris.push(WorldVert {
                    at: [
                        p[0] + q[0] / l * lift,
                        p[1] + q[1] / l * lift,
                        p[2] + q[2] / l * lift,
                    ],
                    colour: [c[0], c[1], c[2], 1.0],
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
    for v in r.vents.iter().take(40) {
        out.push(format!("  {:8.2} {:8.2} {:8.2}", v.x, v.y, v.z));
    }
    out.push(String::new());
    out.push(row.model.clone());
    out
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
