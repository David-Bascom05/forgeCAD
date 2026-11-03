//! The sheet features past base, flange and cutout (forge-sheet's contour, ...), as Forge calls.
//! A child of `sheet`, so it shares its state, anchors and error rendering; `sheet` hooks each
//! call and each new kernel refusal here.
use super::*;
use forge_ops::PathSeg;
use forge_sheet::contour::ContourError;

/// The language's own refusals for these features.
#[derive(Clone, Debug, PartialEq)]
pub enum FeatErr {
    /// A profile piece off the section it must lie in (`distance` off it, NaN when its plane is
    /// not the section's).
    OffSection { span: Span, distance: f64 },
    /// `punch("ID", ..)` with no such tool in the sheet's shop.
    NoSuchTool { span: Span, tool: String },
    /// A tool size the shop could not have: a cluster count not whole, not finite, or outside 1
    /// to `limit`.
    ToolSize {
        span: Span,
        which: &'static str,
        value: f64,
        limit: f64,
    },
}

impl FeatErr {
    pub fn span(&self) -> Span {
        match self {
            FeatErr::OffSection { span, .. }
            | FeatErr::NoSuchTool { span, .. }
            | FeatErr::ToolSize { span, .. } => *span,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            FeatErr::OffSection { .. } => "ProfileOffSection",
            FeatErr::NoSuchTool { .. } => "NoSuchTool",
            FeatErr::ToolSize { .. } => "PunchSize",
        }
    }
    pub fn text(&self) -> String {
        match self {
            FeatErr::OffSection { distance, .. } => format!(
                "the profile does not lie in the section across the edge ({} off): sketch it on a plane square to the edge",
                crate::values::fmt(*distance)
            ),
            FeatErr::ToolSize {
                which, value, limit, ..
            } => format!(
                "tool {which} = {}: a cluster count is a whole number from 1 to {}",
                crate::values::fmt(*value),
                crate::values::fmt(*limit)
            ),
            FeatErr::NoSuchTool { tool, .. } => {
                format!("no tool {tool} in the sheet's shop: add it to shop(tools: ..) or write it out")
            }
        }
    }
    pub fn data(&self) -> Vec<(&'static str, Json)> {
        match self {
            FeatErr::OffSection { distance, .. } => vec![("distance", (*distance).into())],
            FeatErr::NoSuchTool { tool, .. } => vec![("tool", tool.as_str().into())],
            FeatErr::ToolSize {
                which, value, limit, ..
            } => vec![
                ("size", (*which).into()),
                ("value", (*value).into()),
                ("limit", (*limit).into()),
            ],
        }
    }
}

fn feat_err(e: FeatErr) -> Error {
    sheet_err(SheetErr::Feat(e))
}

// ---- kernel refusals of these features ---------------------------------------------------------

pub(super) fn kernel_kind(e: &SheetError) -> Option<&'static str> {
    use ContourError::*;
    Some(match e {
        SheetError::Contour { why, .. } => match why {
            Empty => "ContourEmpty",
            Spline { .. } => "ContourSpline",
            Closed { .. } => "ContourClosed",
            Gap { .. } => "ContourGap",
            ArcAtEnd { .. } => "ContourArcAtEnd",
            ArcsMeet { .. } => "ContourArcsMeet",
            NotTangent { .. } => "ContourNotTangent",
            Turn { .. } => "ContourTurn",
            RadiusBelowMin { .. } => "ContourRadius",
            WallTooShort { .. } => "ContourWallTooShort",
            StartOffEdge { .. } => "ContourStartOffEdge",
            Coplanar { .. } => "ContourCoplanar",
            Width { .. } => "ContourWidth",
        },
        SheetError::Hem { why, .. } => hem_kind(why),
        SheetError::Seam { why, .. } => seam_kind(why),
        SheetError::Punch { .. } => "PunchSize",
        _ => return None,
    })
}

pub(super) fn kernel_text(e: &SheetError) -> Option<String> {
    use ContourError::*;
    let fm = crate::values::fmt;
    let deg = |a: f64| fm(a.to_degrees());
    Some(match e {
        SheetError::Contour { why, .. } => match *why {
            Empty => "the contour's profile has no lines".into(),
            Spline { piece } => format!("profile piece {piece} is a spline: a contour takes lines and arcs"),
            Closed { piece } => format!("profile piece {piece} is a whole circle: a contour is open"),
            Gap { piece, gap, .. } => format!(
                "profile piece {piece} starts {} from where the one before ends",
                fm(gap)
            ),
            ArcAtEnd { piece } => {
                format!("profile piece {piece} is an arc at an end: a contour starts and ends with lines")
            }
            ArcsMeet { piece } => format!(
                "profile pieces {} and {piece} are arcs in a row: a bend needs a wall either side (tangent joints are not built yet)",
                piece - 1
            ),
            NotTangent { piece, angle, .. } => format!(
                "arc {piece} meets a line at {}deg: arcs in a contour are tangent to their lines",
                deg(angle)
            ),
            Turn { piece, angle, .. } => format!(
                "profile piece {piece} turns {}deg: a bend turns less than 180deg",
                deg(angle)
            ),
            RadiusBelowMin { piece, radius, limit } => format!(
                "arc {piece} makes a bend of inside radius {}, below {} (an arc turning towards the material bends at its radius less t)",
                fm(radius),
                fm(limit)
            ),
            WallTooShort { piece, length, .. } => format!(
                "line {piece} has {} left between its bends' setbacks; it needs more than nothing",
                fm(length)
            ),
            StartOffEdge { gap, .. } => format!("the profile starts {} off the edge", fm(gap)),
            Coplanar { .. } => {
                "the profile's first line carries on in the wall's plane: start it with a turn".into()
            }
            Width { width, .. } => format!("width {} is not positive", fm(width)),
        },
        SheetError::Hem { why, .. } => hem_text(why),
        SheetError::Seam { why, .. } => seam_text(why),
        SheetError::Punch {
            why: fs::PunchError::Size { which, value, limit },
            ..
        } => format!("tool size {which} = {} (needs past {})", fm(*value), fm(*limit)),
        _ => return None,
    })
}

pub(super) fn kernel_data(e: &SheetError) -> Option<Vec<(&'static str, Json)>> {
    use ContourError::*;
    let f = |x: f64| Json::Num(x);
    let p = |x: usize| Json::Num(x as f64);
    Some(match e {
        SheetError::Contour { why, .. } => match *why {
            Empty => vec![],
            Spline { piece }
            | Closed { piece }
            | ArcAtEnd { piece }
            | ArcsMeet { piece }
            | Coplanar { piece } => {
                vec![("piece", p(piece))]
            }
            Gap { piece, gap, limit } => vec![("piece", p(piece)), ("gap", f(gap)), ("limit", f(limit))],
            NotTangent { piece, angle, limit } | Turn { piece, angle, limit } => vec![
                ("piece", p(piece)),
                ("angle", f(angle.to_degrees())),
                ("limit", f(limit.to_degrees())),
            ],
            RadiusBelowMin { piece, radius, limit } => {
                vec![("piece", p(piece)), ("radius", f(radius)), ("limit", f(limit))]
            }
            WallTooShort { piece, length, limit } => {
                vec![("piece", p(piece)), ("length", f(length)), ("limit", f(limit))]
            }
            StartOffEdge { gap, limit } => vec![("gap", f(gap)), ("limit", f(limit))],
            Width { width, limit } => vec![("width", f(width)), ("limit", f(limit))],
        },
        SheetError::Hem { why, .. } => hem_data(why),
        SheetError::Seam { why, .. } => seam_data(why),
        SheetError::Punch {
            why: fs::PunchError::Size { which, value, limit },
            ..
        } => vec![
            ("size", (*which).into()),
            ("value", f(*value)),
            ("limit", f(*limit)),
        ],
        _ => return None,
    })
}

// ---- profiles ----------------------------------------------------------------------------------

/// A path value's pieces in `section`'s xy; refused when they leave its plane (or, `along` an
/// edge, a plane parallel to it).
fn section_pieces(v: Value, span: Span, section: &Frame, along: bool) -> R<Vec<Segment>> {
    let path = crate::shapes::path_of(v, span)?;
    let n = section.z().get();
    let local = |p: Point3| section.point_to_local(p);
    let mut out = Vec::with_capacity(path.len());
    let mut z0: Option<f64> = None;
    let mut off = 0.0f64;
    let mut on = |z: f64| {
        // on the section (base), or on one plane square to the edge (along it)
        let at = if along { *z0.get_or_insert(z) } else { 0.0 };
        off = off.max((z - at).abs());
    };
    for p in &path {
        out.push(match p {
            PathSeg::Line { a, b } => {
                let (a, b) = (local(*a), local(*b));
                on(a.z);
                on(b.z);
                Segment::Line {
                    a: Point2::new(a.x, a.y),
                    b: Point2::new(b.x, b.y),
                }
            }
            PathSeg::Arc { circle, start, sweep } => {
                let cz = circle.frame.z().get().dot(n);
                if (cz.abs() - 1.0).abs() > 1e-9 {
                    return Err(feat_err(FeatErr::OffSection {
                        span,
                        distance: f64::NAN,
                    }));
                }
                let c = local(circle.frame.origin);
                on(c.z);
                let (s, k) = dmath::sin_cos(*start);
                let d = section.vec_to_local(circle.frame.vec_to_world(Vec3::new(k, s, 0.0)));
                Segment::Arc {
                    center: Point2::new(c.x, c.y),
                    radius: circle.radius,
                    start: dmath::atan2(d.y, d.x),
                    sweep: if cz > 0.0 { *sweep } else { -*sweep },
                }
            }
            // refused by the kernel, by name, as a spline piece
            PathSeg::Spline(c) => {
                let (a, b) = c.ends();
                let (a, b) = (local(a), local(b));
                let pts = vec![Point2::new(a.x, a.y), Point2::new(b.x, b.y)];
                match PlaneSpline::control(1, pts, None, None) {
                    Some(sp) => Segment::Spline(std::sync::Arc::new(sp)),
                    None => Segment::Circle {
                        center: Point2::ORIGIN,
                        radius: 1.0,
                        ccw: true,
                    },
                }
            }
        });
    }
    if off > 1e-7 {
        return Err(feat_err(FeatErr::OffSection { span, distance: off }));
    }
    Ok(out)
}

fn seg_ends(s: &Segment) -> (Point2, Point2) {
    match *s {
        Segment::Line { a, b } => (a, b),
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => {
            let at = |t: f64| {
                let (s, c) = dmath::sin_cos(t);
                Point2::new(center.x + radius * c, center.y + radius * s)
            };
            (at(start), at(start + sweep))
        }
        Segment::Circle { center, radius, .. } => {
            let p = Point2::new(center.x + radius, center.y);
            (p, p)
        }
        Segment::Spline(ref s) => (s.start(), s.end()),
    }
}

/// The profile run the other way.
fn reversed(p: Vec<Segment>) -> Vec<Segment> {
    p.into_iter()
        .rev()
        .map(|s| match s {
            Segment::Line { a, b } => Segment::Line { a: b, b: a },
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => Segment::Arc {
                center,
                radius,
                start: start + sweep,
                sweep: -sweep,
            },
            o => o,
        })
        .collect()
}

// ---- contour -----------------------------------------------------------------------------------

/// `contour(profile, width, t, r, bend, side, shop)`: a sheet of the profile's walls and bends;
/// `contour(profile, edge)`: the same run off a straight edge of a sheet, as long as the edge.
pub(super) fn contour(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("contour", span)?;
    let a = Args::bind(
        ev,
        "contour",
        &["profile", "edge", "width", "t", "r", "bend", "side", "shop"],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    match a.opt("edge") {
        Some(v) => contour_on(ev, &a, v, id, span),
        None => contour_base(ev, &a, id, span),
    }
}

/// The least inside radius the shop bends sheet `t` at, if it stocks it.
fn min_radius(shop: Option<&Rc<ShopV>>, t: f64) -> f64 {
    shop.and_then(|s| s.shop.stock(t))
        .and_then(|s| s.radii.iter().copied().reduce(f64::min))
        .unwrap_or(0.0)
}

fn contour_base(ev: &mut Eval, a: &Args, id: String, span: Span) -> R<Value> {
    let makes = |f: &crate::eval::FeatInfo| {
        !matches!(
            ev.b.features.get(f.idx),
            None | Some(Feature::Sketch(_) | Feature::Param(_) | Feature::Plane(_) | Feature::Work(_))
        )
    };
    if let Some(f) = ev.feats.iter().find(|f| makes(f) && f.status != Status::Failed) {
        return Err(sheet_err(SheetErr::SheetOnSolid {
            span,
            id,
            solid: f.id.clone(),
        }));
    }
    let pv = a.req("profile")?;
    let sk = match &pv {
        Value::Sketch(s) => s.clone(),
        Value::Ent(e) => e.sk.clone(),
        Value::List(xs) => match xs.first() {
            Some(Value::Ent(e)) => e.sk.clone(),
            _ => return Err(wrong(a.span("profile"), "a contour's profile", Ty::Sketch, &pv)),
        },
        o => return Err(wrong(a.span("profile"), "a contour's profile", Ty::Sketch, o)),
    };
    let section = sk.borrow().frame;
    let profile = section_pieces(pv, a.span("profile"), &section, false)?;
    let width = a.req_num("width", LEN)?;
    let t = a.req_num("t", LEN)?;
    let shop = match a.opt("shop") {
        None => None,
        Some(Value::Sheet(s)) if matches!(*s, SheetVal::Shop(_)) => {
            let SheetVal::Shop(x) = &*s else {
                unreachable!("matched")
            };
            Some(x.clone())
        }
        Some(o) => return Err(wrong(a.span("shop"), "shop", Ty::Shop, &o)),
    };
    let stock = shop.as_ref().and_then(|s| {
        s.stocks
            .iter()
            .find(|x| (x.stock.thickness - t.v).abs() <= 1e-9 * t.v.abs().max(1.0))
            .cloned()
    });
    let r = match (a.num("r", LEN)?, &stock) {
        (Some(r), _) => r,
        (None, Some(s)) => s.radii[0].clone(),
        (None, None) => {
            return Err(Error::MissingArg {
                span,
                func: "contour",
                arg: "r",
            });
        }
    };
    let rule = match (rule_of(a, "bend")?, &stock) {
        (Some(r), _) => r,
        (None, Some(s)) => s.rule.clone(),
        (None, None) => {
            return Err(Error::MissingArg {
                span,
                func: "contour",
                arg: "bend",
            });
        }
    };
    let hand = match word(a, "side", &["left", "right"])?.unwrap_or("right") {
        "left" => fs::Hand::Left,
        _ => fs::Hand::Right,
    };
    let mut dims = vec![dim("width", &width), dim("thickness", &t), dim("radius", &r)];
    if let Some(k) = &rule.k {
        dims.push(dim("k", k));
    }
    let hid = history_id(&id);
    let gauge = fs::Gauge {
        thickness: t.v,
        radius: r.v,
        rule: rule.rule.clone(),
    };
    // reliefs from the shop's stock of this thickness, else the stand-in default
    let relief = shop
        .as_ref()
        .and_then(|s| s.shop.relief_rule(t.v))
        .unwrap_or_default();
    let fix = FixCtx {
        flanges: vec![],
        t: t.clone(),
        r: r.clone(),
        k: rule.k.clone(),
        relief,
    };
    let least = min_radius(shop.as_ref(), t.v);
    let (mut sheet, made) =
        match fs::Sheet::contour(fs::FeatureId(hid), gauge, section, &profile, width.v, hand, least) {
            Ok(x) => x,
            Err(error) => {
                let names = Names {
                    walls: vec![(WallId(0), id.clone())],
                    ..Names::default()
                };
                return Err(kernel_error(&fix, names, span, &id, error));
            }
        };
    sheet.rule = relief;
    let si = ev.sheets.list.len();
    let walls: Vec<(WallId, String, Span)> = made.walls.iter().map(|w| (w.1, id.clone(), span)).collect();
    let joints: Vec<(JointId, String, Span)> = made.joints.iter().map(|j| (j.1, id.clone(), span)).collect();
    let (first, last) = (made.walls[0].1, made.walls[made.walls.len() - 1].1);
    ev.sheets.list.push(SheetState {
        id: id.clone(),
        hid,
        idx: ev.b.features.len(),
        sheet,
        sketch: sk.clone(),
        shop,
        walls,
        joints,
        cuts: vec![],
        corners: vec![],
        cut_srcs: vec![],
        fix,
        folded: None,
        unfolds: vec![],
    });
    push_sheet_step(ev, si, &id, "contour", span, dims)?;
    Ok(op_value(si, &id, OpKind::Contour { first, last }))
}

fn contour_on(ev: &mut Eval, a: &Args, ev_edge: Value, id: String, span: Span) -> R<Value> {
    let edge = match &ev_edge {
        Value::Sheet(s) => match &**s {
            SheetVal::Edge(e) => e.clone(),
            _ => return Err(wrong(a.span("edge"), "a sheet edge", Ty::SheetEdge, &ev_edge)),
        },
        o => return Err(wrong(a.span("edge"), "a sheet edge", Ty::SheetEdge, o)),
    };
    for k in ["width", "t", "r", "bend", "side", "shop"] {
        if a.opt(k).is_some() {
            return Err(Error::NotWith {
                span: a.span(k),
                func: "contour",
                arg: k,
                with: "edge (a contour off an edge is as long as it, and takes its sheet's gauge)",
            });
        }
    }
    let si = edge.op.sheet;
    not_unfolded(ev, si, &id, span)?;
    let st = &ev.sheets.list[si];
    let section = (|| {
        let w = st.sheet.wall(edge.wall)?;
        let (p, q) = w.outline.iter().flat_map(|l| &l.segments).find_map(|s| match s {
            (Segment::Line { a, b }, sid) if *sid == edge.segment => Some((*a, *b)),
            _ => None,
        })?;
        let f = wall_frame(st, edge.wall, None)?;
        let d = (q - p).normalize()?;
        let x = f.vec_to_world(Vec3::new(d.y, -d.x, 0.0));
        let y = f.z().get();
        Frame::from_axes(f.point_to_world(Point3::new(p.x, p.y, 0.0)), x, y, x.cross(y))
    })();
    let Some(section) = section else {
        // nothing built to run it off (check, or an earlier failure): kept, the graph as it was
        push_sheet_step(ev, si, &id, "contour", span, vec![])?;
        return Ok(op_value(
            si,
            &id,
            OpKind::Contour {
                first: WallId(u32::MAX),
                last: WallId(u32::MAX),
            },
        ));
    };
    let mut profile = section_pieces(a.req("profile")?, a.span("profile"), &section, true)?;
    // run from the end on the edge
    if let (Some(f), Some(l)) = (profile.first(), profile.last()) {
        let start = seg_ends(f).0;
        let end = seg_ends(l).1;
        if end.distance(Point2::ORIGIN) < start.distance(Point2::ORIGIN) {
            profile = reversed(profile);
        }
    }
    let st = &mut ev.sheets.list[si];
    let least = min_radius(st.shop.as_ref(), st.sheet.gauge.thickness);
    let made = match st.sheet.contour_on(edge.wall, edge.segment, &profile, least) {
        Ok(m) => m,
        Err(e) => {
            let mut snap = st.clone();
            let next = snap.walls.len() as u32;
            snap.walls.push((WallId(next), id.clone(), span));
            return Err(kernel_err(&snap, span, &id, e));
        }
    };
    for w in &made.walls {
        st.walls.push((w.1, id.clone(), span));
    }
    for j in &made.joints {
        st.joints.push((j.1, id.clone(), span));
    }
    let (first, last) = (made.walls[0].1, made.walls[made.walls.len() - 1].1);
    push_sheet_step(ev, si, &id, "contour", span, vec![])?;
    Ok(op_value(si, &id, OpKind::Contour { first, last }))
}

/// The walls and bends a feature made (a contour's several), by its id.
pub(super) fn parts(st: &SheetState, op: &OpRef) -> (Vec<WallId>, Vec<JointId>) {
    (
        st.walls.iter().filter(|w| w.1 == op.id).map(|w| w.0).collect(),
        st.joints.iter().filter(|j| j.1 == op.id).map(|j| j.0).collect(),
    )
}

fn hem_kind(e: &fs::HemError) -> &'static str {
    use fs::HemError::*;
    match e {
        Gap { .. } => "HemGap",
        Angle { .. } => "HemAngle",
        Radius { .. } => "HemRadius",
        Leg { .. } => "HemLeg",
        NoRoom { .. } => "HemNoRoom",
        NotTabled { .. } => "HemNotTabled",
    }
}

fn hem_text(e: &fs::HemError) -> String {
    use fs::HemError::*;
    let fm = crate::values::fmt;
    match *e {
        Gap { gap, limit } => format!(
            "a hem gap of {} is under the floor {} (a solid has no touching skins)",
            fm(gap),
            fm(limit)
        ),
        Angle { kind, angle, lo, hi } => format!(
            "a {} hem turning {}deg: it turns between {}deg and {}deg",
            kind.name(),
            fm(angle.to_degrees()),
            fm(lo.to_degrees()),
            fm(hi.to_degrees())
        ),
        Radius { radius, .. } => format!("hem radius {} is not positive", fm(radius)),
        Leg { kind, leg, .. } => format!("a {} hem's leg of {} is not positive", kind.name(), fm(leg)),
        NoRoom { have, need, .. } => format!(
            "the wall reaches {} from the edge; the hem's fold takes {}",
            fm(have),
            fm(need)
        ),
        NotTabled { kind, thickness } => format!(
            "a {} hem in {} sheet has no allowance in the shop's hem table (no radius formula holds for it)",
            kind.name(),
            fm(thickness)
        ),
    }
}

fn hem_data(e: &fs::HemError) -> Vec<(&'static str, Json)> {
    use fs::HemError::*;
    let f = |x: f64| Json::Num(x);
    match *e {
        Gap { gap, limit } => vec![("gap", f(gap)), ("limit", f(limit))],
        Angle { kind, angle, lo, hi } => vec![
            ("hem", kind.name().into()),
            ("angle", f(angle.to_degrees())),
            ("lo", f(lo.to_degrees())),
            ("hi", f(hi.to_degrees())),
        ],
        Radius { radius, limit } => vec![("radius", f(radius)), ("limit", f(limit))],
        Leg { kind, leg, limit } => vec![("hem", kind.name().into()), ("leg", f(leg)), ("limit", f(limit))],
        NoRoom { have, need, .. } => vec![("have", f(have)), ("need", f(need))],
        NotTabled { kind, thickness } => vec![("hem", kind.name().into()), ("thickness", f(thickness))],
    }
}

// ---- hems -------------------------------------------------------------------------------------

/// A shop's hems: `closed_hems: [(t, allowance), ..]` (no radius formula holds for a crushed
/// hem), `hem_gap: G` the gap a closed hem is modelled with.
pub(super) fn hem_table(a: &Args) -> R<fs::HemTable> {
    let rows = match a.opt("closed_hems") {
        None => vec![],
        Some(_) => table(a, "closed_hems", &[LEN, LEN])?,
    };
    Ok(fs::HemTable {
        closed_gap: a.num("hem_gap", LEN)?.map_or(fs::GAP_FLOOR, |g| g.v),
        rows: rows
            .iter()
            .map(|r| fs::HemRow {
                kind: fs::HemName::Closed,
                thickness: r[0],
                radius: 0.0,
                angle: dmath::PI,
                allowance: r[1],
            })
            .collect(),
    })
}

/// `hem(EDGE, kind, length, gap, r, angle, side, position)`.
pub(super) fn hem(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("hem", span)?;
    let a = Args::bind(
        ev,
        "hem",
        &["edge", "kind", "length", "gap", "r", "angle", "side", "position"],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let ev_edge = a.req("edge")?;
    let edge = match &ev_edge {
        Value::Sheet(s) => match &**s {
            SheetVal::Edge(e) => e.clone(),
            _ => return Err(wrong(a.span("edge"), "a sheet edge", Ty::SheetEdge, &ev_edge)),
        },
        o => return Err(wrong(a.span("edge"), "a sheet edge", Ty::SheetEdge, o)),
    };
    let si = edge.op.sheet;
    not_unfolded(ev, si, &id, span)?;
    let kind = word(&a, "kind", &["closed", "open", "teardrop", "rolled"])?.unwrap_or("closed");
    let mut dims = vec![];
    let num = |k: &'static str, dims_: &[Dim], dims: &mut Vec<Dimension>| -> R<Option<Num>> {
        let n = a.num(k, dims_)?;
        if let Some(n) = &n {
            dims.push(dim(k, n));
        }
        Ok(n)
    };
    let length = num("length", LEN, &mut dims)?;
    let gap = num("gap", LEN, &mut dims)?;
    let r = num("r", LEN, &mut dims)?;
    let angle = num("angle", ANG, &mut dims)?;
    let need = |n: Option<Num>, arg: &'static str| {
        n.map(|n| n.v).ok_or(Error::MissingArg {
            span,
            func: "hem",
            arg,
        })
    };
    let st = &ev.sheets.list[si];
    let radius = r.as_ref().map_or(st.sheet.gauge.radius, |n| n.v);
    let hk = match kind {
        "closed" => fs::HemKind::Closed,
        "open" => fs::HemKind::Open {
            gap: need(gap, "gap")?,
        },
        "teardrop" => fs::HemKind::Teardrop {
            radius,
            angle: need(angle, "angle")?,
        },
        _ => fs::HemKind::Rolled {
            radius,
            angle: need(angle, "angle")?,
        },
    };
    let leg = match (kind, length) {
        ("rolled", None) => 0.0,
        (_, l) => need(l, "length")?,
    };
    let up = word(&a, "side", &["up", "down"])?.unwrap_or("up") == "up";
    let outside = word(&a, "position", &["inside", "outside"])?.unwrap_or("inside") == "outside";
    let table = st.shop.as_ref().map(|s| s.hems.clone()).unwrap_or_default();
    let st = &mut ev.sheets.list[si];
    let spec = fs::HemSpec { kind: hk, leg, up };
    let (wall, joint) = match st
        .sheet
        .hem_at(edge.wall, edge.segment, spec, Some(&table), outside)
    {
        Ok(x) => x,
        Err(e) => {
            let mut snap = st.clone();
            snap.walls
                .push((WallId(snap.walls.len() as u32), id.clone(), span));
            snap.joints
                .push((JointId(snap.joints.len() as u32), id.clone(), span));
            return Err(kernel_err(&snap, span, &id, e));
        }
    };
    st.walls.push((wall, id.clone(), span));
    st.joints.push((joint, id.clone(), span));
    push_sheet_step(ev, si, &id, "hem", span, dims)?;
    Ok(op_value(si, &id, OpKind::Flange { wall, joint }))
}

fn seam_kind(e: &fs::SeamError) -> &'static str {
    use fs::SeamError::*;
    match e {
        NotACorner => "SeamNotACorner",
        NotAFlange { .. } => "SeamNotAFlange",
        TipTaken { .. } => "SeamTipTaken",
        Gap { .. } => "SeamGap",
        Parallel { .. } => "SeamParallel",
        NoRoom { .. } => "SeamNoRoom",
    }
}

fn seam_text(e: &fs::SeamError) -> String {
    use fs::SeamError::*;
    let fm = crate::values::fmt;
    match *e {
        NotACorner => "the two flanges do not turn off edges of one wall meeting at a corner".into(),
        NotAFlange { .. } => "a seam joins plain flanges (a wall off its whole edge)".into(),
        TipTaken { .. } => "the flange already carries a feature on its tip or sides: seam it first".into(),
        Gap { gap, limit } => format!("a seam gap of {} is under the floor {}", fm(gap), fm(limit)),
        Parallel { .. } => "the seam plane runs along the flange".into(),
        NoRoom { length, .. } => format!("the seam leaves the flange {} long", fm(length)),
    }
}

fn seam_data(e: &fs::SeamError) -> Vec<(&'static str, Json)> {
    use fs::SeamError::*;
    let f = |x: f64| Json::Num(x);
    match *e {
        Gap { gap, limit } => vec![("gap", f(gap)), ("limit", f(limit))],
        NoRoom { length, .. } => vec![("length", f(length))],
        _ => vec![],
    }
}

// ---- seams ------------------------------------------------------------------------------------

/// `seam(a, b, kind: "overlap"|"underlap"|"closed" = "overlap", gap: G)`: flanges a and b, off
/// adjacent edges of one wall, cut to meet at their corner. overlap: a laps over b's end;
/// underlap: b over a's. G defaults to the shop's kerf.
pub(super) fn seam(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("seam", span)?;
    let a = Args::bind(ev, "seam", &["a", "b", "kind", "gap"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let joint = |k: &'static str| -> R<(usize, JointId)> {
        let v = a.req(k)?;
        let op = op_of(&v, a.span(k), "a flange")?;
        match op.kind {
            OpKind::Flange { joint, .. } if op.on == On::Folded => Ok((op.sheet, joint)),
            _ => Err(wrong(a.span(k), "a flange", Ty::Sheet, &v)),
        }
    };
    let ((si, ja), (sb, jb)) = (joint("a")?, joint("b")?);
    if si != sb {
        return Err(sheet_err(SheetErr::NotACorner { span, id }));
    }
    not_unfolded(ev, si, &id, span)?;
    let kind = word(&a, "kind", &["overlap", "underlap", "closed"])?.unwrap_or("overlap");
    let gap = a.num("gap", LEN)?;
    let mut dims = vec![];
    if let Some(g) = &gap {
        dims.push(dim("gap", g));
    }
    let st = &mut ev.sheets.list[si];
    let g = gap.map_or(st.sheet.rule.kerf.max(fs::GAP_FLOOR), |g| g.v);
    let (x, y, seam) = match kind {
        "closed" => (ja, jb, fs::Seam::Closed { gap: g }),
        "underlap" => (jb, ja, fs::Seam::Overlap { gap: g }),
        _ => (ja, jb, fs::Seam::Overlap { gap: g }),
    };
    if let Err(e) = st.sheet.seam(x, y, seam) {
        return Err(kernel_err(st, span, &id, e));
    }
    st.corners.push((ja, jb, id.clone(), span));
    push_sheet_step(ev, si, &id, "seam", span, dims)?;
    Ok(op_value(si, &id, OpKind::Corner { a: ja, b: jb }))
}

// ---- punches ----------------------------------------------------------------------------------

/// A tool written as a tuple: ("ID", "round", D) | ("ID", "obround", L, W) | ("ID", "square", S)
/// | ("ID", "rect", L, W) | ("ID", "d", D, FLAT) | ("ID", "keyhole", D, W, L) | ("ID", "hex", AF)
/// | ("ID", "cluster", D, PX, PY, NX, NY).
fn tool_of(v: &Value, span: Span) -> R<fs::Tool> {
    let bad = || Error::WrongType {
        span,
        what: "a tool: (\"ID\", \"round\"|\"obround\"|\"square\"|\"rect\"|\"d\"|\"keyhole\"|\"hex\"|\"cluster\", sizes..)",
        want: Ty::Tuple,
        found: v.ty(),
    };
    let Value::Tuple(xs) = v else { return Err(bad()) };
    let (Some(Value::Str(id)), Some(Value::Str(kind))) = (xs.first(), xs.get(1)) else {
        return Err(bad());
    };
    let n = |i: usize| -> R<f64> {
        let x = xs.get(i).cloned().ok_or_else(bad)?;
        Ok(num_of(x, &[Dim::Length, Dim::Count, Dim::Ratio], span, "a tool size")?.v)
    };
    // a cluster's counts: whole, finite, 1 to CLUSTER_MAX (the kernel caps their product too)
    let count = |i: usize, which: &'static str| -> R<u32> {
        let v = n(i)?;
        if !v.is_finite() || v < 1.0 || v > fs::CLUSTER_MAX as f64 || v.fract() != 0.0 {
            return Err(feat_err(FeatErr::ToolSize {
                span,
                which,
                value: v,
                limit: fs::CLUSTER_MAX as f64,
            }));
        }
        Ok(v as u32)
    };
    let shape = match &**kind {
        "round" => fs::ToolShape::Round { d: n(2)? },
        "obround" => fs::ToolShape::Obround {
            length: n(2)?,
            width: n(3)?,
        },
        "square" => fs::ToolShape::Square { side: n(2)? },
        "rect" => fs::ToolShape::Rect {
            length: n(2)?,
            width: n(3)?,
        },
        "d" => fs::ToolShape::D {
            d: n(2)?,
            flat: n(3)?,
        },
        "keyhole" => fs::ToolShape::Keyhole {
            d: n(2)?,
            width: n(3)?,
            length: n(4)?,
        },
        "hex" => fs::ToolShape::Hex { across_flats: n(2)? },
        "cluster" => fs::ToolShape::Cluster {
            d: n(2)?,
            px: n(3)?,
            py: n(4)?,
            nx: count(5, "nx")?,
            ny: count(6, "ny")?,
        },
        _ => return Err(bad()),
    };
    Ok(fs::Tool {
        id: id.to_string(),
        shape,
    })
}

/// `shop(tools: [TOOL, ..])`.
pub(super) fn tools_of(a: &Args) -> R<Vec<fs::Tool>> {
    match a.opt("tools") {
        None => Ok(vec![]),
        Some(Value::List(xs)) => xs.iter().map(|x| tool_of(x, a.span("tools"))).collect(),
        Some(o) => Err(wrong(a.span("tools"), "tools", Ty::List, &o)),
    }
}

/// `shop(punch_web: per_t(..))`: the least web between two hits.
pub(super) fn punch_rules_of(a: &Args) -> R<fs::PunchRules> {
    Ok(match a.opt("punch_web") {
        None => fs::PunchRules::default(),
        Some(_) => fs::PunchRules {
            spacing: scaled_of(a, "punch_web")?,
        },
    })
}

/// `punch(TOOL_ID | TOOL, at: POINT | (x, y), wall: W, angle: A = 0)`: a turret hit on wall W,
/// the tool from the shop's library by id (or written out), at a point sketched on W's top or
/// bottom, or (x, y) in W's own coordinates.
pub(super) fn punch(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let id = ev.take_id("punch", span)?;
    let a = Args::bind(ev, "punch", &["tool", "at", "wall", "angle"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let wv = a.req("wall")?;
    let op = op_of(&wv, a.span("wall"), "the wall the hit goes on")?;
    let wall = match op.kind {
        OpKind::Base { wall } | OpKind::Flange { wall, .. } if op.on == On::Folded => wall,
        _ => {
            return Err(wrong(
                a.span("wall"),
                "a base or flange of the folded sheet",
                Ty::Sheet,
                &wv,
            ));
        }
    };
    let si = op.sheet;
    not_unfolded(ev, si, &id, span)?;
    let tool = match a.req("tool")? {
        Value::Str(name) => {
            let st = &ev.sheets.list[si];
            let found = st
                .shop
                .as_ref()
                .and_then(|s| s.shop.tools.iter().find(|t| *t.id == *name).cloned());
            match found {
                Some(t) => t,
                None => {
                    return Err(feat_err(FeatErr::NoSuchTool {
                        span: a.span("tool"),
                        tool: name.to_string(),
                    }));
                }
            }
        }
        v => tool_of(&v, a.span("tool"))?,
    };
    let angle = a.num("angle", ANG)?;
    let mut dims = vec![];
    if let Some(n) = &angle {
        dims.push(dim("angle", n));
    }
    let at = match a.req("at")? {
        Value::Tuple(xs) if xs.len() == 2 => {
            let x = num_of(xs[0].clone(), LEN, a.span("at"), "x")?;
            let y = num_of(xs[1].clone(), LEN, a.span("at"), "y")?;
            dims.push(dim("x", &x));
            dims.push(dim("y", &y));
            Point2::new(x.v, y.v)
        }
        Value::Ent(e) => {
            let st = &ev.sheets.list[si];
            let Some(frame) = wall_frame(st, wall, None) else {
                push_sheet_step(ev, si, &id, "punch", span, vec![])?;
                return Ok(op_value(si, &id, OpKind::Cut { cut: CutId(u32::MAX) }));
            };
            let sk = e.sk.borrow();
            let p = frame.point_to_local(sk.world(sk.pos(e.id)));
            Point2::new(p.x, p.y)
        }
        o => return Err(wrong(a.span("at"), "a point or (x, y)", Ty::Tuple, &o)),
    };
    let st = &mut ev.sheets.list[si];
    let k = match st.sheet.punch(wall, &tool, at, angle.map_or(0.0, |n| n.v)) {
        Ok(k) => k,
        Err(e) => return Err(kernel_err(st, span, &id, e)),
    };
    let cuts = st.sheet.punches[k].cuts.clone();
    for &c in &cuts {
        st.cuts.push((c, id.clone(), span));
    }
    push_sheet_step(ev, si, &id, "punch", span, dims)?;
    Ok(op_value(si, &id, OpKind::Cut { cut: cuts[0] }))
}

pub(super) fn punch_kind(e: &fs::PunchViolation) -> &'static str {
    match e {
        fs::PunchViolation::NotInShop { .. } => "ToolNotInShop",
        fs::PunchViolation::AcrossBend { .. } => "PunchAcrossBend",
        fs::PunchViolation::Spacing { .. } => "PunchSpacing",
    }
}

pub(super) fn punch_text(e: &fs::PunchViolation, n: &Names) -> String {
    let fm = crate::values::fmt;
    match e {
        fs::PunchViolation::NotInShop { tool } => format!("tool {tool} is not in the shop's library"),
        fs::PunchViolation::AcrossBend { joint, depth } => format!(
            "the hit reaches {} into the bend of {}: punch the flat wall clear of its bends",
            fm(*depth),
            n.joint(*joint)
        ),
        fs::PunchViolation::Spacing { other, web, limit } => format!(
            "the web to hit {other} is {}, under the shop's {}",
            fm(*web),
            fm(*limit)
        ),
    }
}

pub(super) fn punch_data(e: &fs::PunchViolation, n: &Names) -> Vec<(&'static str, Json)> {
    match e {
        fs::PunchViolation::NotInShop { tool } => vec![("tool", tool.as_str().into())],
        fs::PunchViolation::AcrossBend { joint, depth } => {
            vec![("joint", n.joint(*joint).into()), ("depth", (*depth).into())]
        }
        fs::PunchViolation::Spacing { other, web, limit } => vec![
            ("other", (*other as f64).into()),
            ("web", (*web).into()),
            ("limit", (*limit).into()),
        ],
    }
}
