//! The shop's checks on a formed part, run on every build: what the tool's surface does to the sheet
//! (draft, undercuts, radii, draw ratios, webbing), what the tool is (shrink, vents) and what the
//! machine takes (gauge, sheet, depth). Each is a typed violation; angles in radians.
use crate::build::{Formed, Side, Spec};
use crate::material::{FormShop, Material};
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::{Point3, dmath};
use forge_topo::{Body, FaceId, Name};

#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    /// A face of the tool leaning off the pull by less than the material's least draft for this
    /// side (`draft` its least, at `at`).
    DraftTooLow {
        face: Name,
        at: Point3,
        draft: f64,
        min: f64,
    },
    /// A face of the tool turned under: the formed sheet locks on it.
    Undercut {
        face: Name,
        at: Point3,
        draft: f64,
    },
    /// A round on the tool tighter than the material forms (`convex` on the tool).
    RadiusTooTight {
        face: Name,
        at: Point3,
        radius: f64,
        min: f64,
        convex: bool,
    },
    /// An edge of the tool with no round at all.
    SharpEdge {
        edge: Name,
        at: Point3,
        convex: bool,
        min: f64,
    },
    /// A region drawn deeper than the material takes over its least width (H:D).
    DrawTooDeep {
        at: Point3,
        depth: f64,
        width: f64,
        max: f64,
    },
    /// The sheet's area grows more than the material takes (areal draw ratio).
    AreaDraw {
        adr: f64,
        max: f64,
    },
    /// Raised parts of a male tool closer than the depth between them allows: the sheet webs.
    Webbing {
        at: Point3,
        depth: f64,
        gap: f64,
        min_gap: f64,
    },
    /// The tool could not be grown by the shrink: the part comes out that much small.
    ShrinkNotApplied {
        what: crate::scale::Unscalable,
        shrink: f64,
    },
    /// Vent holes wider than the sheet bridges without a mark.
    VentTooLarge {
        d: f64,
        max: f64,
    },
    /// A place the sheet meets last with no vent near it (vents given by hand).
    VentMissing {
        at: Point3,
        pitch: f64,
    },
    GaugeNotStocked {
        t: f64,
        nearest: f64,
    },
    /// The sheet given is smaller than the trim and the clamp frame need, or the machine takes.
    SheetTooSmall {
        need: (f64, f64),
        have: (f64, f64),
    },
    SheetTooLarge {
        need: (f64, f64),
        max: (f64, f64),
    },
    TooDeep {
        depth: f64,
        max: f64,
    },
    /// The material's numbers named are stand-ins, not sourced (information).
    StandIn {
        material: String,
        fields: Vec<&'static str>,
    },
}

/// A face's points to read its normal at: a plane's one, else its mesh's vertices.
fn samples(f: &Formed, face: FaceId) -> Vec<Point3> {
    let mut out = vec![];
    for (t, tri) in f.mesh.triangles.iter().enumerate() {
        if f.mesh.face_of[t] as usize == face.index() {
            out.extend(tri.iter().map(|&i| f.mesh.positions[i as usize]));
        }
    }
    out
}

fn normal_at(body: &Body, face: FaceId, p: Point3) -> Option<forge_math::Vec3> {
    let s = &body.face(face).surface;
    body.face_normal(face, s.project(p)).map(|n| n.get())
}

pub fn check(mould: &Body, f: &Formed, spec: &Spec, m: &Material, shop: &FormShop) -> Vec<Violation> {
    let mut out = vec![];
    let male = spec.side == Side::Inner;
    let d = spec.pull.get();
    // the tool's outward normal is the mould's (male) or its reverse
    let s = if male { 1.0 } else { -1.0 };
    let min_draft = if male { m.draft_male } else { m.draft_female };
    let min_r = |convex: bool| {
        if convex {
            m.min_radius.at(spec.t)
        } else {
            m.min_inside_radius.at(spec.t)
        }
    };
    let open = |fid: FaceId| f.open[fid.index()];
    for &(fid, name, opening) in &f.faces {
        if opening {
            continue;
        }
        let face = mould.face(fid);
        let pts = match &face.surface {
            Surface::Plane(p) => vec![p.frame.origin],
            _ => samples(f, fid),
        };
        let mut worst: Option<(f64, Point3)> = None;
        for p in pts {
            let Some(n) = normal_at(mould, fid, p) else {
                continue;
            };
            let a = dmath::asin((n.dot(d) * s).clamp(-1.0, 1.0));
            if worst.is_none_or(|w| a < w.0) {
                worst = Some((a, p));
            }
        }
        if let Some((a, at)) = worst {
            // the plane's own point may lie off the face: report one on it
            let at = if matches!(face.surface, Surface::Plane(_)) {
                samples(f, fid).first().copied().unwrap_or(at)
            } else {
                at
            };
            if a < -1e-4 {
                out.push(Violation::Undercut {
                    face: name,
                    at,
                    draft: a,
                });
            } else if a < min_draft - 1e-9 {
                out.push(Violation::DraftTooLow {
                    face: name,
                    at,
                    draft: a,
                    min: min_draft,
                });
            }
        }
        let radius = match &face.surface {
            Surface::Cylinder(c) => Some(c.radius),
            Surface::Torus(t) => Some(t.minor),
            Surface::Sphere(sp) => Some(sp.radius),
            _ => None,
        };
        let convex = face.same_sense == male;
        if let Some(r) = radius
            && r < min_r(convex) - 1e-9
        {
            out.push(Violation::RadiusTooTight {
                face: name,
                at: samples(f, fid).first().copied().unwrap_or(Point3::ORIGIN),
                radius: r,
                min: min_r(convex),
                convex,
            });
        }
    }
    for e in mould.edge_ids() {
        let edge = mould.edge(e);
        if !edge.alive || edge.coedges.len() != 2 {
            continue;
        }
        let (ca, cb) = (edge.coedges[0], edge.coedges[1]);
        let (fa, fb) = (mould.coedge_face(ca), mould.coedge_face(cb));
        if open(fa) || open(fb) {
            continue;
        }
        let t = 0.5 * (edge.range.lo + edge.range.hi);
        let der = edge.curve.deriv(t);
        let (Some(na), Some(nb)) = (normal_at(mould, fa, der.p), normal_at(mould, fb, der.p)) else {
            continue;
        };
        if na.dot(nb) > dmath::cos(2f64.to_radians()) {
            continue;
        }
        let tan = if mould.coedge(ca).forward { der.d1 } else { -der.d1 };
        let convex = nb.dot(na.cross(tan)) < 0.0;
        out.push(Violation::SharpEdge {
            edge: edge.name,
            at: der.p,
            convex: convex == male,
            min: min_r(convex == male),
        });
    }
    for r in f.field.regions() {
        if r.depth > spec.t && r.depth > m.max_hd * r.width {
            out.push(Violation::DrawTooDeep {
                at: f.field.world(r.at, 0.0),
                depth: r.depth,
                width: r.width,
                max: m.max_hd,
            });
        }
    }
    if f.report.adr > m.max_adr {
        out.push(Violation::AreaDraw {
            adr: f.report.adr,
            max: m.max_adr,
        });
    }
    if male {
        let eps = spec.t.max(f.field.h);
        let mut found: Vec<(Point3, f64, f64)> = vec![];
        for v in f.field.valleys(eps) {
            if v.width >= m.min_gap_ratio * v.depth {
                continue;
            }
            let at = f.field.world(v.at, v.floor);
            // one per place: the worst of the valleys within a gap of each other
            match found
                .iter_mut()
                .find(|w| (w.0 - at).length() < 2.0 * v.width.max(w.2))
            {
                Some(w) if v.depth / v.width > w.1 / w.2 => *w = (at, v.depth, v.width),
                Some(_) => {}
                None => found.push((at, v.depth, v.width)),
            }
        }
        for (at, depth, gap) in found.into_iter().take(8) {
            out.push(Violation::Webbing {
                at,
                depth,
                gap,
                min_gap: m.min_gap_ratio * depth,
            });
        }
    }
    if let Some(what) = f.report.unscaled {
        out.push(Violation::ShrinkNotApplied {
            what,
            shrink: spec.shrink,
        });
    }
    if let Some(dv) = spec.vent {
        let max = m.max_vent.at(spec.t);
        if dv > max + 1e-9 {
            out.push(Violation::VentTooLarge { d: dv, max });
        }
        if let Some(given) = &spec.vents {
            for (p, z) in f.field.vents(dv, spec.vent_pitch, 48) {
                if given.iter().all(|q| (*q - p).length() > spec.vent_pitch) {
                    out.push(Violation::VentMissing {
                        at: f.field.world(p, z),
                        pitch: spec.vent_pitch,
                    });
                }
            }
        }
    }
    let mc = &shop.machine;
    if !mc.gauges.is_empty() && !mc.gauges.iter().any(|g| (g - spec.t).abs() < 1e-9) {
        let nearest = mc
            .gauges
            .iter()
            .copied()
            .min_by(|a, b| (a - spec.t).abs().total_cmp(&(b - spec.t).abs()))
            .unwrap_or(spec.t);
        out.push(Violation::GaugeNotStocked { t: spec.t, nearest });
    }
    let (lo, hi) = f.report.trim_box;
    let need = sorted(hi.x - lo.x + 2.0 * mc.clamp, hi.y - lo.y + 2.0 * mc.clamp);
    if let Some(have) = spec.sheet {
        let have = sorted(have.0, have.1);
        if have.0 < need.0 - 1e-9 || have.1 < need.1 - 1e-9 {
            out.push(Violation::SheetTooSmall { need, have });
        }
    }
    if let Some(max) = mc.max_sheet {
        let max = sorted(max.0, max.1);
        if need.0 > max.0 + 1e-9 || need.1 > max.1 + 1e-9 {
            out.push(Violation::SheetTooLarge { need, max });
        }
    }
    if let Some(max) = mc.max_depth
        && f.report.depth > max + 1e-9
    {
        out.push(Violation::TooDeep {
            depth: f.report.depth,
            max,
        });
    }
    if !m.stand_in.is_empty() {
        out.push(Violation::StandIn {
            material: m.name.clone(),
            fields: m.stand_in.clone(),
        });
    }
    out
}

fn sorted(a: f64, b: f64) -> (f64, f64) {
    (a.min(b), a.max(b))
}
