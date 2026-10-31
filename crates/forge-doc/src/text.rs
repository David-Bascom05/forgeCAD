//! The part file: a feature history as text, one feature per line (grammar in AGENTS.md).
//! Each feature kind is one row of `KINDS`: its keyword, its syntax (shown as a hint on
//! errors), a parser and a printer built from the shared helpers below.

use crate::{expr, sketch};
use forge_geom::Circle;
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_model::{
    At, AxisBy, BodyFeature, Coil, CoilSize, Combine, Convention, Cutter, Datum, Edit, End, Extent, Feature,
    Fit, FontRef, Head, Hole, Keep, Layout, Loft, Motion, Param, Part, PathEnds, Pick, PlaneBy, PointBy, Ref,
    Rib, Row, ScrewThread, Side, Source, Standard, Step, Sweep, Target, Text, TextOn, Thread, ThreadLength,
    ThreadOn, ThreadSpec, Toward, Turn, UcsBy, Unit, Work, standard, work,
};
use forge_ops::{
    Axis2, ChamferSize, Handed, LoftGuides, LoftKind, PathSeg, Profile, ProfileLoop, Radius, Segment,
};
use forge_topo::Name;

pub struct Kind {
    pub name: &'static str,
    pub syntax: &'static str,
    parse: fn(&mut P) -> R<Feature>,
    /// Writes the feature's arguments, or returns false if it is not this kind.
    print: fn(&Feature, &mut W) -> bool,
}

pub const KINDS: &[Kind] = &[
    Kind {
        name: "extrude",
        syntax: "extrude [op] EXT (FRAME LOOPS | SKETCH [E])",
        parse: |p| {
            let op = p.op()?;
            let extent = p.extent()?;
            let profile = p.profile()?;
            Ok(Feature::Extrude { profile, extent, op })
        },
        print: |f, w| {
            let Feature::Extrude { profile, extent, op } = f else {
                return false;
            };
            w.op(*op);
            w.extent(extent);
            w.profile(profile);
            true
        },
    },
    Kind {
        name: "revolve",
        syntax: "revolve [op] [TURN] (FRAME [axis x|y|OX OY DX DY] LOOPS | SKETCH [axis L] [E])",
        parse: |p| {
            let op = p.op()?;
            let extent = p.turn()?;
            let (profile, axis) = p.axial()?;
            Ok(Feature::Revolve {
                profile,
                axis,
                extent,
                op,
            })
        },
        print: |f, w| {
            let Feature::Revolve {
                profile,
                axis,
                extent,
                op,
            } = f
            else {
                return false;
            };
            w.op(*op);
            w.turn(extent);
            w.frame(&profile.plane);
            w.axis(axis);
            w.loops(&profile.loops);
            true
        },
    },
    Kind {
        name: "ball",
        syntax: "ball [op] R FRAME",
        parse: |p| {
            let op = p.op()?;
            let radius = p.num()?;
            Ok(Feature::Ball {
                frame: p.frame()?,
                radius,
                op,
            })
        },
        print: |f, w| {
            let Feature::Ball { frame, radius, op } = f else {
                return false;
            };
            w.op(*op);
            w.num(*radius);
            w.frame(frame);
            true
        },
    },
    Kind {
        name: "fillet",
        syntax: "fillet R [from REF] REF.. [R [from REF] REF..] [setback VREF [REF] D]..   R: 3 | 2..4 | 0:2,0.5:4,1:2 (at:radius)",
        parse: |p| {
            let (mut edges, mut starts, mut setbacks) = (Vec::new(), Vec::new(), Vec::new());
            let mut r = p.radius()?;
            let mut from = None;
            while !p.done() {
                if p.eat("setback") {
                    let v = p.one_ref()?;
                    let e = if is_ref(p.peek()) {
                        Some(p.one_ref()?)
                    } else {
                        None
                    };
                    setbacks.push((v, e, p.num()?));
                } else if p.eat("from") {
                    from = Some(p.one_ref()?);
                } else if !is_ref(p.peek()) {
                    r = p.radius()?;
                    from = None;
                } else {
                    for n in p.one_or_more_refs()? {
                        edges.push((n, r.clone()));
                        if let Some(v) = from {
                            starts.push((n, v));
                        }
                    }
                }
            }
            Ok(Feature::Fillet {
                edges,
                starts,
                setbacks,
            })
        },
        print: |f, w| {
            let Feature::Fillet {
                edges,
                starts,
                setbacks,
            } = f
            else {
                return false;
            };
            let mut at = None;
            for (n, er) in edges {
                let from = starts.iter().find(|s| s.0 == *n).map(|s| s.1);
                if at != Some((er, from)) {
                    w.radius(er);
                    if let Some(v) = from {
                        w.word("from");
                        w.name(v);
                    }
                    at = Some((er, from));
                }
                w.name(*n);
            }
            for &(v, e, d) in setbacks {
                w.word("setback");
                w.name(v);
                if let Some(e) = e {
                    w.name(e);
                }
                w.num(d);
            }
            true
        },
    },
    Kind {
        name: "facefillet",
        syntax: "facefillet R REF.. to REF..",
        parse: |p| {
            let radius = p.num()?;
            let first = p.refs()?;
            p.expect("to")?;
            Ok(Feature::FaceFillet {
                first,
                second: p.refs()?,
                radius,
            })
        },
        print: |f, w| {
            let Feature::FaceFillet {
                first,
                second,
                radius,
            } = f
            else {
                return false;
            };
            w.num(*radius);
            w.names(first);
            w.word("to");
            w.names(second);
            true
        },
    },
    Kind {
        name: "fullround",
        syntax: "fullround SIDE MIDDLE SIDE",
        parse: |p| {
            Ok(Feature::FullRound {
                side: p.one_ref()?,
                middle: p.one_ref()?,
                other: p.one_ref()?,
            })
        },
        print: |f, w| {
            let Feature::FullRound { side, middle, other } = f else {
                return false;
            };
            w.name(*side);
            w.name(*middle);
            w.name(*other);
            true
        },
    },
    Kind {
        name: "chamfer",
        syntax: "chamfer D REF.. | chamfer D D2 face REF REF.. | chamfer D ang A face REF REF..",
        parse: |p| {
            let d = p.num()?;
            let size = if p.eat("ang") {
                let angle = p.angle()?;
                p.expect("face")?;
                ChamferSize::DistanceAngle {
                    face: p.one_ref()?,
                    d,
                    angle,
                }
            } else if p.is_num() {
                let d2 = p.num()?;
                p.expect("face")?;
                ChamferSize::Distances {
                    face: p.one_ref()?,
                    d1: d,
                    d2,
                }
            } else {
                ChamferSize::Equal(d)
            };
            Ok(Feature::Chamfer {
                edges: p.refs()?,
                size,
            })
        },
        print: |f, w| {
            let Feature::Chamfer { edges, size } = f else {
                return false;
            };
            match *size {
                ChamferSize::Equal(d) => w.num(d),
                ChamferSize::Distances { face, d1, d2 } => {
                    w.num(d1);
                    w.num(d2);
                    w.word("face");
                    w.name(face);
                }
                ChamferSize::DistanceAngle { face, d, angle } => {
                    w.num(d);
                    w.word("ang");
                    w.angle(angle);
                    w.word("face");
                    w.name(face);
                }
            }
            w.names(edges);
            true
        },
    },
    Kind {
        name: "shell",
        syntax: "shell T [out] REF..",
        parse: |p| {
            let thickness = p.num()?;
            let outward = p.eat("out");
            Ok(Feature::Shell {
                thickness,
                outward,
                open: p.refs()?,
            })
        },
        print: |f, w| {
            let Feature::Shell {
                thickness,
                outward,
                open,
            } = f
            else {
                return false;
            };
            w.num(*thickness);
            if *outward {
                w.word("out");
            }
            w.names(open);
            true
        },
    },
    Kind {
        name: "draft",
        syntax: "draft A FRAME REF..",
        parse: |p| {
            let angle = p.angle()?;
            let neutral = p.frame()?;
            Ok(Feature::Draft {
                faces: p.refs()?,
                neutral,
                angle,
            })
        },
        print: |f, w| {
            let Feature::Draft {
                faces,
                neutral,
                angle,
            } = f
            else {
                return false;
            };
            w.angle(*angle);
            w.frame(neutral);
            w.names(faces);
            true
        },
    },
    Kind {
        name: "hole",
        syntax: "hole (D | SIZE [iso|ansi] [close|normal|loose|tap] [cbore|csink]) FRAME [cb D2 DEPTH | cs D2 A] (N | thru | next | to FEAT [@face] [off N]) [pt A] [thread SIZE PITCH]",
        parse: |p| {
            let std = p.standard()?;
            let diameter = match std {
                Some(s) => s.diameter(),
                None => p.num()?,
            };
            let std_head = match std {
                Some(s) if p.eat("cbore") => Some(s.counterbore()),
                Some(s) if p.eat("csink") => Some(
                    s.countersink()
                        .ok_or_else(|| format!("no standard countersink for {}", s.screw.name))?,
                ),
                _ => None,
            };
            let at = p.frame()?;
            let head = if let Some(h) = std_head {
                h
            } else if p.eat("cb") {
                Head::Counterbore {
                    diameter: p.num()?,
                    depth: p.num()?,
                }
            } else if p.eat("cs") {
                Head::Countersink {
                    diameter: p.num()?,
                    angle: p.angle()?,
                }
            } else {
                Head::Plain
            };
            let end = if p.eat("thru") {
                End::Through
            } else if p.eat("next") {
                End::ToNext
            } else if p.eat("to") {
                let target = p.target()?;
                End::To {
                    target,
                    offset: p.off(false)?,
                }
            } else {
                End::Depth(p.num()?)
            };
            let point = if p.eat("pt") { Some(p.angle()?) } else { None };
            let thread = if p.eat("thread") {
                // `thread M36x2` as well as `thread 36 2`
                match ThreadSpec::parse(p.peek()) {
                    Some(s) => {
                        p.k += 1;
                        Some(Thread {
                            size: s.major(),
                            pitch: s.pitch(),
                        })
                    }
                    None => Some(Thread {
                        size: p.num()?,
                        pitch: p.num()?,
                    }),
                }
            } else {
                std.and_then(|s| s.thread())
            };
            Ok(Feature::Hole(Box::new(Hole {
                at,
                diameter,
                head,
                end,
                point,
                thread,
                standard: std,
            })))
        },
        print: |f, w| {
            let Feature::Hole(h) = f else { return false };
            // the standard, while the numbers are still its
            let std = h
                .standard
                .filter(|s| s.diameter().to_bits() == h.diameter.to_bits() && s.thread() == h.thread);
            let std_head = std.and_then(|s| {
                if h.head == s.counterbore() {
                    Some("cbore")
                } else if Some(h.head) == s.countersink() {
                    Some("csink")
                } else {
                    None
                }
            });
            match std {
                Some(s) => {
                    w.word(s.screw.name);
                    w.word(CONVENTIONS.iter().find(|c| c.1 == s.convention).expect("named").0);
                    if s.fit != Fit::Normal {
                        w.word(FITS.iter().find(|f| f.1 == s.fit).expect("every fit is named").0);
                    }
                }
                None => w.num(h.diameter),
            }
            if let Some(k) = std_head {
                w.word(k);
            }
            w.frame(&h.at);
            match h.head {
                _ if std_head.is_some() => {}
                Head::Plain => {}
                Head::Counterbore { diameter, depth } => {
                    w.word("cb");
                    w.num(diameter);
                    w.num(depth);
                }
                Head::Countersink { diameter, angle } => {
                    w.word("cs");
                    w.num(diameter);
                    w.angle(angle);
                }
            }
            match h.end {
                End::Through => w.word("thru"),
                End::ToNext => w.word("next"),
                End::Depth(d) => w.num(d),
                End::To { target, offset } => {
                    w.word("to");
                    w.target(&target);
                    w.off(offset, false);
                }
            }
            if let Some(a) = h.point {
                w.word("pt");
                w.angle(a);
            }
            if let (None, Some(t)) = (std, h.thread) {
                w.word("thread");
                w.num(t.size);
                w.num(t.pitch);
            }
            true
        },
    },
    Kind {
        name: "pattern",
        syntax: "pattern (body | FEAT..) (lin DX DY DZ N STEP [DX DY DZ N STEP] | circ OX OY OZ AX AY AZ N [A] [step] | mirror FRAME | path REF.. N (STEP|fit) [fixed] [rev] | pts (X Y Z | REF | SKETCH [P.. | centers])..)",
        parse: |p| {
            let of = if p.eat("body") {
                Source::Body
            } else {
                let mut src = Vec::new();
                while !p.done() && !["lin", "circ", "mirror", "path", "pts"].contains(&p.peek()) {
                    src.push(p.feat()?);
                }
                Source::Features(src)
            };
            let layout = match p.next()? {
                "lin" => {
                    let first = p.row()?;
                    let second = if p.is_num() { Some(p.row()?) } else { None };
                    Layout::Linear { first, second }
                }
                "circ" => Layout::Circular {
                    origin: p.point3()?,
                    axis: p.vec3()?,
                    count: p.count()?,
                    angle: if p.is_num() { p.angle()? } else { dmath::TAU },
                    incremental: p.eat("step"),
                },
                "mirror" => Layout::Mirror { plane: p.frame()? },
                "path" => {
                    let mut along = Vec::new();
                    while !p.done() && !p.is_num() {
                        let feature = p.feat()?;
                        if p.peek().starts_with(['@', '(']) {
                            let names = p.refs()?;
                            along.extend(names.into_iter().map(|n| Ref {
                                feature,
                                name: Some((p.b.own)(feature, n)),
                            }));
                        } else {
                            along.push(Ref { feature, name: None });
                        }
                    }
                    Layout::Path {
                        along,
                        count: p.count()?,
                        spacing: if p.eat("fit") { None } else { Some(p.num()?) },
                        turn: !p.eat("fixed"),
                        reverse: p.eat("rev"),
                    }
                }
                "pts" => {
                    let mut at = Vec::new();
                    while !p.done() {
                        at.push(if p.is_num() {
                            At::Xyz(p.point3()?)
                        } else if let Some(a) = p.sketch_points()? {
                            a
                        } else {
                            At::Ref(p.wref()?)
                        });
                    }
                    Layout::Points(at)
                }
                t => return Err(format!("expected lin, circ, mirror, path or pts, found '{t}'")),
            };
            Ok(Feature::Pattern { of, layout })
        },
        print: |f, w| {
            let Feature::Pattern { of, layout } = f else {
                return false;
            };
            match of {
                Source::Body => w.word("body"),
                Source::Features(src) => src.iter().for_each(|&s| w.feat(s)),
            }
            match layout {
                Layout::Linear { first, second } => {
                    w.word("lin");
                    w.row(first);
                    if let Some(s) = second {
                        w.row(s);
                    }
                }
                Layout::Circular {
                    origin,
                    axis,
                    count,
                    angle,
                    incremental,
                } => {
                    w.word("circ");
                    w.nums(&[origin.x, origin.y, origin.z, axis.x, axis.y, axis.z]);
                    w.word(&count.to_string());
                    if angle.to_bits() != dmath::TAU.to_bits() {
                        w.angle(*angle);
                    }
                    if *incremental {
                        w.word("step");
                    }
                }
                Layout::Mirror { plane } => {
                    w.word("mirror");
                    w.frame(plane);
                }
                Layout::Path {
                    along,
                    count,
                    spacing,
                    turn,
                    reverse,
                } => {
                    w.word("path");
                    let mut last = None;
                    for r in along {
                        if r.name.is_none() || last != Some(r.feature) {
                            w.feat(r.feature);
                        }
                        if let Some(n) = r.name {
                            w.name(n);
                        }
                        last = r.name.map(|_| r.feature);
                    }
                    w.word(&count.to_string());
                    match spacing {
                        Some(s) => w.num(*s),
                        None => w.word("fit"),
                    }
                    if !turn {
                        w.word("fixed");
                    }
                    if *reverse {
                        w.word("rev");
                    }
                }
                Layout::Points(at) => {
                    w.word("pts");
                    for a in at {
                        match a {
                            At::Xyz(p) => w.nums(&[p.x, p.y, p.z]),
                            At::Ref(r) => w.wref(r),
                            At::Sketch {
                                feature,
                                points,
                                centers,
                                ..
                            } => {
                                w.feat(*feature);
                                points.iter().for_each(|t| w.word(t));
                                if *centers {
                                    w.word("centers");
                                }
                            }
                        }
                    }
                }
            }
            true
        },
    },
    Kind {
        name: "sweep",
        syntax: "sweep [op] FRAME LOOPS path PATH [extend N M | to FEAT [@face] [off N]]",
        parse: |p| {
            let op = p.op()?;
            let profile = p.profile()?;
            p.expect("path")?;
            let path = p.path()?;
            let ends = if p.eat("extend") {
                PathEnds::Extended {
                    start: p.num()?,
                    end: p.num()?,
                }
            } else if p.eat("to") {
                let target = p.target()?;
                PathEnds::To {
                    target,
                    offset: p.off(false)?,
                }
            } else {
                PathEnds::Path
            };
            Ok(Feature::Sweep(Box::new(Sweep {
                profile,
                path,
                ends,
                op,
            })))
        },
        print: |f, w| {
            let Feature::Sweep(s) = f else { return false };
            w.op(s.op);
            w.profile(&s.profile);
            w.word("path");
            w.path(&s.path);
            match &s.ends {
                PathEnds::Path => {}
                PathEnds::Extended { start, end } => {
                    w.word("extend");
                    w.nums(&[*start, *end]);
                }
                PathEnds::To { target, offset } => {
                    w.word("to");
                    w.target(target);
                    w.off(*offset, false);
                }
            }
            true
        },
    },
    Kind {
        name: "coil",
        syntax: "coil [op] (FRAME [axis x|y|OX OY DX DY] LOOPS | SKETCH [axis L] [E]) (pitch P turns N | turns N height H | pitch P height H | pitch P to FEAT [@face] [off N]) [taper A] [left]",
        parse: |p| {
            let op = p.op()?;
            let (profile, axis) = p.axial()?;
            let size = if p.eat("pitch") {
                let pitch = p.num()?;
                if p.eat("turns") {
                    CoilSize::PitchTurns {
                        pitch,
                        turns: p.num()?,
                    }
                } else if p.eat("height") {
                    CoilSize::PitchHeight {
                        pitch,
                        height: p.num()?,
                    }
                } else {
                    p.expect("to")?;
                    let target = p.target()?;
                    CoilSize::To {
                        pitch,
                        target,
                        offset: p.off(false)?,
                    }
                }
            } else {
                p.expect("turns")?;
                let turns = p.num()?;
                p.expect("height")?;
                CoilSize::TurnsHeight {
                    turns,
                    height: p.num()?,
                }
            };
            let taper = if p.eat("taper") { p.angle()? } else { 0.0 };
            let handed = if p.eat("left") {
                Handed::Left
            } else {
                Handed::Right
            };
            Ok(Feature::Coil(Box::new(Coil {
                profile,
                axis,
                size,
                taper,
                handed,
                op,
            })))
        },
        print: |f, w| {
            let Feature::Coil(c) = f else { return false };
            w.op(c.op);
            w.frame(&c.profile.plane);
            w.axis(&c.axis);
            w.loops(&c.profile.loops);
            match &c.size {
                CoilSize::PitchTurns { pitch, turns } => {
                    w.word("pitch");
                    w.num(*pitch);
                    w.word("turns");
                    w.num(*turns);
                }
                CoilSize::TurnsHeight { turns, height } => {
                    w.word("turns");
                    w.num(*turns);
                    w.word("height");
                    w.num(*height);
                }
                CoilSize::PitchHeight { pitch, height } => {
                    w.word("pitch");
                    w.num(*pitch);
                    w.word("height");
                    w.num(*height);
                }
                CoilSize::To {
                    pitch,
                    target,
                    offset,
                } => {
                    w.word("pitch");
                    w.num(*pitch);
                    w.word("to");
                    w.target(target);
                    w.off(*offset, false);
                }
            }
            if c.taper.to_bits() != 0 {
                w.word("taper");
                w.angle(c.taper);
            }
            if c.handed == Handed::Left {
                w.word("left");
            }
            true
        },
    },
    Kind {
        name: "loft",
        syntax: "loft [op] [smooth [start W] [end W]] (profile FRAME LOOPS).. [rail I PATH.. | centre PATH]",
        parse: |p| {
            let op = p.op()?;
            let kind = if p.eat("smooth") {
                let start = if p.eat("start") { Some(p.num()?) } else { None };
                let end = if p.eat("end") { Some(p.num()?) } else { None };
                LoftKind::Smooth { start, end }
            } else {
                LoftKind::Ruled
            };
            let mut profiles = Vec::new();
            while p.eat("profile") {
                profiles.push(p.profile()?);
            }
            if profiles.is_empty() {
                return Err(format!("expected 'profile', found '{}'", p.peek()));
            }
            let guides = if p.peek() == "rail" {
                let mut rails = Vec::new();
                while p.eat("rail") {
                    rails.push((p.count()?, p.path()?));
                }
                LoftGuides::Rails(rails)
            } else if p.eat("centre") {
                LoftGuides::CentreLine(p.path()?)
            } else {
                LoftGuides::None
            };
            Ok(Feature::Loft(Box::new(Loft {
                profiles,
                kind,
                guides,
                op,
            })))
        },
        print: |f, w| {
            let Feature::Loft(l) = f else { return false };
            w.op(l.op);
            if let LoftKind::Smooth { start, end } = l.kind {
                w.word("smooth");
                if let Some(x) = start {
                    w.word("start");
                    w.num(x);
                }
                if let Some(x) = end {
                    w.word("end");
                    w.num(x);
                }
            }
            for p in &l.profiles {
                w.word("profile");
                w.profile(p);
            }
            match &l.guides {
                LoftGuides::None => {}
                LoftGuides::Rails(rails) => {
                    for (i, path) in rails {
                        w.word("rail");
                        w.word(&i.to_string());
                        w.path(path);
                    }
                }
                LoftGuides::CentreLine(path) => {
                    w.word("centre");
                    w.path(path);
                }
            }
            true
        },
    },
    Kind {
        name: "param",
        syntax: "NAME = EXPR [mm|deg]",
        parse: |p| {
            let name = p.next()?.to_string();
            p.expect("=")?;
            let mut toks: Vec<&str> = p.t[p.k..].iter().map(|s| s.as_str()).collect();
            p.k = p.t.len();
            let unit = match toks.last() {
                Some(&"mm") => Unit::Mm,
                Some(&"deg") => Unit::Deg,
                _ => Unit::None,
            };
            if unit != Unit::None {
                toks.pop();
            }
            Ok(Feature::Param(Box::new(new_param(
                p.b.features,
                name,
                toks.join(" "),
                unit,
            )?)))
        },
        print: |f, w| {
            let Feature::Param(p) = f else { return false };
            w.s.push_str(&format!("{} = {}", p.name, p.expr));
            match p.unit {
                Unit::None => {}
                Unit::Mm => w.word("mm"),
                Unit::Deg => w.word("deg"),
            }
            true
        },
    },
    Kind {
        name: "sketch",
        syntax: "sketch [FRAME | on REF], then rows to `end`: pt line poly rect circle arc | fix co hor ver par perp tan eq conc mid sym | dist hdist vdist rad dia ang",
        parse: |p| {
            if p.peek() == "-on" && p.t.get(p.k + 1).is_some_and(|t| is_ref(t)) {
                return Err("a sketch goes on a face's outside: `on REF`, not `-on`".into());
            }
            let plane = p.frame()?;
            let on = p.on.take();
            let at = p.at.filter(|_| on.is_some());
            let mut rows = Vec::new();
            while p.eat(";") {
                let start = p.k;
                while !p.done() && p.peek() != ";" {
                    p.k += 1;
                }
                rows.push(p.t[start..p.k].to_vec());
            }
            let fs = p.b.features;
            let was = match p.b.was {
                Some(Feature::Sketch(w)) => Some(&**w),
                _ => None,
            };
            let mut s = sketch::build(plane, rows, &|t, want| value(t, fs, want), was)?;
            (s.on, s.at) = (on, at);
            Ok(Feature::Sketch(Box::new(s)))
        },
        print: |f, w| {
            let Feature::Sketch(s) = f else { return false };
            match &s.on {
                None => w.frame(&s.plane),
                Some(on) => {
                    match on {
                        forge_model::On::Face(n) => {
                            w.word("on");
                            w.name(*n);
                        }
                        forge_model::On::Feature { feature, point, flip } => {
                            w.word(if *flip { "-on" } else { "on" });
                            w.feat(*feature);
                            if let Some(pt) = point {
                                w.word(pt);
                            }
                        }
                    }
                    if let Some(o) = s.at {
                        w.word("at");
                        w.nums(&[o.x, o.y, o.z]);
                    }
                }
            }
            for r in &s.rows {
                w.s.push_str("\n ");
                r.iter().for_each(|t| w.word(t));
            }
            w.s.push_str("\nend");
            true
        },
    },
    Kind {
        name: "plane",
        syntax: "plane FRAME | off REF D | ang REF REF A | tan REF REF | pts REF REF REF | mid REF REF | norm REF REF",
        parse: |p| {
            let by = match p.next_of(&["off", "ang", "tan", "pts", "mid", "norm"]) {
                Some("off") => PlaneBy::Offset(p.wref()?, p.num()?),
                Some("ang") => PlaneBy::Angle {
                    plane: p.wref()?,
                    axis: p.wref()?,
                    angle: p.angle()?,
                },
                Some("tan") => PlaneBy::Tangent {
                    face: p.wref()?,
                    plane: p.wref()?,
                },
                Some("pts") => PlaneBy::Points(p.wref3()?),
                Some("mid") => PlaneBy::Mid(p.wref()?, p.wref()?),
                Some(_) => PlaneBy::Normal {
                    curve: p.wref()?,
                    at: p.wref()?,
                },
                None => return Ok(Feature::Plane(p.frame()?)),
            };
            Ok(Feature::Work(Work::Plane(by)))
        },
        print: |f, w| {
            let by = match f {
                Feature::Plane(frame) => {
                    w.frame(frame);
                    return true;
                }
                Feature::Work(Work::Plane(by)) => by,
                _ => return false,
            };
            match *by {
                PlaneBy::Offset(r, d) => {
                    w.word("off");
                    w.wref(&r);
                    w.num(d);
                }
                PlaneBy::Angle { plane, axis, angle } => {
                    w.word("ang");
                    w.wrefs(&[plane, axis]);
                    w.angle(angle);
                }
                PlaneBy::Tangent { face, plane } => {
                    w.word("tan");
                    w.wrefs(&[face, plane]);
                }
                PlaneBy::Points(rs) => {
                    w.word("pts");
                    w.wrefs(&rs);
                }
                PlaneBy::Mid(a, b) => {
                    w.word("mid");
                    w.wrefs(&[a, b]);
                }
                PlaneBy::Normal { curve, at } => {
                    w.word("norm");
                    w.wrefs(&[curve, at]);
                }
            }
            true
        },
    },
    Kind {
        name: "axis",
        syntax: "axis REF | pts REF REF | meet REF REF | norm REF REF   REF: FEAT [@h|(sel)]",
        parse: |p| {
            Ok(Feature::Work(Work::Axis(
                match p.next_of(&["pts", "meet", "norm"]) {
                    Some("pts") => AxisBy::Points(p.wref()?, p.wref()?),
                    Some("meet") => AxisBy::Meet(p.wref()?, p.wref()?),
                    Some(_) => AxisBy::Normal {
                        plane: p.wref()?,
                        at: p.wref()?,
                    },
                    None => AxisBy::Along(p.wref()?),
                },
            )))
        },
        print: |f, w| {
            let Feature::Work(Work::Axis(by)) = f else {
                return false;
            };
            match *by {
                AxisBy::Points(a, b) => {
                    w.word("pts");
                    w.wrefs(&[a, b]);
                }
                AxisBy::Along(r) => w.wref(&r),
                AxisBy::Meet(a, b) => {
                    w.word("meet");
                    w.wrefs(&[a, b]);
                }
                AxisBy::Normal { plane, at } => {
                    w.word("norm");
                    w.wrefs(&[plane, at]);
                }
            }
            true
        },
    },
    Kind {
        name: "point",
        syntax: "point REF | mid REF | meet REF REF   REF: FEAT [@h|(sel)]",
        parse: |p| {
            Ok(Feature::Work(Work::Point(match p.next_of(&["mid", "meet"]) {
                Some("mid") => PointBy::Mid(p.wref()?),
                Some(_) => PointBy::Meet(p.wref()?, p.wref()?),
                None => PointBy::At(p.wref()?),
            })))
        },
        print: |f, w| {
            let Feature::Work(Work::Point(by)) = f else {
                return false;
            };
            match *by {
                PointBy::At(r) => w.wref(&r),
                PointBy::Mid(r) => {
                    w.word("mid");
                    w.wref(&r);
                }
                PointBy::Meet(a, b) => {
                    w.word("meet");
                    w.wrefs(&[a, b]);
                }
            }
            true
        },
    },
    Kind {
        name: "ucs",
        syntax: "ucs FRAME | ucs pts REF REF REF",
        parse: |p| {
            Ok(Feature::Work(Work::Ucs(if p.eat("pts") {
                UcsBy::Points(p.wref3()?)
            } else {
                UcsBy::Frame(p.frame()?)
            })))
        },
        print: |f, w| {
            match f {
                Feature::Work(Work::Ucs(UcsBy::Frame(frame))) => w.frame(frame),
                Feature::Work(Work::Ucs(UcsBy::Points(rs))) => {
                    w.word("pts");
                    w.wrefs(rs);
                }
                _ => return false,
            }
            true
        },
    },
    Kind {
        name: "rib",
        syntax: "rib T [pos|neg] [web] [flip] [draft A] FRAME (line AX AY BX BY | arc CX CY R START SWEEP)..",
        parse: |p| {
            let thickness = p.num()?;
            let side = p.side(Side::Both);
            let web = p.eat("web");
            let flip = p.eat("flip");
            let draft = if p.eat("draft") { p.angle()? } else { 0.0 };
            let plane = p.frame()?;
            let mut path = Vec::new();
            while !p.done() {
                path.push(match p.next()? {
                    "line" => Segment::Line {
                        a: p.p2()?,
                        b: p.p2()?,
                    },
                    "arc" => Segment::Arc {
                        center: p.p2()?,
                        radius: p.num()?,
                        start: p.angle()?,
                        sweep: p.angle()?,
                    },
                    t => return Err(format!("expected line or arc, found '{t}'")),
                });
            }
            Ok(Feature::Edit(Edit::Rib(Box::new(Rib {
                plane,
                path,
                thickness,
                side,
                web,
                flip,
                draft,
            }))))
        },
        print: |f, w| {
            let Feature::Edit(Edit::Rib(r)) = f else {
                return false;
            };
            w.num(r.thickness);
            w.side(r.side, Side::Both);
            for (on, word) in [(r.web, "web"), (r.flip, "flip")] {
                if on {
                    w.word(word);
                }
            }
            if r.draft.to_bits() != 0 {
                w.word("draft");
                w.angle(r.draft);
            }
            w.frame(&r.plane);
            for s in &r.path {
                match *s {
                    Segment::Line { a, b } => {
                        w.word("line");
                        w.nums(&[a.x, a.y, b.x, b.y]);
                    }
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } => {
                        w.word("arc");
                        w.nums(&[center.x, center.y, radius]);
                        w.angle(start);
                        w.angle(sweep);
                    }
                    Segment::Circle { .. } => unreachable!("a rib's path has no whole circles"),
                }
            }
            true
        },
    },
    Kind {
        name: "offset",
        syntax: "offset D REF..",
        parse: |p| {
            let distance = p.num()?;
            Ok(Feature::Edit(Edit::Offset {
                faces: p.refs()?,
                distance,
            }))
        },
        print: |f, w| {
            let Feature::Edit(Edit::Offset { faces, distance }) = f else {
                return false;
            };
            w.num(*distance);
            w.names(faces);
            true
        },
    },
    Kind {
        name: "thicken",
        syntax: "thicken [op] T [neg|sym] REF..",
        parse: |p| {
            let op = p.op()?;
            let thickness = p.num()?;
            let side = p.side(Side::Pos);
            Ok(Feature::Edit(Edit::Thicken {
                faces: p.refs()?,
                thickness,
                side,
                op,
            }))
        },
        print: |f, w| {
            let Feature::Edit(Edit::Thicken {
                faces,
                thickness,
                side,
                op,
            }) = f
            else {
                return false;
            };
            w.op(*op);
            w.num(*thickness);
            w.side(*side, Side::Pos);
            w.names(faces);
            true
        },
    },
    Kind {
        name: "delete",
        syntax: "delete REF..",
        parse: |p| Ok(Feature::Edit(Edit::Delete { faces: p.refs()? })),
        print: |f, w| {
            let Feature::Edit(Edit::Delete { faces }) = f else {
                return false;
            };
            w.names(faces);
            true
        },
    },
    Kind {
        name: "replace",
        syntax: "replace REF with FEAT [@face]",
        parse: |p| {
            let face = p.one_ref()?;
            p.expect("with")?;
            Ok(Feature::Edit(Edit::Replace {
                face,
                with: p.target()?,
            }))
        },
        print: |f, w| {
            let Feature::Edit(Edit::Replace { face, with }) = f else {
                return false;
            };
            w.name(*face);
            w.word("with");
            w.target(with);
            true
        },
    },
    Kind {
        name: "move",
        syntax: "move REF.. (by DX DY DZ | turn A OX OY OZ AX AY AZ)",
        parse: |p| {
            let faces = p.refs()?;
            let motion = match p.next()? {
                "by" => Motion::Translate(p.vec3()?),
                "turn" => Motion::Rotate {
                    angle: p.angle()?,
                    origin: p.point3()?,
                    axis: p.vec3()?,
                },
                t => return Err(format!("expected by or turn, found '{t}'")),
            };
            Ok(Feature::Edit(Edit::Move { faces, motion }))
        },
        print: |f, w| {
            let Feature::Edit(Edit::Move { faces, motion }) = f else {
                return false;
            };
            w.names(faces);
            match *motion {
                Motion::Translate(d) => {
                    w.word("by");
                    w.nums(&[d.x, d.y, d.z]);
                }
                Motion::Rotate { origin, axis, angle } => {
                    w.word("turn");
                    w.angle(angle);
                    w.nums(&[origin.x, origin.y, origin.z, axis.x, axis.y, axis.z]);
                }
            }
            true
        },
    },
    Kind {
        name: "thread",
        syntax: "thread (SPEC FRAME ext|int | SPEC face FEAT REF [flip] | hole FEAT) (N | full | to FEAT [@face] [off N]) [left] [chamfer] [runout]   SPEC: M6 | M6x0.75 | 1/4-20 | 0.19-24",
        parse: |p| {
            let on = if p.eat("hole") {
                ThreadOn::Hole(p.feat()?)
            } else {
                let t = p.next()?;
                let spec = ThreadSpec::parse(t).ok_or_else(|| {
                    format!("expected a thread size (M6, M6x0.75, 1/4-20, 0.19-24), found '{t}'")
                })?;
                if p.eat("face") {
                    let feature = p.feat()?;
                    ThreadOn::Face {
                        feature,
                        name: p.one_ref()?,
                        spec,
                        flip: p.eat("flip"),
                    }
                } else {
                    let at = p.frame()?;
                    let internal = match p.next()? {
                        "ext" => false,
                        "int" => true,
                        t => return Err(format!("expected ext or int, found '{t}'")),
                    };
                    ThreadOn::Axis { at, spec, internal }
                }
            };
            let length = if p.eat("full") {
                ThreadLength::Full
            } else if p.eat("to") {
                let target = p.target()?;
                ThreadLength::To {
                    target,
                    offset: p.off(false)?,
                }
            } else {
                ThreadLength::Distance(p.num()?)
            };
            let handed = if p.eat("left") {
                Handed::Left
            } else {
                Handed::Right
            };
            Ok(Feature::Thread(Box::new(ScrewThread {
                on,
                length,
                handed,
                chamfer: p.eat("chamfer"),
                runout: p.eat("runout"),
            })))
        },
        print: |f, w| {
            let Feature::Thread(t) = f else { return false };
            match t.on {
                ThreadOn::Hole(k) => {
                    w.word("hole");
                    w.feat(k);
                }
                ThreadOn::Face {
                    feature,
                    name,
                    spec,
                    flip,
                } => {
                    w.word(&spec.designation());
                    w.word("face");
                    w.feat(feature);
                    w.name(name);
                    if flip {
                        w.word("flip");
                    }
                }
                ThreadOn::Axis { at, spec, internal } => {
                    w.word(&spec.designation());
                    w.frame(&at);
                    w.word(if internal { "int" } else { "ext" });
                }
            }
            match t.length {
                ThreadLength::Full => w.word("full"),
                ThreadLength::Distance(d) => w.num(d),
                ThreadLength::To { target, offset } => {
                    w.word("to");
                    w.target(&target);
                    w.off(offset, false);
                }
            }
            if t.handed == Handed::Left {
                w.word("left");
            }
            if t.chamfer {
                w.word("chamfer");
            }
            if t.runout {
                w.word("runout");
            }
            true
        },
    },
    Kind {
        name: "text",
        syntax: "text [add|cut] STRING font PATH SIZE DEPTH FRAME [wrap R]   STRING, PATH: quoted; SIZE: the em; wrap: round FRAME's z",
        parse: |p| {
            let op = if p.eat("cut") {
                Combine::Cut
            } else {
                p.eat("add");
                Combine::Add
            };
            let text = p.string()?;
            p.expect("font")?;
            let name = p.string()?;
            let font = FontRef {
                font: (p.b.fonts)(&name)?,
                name,
            };
            let (size, depth) = (p.num()?, p.num()?);
            let frame = p.frame()?;
            let on = if p.eat("wrap") {
                TextOn::Cylinder {
                    axis: frame,
                    radius: p.num()?,
                }
            } else {
                TextOn::Plane(frame)
            };
            Ok(Feature::Text(Box::new(Text {
                text,
                font,
                size,
                on,
                depth,
                op,
            })))
        },
        print: |f, w| {
            let Feature::Text(t) = f else { return false };
            if t.op != Combine::Add {
                w.word(OPS.iter().find(|o| o.1 == t.op).expect("every op is named").0);
            }
            w.string(&t.text);
            w.word("font");
            w.string(&t.font.name);
            w.nums(&[t.size, t.depth]);
            match t.on {
                TextOn::Plane(f) => w.frame(&f),
                TextOn::Cylinder { axis, radius } => {
                    w.frame(&axis);
                    w.word("wrap");
                    w.num(radius);
                }
            }
            true
        },
    },
    Kind {
        name: "combine",
        syntax: "combine (add|cut|keep) TARGET TOOL.. [keep]   bodies by the lines that made them",
        parse: |p| {
            let op = match p.next()? {
                "add" => Combine::Add,
                "cut" => Combine::Cut,
                "keep" => Combine::Keep,
                t => return Err(format!("expected add, cut or keep, found '{t}'")),
            };
            let target = p.feat()?;
            let (mut tools, mut keep) = (vec![], false);
            while !p.done() {
                if p.peek() == "keep" && p.k + 1 == p.t.len() {
                    keep = p.eat("keep");
                } else {
                    tools.push(p.feat()?);
                }
            }
            if tools.is_empty() {
                return Err("combine needs a tool body".into());
            }
            Ok(body(BodyFeature::Combine {
                op,
                target,
                tools,
                keep,
            }))
        },
        print: |f, w| {
            let Some(BodyFeature::Combine {
                op,
                target,
                tools,
                keep,
            }) = body_of(f)
            else {
                return false;
            };
            w.word(OPS.iter().find(|o| o.1 == *op).expect("every op is named").0);
            w.feat(*target);
            tools.iter().for_each(|&t| w.feat(t));
            if *keep {
                w.word("keep");
            }
            true
        },
    },
    Kind {
        name: "split",
        syntax: "split BY [+|-]   BY: FEAT [@face] | plane FRAME | curve FRAME (line AX AY BX BY | arc CX CY R A0 SWEEP | circle CX CY R)",
        parse: |p| {
            let by = p.cutter()?;
            let keep = if p.eat("+") {
                Keep::Plus
            } else if p.eat("-") {
                Keep::Minus
            } else {
                Keep::Both
            };
            Ok(body(BodyFeature::Split { by, keep }))
        },
        print: |f, w| {
            let Some(BodyFeature::Split { by, keep }) = body_of(f) else {
                return false;
            };
            w.cutter(by);
            match keep {
                Keep::Plus => w.word("+"),
                Keep::Minus => w.word("-"),
                Keep::Both => {}
            }
            true
        },
    },
    Kind {
        name: "splitface",
        syntax: "splitface REF.. by BY",
        parse: |p| {
            let mut faces = p.one_or_more_refs()?;
            faces.extend(p.refs()?);
            p.expect("by")?;
            Ok(body(BodyFeature::SplitFace {
                faces,
                by: p.cutter()?,
            }))
        },
        print: |f, w| {
            let Some(BodyFeature::SplitFace { faces, by }) = body_of(f) else {
                return false;
            };
            w.names(faces);
            w.word("by");
            w.cutter(by);
            true
        },
    },
    Kind {
        name: "movebody",
        syntax: "movebody (by DX DY DZ | rot OX OY OZ AX AY AZ A | frame FRAME)..",
        parse: |p| {
            Ok(body(BodyFeature::Move {
                steps: p.steps()?,
                copy: false,
            }))
        },
        print: |f, w| match body_of(f) {
            Some(BodyFeature::Move { steps, copy: false }) => {
                w.steps(steps);
                true
            }
            _ => false,
        },
    },
    Kind {
        name: "copybody",
        syntax: "copybody (by DX DY DZ | rot OX OY OZ AX AY AZ A | frame FRAME)..",
        parse: |p| {
            Ok(body(BodyFeature::Move {
                steps: p.steps()?,
                copy: true,
            }))
        },
        print: |f, w| match body_of(f) {
            Some(BodyFeature::Move { steps, copy: true }) => {
                w.steps(steps);
                true
            }
            _ => false,
        },
    },
];

fn body(b: BodyFeature) -> Feature {
    Feature::Body(Box::new(b))
}

fn body_of(f: &Feature) -> Option<&BodyFeature> {
    match f {
        Feature::Body(b) => Some(b),
        _ => None,
    }
}

/// A feature without its `in` bodies, and those.
fn unwrap_in(f: &Feature) -> (&Feature, Option<&Pick>) {
    match body_of(f) {
        Some(BodyFeature::In { bodies, feature }) => (feature, Some(bodies)),
        _ => (f, None),
    }
}

type R<T> = Result<T, String>;

/// A parameter `name = expr`, checked against the lines above.
fn new_param(above: &[Feature], name: String, expr: String, unit: Unit) -> R<Param> {
    if !expr::is_name(&name) {
        return Err(format!(
            "'{name}': a parameter's name is letters, digits and _, from a letter"
        ));
    }
    if expr::reserved(&name)
        || KEYWORDS.contains(&name.as_str())
        || kind(&name).is_some()
        || sketch::is_ref(&name)
    {
        return Err(format!("'{name}' is a word the file uses; pick another name"));
    }
    if param(above, &name).is_some() {
        return Err(format!("parameter '{name}' is already defined above"));
    }
    if expr::names(&expr).contains(&name.as_str()) {
        return Err(format!("'{name}' refers to itself"));
    }
    if expr.is_empty() {
        return Err(format!("{name} = needs a value"));
    }
    let (value, got) = expr::eval(&expr, &|n| param(above, n))?;
    let unit = match (unit, got) {
        (u, Unit::None) | (Unit::None, u) => u,
        (u, g) if u == g => u,
        _ => {
            return Err(format!(
                "'{expr}' is in {}",
                if got == Unit::Mm { "mm" } else { "deg" }
            ));
        }
    };
    Ok(Param {
        name,
        expr,
        unit,
        value,
    })
}

/// One line of a part file that holds a feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The id its feature's names derive from: given in the file, or its place from 1.
    pub id: u64,
    /// 1-based, in the file.
    pub no: usize,
    pub label: Option<String>,
    /// The kind first, then its arguments; a parenthesized selector is one token.
    pub toks: Vec<String>,
}

/// A line's tokens: a parenthesized selector is one token, and so is an expression whose
/// parentheses touch its other characters (`(w+2)/2`, `sqrt(a)`).
pub fn tokens(text: &str) -> Vec<String> {
    let mut toks: Vec<String> = Vec::new();
    let mut start = None;
    let mut depth = 0;
    let (mut quoted, mut escaped) = (false, false);
    for (k, c) in text.char_indices() {
        match start {
            Some(s) if c.is_whitespace() && depth == 0 && !quoted => {
                toks.push(text[s..k].to_string());
                start = None;
            }
            None if c.is_whitespace() => continue,
            None => start = Some(k),
            _ => {}
        }
        if quoted {
            (quoted, escaped) = (escaped || c != '"', !escaped && c == '\\');
        } else if c == '"' {
            quoted = true;
        } else {
            depth = (depth + (c == '(') as i32 - (c == ')') as i32).max(0);
        }
    }
    if let Some(s) = start {
        toks.push(text[s..].to_string());
    }
    toks
}

/// The line up to its comment: a `#` outside a quoted string.
pub(crate) fn uncommented(raw: &str) -> &str {
    let (mut quoted, mut escaped) = (false, false);
    for (k, c) in raw.char_indices() {
        if quoted {
            (quoted, escaped) = (escaped || c != '"', !escaped && c == '\\');
        } else if c == '"' {
            quoted = true;
        } else if c == '#' {
            return &raw[..k];
        }
    }
    raw
}

/// The feature lines of a file (blank and comment-only lines skipped). A sketch block is one
/// line: its header, then each row after a `;` token, up to `end`.
pub fn lines(src: &str) -> Vec<Line> {
    crate::doc::Document::parse(src).lines()
}

/// What a line may refer to: the features before it, their labels and kinds, and a resolver
/// for `@handle` and `(selector)` references against the part as it stands there.
pub struct Before<'a> {
    pub features: &'a [Feature],
    /// The id each of `features` is named by.
    pub ids: &'a [u64],
    pub labels: &'a [Option<String>],
    pub kinds: &'a [&'static str],
    pub refs: &'a dyn Fn(&str) -> R<Vec<Name>>,
    /// A reference to a planar face: its name, and its plane with z out of the part.
    pub face: &'a dyn Fn(&str) -> R<(Name, Frame)>,
    /// The font a text line names, found by the host; None where it does not look.
    pub fonts: &'a dyn Fn(&str) -> R<Option<std::sync::Arc<forge_font::Font>>>,
    /// What this line read as last time, if known: a sketch takes its unchanged parts.
    pub was: Option<&'a Feature>,
    /// The name feature `k`'s own solid gives an entity of the part named `n` (a boolean renames
    /// what it splits); `n` itself where nothing is built to look at.
    pub own: &'a dyn Fn(usize, Name) -> Name,
}

pub fn kind(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.name == name)
}

/// Parses one feature line.
pub fn parse_line(line: &Line, before: &Before) -> R<Feature> {
    parse_line_refs(line, before).map(|f| f.0)
}

/// Parses one feature line, and says which of its tokens name an earlier feature (the token's
/// index and the feature's).
pub fn parse_line_refs(line: &Line, before: &Before) -> R<(Feature, Vec<(usize, usize)>)> {
    let param = line.toks.get(1).is_some_and(|t| t == "=");
    let k = if param { kind("param") } else { kind(&line.toks[0]) }.ok_or_else(|| {
        let names: Vec<&str> = KINDS.iter().map(|k| k.name).collect();
        format!(
            "unknown feature '{}' (one of: {} | NAME = EXPR)",
            line.toks[0],
            names.join(" ")
        )
    })?;
    // `in FEAT[,FEAT..]` or `in all` ends any line
    let n = line.toks.len();
    let (toks, pick) = match !param && n >= 3 && line.toks[n - 2] == "in" {
        true => (&line.toks[..n - 2], Some(line.toks[n - 1].as_str())),
        false => (&line.toks[..], None),
    };
    let mut p = P {
        t: toks,
        k: if param { 0 } else { 1 },
        b: before,
        on: None,
        at: None,
        feats: Vec::new(),
    };
    let f = (k.parse)(&mut p)?;
    if let Some(t) = p.t.get(p.k) {
        return Err(format!("unexpected '{t}'"));
    }
    let mut feats = p.feats;
    let bodies = match pick {
        None => return Ok((f, feats)),
        Some("all") => Pick::All,
        Some(w) => {
            let ids = w
                .split(',')
                .map(|x| {
                    let one = [x.to_string()];
                    P {
                        t: &one,
                        k: 0,
                        b: before,
                        on: None,
                        at: None,
                        feats: Vec::new(),
                    }
                    .feat_named()
                })
                .collect::<R<Vec<usize>>>()?;
            // one body named: the reference follows its line through edits
            if let [id] = ids[..] {
                feats.push((n - 1, id));
            }
            Pick::Some(ids)
        }
    };
    Ok((body(BodyFeature::In { bodies, feature: f }), feats))
}

/// The kind of a feature.
pub fn kind_of(f: &Feature) -> &'static Kind {
    let f = unwrap_in(f).0;
    // rows without printing them
    match f {
        Feature::Param(_) => return kind("param").expect("a row"),
        Feature::Sketch(_) => return kind("sketch").expect("a row"),
        _ => {}
    }
    let mut w = W::new(&[], &[]);
    KINDS
        .iter()
        .find(|k| (k.print)(f, &mut w))
        .expect("every feature has a row in KINDS")
}

/// Prints a part (names in full, so it parses back without a body to resolve against).
pub fn print(part: &Part) -> String {
    let kinds: Vec<&'static str> = part.features.iter().map(|f| kind_of(f).name).collect();
    let mut out = String::new();
    for (i, f) in part.features.iter().enumerate() {
        let mut w = W::new(&part.features[..i], &kinds);
        if !matches!(f, Feature::Param(_)) {
            w.s.push_str(kinds[i]);
        }
        let (f, pick) = unwrap_in(f);
        (kind_of(f).print)(f, &mut w);
        match pick {
            None => {}
            Some(Pick::All) => w.word("in all"),
            Some(Pick::Some(ids)) => {
                let list: Vec<String> = ids.iter().map(|&i| w.feat_name(i)).collect();
                w.word("in");
                w.word(&list.join(","));
            }
        }
        out.push_str(&w.s);
        out.push('\n');
    }
    out
}

/// Parses a file whose references are all full names (as `print` writes them). A `Part` is
/// named by places, so a file whose lines keep other ids is refused: it builds through its
/// document, which hands the regen those ids.
pub fn parse(src: &str) -> Result<Part, (usize, String)> {
    let full = |r: &str| -> R<Vec<Name>> {
        match full_name(r) {
            Some(n) => Ok(vec![n]),
            None => Err(format!("'{r}' needs a part to resolve against")),
        }
    };
    let face = |r: &str| -> R<(Name, Frame)> { Err(format!("'{r}' needs a part to resolve against")) };
    let ls = lines(src);
    let labels: Vec<Option<String>> = ls.iter().map(|l| l.label.clone()).collect();
    let ids: Vec<u64> = (1..=ls.len() as u64).collect();
    if let Some(l) = ls.iter().zip(&ids).find(|(l, id)| l.id != **id).map(|p| p.0) {
        return Err((
            l.no,
            format!("id {} is not the line's place: read the file as a document", l.id),
        ));
    }
    let mut kinds = Vec::new();
    let mut features = Vec::new();
    for (i, l) in ls.iter().enumerate() {
        let b = Before {
            features: &features,
            ids: &ids[..i],
            labels: &labels[..i],
            kinds: &kinds,
            refs: &full,
            face: &face,
            fonts: &|_| Ok(None),
            was: None,
            own: &|_, n| n,
        };
        let f = parse_line(l, &b).map_err(|e| (l.no, e))?;
        kinds.push(kind_of(&f).name);
        features.push(f);
    }
    Ok(Part { features })
}

/// `@` and 16 hex digits: a whole name.
pub fn full_name(r: &str) -> Option<Name> {
    let h = r.strip_prefix('@')?;
    (h.len() == 16).then(|| u64::from_str_radix(h, 16).ok().map(Name))?
}

/// The op a solid feature has when its line names none: new until a solid exists, then add.
fn default_op(before: &[Feature]) -> Combine {
    let solid = |f: &Feature| {
        matches!(
            unwrap_in(f).0,
            Feature::Extrude { .. }
                | Feature::Revolve { .. }
                | Feature::Ball { .. }
                | Feature::Sweep(_)
                | Feature::Coil(_)
                | Feature::Loft(_)
        )
    };
    if before.iter().any(solid) {
        Combine::Add
    } else {
        Combine::New
    }
}

const CONVENTIONS: [(&str, Convention); 2] = [("iso", Convention::Iso), ("ansi", Convention::Ansi)];

const FITS: [(&str, Fit); 4] = [
    ("close", Fit::Close),
    ("normal", Fit::Normal),
    ("loose", Fit::Loose),
    ("tap", Fit::Tap),
];

pub(crate) const OPS: [(&str, Combine); 4] = [
    ("new", Combine::New),
    ("add", Combine::Add),
    ("cut", Combine::Cut),
    ("keep", Combine::Keep),
];

const SIDES: [(&str, Side); 3] = [("sym", Side::Both), ("pos", Side::Pos), ("neg", Side::Neg)];

/// Named orientations: x, y, z axes.
const ORIENT: [(&str, [Vec3; 3]); 6] = [
    ("xy", [Vec3::X, Vec3::Y, Vec3::Z]),
    ("xz", [Vec3::X, Vec3::Z, Vec3::new(0.0, -1.0, 0.0)]),
    ("yz", [Vec3::Y, Vec3::Z, Vec3::X]),
    (
        "-xy",
        [Vec3::X, Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)],
    ),
    ("-xz", [Vec3::X, Vec3::new(0.0, 0.0, -1.0), Vec3::Y]),
    (
        "-yz",
        [Vec3::Y, Vec3::new(0.0, 0.0, -1.0), Vec3::new(-1.0, 0.0, 0.0)],
    ),
];

const LOOPS: [&str; 5] = ["rect", "rrect", "circle", "poly", "loop"];

fn deg(d: f64) -> f64 {
    d / 180.0 * dmath::PI
}

pub(crate) fn fmt_num(x: f64) -> String {
    let s = format!("{x}");
    let e = format!("{x:e}");
    if e.len() + 3 < s.len() { e } else { s }
}

/// An angle in degrees when that reads back to the same radians, else radians with an `r`.
fn fmt_angle(r: f64) -> String {
    let d = r / dmath::PI * 180.0;
    for p in 0..17 {
        if let Ok(v) = format!("{d:.p$e}").parse::<f64>()
            && deg(v).to_bits() == r.to_bits()
        {
            return fmt_num(v);
        }
    }
    format!("{}r", fmt_num(r))
}

pub(crate) struct P<'a> {
    t: &'a [String],
    k: usize,
    b: &'a Before<'a>,
    /// What the last frame was read from, if a reference, and the origin written after it.
    on: Option<forge_model::On>,
    at: Option<Point3>,
    /// Each token read as an earlier feature, and that feature.
    feats: Vec<(usize, usize)>,
}

/// What a number slot holds: a length (or plain number), an angle, or a count.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Want {
    Len,
    /// Degrees in the file, radians out; `1.2r` is radians.
    Angle,
    Count,
}

/// The value of the parameter `name` defined above.
fn param(features: &[Feature], name: &str) -> Option<(f64, Unit)> {
    features.iter().rev().find_map(|f| match f {
        Feature::Param(p) if p.name == name => Some((p.value, p.unit)),
        _ => None,
    })
}

/// A number token, or an expression over the parameters above, as `want` needs it.
pub(crate) fn value(t: &str, features: &[Feature], want: Want) -> R<f64> {
    let (v, unit) = match (t.parse::<f64>(), want) {
        (Ok(v), _) => (v, Unit::None),
        (Err(_), Want::Angle) if t.strip_suffix('r').is_some_and(|r| r.parse::<f64>().is_ok()) => {
            return Ok(t[..t.len() - 1].parse().expect("checked"));
        }
        _ => expr::eval(t, &|n| param(features, n)).map_err(|e| {
            if expr::is_name(t) && param(features, t).is_none() {
                let what = if want == Want::Angle {
                    "an angle"
                } else {
                    "a number"
                };
                format!("expected {what} or a parameter, found '{t}'")
            } else {
                format!("'{t}': {e}")
            }
        })?,
    };
    match (want, unit) {
        (Want::Angle, Unit::Mm) => Err(format!("'{t}' is in mm; an angle is wanted")),
        (Want::Len | Want::Count, Unit::Deg) => Err(format!("'{t}' is in deg; a length is wanted")),
        (Want::Angle, _) => Ok(deg(v)),
        (Want::Count, _) if v < 0.0 || v.fract() != 0.0 || v > u32::MAX as f64 => {
            Err(format!("expected a count, found '{t}' ({v})"))
        }
        _ => Ok(v),
    }
}

/// `@handle` or a `(selector)`, not a parenthesized expression.
pub fn is_ref(t: &str) -> bool {
    t.starts_with('@')
        || t.strip_prefix('(').is_some_and(|r| {
            ["faces", "edges", "verts"]
                .iter()
                .any(|w| r.trim_start().starts_with(w))
        })
}

/// Words the grammar uses, which a parameter may not be named.
const KEYWORDS: &[&str] = &[
    "x", "y", "z", "axes", "on", "at", "axis", "id", "cw", "rect", "rrect", "circle", "poly", "loop", "line",
    "arc", "circ", "sym", "two", "thru", "back", "both", "next", "to", "off", "new", "add", "cut", "keep",
    "face", "ang", "cb", "cs", "pt", "thread", "body", "lin", "mirror", "step", "out", "end", "cons", "open",
];

impl<'a> P<'a> {
    fn peek(&self) -> &str {
        self.t.get(self.k).map_or("", |s| s.as_str())
    }
    fn done(&self) -> bool {
        self.k >= self.t.len()
    }
    fn next(&mut self) -> R<&'a str> {
        let t = self.t.get(self.k).ok_or("line ends too soon")?;
        self.k += 1;
        Ok(t)
    }
    fn eat(&mut self, w: &str) -> bool {
        let hit = self.peek() == w;
        self.k += hit as usize;
        hit
    }
    fn expect(&mut self, w: &str) -> R<()> {
        if self.eat(w) {
            Ok(())
        } else {
            Err(format!("expected '{w}', found '{}'", self.peek()))
        }
    }
    fn is_num(&self) -> bool {
        let t = self.peek();
        let fs = self.b.features;
        !t.is_empty()
            && !is_ref(t)
            && (value(t, fs, Want::Angle).is_ok() || expr::eval(t, &|n| param(fs, n)).is_ok())
    }
    fn value(&mut self, want: Want) -> R<f64> {
        let t = self.next()?;
        value(t, self.b.features, want).map_err(|e| {
            let axisy = t
                .trim_start_matches(['+', '-'])
                .chars()
                .all(|c| "xyz".contains(c));
            if axisy {
                format!("{e} (orientations: xy xz yz -xy -xz -yz, or z A B C)")
            } else {
                e
            }
        })
    }
    fn num(&mut self) -> R<f64> {
        self.value(Want::Len)
    }
    /// A fillet radius: constant, linear from the edge's start to its other end, or
    /// radii at fractions along from the start.
    fn radius(&mut self) -> R<Radius> {
        let t = self.next()?.to_string();
        let bad = || format!("expected a radius (3, 2..4 or 0:2,1:4), found '{t}'");
        let f = |x: &str| value(x, self.b.features, Want::Len).map_err(|e| format!("{}: {e}", bad()));
        if t == ":" {
            Ok(Radius::Points(vec![]))
        } else if t.contains(':') {
            let pts = t.split(',').map(|pr| {
                let (a, b) = pr.split_once(':').ok_or_else(bad)?;
                Ok((f(a)?, f(b)?))
            });
            Ok(Radius::Points(pts.collect::<R<Vec<_>>>()?))
        } else if let Some((a, b)) = t.split_once("..") {
            Ok(Radius::Linear(f(a)?, f(b)?))
        } else {
            Ok(Radius::Constant(f(&t)?))
        }
    }
    fn angle(&mut self) -> R<f64> {
        self.value(Want::Angle)
    }
    /// Which way a thickness goes: `pos`, `neg` or `sym`, else `default`.
    fn side(&mut self, default: Side) -> Side {
        match SIDES.iter().find(|s| s.0 == self.peek()) {
            Some(s) => {
                self.k += 1;
                s.1
            }
            None => default,
        }
    }
    fn count(&mut self) -> R<usize> {
        Ok(self.value(Want::Count)? as usize)
    }
    fn point3(&mut self) -> R<Point3> {
        Ok(Point3::new(self.num()?, self.num()?, self.num()?))
    }
    fn vec3(&mut self) -> R<Vec3> {
        Ok(Vec3::new(self.num()?, self.num()?, self.num()?))
    }
    fn p2(&mut self) -> R<Point2> {
        Ok(Point2::new(self.num()?, self.num()?))
    }
    fn row(&mut self) -> R<Row> {
        Ok(Row {
            dir: self.vec3()?,
            count: self.count()?,
            spacing: self.num()?,
        })
    }
    fn op(&mut self) -> R<Combine> {
        match OPS.iter().find(|o| o.0 == self.peek()) {
            Some(o) => {
                self.k += 1;
                Ok(o.1)
            }
            None => Ok(default_op(self.b.features)),
        }
    }
    /// An earlier feature: a label, kind and count (`hole2`), or `fN`.
    fn feat(&mut self) -> R<usize> {
        let i = self.feat_named()?;
        self.feats.push((self.k - 1, i));
        Ok(i)
    }
    fn feat_named(&mut self) -> R<usize> {
        let t = self.next()?.to_string();
        let t = t.as_str();
        if let Some(i) = self.b.labels.iter().position(|l| l.as_deref() == Some(t)) {
            return Ok(i);
        }
        let digits = t.trim_end_matches(|c: char| c.is_ascii_digit());
        let n: usize = t[digits.len()..]
            .parse()
            .map_err(|_| format!("no feature '{t}' above this line"))?;
        let i = if digits == "f" {
            n.checked_sub(1)
        } else {
            self.b
                .kinds
                .iter()
                .enumerate()
                .filter(|k| *k.1 == digits)
                .nth(n.wrapping_sub(1))
                .map(|k| k.0)
        };
        i.ok_or_else(|| format!("no feature '{t}' above this line"))
    }
    /// The word if it is one of `words`.
    fn next_of(&mut self, words: &[&'static str]) -> Option<&'static str> {
        let w = *words.iter().find(|w| **w == self.peek())?;
        self.k += 1;
        Some(w)
    }
    /// A work feature or plane, or an entity of an earlier feature's solid.
    /// A work feature, plane or sketch; an entity of an earlier feature's solid; or a sketch's
    /// point (`sk l2.b`).
    fn wref(&mut self) -> R<Ref> {
        let feature = self.feat()?;
        let name = if self.peek().starts_with(['@', '(']) {
            Some((self.b.own)(feature, self.one_ref()?))
        } else if let Some(Feature::Sketch(s)) = self.b.features.get(feature)
            && sketch::is_ref(self.peek())
        {
            let t = self.next()?;
            sketch::point(s, t)?;
            Some(forge_model::params::point_name(t))
        } else {
            None
        };
        Ok(Ref { feature, name })
    }
    fn wref3(&mut self) -> R<[Ref; 3]> {
        Ok([self.wref()?, self.wref()?, self.wref()?])
    }
    /// A screw size and fit, or None for a plain diameter.
    fn standard(&mut self) -> R<Option<Standard>> {
        let t = self.peek();
        // an explicit coarse pitch names the coarse size: M36x4 is M36
        let coarse = match ThreadSpec::parse(t) {
            Some(ThreadSpec::Iso { major, pitch }) => {
                standard::screw(&format!("M{major}")).filter(|s| s.pitch == pitch)
            }
            _ => None,
        };
        let Some(screw) = standard::screw(t).or(coarse) else {
            if let Some(s) = ThreadSpec::parse(t).filter(|_| t.parse::<f64>().is_err()) {
                return Err(format!(
                    "{t} has no standard hole here; drill it plainly and thread it: hole {} FRAME .. thread {t} \
                     (tap drill about major - pitch)",
                    fmt_num(s.major() - s.pitch())
                ));
            }
            return Ok(None);
        };
        self.k += 1;
        let convention = match CONVENTIONS.iter().find(|c| c.0 == self.peek()) {
            Some(c) => {
                self.k += 1;
                c.1
            }
            None => screw.default_convention(),
        };
        if screw.inch() && convention != Convention::Ansi {
            return Err(format!("{} is an inch size: its tables are ANSI's", screw.name));
        }
        self.eat("clearance");
        let fit = match FITS.iter().find(|f| f.0 == self.peek()) {
            Some(f) => {
                self.k += 1;
                f.1
            }
            None => Fit::Normal,
        };
        Ok(Some(Standard {
            screw,
            convention,
            fit,
        }))
    }
    /// What work feature `i` comes to.
    fn datum(&self, i: usize, what: &str) -> R<Datum> {
        work::datum(forge_model::History::new(self.b.features, Some(self.b.ids)), i)
            .map_err(|e| format!("'{what}' needs a work feature that builds; line {}: {e:?}", i + 1))
    }
    fn cutter(&mut self) -> R<Cutter> {
        if self.eat("plane") {
            return Ok(Cutter::Plane(self.frame()?));
        }
        if !self.eat("curve") {
            return Ok(Cutter::Target(self.target()?));
        }
        let frame = self.frame()?;
        let seg = match self.next()? {
            "line" => Segment::Line {
                a: self.p2()?,
                b: self.p2()?,
            },
            "arc" => Segment::Arc {
                center: self.p2()?,
                radius: self.num()?,
                start: self.angle()?,
                sweep: self.angle()?,
            },
            "circle" => Segment::Circle {
                center: self.p2()?,
                radius: self.num()?,
                ccw: true,
            },
            t => return Err(format!("expected line, arc or circle, found '{t}'")),
        };
        Ok(Cutter::Curve(frame, seg))
    }
    fn steps(&mut self) -> R<Vec<Step>> {
        let mut out = Vec::new();
        loop {
            out.push(match self.peek() {
                "by" => {
                    self.k += 1;
                    Step::Translate(self.vec3()?)
                }
                "rot" => {
                    self.k += 1;
                    Step::Rotate {
                        origin: self.point3()?,
                        axis: self.vec3()?,
                        angle: self.angle()?,
                    }
                }
                "frame" => {
                    self.k += 1;
                    Step::Frame(self.frame()?)
                }
                _ if out.is_empty() => {
                    return Err(format!("expected by, rot or frame, found '{}'", self.peek()));
                }
                _ => return Ok(out),
            });
        }
    }
    fn target(&mut self) -> R<Target> {
        let feature = self.feat()?;
        if is_ref(self.peek()) {
            Ok(Target::Face {
                feature,
                name: (self.b.own)(feature, self.one_ref()?),
            })
        } else {
            Ok(Target::Plane(feature))
        }
    }
    /// A quoted string: \" \\ and \n inside it stand for a quote, a backslash and a newline.
    fn string(&mut self) -> R<String> {
        let t = self.next()?;
        let inner = t
            .strip_prefix('"')
            .and_then(|t| t.strip_suffix('"'))
            .ok_or_else(|| format!("expected a quoted string, found '{t}'"))?;
        let mut out = String::new();
        let mut cs = inner.chars();
        while let Some(c) = cs.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            out.push(match cs.next() {
                Some('n') => '\n',
                Some(e) => e,
                None => return Err(format!("unterminated string {t}")),
            });
        }
        Ok(out)
    }
    fn off(&mut self, angle: bool) -> R<f64> {
        match (self.eat("off"), angle) {
            (false, _) => Ok(0.0),
            (true, false) => self.num(),
            (true, true) => self.angle(),
        }
    }
    fn extent(&mut self) -> R<Extent> {
        Ok(match self.peek() {
            "sym" => {
                self.k += 1;
                Extent::Symmetric(self.num()?)
            }
            "two" => {
                self.k += 1;
                Extent::TwoSided {
                    forward: self.num()?,
                    back: self.num()?,
                }
            }
            "thru" => {
                self.k += 1;
                Extent::Through(if self.eat("back") {
                    Toward::Back
                } else if self.eat("both") {
                    Toward::Both
                } else {
                    Toward::Forward
                })
            }
            "next" => {
                self.k += 1;
                Extent::ToNext {
                    reverse: self.eat("back"),
                }
            }
            "to" => {
                self.k += 1;
                let target = self.target()?;
                Extent::To {
                    target,
                    offset: self.off(false)?,
                }
            }
            _ => Extent::Distance(self.num()?),
        })
    }
    fn turn(&mut self) -> R<Turn> {
        Ok(match self.peek() {
            "sym" => {
                self.k += 1;
                Turn::Symmetric(self.angle()?)
            }
            "two" => {
                self.k += 1;
                Turn::TwoSided {
                    forward: self.angle()?,
                    back: self.angle()?,
                }
            }
            "to" => {
                self.k += 1;
                let target = self.target()?;
                Turn::To {
                    target,
                    offset: self.off(true)?,
                }
            }
            _ if self.is_num() => Turn::Angle(self.angle()?),
            _ => Turn::Angle(dmath::TAU),
        })
    }
    fn frame(&mut self) -> R<Frame> {
        let t = self.peek().to_string();
        self.on = None;
        let mut f = if let Some((_, [x, y, z])) = ORIENT.iter().find(|o| o.0 == t) {
            self.k += 1;
            Frame::from_axes(Point3::ORIGIN, *x, *y, *z).expect("named orientations are orthonormal")
        } else if t == "z" {
            self.k += 1;
            let z = self.vec3()?.normalize().ok_or("a zero z axis")?;
            if self.eat("x") {
                Frame::from_zx(Point3::ORIGIN, z, self.vec3()?).ok_or("x along z")?
            } else {
                Frame::from_z(Point3::ORIGIN, z)
            }
        } else if t == "axes" {
            self.k += 1;
            let (x, y, z) = (self.vec3()?, self.vec3()?, self.vec3()?);
            Frame::from_axes(Point3::ORIGIN, x, y, z).ok_or("axes not orthonormal and right-handed")?
        } else if t == "on" || t == "-on" {
            self.k += 1;
            let f = if is_ref(self.peek()) {
                let (name, f) = (self.b.face)(self.next()?)?;
                self.on = Some(forge_model::On::Face(name));
                f
            } else {
                let i = self.feat()?;
                let point = match &self.b.features[i] {
                    Feature::Sketch(_) if sketch::is_ref(self.peek()) => Some(self.next()?.to_string()),
                    _ => None,
                };
                self.on = Some(forge_model::On::Feature {
                    feature: i,
                    point: point.clone(),
                    flip: t == "-on",
                });
                match &self.b.features[i] {
                    Feature::Plane(f) => *f,
                    Feature::Sketch(s) => match &point {
                        Some(pt) => s.plane.with_origin(s.plane.plane_point(sketch::point(s, pt)?)),
                        None => s.plane,
                    },
                    Feature::Work(_) => match self.datum(i, "on")? {
                        Datum::Plane(f) | Datum::Frame(f) => f,
                        Datum::Axis(l) => Frame::from_z(l.origin, l.dir),
                        Datum::Point(p) => Frame::WORLD.with_origin(p),
                    },
                    _ => {
                        return Err(format!(
                            "'on' needs a plane, sketch or work feature, or a face; line {} is none",
                            i + 1
                        ));
                    }
                }
            };
            if t == "-on" { f.flipped() } else { f }
        } else {
            Frame::WORLD
        };
        self.at = None;
        if self.eat("at") {
            let at = if self.is_num() {
                self.point3()?
            } else {
                let r = self.wref()?;
                work::locate(self.b.features, r).map_err(|e| {
                    format!("'at' needs a point (a work point, vertex, sketch point or ucs): {e:?}")
                })?
            };
            self.at = Some(at);
            f = f.with_origin(at);
        }
        Ok(f)
    }
    /// A sketch and the points of it named after it (none: all its `pt` points), if the line
    /// names a sketch here.
    fn sketch_points(&mut self) -> R<Option<At>> {
        let (k, n) = (self.k, self.feats.len());
        let Ok(feature) = self.feat() else {
            self.k = k;
            self.feats.truncate(n);
            return Ok(None);
        };
        let (Some(Feature::Sketch(s)), false) =
            (self.b.features.get(feature), self.peek().starts_with(['@', '(']))
        else {
            self.k = k;
            self.feats.truncate(n);
            return Ok(None);
        };
        let mut points = Vec::new();
        while sketch::is_ref(self.peek()) {
            points.push(self.next()?.to_string());
        }
        let centers = points.is_empty() && self.eat("centers");
        let kind = |n: &str, k: &[char]| {
            n.len() > 1 && n.starts_with(k) && n[1..].bytes().all(|c| c.is_ascii_digit())
        };
        let names: Vec<&str> = if points.is_empty() {
            s.names
                .iter()
                .map(|(n, _)| n.as_str())
                .filter(|n| kind(n, &['p']) || centers && kind(n, &['c', 'a']))
                .collect()
        } else {
            points.iter().map(String::as_str).collect()
        };
        if names.is_empty() {
            return Err(format!("sketch on line {} has no points", feature + 1));
        }
        let at = names
            .iter()
            .map(|n| Ok(s.plane.plane_point(sketch::point(s, n)?)))
            .collect::<R<Vec<_>>>()?;
        Ok(Some(At::Sketch {
            feature,
            points,
            centers,
            at,
        }))
    }
    /// The sketch a line names here, if it names one.
    fn sketch(&mut self) -> Option<&'a forge_model::Sketch> {
        let (k, n) = (self.k, self.feats.len());
        match self.feat().map(|i| &self.b.features[i]) {
            Ok(Feature::Sketch(s)) => Some(s),
            _ => {
                self.k = k;
                self.feats.truncate(n);
                None
            }
        }
    }
    /// A profile and an axis in its plane: `FRAME [axis ..] LOOPS | SKETCH [axis L] [E]`.
    fn axial(&mut self) -> R<(Profile, Axis2)> {
        if let Some(s) = self.sketch() {
            let axis = self.axis(Some(s))?;
            return Ok((self.region(s)?, axis));
        }
        let plane = self.frame()?;
        let axis = self.axis(None)?;
        Ok((
            Profile {
                plane,
                loops: self.loops()?,
            },
            axis,
        ))
    }
    fn axis(&mut self, sk: Option<&forge_model::Sketch>) -> R<Axis2> {
        if !self.eat("axis") {
            return Ok(Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
            });
        }
        if let Some(s) = sk.filter(|_| sketch::is_ref(self.peek())) {
            let (a, b) = sketch::line(s, self.next()?)?;
            return Ok(Axis2 {
                origin: a,
                dir: b - a,
            });
        }
        Ok(match self.peek() {
            "x" | "y" => Axis2 {
                origin: Point2::ORIGIN,
                dir: if self.next()? == "x" { Vec2::X } else { Vec2::Y },
            },
            _ => Axis2 {
                origin: self.p2()?,
                dir: Vec2::new(self.num()?, self.num()?),
            },
        })
    }
    fn id(&mut self, default: u64) -> R<u64> {
        if !self.eat("id") {
            return Ok(default);
        }
        let t = self.next()?;
        t.parse().map_err(|_| format!("expected an id, found '{t}'"))
    }
    fn profile(&mut self) -> R<Profile> {
        if let Some(s) = self.sketch() {
            return self.region(s);
        }
        Ok(Profile {
            plane: self.frame()?,
            loops: self.loops()?,
        })
    }
    /// A sketch's region: its only one, or the one whose outside runs through the entity named.
    fn region(&mut self, s: &forge_model::Sketch) -> R<Profile> {
        let through = if sketch::is_ref(self.peek()) {
            Some(self.next()?)
        } else {
            None
        };
        sketch::profile(s, through)
    }
    fn loops(&mut self) -> R<Vec<ProfileLoop>> {
        let mut next = 0u64;
        let mut out = Vec::new();
        while LOOPS.contains(&self.peek()) {
            let l = match self.next()? {
                "rect" => {
                    let (c, w, h) = (self.p2()?, self.num()?, self.num()?);
                    Profile::rectangle(c, w, h, self.id(next)?)
                }
                "rrect" => {
                    let (c, w, h, r) = (self.p2()?, self.num()?, self.num()?, self.num()?);
                    Profile::rounded_rectangle(c, w, h, r, self.id(next)?)
                }
                "circle" => {
                    let (center, radius) = (self.p2()?, self.num()?);
                    let ccw = !self.eat("cw");
                    ProfileLoop::new(vec![(Segment::Circle { center, radius, ccw }, self.id(next)?)])
                }
                "poly" => {
                    let mut pts = Vec::new();
                    while self.is_num() {
                        pts.push(self.p2()?);
                    }
                    if pts.len() < 2 {
                        return Err("poly needs at least two points".into());
                    }
                    let first = self.id(next)?;
                    let n = pts.len();
                    ProfileLoop::new(
                        (0..n)
                            .map(|i| {
                                (
                                    Segment::Line {
                                        a: pts[i],
                                        b: pts[(i + 1) % n],
                                    },
                                    first.wrapping_add(i as u64),
                                )
                            })
                            .collect(),
                    )
                }
                _ => {
                    let mut segs = Vec::new();
                    let mut want = next;
                    loop {
                        let s = match self.peek() {
                            "line" => {
                                self.k += 1;
                                Segment::Line {
                                    a: self.p2()?,
                                    b: self.p2()?,
                                }
                            }
                            "arc" => {
                                self.k += 1;
                                Segment::Arc {
                                    center: self.p2()?,
                                    radius: self.num()?,
                                    start: self.angle()?,
                                    sweep: self.angle()?,
                                }
                            }
                            "circ" => {
                                self.k += 1;
                                let (center, radius) = (self.p2()?, self.num()?);
                                Segment::Circle {
                                    center,
                                    radius,
                                    ccw: !self.eat("cw"),
                                }
                            }
                            _ => break,
                        };
                        let id = self.id(want)?;
                        want = id.wrapping_add(1);
                        segs.push((s, id));
                    }
                    if segs.is_empty() {
                        return Err("loop needs segments: line AX AY BX BY | arc CX CY R A0 SWEEP".into());
                    }
                    ProfileLoop::new(segs)
                }
            };
            next = l.segments.last().map_or(next, |s| s.1.wrapping_add(1));
            out.push(l);
        }
        if out.is_empty() {
            return Err(format!(
                "expected a loop (rect rrect circle poly loop), found '{}'",
                self.peek()
            ));
        }
        Ok(out)
    }
    /// A path: lines and arcs, each from where the one before ends.
    fn path(&mut self) -> R<Vec<PathSeg>> {
        if let Some(s) = self.sketch() {
            let through = if sketch::is_ref(self.peek()) {
                Some(self.next()?)
            } else {
                None
            };
            return sketch::path(s, through);
        }
        let mut out = Vec::new();
        loop {
            out.push(match self.peek() {
                "line" => {
                    self.k += 1;
                    PathSeg::Line {
                        a: self.point3()?,
                        b: self.point3()?,
                    }
                }
                "arc" => {
                    self.k += 1;
                    let frame = self.frame()?;
                    PathSeg::Arc {
                        circle: Circle {
                            frame,
                            radius: self.num()?,
                        },
                        start: self.angle()?,
                        sweep: self.angle()?,
                    }
                }
                _ => break,
            });
        }
        if out.is_empty() {
            return Err(format!(
                "expected a path (line AX AY AZ BX BY BZ | arc FRAME R START SWEEP), found '{}'",
                self.peek()
            ));
        }
        Ok(out)
    }
    fn refs(&mut self) -> R<Vec<Name>> {
        let mut out = Vec::new();
        while is_ref(self.peek()) {
            let t = self.next()?.to_string();
            out.extend((self.b.refs)(&t)?);
        }
        Ok(out)
    }
    fn one_or_more_refs(&mut self) -> R<Vec<Name>> {
        if !is_ref(self.peek()) {
            return Err(format!("expected @handle or (selector), found '{}'", self.peek()));
        }
        let t = self.next()?.to_string();
        let names = (self.b.refs)(&t)?;
        if names.is_empty() {
            return Err(format!("{t} selects nothing"));
        }
        Ok(names)
    }
    fn one_ref(&mut self) -> R<Name> {
        let t = self.peek().to_string();
        match self.one_or_more_refs()?[..] {
            [n] => Ok(n),
            ref ns => Err(format!("{t} selects {} entities; one is needed", ns.len())),
        }
    }
}

pub(crate) struct W<'a> {
    pub s: String,
    before: &'a [Feature],
    kinds: &'a [&'static str],
}

impl<'a> W<'a> {
    fn new(before: &'a [Feature], kinds: &'a [&'static str]) -> Self {
        W {
            s: String::new(),
            before,
            kinds,
        }
    }
    fn word(&mut self, w: &str) {
        self.s.push(' ');
        self.s.push_str(w);
    }
    fn num(&mut self, x: f64) {
        self.word(&fmt_num(x));
    }
    fn nums(&mut self, xs: &[f64]) {
        xs.iter().for_each(|&x| self.num(x));
    }
    fn radius(&mut self, r: &Radius) {
        let w = match r {
            Radius::Constant(x) => fmt_num(*x),
            Radius::Linear(a, b) => format!("{}..{}", fmt_num(*a), fmt_num(*b)),
            Radius::Points(p) if p.is_empty() => ":".into(),
            Radius::Points(p) => p
                .iter()
                .map(|(a, b)| format!("{}:{}", fmt_num(*a), fmt_num(*b)))
                .collect::<Vec<_>>()
                .join(","),
        };
        self.word(&w);
    }
    fn angle(&mut self, a: f64) {
        self.word(&fmt_angle(a));
    }
    fn name(&mut self, n: Name) {
        self.word(&format!("@{:016x}", n.0));
    }
    fn names(&mut self, ns: &[Name]) {
        ns.iter().for_each(|&n| self.name(n));
    }
    /// Always written, so moving lines never changes which bodies there are.
    fn op(&mut self, op: Combine) {
        self.word(OPS.iter().find(|o| o.1 == op).expect("every op is named").0);
    }
    fn side(&mut self, side: Side, default: Side) {
        if side != default {
            self.word(SIDES.iter().find(|s| s.1 == side).expect("every side is named").0);
        }
    }
    fn feat(&mut self, i: usize) {
        let w = self.feat_name(i);
        self.word(&w);
    }
    fn feat_name(&self, i: usize) -> String {
        if i < self.before.len() {
            let k = self.kinds[i];
            let n = self.kinds[..=i].iter().filter(|x| **x == k).count();
            format!("{k}{n}")
        } else {
            format!("f{}", i + 1)
        }
    }
    fn cutter(&mut self, c: &Cutter) {
        match c {
            Cutter::Target(t) => self.target(t),
            Cutter::Plane(f) => {
                self.word("plane");
                self.frame(f);
            }
            Cutter::Curve(f, s) => {
                self.word("curve");
                self.frame(f);
                match *s {
                    Segment::Line { a, b } => {
                        self.word("line");
                        self.nums(&[a.x, a.y, b.x, b.y]);
                    }
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } => {
                        self.word("arc");
                        self.nums(&[center.x, center.y, radius]);
                        self.angle(start);
                        self.angle(sweep);
                    }
                    Segment::Circle { center, radius, .. } => {
                        self.word("circle");
                        self.nums(&[center.x, center.y, radius]);
                    }
                }
            }
        }
    }
    fn steps(&mut self, steps: &[Step]) {
        for s in steps {
            match s {
                Step::Translate(v) => {
                    self.word("by");
                    self.nums(&[v.x, v.y, v.z]);
                }
                Step::Rotate { origin, axis, angle } => {
                    self.word("rot");
                    self.nums(&[origin.x, origin.y, origin.z, axis.x, axis.y, axis.z]);
                    self.angle(*angle);
                }
                Step::Frame(f) => {
                    self.word("frame");
                    self.frame(f);
                }
            }
        }
    }
    fn wref(&mut self, r: &Ref) {
        self.feat(r.feature);
        let point = match (self.before.get(r.feature), r.name) {
            (Some(Feature::Sketch(s)), Some(n)) => s.point_by(n).map(|p| p.0),
            _ => None,
        };
        match (point, r.name) {
            (Some(t), _) => self.word(&t),
            (None, Some(n)) => self.name(n),
            (None, None) => {}
        }
    }
    fn wrefs(&mut self, rs: &[Ref]) {
        rs.iter().for_each(|r| self.wref(r));
    }
    fn target(&mut self, t: &Target) {
        match *t {
            Target::Plane(i) => self.feat(i),
            Target::Face { feature, name } => {
                self.feat(feature);
                self.name(name);
            }
        }
    }
    fn string(&mut self, v: &str) {
        let mut t = String::from('"');
        for c in v.chars() {
            match c {
                '"' => t.push_str("\\\""),
                '\\' => t.push_str("\\\\"),
                '\n' => t.push_str("\\n"),
                c => t.push(c),
            }
        }
        t.push('"');
        self.word(&t);
    }
    fn off(&mut self, offset: f64, angle: bool) {
        if offset.to_bits() != 0 {
            self.word("off");
            if angle {
                self.angle(offset)
            } else {
                self.num(offset)
            }
        }
    }
    fn row(&mut self, r: &Row) {
        self.nums(&[r.dir.x, r.dir.y, r.dir.z]);
        self.word(&r.count.to_string());
        self.num(r.spacing);
    }
    fn extent(&mut self, e: &Extent) {
        match *e {
            Extent::Distance(d) => self.num(d),
            Extent::Symmetric(d) => {
                self.word("sym");
                self.num(d);
            }
            Extent::TwoSided { forward, back } => {
                self.word("two");
                self.nums(&[forward, back]);
            }
            Extent::Through(t) => {
                self.word("thru");
                match t {
                    Toward::Forward => {}
                    Toward::Back => self.word("back"),
                    Toward::Both => self.word("both"),
                }
            }
            Extent::ToNext { reverse } => {
                self.word("next");
                if reverse {
                    self.word("back");
                }
            }
            Extent::To { target, offset } => {
                self.word("to");
                self.target(&target);
                self.off(offset, false);
            }
        }
    }
    fn turn(&mut self, t: &Turn) {
        match *t {
            Turn::Angle(a) if a.to_bits() == dmath::TAU.to_bits() => {}
            Turn::Angle(a) => self.angle(a),
            Turn::Symmetric(a) => {
                self.word("sym");
                self.angle(a);
            }
            Turn::TwoSided { forward, back } => {
                self.word("two");
                self.angle(forward);
                self.angle(back);
            }
            Turn::To { target, offset } => {
                self.word("to");
                self.target(&target);
                self.off(offset, true);
            }
        }
    }
    fn frame(&mut self, f: &Frame) {
        let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
        let same = |g: &Frame| g.x() == f.x() && g.y() == f.y() && g.z() == f.z();
        if let Some(o) = ORIENT.iter().find(|o| o.1 == [x, y, z]) {
            if o.0 != "xy" {
                self.word(o.0);
            }
        } else if let Some(u) = z.normalize()
            && Frame::from_zx(Point3::ORIGIN, u, x).is_some_and(|g| same(&g))
        {
            self.word("z");
            self.nums(&[z.x, z.y, z.z]);
            if !same(&Frame::from_z(Point3::ORIGIN, u)) {
                self.word("x");
                self.nums(&[x.x, x.y, x.z]);
            }
        } else {
            self.word("axes");
            self.nums(&[x.x, x.y, x.z, y.x, y.y, y.z, z.x, z.y, z.z]);
        }
        let o = f.origin;
        if o != Point3::ORIGIN {
            self.word("at");
            self.nums(&[o.x, o.y, o.z]);
        }
    }
    fn axis(&mut self, a: &Axis2) {
        if a.origin == Point2::ORIGIN && a.dir == Vec2::Y {
        } else if a.origin == Point2::ORIGIN && a.dir == Vec2::X {
            self.word("axis x");
        } else {
            self.word("axis");
            self.nums(&[a.origin.x, a.origin.y, a.dir.x, a.dir.y]);
        }
    }
    fn profile(&mut self, p: &Profile) {
        self.frame(&p.plane);
        self.loops(&p.loops);
    }
    fn path(&mut self, path: &[PathSeg]) {
        for s in path {
            match *s {
                PathSeg::Line { a, b } => {
                    self.word("line");
                    self.nums(&[a.x, a.y, a.z, b.x, b.y, b.z]);
                }
                PathSeg::Arc { circle, start, sweep } => {
                    self.word("arc");
                    self.frame(&circle.frame);
                    self.num(circle.radius);
                    self.angle(start);
                    self.angle(sweep);
                }
            }
        }
    }
    fn loops(&mut self, loops: &[ProfileLoop]) {
        let mut next = 0u64;
        for l in loops {
            self.one_loop(l, next);
            next = l.segments.last().map_or(next, |s| s.1.wrapping_add(1));
        }
    }
    fn one_loop(&mut self, l: &ProfileLoop, next: u64) {
        let segs = &l.segments;
        let first = segs.first().map_or(next, |s| s.1);
        let consecutive = segs
            .iter()
            .enumerate()
            .all(|(i, s)| s.1 == first.wrapping_add(i as u64));
        let line = |k: usize| match segs.get(k) {
            Some((Segment::Line { a, b }, _)) => Some((*a, *b)),
            _ => None,
        };
        let mut short: Option<(&str, Vec<f64>)> = None;
        if consecutive {
            if let (4, Some((a, b)), Some((_, c))) = (segs.len(), line(0), line(1)) {
                let (w, h) = (b.x - a.x, c.y - b.y);
                if Profile::rectangle(a, w, h, first) == *l {
                    short = Some(("rect", vec![a.x, a.y, w, h]));
                }
            }
            if let (8, Some((a, _)), Some((Segment::Arc { radius: r, .. }, _)), Some((c, _)), Some((d, _))) =
                (segs.len(), line(0), segs.get(1), line(2), line(4))
            {
                let (x0, y0) = (a.x - r, a.y);
                let (w, h) = (c.x - x0, d.y - y0);
                if Profile::rounded_rectangle(Point2::new(x0, y0), w, h, *r, first) == *l {
                    short = Some(("rrect", vec![x0, y0, w, h, *r]));
                }
            }
            if let [(Segment::Circle { center, radius, ccw }, _)] = segs[..] {
                self.word("circle");
                self.nums(&[center.x, center.y, radius]);
                if !ccw {
                    self.word("cw");
                }
                short = Some(("", vec![]));
            }
            if short.is_none() && segs.len() >= 2 && (0..segs.len()).all(|k| line(k).is_some()) {
                let pts: Vec<Point2> = (0..segs.len()).map(|k| line(k).expect("all lines").0).collect();
                let n = pts.len();
                let poly: Vec<(Segment, u64)> = (0..n)
                    .map(|i| {
                        (
                            Segment::Line {
                                a: pts[i],
                                b: pts[(i + 1) % n],
                            },
                            first.wrapping_add(i as u64),
                        )
                    })
                    .collect();
                if poly == *segs {
                    short = Some(("poly", pts.iter().flat_map(|p| [p.x, p.y]).collect()));
                }
            }
        }
        if let Some((kw, args)) = short {
            if !kw.is_empty() {
                self.word(kw);
                self.nums(&args);
            }
            if first != next {
                self.word(&format!("id {first}"));
            }
            return;
        }
        self.word("loop");
        let mut want = next;
        for &(s, id) in segs {
            match s {
                Segment::Line { a, b } => {
                    self.word("line");
                    self.nums(&[a.x, a.y, b.x, b.y]);
                }
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => {
                    self.word("arc");
                    self.nums(&[center.x, center.y, radius]);
                    self.angle(start);
                    self.angle(sweep);
                }
                Segment::Circle { center, radius, ccw } => {
                    self.word("circ");
                    self.nums(&[center.x, center.y, radius]);
                    if !ccw {
                        self.word("cw");
                    }
                }
            }
            if id != want {
                self.word(&format!("id {id}"));
            }
            want = id.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_model::{adversarial, edits, fillets, finishing, suite, torture};

    fn round_trip(what: &str, part: &Part) {
        let text = print(part);
        match parse(&text) {
            Ok(back) => {
                for (i, (a, b)) in part.features.iter().zip(&back.features).enumerate() {
                    assert_eq!(
                        a,
                        b,
                        "{what}: feature {i} printed as\n{}",
                        text.lines().nth(i).unwrap_or("")
                    );
                }
                assert_eq!(part.features.len(), back.features.len(), "{what}");
            }
            Err((line, e)) => panic!(
                "{what}: line {line}: {e}\n{}",
                text.lines().nth(line - 1).unwrap_or("")
            ),
        }
    }

    /// Any value at all: round, awkward, tiny, huge, negative zero.
    fn num(r: &mut Rng) -> f64 {
        match r.below(6) {
            0 => r.below(200) as f64 - 100.0,
            1 => r.range(-1e3, 1e3),
            2 => r.range(-1.0, 1.0) * 1e-9,
            3 => r.range(1e10, 1e20),
            4 => -0.0,
            _ => r.below(16) as f64 * 0.25,
        }
    }

    fn angle(r: &mut Rng) -> f64 {
        match r.below(3) {
            0 => deg(r.below(720) as f64 - 360.0),
            1 => r.range(-7.0, 7.0),
            _ => dmath::TAU,
        }
    }

    fn frame(r: &mut Rng) -> Frame {
        let o = if r.bool() {
            Point3::ORIGIN
        } else {
            Point3::new(num(r), num(r), num(r))
        };
        let f = match r.below(4) {
            0 => {
                let [x, y, z] = ORIENT[r.below(6)].1;
                Frame::from_axes(o, x, y, z).expect("orthonormal")
            }
            1 => {
                let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0) + 2.0);
                Frame::from_z(o, z.normalize().expect("nonzero"))
            }
            _ => {
                let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0) + 2.0);
                let x = Vec3::new(1.0, r.range(-1.0, 1.0), 0.0);
                Frame::from_zx(o, z.normalize().expect("nonzero"), x).expect("not along z")
            }
        };
        if r.bool() {
            f.rotated_about_z(r.range(0.0, 6.0))
        } else {
            f
        }
    }

    fn p2(r: &mut Rng) -> Point2 {
        Point2::new(num(r), num(r))
    }

    fn profile(r: &mut Rng) -> Profile {
        let mut loops = Vec::new();
        let mut id = if r.bool() { 0 } else { r.below(1000) as u64 };
        for _ in 0..1 + r.below(3) {
            let l = match r.below(5) {
                0 => Profile::rectangle(p2(r), num(r), num(r), id),
                1 => Profile::rounded_rectangle(p2(r), num(r), num(r), num(r), id),
                2 => ProfileLoop::new(vec![(
                    Segment::Circle {
                        center: p2(r),
                        radius: num(r),
                        ccw: r.bool(),
                    },
                    id,
                )]),
                3 => {
                    let pts: Vec<Point2> = (0..2 + r.below(5)).map(|_| p2(r)).collect();
                    let n = pts.len();
                    ProfileLoop::new(
                        (0..n)
                            .map(|i| {
                                (
                                    Segment::Line {
                                        a: pts[i],
                                        b: pts[(i + 1) % n],
                                    },
                                    id + i as u64,
                                )
                            })
                            .collect(),
                    )
                }
                _ => ProfileLoop::new(
                    (0..1 + r.below(5))
                        .map(|k| {
                            let s = match r.below(3) {
                                0 => Segment::Line { a: p2(r), b: p2(r) },
                                1 => Segment::Arc {
                                    center: p2(r),
                                    radius: num(r),
                                    start: angle(r),
                                    sweep: angle(r),
                                },
                                _ => Segment::Circle {
                                    center: p2(r),
                                    radius: num(r),
                                    ccw: r.bool(),
                                },
                            };
                            (
                                s,
                                if r.below(4) == 0 {
                                    r.below(5000) as u64
                                } else {
                                    id + k as u64
                                },
                            )
                        })
                        .collect(),
                ),
            };
            id = l.segments.last().map_or(id, |s| s.1) + if r.bool() { 1 } else { 1 + r.below(100) as u64 };
            loops.push(l);
        }
        Profile {
            plane: frame(r),
            loops,
        }
    }

    fn names(r: &mut Rng) -> Vec<Name> {
        (0..r.below(4)).map(|_| Name(r.next_u64())).collect()
    }

    fn op(r: &mut Rng) -> Combine {
        OPS[r.below(OPS.len())].1
    }

    fn cutter(r: &mut Rng, i: usize) -> Cutter {
        match r.below(5) {
            0 => Cutter::Plane(frame(r)),
            1 => Cutter::Curve(frame(r), Segment::Line { a: p2(r), b: p2(r) }),
            2 => Cutter::Curve(
                frame(r),
                Segment::Arc {
                    center: p2(r),
                    radius: num(r),
                    start: angle(r),
                    sweep: angle(r),
                },
            ),
            3 => Cutter::Curve(
                frame(r),
                Segment::Circle {
                    center: p2(r),
                    radius: num(r),
                    ccw: true,
                },
            ),
            _ => Cutter::Target(target(r, i)),
        }
    }

    fn steps(r: &mut Rng) -> Vec<Step> {
        (0..1 + r.below(3))
            .map(|_| match r.below(3) {
                0 => Step::Translate(Vec3::new(num(r), num(r), num(r))),
                1 => Step::Rotate {
                    origin: Point3::new(num(r), num(r), num(r)),
                    axis: Vec3::new(num(r), num(r), num(r)),
                    angle: angle(r),
                },
                _ => Step::Frame(frame(r)),
            })
            .collect()
    }

    fn target(r: &mut Rng, i: usize) -> Target {
        let feature = r.below(i + 3);
        if r.bool() {
            Target::Plane(feature)
        } else {
            Target::Face {
                feature,
                name: Name(r.next_u64()),
            }
        }
    }

    fn row(r: &mut Rng) -> Row {
        Row {
            dir: Vec3::new(num(r), num(r), num(r)),
            count: r.below(9),
            spacing: num(r),
        }
    }

    /// A parameter, maybe over one above it.
    fn param(r: &mut Rng, before: &[Feature]) -> Feature {
        let above: Vec<&Param> = before
            .iter()
            .filter_map(|f| match f {
                Feature::Param(p) => Some(&**p),
                _ => None,
            })
            .collect();
        let (expr, unit) = match (r.below(3), above.get(r.below(above.len().max(1)))) {
            (0, Some(p)) => (format!("{}*2 + 1", p.name), Unit::None),
            (1, _) => ("sqrt(16) - (1 + 2)^2/9".into(), Unit::None),
            _ => (fmt_num(num(r)), [Unit::None, Unit::Mm, Unit::Deg][r.below(3)]),
        };
        let name = format!("v{}", before.len());
        Feature::Param(Box::new(
            new_param(before, name, expr, unit).expect("a good parameter"),
        ))
    }

    /// A sketch: a rectangle, maybe a hole, held by a few constraints.
    fn sketch(r: &mut Rng, before: &[Feature]) -> Feature {
        let w = |r: &mut Rng| fmt_num(1.0 + r.below(400) as f64 * 0.25);
        let (x, y) = (fmt_num(r.range(-100.0, 100.0)), fmt_num(r.range(-100.0, 100.0)));
        let (ww, hh) = (w(r), w(r));
        let mut rows = vec![format!("rect {x} {y} {ww} {hh}"), "fix l1.a".into()];
        if r.bool() {
            rows.push(format!("dist l1 {ww}"));
        }
        if r.bool() {
            rows.push("circle l1.a 0.5 cons".into());
            rows.push("rad c1 (1 + 1)/4".into());
        }
        let rows = rows.iter().map(|row| tokens(row)).collect();
        // on a plane or sketch above, flipped or not, or on a frame of its own
        let on = before
            .iter()
            .rposition(|f| matches!(f, Feature::Plane(_) | Feature::Sketch(_)))
            .filter(|_| r.bool())
            .map(|feature| (feature, r.bool()));
        let plane = match on.map(|(i, flip)| (&before[i], flip)) {
            Some((Feature::Plane(f), flip)) => {
                if flip {
                    f.flipped()
                } else {
                    *f
                }
            }
            Some((Feature::Sketch(s), flip)) => {
                if flip {
                    s.plane.flipped()
                } else {
                    s.plane
                }
            }
            _ => frame(r),
        };
        let mut s =
            sketch::build(plane, rows, &|t, want| value(t, before, want), None).expect("a good sketch");
        s.on = on.map(|(feature, flip)| forge_model::On::Feature {
            feature,
            point: None,
            flip,
        });
        Feature::Sketch(Box::new(s))
    }

    /// One random feature of every kind there is, every field drawn at random.
    /// One random feature of every kind there is, every field drawn at random, sometimes on
    /// chosen bodies.
    fn feature(r: &mut Rng, i: usize, before: &[Feature]) -> Feature {
        let f = bare(r, i, before);
        if matches!(f, Feature::Param(_) | Feature::Sketch(_)) {
            return f;
        }
        match r.below(6) {
            0 => body(BodyFeature::In {
                bodies: Pick::All,
                feature: f,
            }),
            1 => body(BodyFeature::In {
                bodies: Pick::Some((0..1 + r.below(3)).map(|_| r.below(i + 3)).collect()),
                feature: f,
            }),
            _ => f,
        }
    }

    fn bare(r: &mut Rng, i: usize, before: &[Feature]) -> Feature {
        match r.below(KINDS.len()) {
            0 => Feature::Extrude {
                profile: profile(r),
                extent: match r.below(6) {
                    5 => Extent::ToNext { reverse: r.bool() },
                    0 => Extent::Distance(num(r)),
                    1 => Extent::Symmetric(num(r)),
                    2 => Extent::TwoSided {
                        forward: num(r),
                        back: num(r),
                    },
                    3 => Extent::Through([Toward::Forward, Toward::Back, Toward::Both][r.below(3)]),
                    _ => Extent::To {
                        target: target(r, i),
                        offset: if r.bool() { 0.0 } else { num(r) },
                    },
                },
                op: op(r),
            },
            1 => Feature::Revolve {
                profile: profile(r),
                axis: match r.below(3) {
                    0 => Axis2 {
                        origin: Point2::ORIGIN,
                        dir: Vec2::Y,
                    },
                    1 => Axis2 {
                        origin: Point2::ORIGIN,
                        dir: Vec2::X,
                    },
                    _ => Axis2 {
                        origin: p2(r),
                        dir: Vec2::new(num(r), num(r)),
                    },
                },
                extent: match r.below(4) {
                    0 => Turn::Angle(angle(r)),
                    1 => Turn::Symmetric(angle(r)),
                    2 => Turn::TwoSided {
                        forward: angle(r),
                        back: angle(r),
                    },
                    _ => Turn::To {
                        target: target(r, i),
                        offset: if r.bool() { 0.0 } else { angle(r) },
                    },
                },
                op: op(r),
            },
            2 => Feature::Ball {
                frame: frame(r),
                radius: num(r),
                op: op(r),
            },
            3 => {
                let mut edges = Vec::new();
                let law = |r: &mut Rng| match r.below(4) {
                    0 => Radius::Linear(num(r), num(r)),
                    1 => Radius::Points((0..r.below(4)).map(|_| (num(r), num(r))).collect()),
                    _ => Radius::Constant(num(r)),
                };
                let rad = law(r);
                for _ in 0..1 + r.below(5) {
                    edges.push((
                        Name(r.next_u64()),
                        if r.below(3) == 0 { law(r) } else { rad.clone() },
                    ));
                }
                let mut starts = Vec::new();
                for e in &edges {
                    if r.below(4) == 0 {
                        starts.push((e.0, Name(r.next_u64())));
                    }
                }
                let setbacks = (0..r.below(3))
                    .map(|_| (Name(r.next_u64()), r.bool().then(|| Name(r.next_u64())), num(r)))
                    .collect();
                Feature::Fillet {
                    edges,
                    starts,
                    setbacks,
                }
            }
            4 => Feature::Chamfer {
                edges: names(r),
                size: match r.below(3) {
                    0 => ChamferSize::Equal(num(r)),
                    1 => ChamferSize::Distances {
                        face: Name(r.next_u64()),
                        d1: num(r),
                        d2: num(r),
                    },
                    _ => ChamferSize::DistanceAngle {
                        face: Name(r.next_u64()),
                        d: num(r),
                        angle: angle(r),
                    },
                },
            },
            5 => Feature::Shell {
                thickness: num(r),
                outward: r.bool(),
                open: names(r),
            },
            6 => Feature::Draft {
                faces: names(r),
                neutral: frame(r),
                angle: angle(r),
            },
            7 if r.below(3) == 0 => {
                let screw = &standard::SCREWS[r.below(standard::SCREWS.len())];
                let std = Standard {
                    screw,
                    convention: if screw.inch() {
                        Convention::Ansi
                    } else {
                        CONVENTIONS[r.below(2)].1
                    },
                    fit: FITS[r.below(4)].1,
                };
                Feature::Hole(Box::new(Hole {
                    at: frame(r),
                    diameter: std.diameter(),
                    head: match r.below(4) {
                        0 => std.counterbore(),
                        1 => std.countersink().unwrap_or(Head::Plain),
                        2 => Head::Counterbore {
                            diameter: num(r),
                            depth: num(r),
                        },
                        _ => Head::Plain,
                    },
                    end: End::Depth(num(r)),
                    point: r.bool().then(|| angle(r)),
                    thread: std.thread(),
                    standard: Some(std),
                }))
            }
            7 => Feature::Hole(Box::new(Hole {
                at: frame(r),
                diameter: num(r),
                head: match r.below(3) {
                    0 => Head::Plain,
                    1 => Head::Counterbore {
                        diameter: num(r),
                        depth: num(r),
                    },
                    _ => Head::Countersink {
                        diameter: num(r),
                        angle: angle(r),
                    },
                },
                end: match r.below(4) {
                    3 => End::ToNext,
                    0 => End::Through,
                    1 => End::Depth(num(r)),
                    _ => End::To {
                        target: target(r, i),
                        offset: num(r),
                    },
                },
                point: r.bool().then(|| angle(r)),
                thread: r.bool().then(|| Thread {
                    size: num(r),
                    pitch: num(r),
                }),
                standard: None,
            })),
            8 => Feature::Pattern {
                of: if r.bool() {
                    Source::Body
                } else {
                    Source::Features((0..r.below(4)).map(|_| r.below(i + 3)).collect())
                },
                layout: match r.below(5) {
                    3 => Layout::Path {
                        along: (0..1 + r.below(4)).map(|_| wref(r, i)).collect(),
                        count: r.below(30),
                        spacing: r.bool().then(|| num(r)),
                        turn: r.bool(),
                        reverse: r.bool(),
                    },
                    4 => Layout::Points(
                        (0..r.below(5))
                            .map(|_| {
                                if r.bool() {
                                    At::Xyz(Point3::new(num(r), num(r), num(r)))
                                } else {
                                    // a sketch named alone stands for its points
                                    let mut w = wref(r, i);
                                    if matches!(before.get(w.feature), Some(Feature::Sketch(_))) {
                                        w.name = Some(Name(r.next_u64()));
                                    }
                                    At::Ref(w)
                                }
                            })
                            .collect(),
                    ),
                    0 => Layout::Linear {
                        first: row(r),
                        second: r.bool().then(|| row(r)),
                    },
                    1 => Layout::Circular {
                        origin: Point3::new(num(r), num(r), num(r)),
                        axis: Vec3::new(num(r), num(r), num(r)),
                        count: r.below(30),
                        angle: angle(r),
                        incremental: r.bool(),
                    },
                    _ => Layout::Mirror { plane: frame(r) },
                },
            },
            10 => Feature::Sweep(Box::new(Sweep {
                profile: profile(r),
                path: path(r),
                ends: match r.below(3) {
                    0 => PathEnds::Path,
                    1 => PathEnds::Extended {
                        start: num(r),
                        end: num(r),
                    },
                    _ => PathEnds::To {
                        target: target(r, i),
                        offset: num(r),
                    },
                },
                op: op(r),
            })),
            11 => Feature::Coil(Box::new(Coil {
                profile: profile(r),
                axis: Axis2 {
                    origin: p2(r),
                    dir: p2(r).to_vec(),
                },
                size: match r.below(4) {
                    0 => CoilSize::PitchTurns {
                        pitch: num(r),
                        turns: num(r),
                    },
                    1 => CoilSize::TurnsHeight {
                        turns: num(r),
                        height: num(r),
                    },
                    2 => CoilSize::PitchHeight {
                        pitch: num(r),
                        height: num(r),
                    },
                    _ => CoilSize::To {
                        pitch: num(r),
                        target: target(r, i),
                        offset: num(r),
                    },
                },
                taper: if r.bool() { 0.0 } else { angle(r) },
                handed: if r.bool() { Handed::Right } else { Handed::Left },
                op: op(r),
            })),
            12 => Feature::Loft(Box::new(Loft {
                profiles: (0..1 + r.below(4)).map(|_| profile(r)).collect(),
                kind: if r.bool() {
                    LoftKind::Ruled
                } else {
                    LoftKind::Smooth {
                        start: r.bool().then(|| num(r)),
                        end: r.bool().then(|| num(r)),
                    }
                },
                guides: match r.below(3) {
                    0 => LoftGuides::None,
                    1 => LoftGuides::Rails((0..1 + r.below(3)).map(|_| (r.below(8), path(r))).collect()),
                    _ => LoftGuides::CentreLine(path(r)),
                },
                op: op(r),
            })),
            14 => Feature::Edit(Edit::Rib(Box::new(Rib {
                plane: frame(r),
                path: (0..1 + r.below(3))
                    .map(|_| {
                        if r.bool() {
                            Segment::Line { a: p2(r), b: p2(r) }
                        } else {
                            Segment::Arc {
                                center: p2(r),
                                radius: num(r),
                                start: angle(r),
                                sweep: angle(r),
                            }
                        }
                    })
                    .collect(),
                thickness: num(r),
                side: SIDES[r.below(3)].1,
                web: r.bool(),
                flip: r.bool(),
                draft: if r.bool() { 0.0 } else { angle(r) },
            }))),
            15 => Feature::Edit(Edit::Offset {
                faces: names(r),
                distance: num(r),
            }),
            16 => Feature::Edit(Edit::Thicken {
                faces: names(r),
                thickness: num(r),
                side: SIDES[r.below(3)].1,
                op: op(r),
            }),
            17 => Feature::Edit(Edit::Delete { faces: names(r) }),
            18 => Feature::Edit(Edit::Replace {
                face: Name(r.next_u64()),
                with: target(r, i),
            }),
            13 => Feature::Edit(Edit::Move {
                faces: names(r),
                motion: if r.bool() {
                    Motion::Translate(Vec3::new(num(r), num(r), num(r)))
                } else {
                    Motion::Rotate {
                        origin: Point3::new(num(r), num(r), num(r)),
                        axis: Vec3::new(num(r), num(r), num(r)),
                        angle: angle(r),
                    }
                },
            }),
            19 => param(r, before),
            20 => sketch(r, before),
            21 => {
                let spec = |r: &mut Rng| {
                    let pos = |r: &mut Rng| num(r).abs().max(1e-3);
                    if r.bool() {
                        ThreadSpec::Iso {
                            major: pos(r),
                            pitch: pos(r),
                        }
                    } else {
                        ThreadSpec::Uts {
                            major: pos(r),
                            tpi: pos(r),
                        }
                    }
                };
                Feature::Thread(Box::new(ScrewThread {
                    on: match r.below(3) {
                        0 => ThreadOn::Hole(r.below(i + 3)),
                        1 => ThreadOn::Face {
                            feature: r.below(i + 3),
                            name: Name(r.next_u64()),
                            spec: spec(r),
                            flip: r.bool(),
                        },
                        _ => ThreadOn::Axis {
                            at: frame(r),
                            spec: spec(r),
                            internal: r.bool(),
                        },
                    },
                    length: match r.below(3) {
                        0 => ThreadLength::Full,
                        1 => ThreadLength::Distance(num(r)),
                        _ => ThreadLength::To {
                            target: target(r, i),
                            offset: num(r),
                        },
                    },
                    handed: if r.bool() { Handed::Right } else { Handed::Left },
                    chamfer: r.bool(),
                    runout: r.bool(),
                }))
            }
            22 => {
                let chars = ['a', 'Z', ' ', '"', '\\', '\n', '#', '(', ')', 'é', '字', ':'];
                let string = |r: &mut Rng| -> String {
                    (0..r.below(8)).map(|_| chars[r.below(chars.len())]).collect()
                };
                Feature::Text(Box::new(Text {
                    text: string(r),
                    font: FontRef::named(&string(r)),
                    size: num(r),
                    on: if r.bool() {
                        TextOn::Plane(frame(r))
                    } else {
                        TextOn::Cylinder {
                            axis: frame(r),
                            radius: num(r),
                        }
                    },
                    depth: num(r),
                    op: if r.bool() { Combine::Add } else { Combine::Cut },
                }))
            }
            23 => Feature::Work(Work::Plane(match r.below(7) {
                0 => PlaneBy::Offset(wref(r, i), num(r)),
                1 => PlaneBy::Angle {
                    plane: wref(r, i),
                    axis: wref(r, i),
                    angle: angle(r),
                },
                2 => PlaneBy::Tangent {
                    face: wref(r, i),
                    plane: wref(r, i),
                },
                3 => PlaneBy::Points([wref(r, i), wref(r, i), wref(r, i)]),
                4 => PlaneBy::Mid(wref(r, i), wref(r, i)),
                5 => PlaneBy::Normal {
                    curve: wref(r, i),
                    at: wref(r, i),
                },
                _ => return Feature::Plane(frame(r)),
            })),
            24 => Feature::Work(Work::Axis(match r.below(4) {
                0 => AxisBy::Points(wref(r, i), wref(r, i)),
                1 => AxisBy::Along(wref(r, i)),
                2 => AxisBy::Meet(wref(r, i), wref(r, i)),
                _ => AxisBy::Normal {
                    plane: wref(r, i),
                    at: wref(r, i),
                },
            })),
            25 => Feature::Work(Work::Point(match r.below(3) {
                0 => PointBy::At(wref(r, i)),
                1 => PointBy::Mid(wref(r, i)),
                _ => PointBy::Meet(wref(r, i), wref(r, i)),
            })),
            26 => Feature::Work(Work::Ucs(if r.bool() {
                UcsBy::Frame(frame(r))
            } else {
                UcsBy::Points([wref(r, i), wref(r, i), wref(r, i)])
            })),
            27 => body(BodyFeature::Combine {
                op: [Combine::Add, Combine::Cut, Combine::Keep][r.below(3)],
                target: r.below(i + 3),
                tools: (0..1 + r.below(3)).map(|_| r.below(i + 3)).collect(),
                keep: r.bool(),
            }),
            28 => body(BodyFeature::Split {
                by: cutter(r, i),
                keep: [Keep::Plus, Keep::Minus, Keep::Both][r.below(3)],
            }),
            29 => body(BodyFeature::SplitFace {
                faces: (0..1 + r.below(3)).map(|_| Name(r.next_u64())).collect(),
                by: cutter(r, i),
            }),
            _ => body(BodyFeature::Move {
                steps: steps(r),
                copy: r.bool(),
            }),
        }
    }

    /// A reference to a feature near `i`, or to an entity of its solid.
    fn wref(r: &mut Rng, i: usize) -> Ref {
        Ref {
            feature: r.below(i + 3),
            name: r.bool().then(|| Name(r.next_u64())),
        }
    }

    fn path(r: &mut Rng) -> Vec<PathSeg> {
        (0..1 + r.below(4))
            .map(|_| {
                if r.bool() {
                    PathSeg::Line {
                        a: Point3::new(num(r), num(r), num(r)),
                        b: Point3::new(num(r), num(r), num(r)),
                    }
                } else {
                    PathSeg::Arc {
                        circle: Circle {
                            frame: frame(r),
                            radius: num(r),
                        },
                        start: angle(r),
                        sweep: angle(r),
                    }
                }
            })
            .collect()
    }

    #[test]
    fn every_kind_round_trips() {
        for seed in 0..400 {
            let mut r = Rng::new(seed);
            let mut part = Part::default();
            for i in 0..1 + r.below(12) {
                let f = feature(&mut r, i, &part.features);
                part.features.push(f);
            }
            round_trip(&format!("random {seed}"), &part);
        }
    }

    #[test]
    fn the_suites_round_trip() {
        for (name, part) in suite::parts() {
            round_trip(name, &part);
        }
        for seed in 1..=20 {
            round_trip("fuzz", &suite::fuzz(seed, 80));
            round_trip("finishing fuzz", &finishing::fuzz(seed));
            round_trip("edits fuzz", &edits::fuzz(seed));
        }
        round_trip("torture", &torture::part(300));
        for (name, part, _) in fillets::cases()
            .into_iter()
            .chain(finishing::parts())
            .chain(edits::parts())
        {
            round_trip(name, &part);
        }
        for (name, part, _) in fillets::refusals() {
            round_trip(name, &part);
        }
        for (name, family) in adversarial::families() {
            for seed in 0..5 {
                round_trip(name, &family(seed));
            }
        }
    }

    #[test]
    fn parameters_and_sketches_print_as_written() {
        let src = "w = 40\nd = w*3 + 1 mm\na = -30 deg\nsketch at 0 0 6\n  rect 0 0 w (d - 1)\n  circle l1.a 2 cons\n  ang l1 l2 90\nend\nextrude 6 sketch1\n";
        let part = parse(src).expect("parses");
        let printed = print(&part);
        assert_eq!(
            printed.lines().take(8).collect::<Vec<_>>(),
            src.lines().take(8).collect::<Vec<_>>()
        );
        assert_eq!(parse(&printed), Ok(part));
    }

    #[test]
    fn reads_like_the_examples() {
        let part = Part {
            features: vec![
                Feature::Extrude {
                    profile: Profile {
                        plane: Frame::WORLD,
                        loops: vec![Profile::rectangle(Point2::new(-30.0, -20.0), 60.0, 40.0, 0)],
                    },
                    extent: Extent::Distance(6.0),
                    op: Combine::New,
                },
                Feature::Hole(Box::new(Hole {
                    at: Frame::from_zx(Point3::new(-20.0, -10.0, 6.0), -forge_math::UnitVec3::Z, Vec3::X)
                        .expect("frame"),
                    diameter: 5.0,
                    head: Head::Plain,
                    end: End::Through,
                    point: None,
                    thread: None,
                    standard: None,
                })),
                Feature::Pattern {
                    of: Source::Features(vec![1]),
                    layout: Layout::Linear {
                        first: Row {
                            dir: Vec3::X,
                            count: 3,
                            spacing: 20.0,
                        },
                        second: None,
                    },
                },
            ],
        };
        assert_eq!(
            print(&part),
            "extrude new 6 rect -30 -20 60 40\nhole 5 -xy at -20 -10 6 thru\npattern hole1 lin 1 0 0 3 20\n"
        );
    }
}
