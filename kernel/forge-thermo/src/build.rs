//! A part generated from its modelled shape. The shape's surface is the formed part's inner skin
//! (a male mould, the default) or its outer one (female); the part is that surface offset by the
//! sheet's thickness, trimmed square to the trim plane along the trim outline, with the flange
//! lying on the table (male) or the mould's top (female). The mould (the tool) is the shape grown
//! by the plastic's shrink, on a base (male) or cut into a block (female), with vents drilled where
//! the sheet meets it last.
//!
//! Male: the shape shelled outward by t, opened at its foot, joined to the flange's slab less the
//! shape. Female: M, a block of the trim outline less the shape, offset outward by t but where the
//! block is cut off, less M.
use crate::field::Field;
use forge_bool::{BoolError, Op, boolean};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath, tol};
use forge_mesh::{Quality, TriMesh};
use forge_ops::{ExtrudeError, Profile, ProfileLoop, Segment, ShellError};
use forge_topo::{Body, FaceId, Name};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The shape is the part's inside: a male tool.
    Inner,
    /// The shape is the part's outside: a female tool (a cavity).
    Outer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub side: Side,
    /// The way the part comes off the tool.
    pub pull: UnitVec3,
    /// The sheet's thickness.
    pub t: f64,
    /// The trim outline, on a plane square to the pull (any height); None: the shape's outline's
    /// box grown by `flange`, its corners rounded to it.
    pub trim: Option<Profile>,
    pub flange: f64,
    /// The tool is the shape grown by 1 + shrink about the trim plane under its centre.
    pub shrink: f64,
    /// The tool's base under the trim plane (male) or under the cavity (female), and how far it
    /// reaches past the trim outline.
    pub base: f64,
    pub margin: f64,
    /// Vent holes across (None: none drilled).
    pub vent: Option<f64>,
    /// Where the vents go, on the trim plane's axes (None: where the sheet arrives last).
    pub vents: Option<Vec<Point2>>,
    /// Least distance between vents placed here.
    pub vent_pitch: f64,
    /// The sheet blank, (shorter, longer); None: as the trim and the clamp need.
    pub sheet: Option<(f64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FormError {
    /// The thickness or a size is not positive.
    Size {
        what: &'static str,
        value: f64,
    },
    /// The shape has no flat face square to the pull at its foot (male: lowest; female: highest),
    /// so there is no trim plane.
    NoOpening {
        side: Side,
    },
    /// The trim plane's normal is not along the pull.
    TrimNotSquare,
    /// The trim outline does not hold the shape: a point of the shape outside it, `clear` short of
    /// the sheet's thickness inside.
    TrimCutsShape {
        at: Point3,
        clear: f64,
    },
    Extrude(ExtrudeError),
    Bool(BoolError),
    /// The offset by the sheet's thickness failed: the tool's surface curves tighter than the
    /// sheet is thick (`Collapses`) or its walls meet.
    Offset(ShellError),
    Mesh,
}

/// What a build reports: the numbers a shop reads, and every check reads.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// Area of the shape's surface the sheet forms onto, and of its opening on the trim plane.
    pub formed_area: f64,
    pub opening_area: f64,
    /// Area inside the trim outline.
    pub trim_area: f64,
    /// formed / opening (areal draw ratio); t / it, the mean wall over the draw.
    pub adr: f64,
    pub mean_wall: f64,
    pub ldr: f64,
    /// The deepest draw, along the pull.
    pub depth: f64,
    /// The trim outline's box on the plane, and the sheet it needs with the machine's clamp.
    pub trim_box: (Point2, Point2),
    /// The tool's scale (1 + shrink), or 1 where the shape could not be scaled.
    pub scale: f64,
    pub unscaled: Option<crate::scale::Unscalable>,
    pub vents: Vec<Point3>,
}

#[derive(Clone, Debug)]
pub struct Formed {
    pub part: Body,
    pub mould: Body,
    /// The trim plane: z the pull, origin on it under the world's origin.
    pub frame: Frame,
    /// The shape's surface, meshed, and which of its triangles the sheet forms onto (all but the
    /// opening): what the checks and the thickness model read.
    pub mesh: TriMesh,
    pub contact: Vec<usize>,
    /// The shape's faces: (face, name, is the opening), and the same by face index.
    pub faces: Vec<(FaceId, Name, bool)>,
    pub open: Vec<bool>,
    /// The trim outline on the trim plane, and the middle of the shape's box there.
    pub trim: ProfileLoop,
    pub centre: Point2,
    pub field: Field,
    pub report: Report,
}

fn err_b(e: BoolError) -> FormError {
    FormError::Bool(e)
}

fn err_e(e: ExtrudeError) -> FormError {
    FormError::Extrude(e)
}

/// The trim plane's axes for pull `d`: x along the world's x where it can be.
pub fn plane_frame(d: UnitVec3, z0: f64) -> Frame {
    let hint = if d.get().x.abs() > 0.9 { Vec3::Y } else { Vec3::X };
    let f = Frame::from_zx(Point3::ORIGIN, d, hint).expect("a hint off the pull");
    f.with_origin(Point3::ORIGIN + d.get() * z0)
}

/// `l` drawn on `from`'s plane, as drawn on `to`'s (its normal along `from`'s either way).
pub fn carried(l: &ProfileLoop, from: &Frame, to: &Frame) -> ProfileLoop {
    let flip = from.z().dot(to.z().get()) < 0.0;
    let p = |q: Point2| {
        let w = to.point_to_local(from.plane_point(q));
        Point2::new(w.x, w.y)
    };
    let ax = to.vec_to_local(from.x().get());
    let turn = dmath::atan2(ax.y, ax.x);
    let ang = |a: f64| if flip { turn - a } else { turn + a };
    let segs = l
        .segments
        .iter()
        .map(|(s, id)| {
            let s = match s {
                Segment::Line { a, b } => Segment::Line { a: p(*a), b: p(*b) },
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => Segment::Arc {
                    center: p(*center),
                    radius: *radius,
                    start: ang(*start),
                    sweep: if flip { -sweep } else { *sweep },
                },
                Segment::Circle { center, radius, ccw } => Segment::Circle {
                    center: p(*center),
                    radius: *radius,
                    ccw: *ccw != flip,
                },
                other => other.clone(),
            };
            (s, *id)
        })
        .collect();
    let out = ProfileLoop { segments: segs };
    if out.signed_area() < 0.0 {
        out.reversed()
    } else {
        out
    }
}

/// The loop as a polygon, its arcs in short chords.
pub fn polygon(l: &ProfileLoop) -> Vec<Point2> {
    let mut out = vec![];
    for (s, _) in &l.segments {
        out.extend(s.samples(16));
    }
    out
}

/// Inside a polygon (even-odd), and the distance to its boundary.
pub fn inside(poly: &[Point2], p: Point2) -> (bool, f64) {
    let mut c = false;
    let mut d = f64::INFINITY;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            c = !c;
        }
        let ab = b - a;
        let l2 = ab.length_sq();
        let f = if l2 > 0.0 {
            ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        d = d.min((p - (a + ab * f)).length());
    }
    (c, d)
}

fn prism(l: &ProfileLoop, frame: Frame, from: f64, to: f64, feature: u64) -> Result<Body, FormError> {
    let p = Profile {
        plane: frame.with_origin(frame.origin + frame.z().get() * from),
        loops: vec![l.clone()],
    };
    forge_ops::extrude(&p, to - from, feature).map_err(err_e)
}

/// The box of a loop's polygon.
fn bounds(poly: &[Point2]) -> (Point2, Point2) {
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in poly {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

fn rounded(lo: Point2, hi: Point2, grow: f64, r: f64, id: u64) -> ProfileLoop {
    let (w, h) = (hi.x - lo.x + 2.0 * grow, hi.y - lo.y + 2.0 * grow);
    let c = Point2::new(lo.x - grow, lo.y - grow);
    if r > tol::LINEAR {
        Profile::rounded_rectangle(c, w, h, r.min(0.49 * w.min(h)), id)
    } else {
        Profile::rectangle(c, w, h, id)
    }
}

/// What the sheet meets, without the solids: the shape meshed, its height field and the draw's
/// numbers. The checks read only this, so they run where the solids cannot be built.
pub fn analyse(shape: &Body, spec: &Spec) -> Result<Formed, FormError> {
    for (what, v) in [("t", spec.t), ("base", spec.base)] {
        if !v.is_finite() || v <= tol::LINEAR {
            return Err(FormError::Size { what, value: v });
        }
    }
    for (what, v, lo) in [
        ("flange", spec.flange, 0.0),
        ("shrink", spec.shrink, -0.5),
        ("margin", spec.margin, 0.0),
        ("vent_pitch", spec.vent_pitch, 0.0),
        ("vent", spec.vent.unwrap_or(0.0), 0.0),
    ] {
        if !v.is_finite() || v < lo || (what == "shrink" && v > 0.5) {
            return Err(FormError::Size { what, value: v });
        }
    }
    let male = spec.side == Side::Inner;
    let d = spec.pull;
    let mesh = forge_mesh::tessellate(
        shape,
        &Quality {
            chord: 0.02,
            max_angle: dmath::PI / 24.0,
        },
    )
    .map_err(|_| FormError::Mesh)?;
    let along = |p: Point3| (p - Point3::ORIGIN).dot(d.get());
    let (zmin, zmax) = mesh
        .positions
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &p| {
            (a.min(along(p)), b.max(along(p)))
        });
    let z0 = if male { zmin } else { zmax };
    let frame = plane_frame(d, z0);
    let depth = zmax - zmin;
    // the opening: flat faces on the trim plane facing off the part's side of it
    let mut open = vec![false; shape.faces.len()];
    let faces: Vec<(FaceId, Name, bool)> = shape
        .face_ids()
        .map(|f| {
            let face = shape.face(f);
            let is_open = match &face.surface {
                forge_geom::Surface::Plane(p) => {
                    let n = if face.same_sense {
                        p.frame.z()
                    } else {
                        -p.frame.z()
                    };
                    let want = if male { -1.0 } else { 1.0 };
                    n.dot(d.get()) * want > 1.0 - 1e-9 && (along(p.frame.origin) - z0).abs() < tol::LINEAR
                }
                _ => false,
            };
            (f, face.name, is_open)
        })
        .collect();
    for f in &faces {
        open[f.0.index()] = f.2;
    }
    if !faces.iter().any(|f| f.2) {
        return Err(FormError::NoOpening { side: spec.side });
    }
    let contact: Vec<usize> = (0..mesh.triangles.len())
        .filter(|&t| !open[mesh.face_of[t] as usize])
        .collect();
    let local: Vec<Point3> = mesh.positions.iter().map(|&p| frame.point_to_local(p)).collect();
    let (lo, hi) = bounds(&local.iter().map(|p| Point2::new(p.x, p.y)).collect::<Vec<_>>());
    let trim = match &spec.trim {
        Some(p) => {
            if p.plane.z().dot(d.get()).abs() < 1.0 - 1e-9 || p.loops.len() != 1 {
                return Err(FormError::TrimNotSquare);
            }
            carried(&p.loops[0], &p.plane, &frame)
        }
        None => rounded(lo, hi, spec.flange, spec.flange, 1),
    };
    // sizes out of all proportion to the shape are refused before anything is laid out
    let limit = (10.0 * (hi.x - lo.x).max(hi.y - lo.y).max(depth)).max(1000.0);
    let tpoly = polygon(&trim);
    let (a, b) = bounds(&tpoly);
    for (what, v) in [
        ("trim", (b.x - a.x).max(b.y - a.y)),
        ("t", spec.t),
        ("flange", spec.flange),
        ("base", spec.base),
        ("margin", spec.margin),
        ("vent_pitch", spec.vent_pitch),
        ("vent", spec.vent.unwrap_or(0.0)),
    ] {
        if !v.is_finite() || v > limit {
            return Err(FormError::Size { what, value: v });
        }
    }
    let poly = tpoly;
    for (i, p) in local.iter().enumerate() {
        let (inn, dist) = inside(&poly, Point2::new(p.x, p.y));
        if !inn || dist < spec.t {
            return Err(FormError::TrimCutsShape {
                at: mesh.positions[i],
                clear: if inn { dist } else { -dist },
            });
        }
    }
    let (tlo, thi) = bounds(&poly);
    let field = Field::new(frame, tlo, thi, &mesh, &contact, male, |p| inside(&poly, p).0);
    let area = forge_topo::mass::mass(shape).map(|m| m.faces).unwrap_or_default();
    let (mut formed_area, mut opening_area) = (0.0, 0.0);
    for (f, a) in area {
        if open[f.index()] {
            opening_area += a;
        } else {
            formed_area += a;
        }
    }
    let adr = formed_area / opening_area;
    let report = Report {
        formed_area,
        opening_area,
        trim_area: trim.signed_area().abs(),
        adr,
        mean_wall: spec.t / adr,
        ldr: field.linear_draw(),
        depth,
        trim_box: (tlo, thi),
        scale: 1.0,
        unscaled: None,
        vents: vec![],
    };
    Ok(Formed {
        part: Body::new(),
        mould: Body::new(),
        frame,
        mesh,
        contact,
        faces,
        open,
        trim,
        centre: Point2::new(0.5 * (lo.x + hi.x), 0.5 * (lo.y + hi.y)),
        field,
        report,
    })
}

/// Forms the sheet over `shape`: the part and the tool. `feature` names what is made.
pub fn form(shape: &Body, spec: &Spec, feature: u64) -> Result<Formed, FormError> {
    let mut f = analyse(shape, spec)?;
    let male = spec.side == Side::Inner;
    let (frame, trim, depth) = (f.frame, f.trim.clone(), f.report.depth);
    let slab_id = feature ^ 0x5eed;
    let part = if male {
        // the shape's skin grown by t, opened at its foot, and the flange's slab less the shape
        let open: Vec<Name> = f.faces.iter().filter(|f| f.2).map(|f| f.1).collect();
        let skin = forge_ops::shell(shape, spec.t, true, &open, feature).map_err(FormError::Offset)?;
        let slab = prism(&trim, frame, 0.0, spec.t, slab_id)?;
        let flange = boolean(&slab, shape, Op::Subtract, feature).map_err(err_b)?;
        boolean(&skin, &flange, Op::Union, feature).map_err(err_b)?
    } else {
        // M, the block of the trim less the shape, grown by t but where the block is cut off
        let block = prism(&trim, frame, -(depth + 2.0 * spec.t), 0.0, slab_id)?;
        let cut = slab_names(&block, &frame, 0.0);
        let m = boolean(&block, shape, Op::Subtract, feature).map_err(err_b)?;
        let open: Vec<Name> = m
            .face_ids()
            .map(|f| m.face(f).name)
            .filter(|n| cut.iter().any(|c| n == c || n.is_piece_of(*c)))
            .collect();
        let grown = forge_ops::offset_solid(&m, spec.t, true, &open, feature)
            .map_err(FormError::Offset)?
            .ok_or(FormError::Offset(ShellError::AllOpen))?;
        boolean(&grown, &m, Op::Subtract, feature).map_err(err_b)?
    };

    // the tool
    let k = 1.0 + spec.shrink;
    let c = frame.point_to_world(Point3::new(f.centre.x, f.centre.y, 0.0));
    let (grown_shape, unscaled) = match crate::scale::scaled(shape, c, k) {
        Ok(b) => (b, None),
        Err(e) => (shape.clone(), Some(e)),
    };
    let (tlo, thi) = f.report.trim_box;
    let tool_id = feature ^ 0x7001;
    let outline = rounded(tlo, thi, spec.margin, 0.0, 1);
    let mut tool = if male {
        let base = prism(&outline, frame, -spec.base, 0.0, tool_id)?;
        boolean(&grown_shape, &base, Op::Union, tool_id).map_err(err_b)?
    } else {
        let block = prism(&outline, frame, -(depth * k + spec.base), 0.0, tool_id)?;
        boolean(&block, &grown_shape, Op::Subtract, tool_id).map_err(err_b)?
    };
    let mut vents = vec![];
    if let Some(dv) = spec.vent {
        let places: Vec<(Point2, f64)> = match &spec.vents {
            Some(ps) => ps.iter().map(|&p| (p, height_at(&f.field, p))).collect(),
            None => f.field.vents(dv, spec.vent_pitch, 48),
        };
        let bottom = if male {
            -spec.base
        } else {
            -(depth * k + spec.base)
        };
        for (i, (p, z)) in places.into_iter().enumerate() {
            // the floor where the tool was grown
            let top = z * k;
            let hole = prism(
                &Profile::circle(p, 0.5 * dv, 1),
                frame,
                bottom - 1.0,
                top + 1.0,
                tool_id ^ (0x100 + i as u64),
            )?;
            tool = boolean(&tool, &hole, Op::Subtract, tool_id).map_err(err_b)?;
            vents.push(frame.point_to_world(Point3::new(p.x, p.y, top)));
        }
    }
    f.report.scale = if unscaled.is_some() { 1.0 } else { k };
    f.report.unscaled = unscaled;
    f.report.vents = vents;
    f.part = part;
    f.mould = tool;
    Ok(f)
}

/// The names of a slab's faces but its top, at `top` along the frame's z.
fn slab_names(slab: &Body, frame: &Frame, top: f64) -> Vec<Name> {
    slab.face_ids()
        .filter(|&f| {
            let face = slab.face(f);
            !matches!(&face.surface, forge_geom::Surface::Plane(p)
                if p.frame.z().dot(frame.z().get()).abs() > 1.0 - 1e-9
                    && (frame.point_to_local(p.frame.origin).z - top).abs() < tol::LINEAR
                    && face.same_sense == (p.frame.z().dot(frame.z().get()) > 0.0))
        })
        .map(|f| slab.face(f).name)
        .collect()
}

/// The field's height at a point (the nearest cell's).
fn height_at(field: &Field, p: Point2) -> f64 {
    let i = (((p.x - field.x0) / field.h) as usize).min(field.nx - 1);
    let j = (((p.y - field.y0) / field.h) as usize).min(field.ny - 1);
    let z = field.z[j * field.nx + i];
    if z.is_nan() { 0.0 } else { z }
}
