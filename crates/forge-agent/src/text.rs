//! The part file: a feature history as text, one feature per line (grammar in AGENTS.md).
//! Each feature kind is one row of `KINDS`: its keyword, its syntax (shown as a hint on
//! errors), a parser and a printer built from the shared helpers below.

use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_model::{
    Combine, End, Extent, Feature, Head, Hole, Layout, Part, Row, Source, Target, Thread, Toward, Turn,
};
use forge_ops::{Axis2, ChamferSize, Profile, ProfileLoop, Segment};
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
        syntax: "extrude [op] EXT FRAME LOOPS",
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
        syntax: "revolve [op] [TURN] FRAME [axis x|y|OX OY DX DY] LOOPS",
        parse: |p| {
            let op = p.op()?;
            let extent = p.turn()?;
            let plane = p.frame()?;
            let axis = p.axis()?;
            let loops = p.loops()?;
            Ok(Feature::Revolve {
                profile: Profile { plane, loops },
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
        syntax: "fillet R REF.. [R REF..]",
        parse: |p| {
            let mut edges = Vec::new();
            let mut r = p.num()?;
            while !p.done() {
                if p.is_num() {
                    r = p.num()?;
                } else {
                    edges.extend(p.one_or_more_refs()?.into_iter().map(|n| (n, r)));
                }
            }
            Ok(Feature::Fillet { edges })
        },
        print: |f, w| {
            let Feature::Fillet { edges } = f else {
                return false;
            };
            let mut r = None;
            for &(n, er) in edges {
                if r.is_none_or(|r: f64| r.to_bits() != er.to_bits()) {
                    w.num(er);
                    r = Some(er);
                }
                w.name(n);
            }
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
        syntax: "hole D FRAME [cb D2 DEPTH | cs D2 A] (N | thru | to FEAT [@face] [off N]) [pt A] [thread SIZE PITCH]",
        parse: |p| {
            let diameter = p.num()?;
            let at = p.frame()?;
            let head = if p.eat("cb") {
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
                Some(Thread {
                    size: p.num()?,
                    pitch: p.num()?,
                })
            } else {
                None
            };
            Ok(Feature::Hole(Box::new(Hole {
                at,
                diameter,
                head,
                end,
                point,
                thread,
            })))
        },
        print: |f, w| {
            let Feature::Hole(h) = f else { return false };
            w.num(h.diameter);
            w.frame(&h.at);
            match h.head {
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
            if let Some(t) = h.thread {
                w.word("thread");
                w.num(t.size);
                w.num(t.pitch);
            }
            true
        },
    },
    Kind {
        name: "pattern",
        syntax: "pattern (body | FEAT..) (lin DX DY DZ N STEP [DX DY DZ N STEP] | circ OX OY OZ AX AY AZ N [A] [step] | mirror FRAME)",
        parse: |p| {
            let of = if p.eat("body") {
                Source::Body
            } else {
                let mut src = Vec::new();
                while !p.done() && !["lin", "circ", "mirror"].contains(&p.peek()) {
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
                t => return Err(format!("expected lin, circ or mirror, found '{t}'")),
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
            }
            true
        },
    },
    Kind {
        name: "plane",
        syntax: "plane FRAME",
        parse: |p| Ok(Feature::Plane(p.frame()?)),
        print: |f, w| {
            let Feature::Plane(frame) = f else { return false };
            w.frame(frame);
            true
        },
    },
];

type R<T> = Result<T, String>;

/// One line of a part file that holds a feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// 1-based, in the file.
    pub no: usize,
    pub label: Option<String>,
    /// The kind first, then its arguments; a parenthesized selector is one token.
    pub toks: Vec<String>,
}

/// The feature lines of a file (blank and comment-only lines skipped).
pub fn lines(src: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for (k, raw) in src.lines().enumerate() {
        let text = raw.split('#').next().unwrap_or("");
        let mut toks: Vec<String> = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else if c == '(' {
                let mut t = String::new();
                let mut depth = 0;
                for c in chars.by_ref() {
                    t.push(c);
                    depth += (c == '(') as i32 - (c == ')') as i32;
                    if depth == 0 {
                        break;
                    }
                }
                toks.push(t);
            } else {
                let mut t = String::new();
                while let Some(&c) = chars.peek().filter(|c| !c.is_whitespace() && **c != '(') {
                    t.push(c);
                    chars.next();
                }
                toks.push(t);
            }
        }
        let label = match toks.first() {
            Some(t) if t.len() > 1 && t.ends_with(':') => {
                Some(toks.remove(0).trim_end_matches(':').to_string())
            }
            _ => None,
        };
        if !toks.is_empty() {
            out.push(Line {
                no: k + 1,
                label,
                toks,
            });
        }
    }
    out
}

/// What a line may refer to: the features before it, their labels and kinds, and a resolver
/// for `@handle` and `(selector)` references against the part as it stands there.
pub struct Before<'a> {
    pub features: &'a [Feature],
    pub labels: &'a [Option<String>],
    pub kinds: &'a [&'static str],
    pub refs: &'a dyn Fn(&str) -> R<Vec<Name>>,
}

pub fn kind(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.name == name)
}

/// Parses one feature line.
pub fn parse_line(line: &Line, before: &Before) -> R<Feature> {
    let k = kind(&line.toks[0]).ok_or_else(|| {
        let names: Vec<&str> = KINDS.iter().map(|k| k.name).collect();
        format!("unknown feature '{}' (one of: {})", line.toks[0], names.join(" "))
    })?;
    let mut p = P {
        t: &line.toks,
        k: 1,
        b: before,
    };
    let f = (k.parse)(&mut p)?;
    match p.t.get(p.k) {
        Some(t) => Err(format!("unexpected '{t}'")),
        None => Ok(f),
    }
}

/// The kind of a feature.
pub fn kind_of(f: &Feature) -> &'static Kind {
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
        w.s.push_str(kinds[i]);
        (kind_of(f).print)(f, &mut w);
        out.push_str(&w.s);
        out.push('\n');
    }
    out
}

/// Parses a file whose references are all full names (as `print` writes them).
pub fn parse(src: &str) -> Result<Part, (usize, String)> {
    let full = |r: &str| -> R<Vec<Name>> {
        match full_name(r) {
            Some(n) => Ok(vec![n]),
            None => Err(format!("'{r}' needs a part to resolve against")),
        }
    };
    let ls = lines(src);
    let labels: Vec<Option<String>> = ls.iter().map(|l| l.label.clone()).collect();
    let mut kinds = Vec::new();
    let mut features = Vec::new();
    for (i, l) in ls.iter().enumerate() {
        let b = Before {
            features: &features,
            labels: &labels[..i],
            kinds: &kinds,
            refs: &full,
        };
        let f = parse_line(l, &b).map_err(|e| (l.no, e))?;
        features.push(f);
        kinds.push(kind(&l.toks[0]).map(|k| k.name).unwrap_or(""));
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
            f,
            Feature::Extrude { .. } | Feature::Revolve { .. } | Feature::Ball { .. }
        )
    };
    if before.iter().any(solid) {
        Combine::Add
    } else {
        Combine::New
    }
}

const OPS: [(&str, Combine); 4] = [
    ("new", Combine::New),
    ("add", Combine::Add),
    ("cut", Combine::Cut),
    ("keep", Combine::Keep),
];

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
}

impl P<'_> {
    fn peek(&self) -> &str {
        self.t.get(self.k).map_or("", |s| s.as_str())
    }
    fn done(&self) -> bool {
        self.k >= self.t.len()
    }
    fn next(&mut self) -> R<&str> {
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
        t.parse::<f64>().is_ok() || t.strip_suffix('r').is_some_and(|s| s.parse::<f64>().is_ok())
    }
    fn num(&mut self) -> R<f64> {
        let t = self.next()?;
        t.parse().map_err(|_| {
            let axisy = t
                .trim_start_matches(['+', '-'])
                .chars()
                .all(|c| "xyz".contains(c));
            let orient = if axisy {
                " (orientations: xy xz yz -xy -xz -yz, or z A B C)"
            } else {
                ""
            };
            format!("expected a number, found '{t}'{orient}")
        })
    }
    fn angle(&mut self) -> R<f64> {
        let t = self.next()?;
        match t.strip_suffix('r') {
            Some(r) => r.parse().map_err(|_| format!("expected an angle, found '{t}'")),
            None => t
                .parse()
                .map(deg)
                .map_err(|_| format!("expected an angle, found '{t}'")),
        }
    }
    fn count(&mut self) -> R<usize> {
        let t = self.next()?;
        t.parse().map_err(|_| format!("expected a count, found '{t}'"))
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
    fn target(&mut self) -> R<Target> {
        let feature = self.feat()?;
        if self.peek().starts_with(['@', '(']) {
            Ok(Target::Face {
                feature,
                name: self.one_ref()?,
            })
        } else {
            Ok(Target::Plane(feature))
        }
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
        } else if t == "on" {
            self.k += 1;
            let i = self.feat()?;
            match self.b.features[i] {
                Feature::Plane(f) => f,
                _ => return Err(format!("'on' needs a plane feature; line {} is not one", i + 1)),
            }
        } else {
            Frame::WORLD
        };
        if self.eat("at") {
            f = f.with_origin(self.point3()?);
        }
        Ok(f)
    }
    fn axis(&mut self) -> R<Axis2> {
        if !self.eat("axis") {
            return Ok(Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
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
        Ok(Profile {
            plane: self.frame()?,
            loops: self.loops()?,
        })
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
    fn refs(&mut self) -> R<Vec<Name>> {
        let mut out = Vec::new();
        while self.peek().starts_with(['@', '(']) {
            let t = self.next()?.to_string();
            out.extend((self.b.refs)(&t)?);
        }
        Ok(out)
    }
    fn one_or_more_refs(&mut self) -> R<Vec<Name>> {
        if !self.peek().starts_with(['@', '(']) {
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
    fn angle(&mut self, a: f64) {
        self.word(&fmt_angle(a));
    }
    fn name(&mut self, n: Name) {
        self.word(&format!("@{:016x}", n.0));
    }
    fn names(&mut self, ns: &[Name]) {
        ns.iter().for_each(|&n| self.name(n));
    }
    fn op(&mut self, op: Combine) {
        if op != default_op(self.before) {
            self.word(OPS.iter().find(|o| o.1 == op).expect("every op is named").0);
        }
    }
    fn feat(&mut self, i: usize) {
        if i < self.before.len() {
            let k = self.kinds[i];
            let n = self.kinds[..=i].iter().filter(|x| **x == k).count();
            self.word(&format!("{k}{n}"));
        } else {
            self.word(&format!("f{}", i + 1));
        }
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
    use forge_model::{adversarial, fillets, finishing, suite, torture};

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
        OPS[r.below(4)].1
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

    /// One random feature of every kind there is, every field drawn at random.
    fn feature(r: &mut Rng, i: usize) -> Feature {
        match r.below(KINDS.len()) {
            0 => Feature::Extrude {
                profile: profile(r),
                extent: match r.below(5) {
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
                let rad = num(r);
                for _ in 0..1 + r.below(5) {
                    edges.push((Name(r.next_u64()), if r.below(3) == 0 { num(r) } else { rad }));
                }
                Feature::Fillet { edges }
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
                end: match r.below(3) {
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
            })),
            8 => Feature::Pattern {
                of: if r.bool() {
                    Source::Body
                } else {
                    Source::Features((0..r.below(4)).map(|_| r.below(i + 3)).collect())
                },
                layout: match r.below(3) {
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
            _ => Feature::Plane(frame(r)),
        }
    }

    #[test]
    fn every_kind_round_trips() {
        for seed in 0..400 {
            let mut r = Rng::new(seed);
            let mut part = Part::default();
            for i in 0..1 + r.below(12) {
                part.features.push(feature(&mut r, i));
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
        }
        round_trip("torture", &torture::part(300));
        for (name, part, _) in fillets::cases().into_iter().chain(finishing::parts()) {
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
            "extrude 6 rect -30 -20 60 40\nhole 5 -xy at -20 -10 6 thru\npattern hole1 lin 1 0 0 3 20\n"
        );
    }
}
