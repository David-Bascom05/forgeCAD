//! A URDF file as an unlinked robot document (`forgecad import ROBOT.urdf`), for editing robots
//! made elsewhere. Every link, inertial, visual and collision (mesh, box, cylinder, sphere),
//! joint (type, origin, axis, limits, effort, velocity, dynamics, mimic) and simple transmission
//! is written as the document's own statements, in the URDF's order; meshes are referenced where
//! they are (a relative path from the document, or a `package://` URI as written), not copied.
//! Units become the document's: mm, kg, kg mm², angles in radians as bare numbers (exact).
//! What URDF has and the document does not (gazebo tags, sensors, a calibration, safety
//! controllers) is listed as dropped, never silently.

use crate::robot::is_robot;
use forge_math::{Frame, Point3, Vec3, dmath};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// An XML element: name, attributes in order, children, text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct El {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<El>,
    pub text: String,
}

impl El {
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
    }
    pub fn kid(&self, n: &str) -> Option<&El> {
        self.kids.iter().find(|k| k.name == n)
    }
    pub fn all<'a>(&'a self, n: &'a str) -> impl Iterator<Item = &'a El> + 'a {
        self.kids.iter().filter(move |k| k.name == n)
    }
}

fn unescape(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = s;
    while let Some(a) = rest.find('&') {
        out.push_str(&rest[..a]);
        let e = rest[a..].find(';').ok_or("an unterminated entity")? + a;
        let name = &rest[a + 1..e];
        match name {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let code = if let Some(h) = name.strip_prefix("#x") {
                    u32::from_str_radix(h, 16).ok()
                } else if let Some(d) = name.strip_prefix('#') {
                    d.parse().ok()
                } else {
                    None
                };
                out.push(code.and_then(char::from_u32).ok_or(format!("entity &{name};"))?);
            }
        }
        rest = &rest[e + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Reads an XML document: its root element. Declarations, comments, processing instructions
/// and a DOCTYPE are skipped; CDATA is text.
pub fn parse_xml(xml: &str) -> Result<El, String> {
    let mut stack: Vec<El> = vec![El::default()];
    let mut i = 0;
    let b = xml.as_bytes();
    while i < b.len() {
        if b[i] != b'<' {
            let j = xml[i..].find('<').map_or(b.len(), |k| i + k);
            let t = unescape(&xml[i..j])?;
            if stack.len() == 1 {
                if !t.trim().is_empty() {
                    return Err("text outside the root element".into());
                }
            } else if let Some(top) = stack.last_mut() {
                top.text.push_str(&t);
            }
            i = j;
            continue;
        }
        let rest = &xml[i..];
        if let Some(r) = rest.strip_prefix("<!--") {
            i += 4 + r.find("-->").ok_or("an unterminated comment")? + 3;
            continue;
        }
        if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let e = r.find("]]>").ok_or("an unterminated CDATA")?;
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&r[..e]);
            }
            i += 9 + e + 3;
            continue;
        }
        if rest.starts_with("<?") || rest.starts_with("<!") {
            i += rest.find('>').ok_or("an unterminated declaration")? + 1;
            continue;
        }
        // a tag: find its end outside quotes
        let mut q = None;
        let mut end = None;
        for (k, c) in rest.char_indices().skip(1) {
            match (q, c) {
                (None, '"' | '\'') => q = Some(c),
                (Some(x), c) if c == x => q = None,
                (None, '>') => {
                    end = Some(k);
                    break;
                }
                _ => {}
            }
        }
        let end = end.ok_or("an unterminated tag")?;
        let tag = &rest[1..end];
        i += end + 1;
        if let Some(name) = tag.strip_prefix('/') {
            let el = stack.pop().ok_or("a closing tag with nothing open")?;
            if el.name != name.trim() || stack.is_empty() {
                return Err(format!("</{}> closes <{}>", name.trim(), el.name));
            }
            stack.last_mut().ok_or("no parent")?.kids.push(el);
            continue;
        }
        let selfclose = tag.trim_end().ends_with('/');
        let tag = tag.trim_end().trim_end_matches('/');
        let (name, mut rest) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        let mut el = El {
            name: name.to_string(),
            ..El::default()
        };
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            let eq = rest
                .find('=')
                .ok_or_else(|| format!("<{name}>: an attribute with no ="))?;
            let k = rest[..eq].trim().to_string();
            let after = rest[eq + 1..].trim_start();
            let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'');
            let quote = quote.ok_or_else(|| format!("<{name}>: attribute {k} not quoted"))?;
            let close = after[1..].find(quote).ok_or("an unterminated attribute")? + 1;
            el.attrs.push((k, unescape(&after[1..close])?));
            rest = &after[close + 1..];
        }
        if selfclose {
            stack.last_mut().ok_or("no parent")?.kids.push(el);
        } else {
            stack.push(el);
        }
    }
    if stack.len() != 1 {
        return Err(format!(
            "<{}> is not closed",
            stack.last().map_or("", |e| e.name.as_str())
        ));
    }
    let mut doc = stack.pop().ok_or("no document")?;
    match doc.kids.len() {
        1 => Ok(doc.kids.pop().ok_or("no root")?),
        n => Err(format!("{n} root elements")),
    }
}

/// What came in: the document's text, and what the URDF had that it could not carry.
#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub text: String,
    pub dropped: Vec<String>,
    pub links: usize,
    pub joints: usize,
}

fn floats(s: Option<&str>, n: usize, what: &str) -> Result<Vec<f64>, String> {
    let Some(s) = s else { return Ok(vec![0.0; n]) };
    let v: Vec<f64> = s
        .split_whitespace()
        .map(|x| {
            x.parse::<f64>()
                .map_err(|_| format!("{what}: `{x}` is not a number"))
        })
        .collect::<Result<_, _>>()?;
    if v.len() != n {
        return Err(format!("{what}: {n} numbers, not {}", v.len()));
    }
    Ok(v)
}

fn f1(e: &El, k: &str, what: &str) -> Result<Option<f64>, String> {
    e.attr(k)
        .map(|x| {
            x.trim()
                .parse::<f64>()
                .map_err(|_| format!("{what} {k}: `{x}` is not a number"))
        })
        .transpose()
}

/// URDF's `<origin>`: metres and rpy (R = Rz Ry Rx), as a frame in mm.
fn origin(e: Option<&El>) -> Result<Frame, String> {
    let Some(e) = e else { return Ok(Frame::WORLD) };
    let xyz = floats(e.attr("xyz"), 3, "origin xyz")?;
    let rpy = floats(e.attr("rpy"), 3, "origin rpy")?;
    let (sr, cr) = dmath::sin_cos(rpy[0]);
    let (sp, cp) = dmath::sin_cos(rpy[1]);
    let (sy, cy) = dmath::sin_cos(rpy[2]);
    // columns of Rz(y) Ry(p) Rx(r)
    let x = Vec3::new(cy * cp, sy * cp, -sp);
    let z = Vec3::new(cy * sp * cr + sy * sr, sy * sp * cr - cy * sr, cp * cr);
    let z = z.normalize().ok_or("origin: a degenerate rotation")?;
    Frame::from_zx(
        Point3::new(xyz[0] * 1000.0, xyz[1] * 1000.0, xyz[2] * 1000.0),
        z,
        x,
    )
    .ok_or_else(|| "origin: a degenerate rotation".to_string())
}

fn num(x: f64) -> String {
    let a = x.abs();
    if x == 0.0 {
        "0".into()
    } else if (1e-4..1e15).contains(&a) {
        format!("{x}")
    } else {
        format!("{x:e}")
    }
}

fn tuple(v: &[f64]) -> String {
    format!("({})", v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(", "))
}

/// `pose(..)`, leaving out what is the world's.
fn pose(f: &Frame) -> String {
    let o = f.origin;
    let mut args = vec![];
    if o != Point3::ORIGIN {
        args.push(format!("at: {}", tuple(&[o.x, o.y, o.z])));
    }
    let (x, z) = (f.x().get(), f.z().get());
    if z != Vec3::new(0.0, 0.0, 1.0) || x != Vec3::new(1.0, 0.0, 0.0) {
        args.push(format!("z: {}", tuple(&[z.x, z.y, z.z])));
        args.push(format!("x: {}", tuple(&[x.x, x.y, x.z])));
    }
    format!("pose({})", args.join(", "))
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A name the document can bind: the URDF's, with anything else as `_`, never a keyword.
fn binding(n: &str, taken: &mut BTreeSet<String>) -> String {
    let mut b: String = n
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if b.is_empty() || b.starts_with(|c: char| c.is_ascii_digit()) {
        b.insert(0, '_');
    }
    if crate::syntax::parse(&format!("{b} = 1\n")).is_err() {
        b.push_str("_l");
    }
    let mut out = b.clone();
    let mut k = 2;
    while !taken.insert(out.clone()) {
        out = format!("{b}_{k}");
        k += 1;
    }
    out
}

/// A mesh's filename as the document writes it: a `package://` (or other) URI as it is, a path
/// relative to the URDF made relative to the document where it can be.
fn mesh_path(file: &str, urdf_dir: &Path, doc_dir: &Path) -> String {
    let file = file.strip_prefix("file://").unwrap_or(file);
    if file.contains("://") {
        return file.to_string();
    }
    let abs = if Path::new(file).is_absolute() {
        PathBuf::from(file)
    } else {
        urdf_dir.join(file)
    };
    relative(&abs, doc_dir)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn relative(path: &Path, base: &Path) -> Option<PathBuf> {
    let (p, b) = (normal(path), normal(base));
    let common = p.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &p[common..] {
        out.push(c);
    }
    Some(out)
}

fn normal(p: &Path) -> Vec<std::ffi::OsString> {
    let mut out: Vec<std::ffi::OsString> = vec![];
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c.as_os_str().to_os_string()),
        }
    }
    out
}

/// The URDF at `urdf_dir` as an unlinked robot document to be saved in `doc_dir`.
#[allow(clippy::too_many_lines)]
pub fn import(xml: &str, urdf_dir: &Path, doc_dir: &Path) -> Result<Imported, String> {
    let robot = parse_xml(xml)?;
    if robot.name != "robot" {
        return Err(format!("the root element is <{}>, not <robot>", robot.name));
    }
    let mut dropped = vec![];
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Imported from URDF robot {} by forgecad import: an unlinked robot (research/export/urdf-editor.md).",
        robot.attr("name").unwrap_or("?")
    );
    let _ = writeln!(s, "# Units: mm, kg, kg mm², radians.");
    let _ = writeln!(s, "robot\n\nsrc = source(none)\ncollisions(\"mesh\")\n");
    // named materials at the top
    let mut colours: BTreeMap<String, [f64; 3]> = BTreeMap::new();
    for m in robot.all("material") {
        if let (Some(n), Some(c)) = (m.attr("name"), m.kid("color")) {
            let c = floats(c.attr("rgba"), 4, "color")?;
            colours.insert(n.to_string(), [c[0], c[1], c[2]]);
        }
    }
    let mut taken = BTreeSet::new();
    let mut link_bind: BTreeMap<String, String> = BTreeMap::new();
    let named = |b: &str, n: &str| {
        if b == n {
            String::new()
        } else {
            format!("name: {}, ", quote(n))
        }
    };
    let mut geo_lines = vec![];
    for l in robot.all("link") {
        let n = l.attr("name").ok_or("a link with no name")?;
        let b = binding(n, &mut taken);
        link_bind.insert(n.to_string(), b.clone());
        let mut args = named(&b, n);
        if let Some(i) = l.kid("inertial") {
            let mass = i
                .kid("mass")
                .map(|m| f1(m, "value", "mass"))
                .transpose()?
                .flatten()
                .unwrap_or(0.0);
            let f = origin(i.kid("origin"))?;
            let t = i.kid("inertia");
            let g = |k: &str| -> Result<f64, String> {
                Ok(t.map(|t| f1(t, k, "inertia"))
                    .transpose()?
                    .flatten()
                    .unwrap_or(0.0))
            };
            let m = [
                [g("ixx")?, g("ixy")?, g("ixz")?],
                [g("ixy")?, g("iyy")?, g("iyz")?],
                [g("ixz")?, g("iyz")?, g("izz")?],
            ];
            // the tensor turned from the inertial's axes into the link's: R I Rᵀ, kg mm²
            let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
            let r = [[x.x, y.x, z.x], [x.y, y.y, z.y], [x.z, y.z, z.z]];
            let w: [[f64; 3]; 3] = std::array::from_fn(|a| {
                std::array::from_fn(|c| {
                    let mut s = 0.0;
                    for p in 0..3 {
                        for q in 0..3 {
                            s += r[a][p] * m[p][q] * r[c][q];
                        }
                    }
                    s * 1e6
                })
            });
            let o = f.origin;
            args += &format!(
                "mass: {}, com: {}, inertia: {}",
                num(mass),
                tuple(&[o.x, o.y, o.z]),
                tuple(&[w[0][0], w[0][1], w[0][2], w[1][1], w[1][2], w[2][2]])
            );
        }
        let _ = writeln!(s, "{b} = link({})", args.trim_end_matches([',', ' ']));
        for (tag, coll) in [("visual", false), ("collision", true)] {
            for v in l.all(tag) {
                let at = origin(v.kid("origin"))?;
                let g = v
                    .kid("geometry")
                    .and_then(|g| g.kids.first())
                    .ok_or(format!("{n}: a {tag} with no geometry"))?;
                let at_s = if at == Frame::WORLD {
                    String::new()
                } else {
                    format!(", at: {}", pose(&at))
                };
                let geo = match g.name.as_str() {
                    "mesh" => {
                        let file = g.attr("filename").ok_or("a mesh with no filename")?;
                        let sc = match g.attr("scale") {
                            Some(x) => floats(Some(x), 3, "mesh scale")?,
                            None => vec![1.0; 3],
                        };
                        format!(
                            "mesh({}, scale: {}{at_s})",
                            quote(&mesh_path(file, urdf_dir, doc_dir)),
                            tuple(&sc)
                        )
                    }
                    "box" => {
                        let sz = floats(g.attr("size"), 3, "box size")?;
                        format!(
                            "box(size: {}{at_s})",
                            tuple(&sz.iter().map(|x| x * 1000.0).collect::<Vec<_>>())
                        )
                    }
                    "cylinder" => format!(
                        "cylinder(r: {}, len: {}{at_s})",
                        num(f1(g, "radius", "cylinder")?.unwrap_or(0.0) * 1000.0),
                        num(f1(g, "length", "cylinder")?.unwrap_or(0.0) * 1000.0)
                    ),
                    "sphere" => format!(
                        "sphere(r: {}{at_s})",
                        num(f1(g, "radius", "sphere")?.unwrap_or(0.0) * 1000.0)
                    ),
                    other => {
                        dropped.push(format!("{n}: a {tag} of {other} geometry"));
                        continue;
                    }
                };
                let mut extra = String::new();
                if !coll && let Some(m) = v.kid("material") {
                    let name = m.attr("name").unwrap_or("colour");
                    let c = match m.kid("color") {
                        Some(c) => Some(floats(c.attr("rgba"), 4, "color")?),
                        None => colours.get(name).map(|c| c.to_vec()),
                    };
                    if let Some(c) = c {
                        extra = format!(", material: {}, colour: {}", quote(name), tuple(&c[..3]));
                    }
                }
                geo_lines.push(format!("{tag}({b}, {geo}{extra})"));
            }
        }
    }
    if !geo_lines.is_empty() {
        let _ = writeln!(s);
        for g in &geo_lines {
            let _ = writeln!(s, "{g}");
        }
    }
    let _ = writeln!(s);
    let mut joint_bind: BTreeMap<String, String> = BTreeMap::new();
    let mut kinds: BTreeMap<String, String> = BTreeMap::new();
    for j in robot.all("joint") {
        let n = j.attr("name").ok_or("a joint with no name")?;
        let b = binding(n, &mut taken);
        joint_bind.insert(n.to_string(), b);
        kinds.insert(n.to_string(), j.attr("type").unwrap_or("").to_string());
    }
    let unit = |kind: &str| if kind == "prismatic" { 1000.0 } else { 1.0 };
    for j in robot.all("joint") {
        let n = j.attr("name").ok_or("a joint with no name")?;
        let b = &joint_bind[n];
        let kind = j.attr("type").ok_or_else(|| format!("joint {n}: no type"))?;
        let link = |k: &str| -> Result<String, String> {
            let l = j
                .kid(k)
                .and_then(|e| e.attr("link"))
                .ok_or(format!("joint {n}: no {k}"))?;
            link_bind.get(l).cloned().ok_or(format!("joint {n}: no link {l}"))
        };
        let mut args = vec![];
        if b != n {
            args.push(format!("name: {}", quote(n)));
        }
        args.push(format!("parent: {}", link("parent")?));
        args.push(format!("child: {}", link("child")?));
        args.push(format!("type: {}", quote(kind)));
        let at = origin(j.kid("origin"))?;
        if at != Frame::WORLD {
            args.push(format!("at: {}", pose(&at)));
        }
        if !matches!(kind, "fixed" | "floating") {
            let a = floats(
                j.kid("axis").and_then(|a| a.attr("xyz")).or(Some("1 0 0")),
                3,
                "axis",
            )?;
            args.push(format!("axis: {}", tuple(&a)));
        }
        let u = unit(kind);
        if let Some(l) = j.kid("limit") {
            if matches!(kind, "revolute" | "prismatic") {
                let lo = f1(l, "lower", "limit")?.unwrap_or(0.0);
                let hi = f1(l, "upper", "limit")?.unwrap_or(0.0);
                args.push(format!("limit: {}", tuple(&[lo * u, hi * u])));
            }
            if let Some(e) = f1(l, "effort", "limit")? {
                args.push(format!("effort: {}", num(e)));
            }
            if let Some(v) = f1(l, "velocity", "limit")? {
                args.push(format!("velocity: {}", num(v)));
            }
        }
        if let Some(d) = j.kid("dynamics") {
            if let Some(x) = f1(d, "damping", "dynamics")? {
                args.push(format!("damping: {}", num(x)));
            }
            if let Some(x) = f1(d, "friction", "dynamics")? {
                args.push(format!("friction: {}", num(x)));
            }
        }
        if let Some(m) = j.kid("mimic") {
            let leader = m.attr("joint").ok_or(format!("joint {n}: a mimic of nothing"))?;
            let lb = joint_bind
                .get(leader)
                .ok_or(format!("joint {n}: mimics no joint {leader}"))?;
            let (fu, lu) = (u, unit(kinds.get(leader).map_or("", String::as_str)));
            let mult = f1(m, "multiplier", "mimic")?.unwrap_or(1.0) * fu / lu;
            let off = f1(m, "offset", "mimic")?.unwrap_or(0.0) * fu;
            args.push(format!("mimic: ({lb}, {}, {})", num(mult), num(off)));
        }
        for k in ["calibration", "safety_controller"] {
            if j.kid(k).is_some() {
                dropped.push(format!("joint {n}: its {k}"));
            }
        }
        let _ = writeln!(s, "{b} = joint({})", args.join(", "));
    }
    let mut trans = vec![];
    for t in robot.all("transmission") {
        let (Some(j), Some(a)) = (t.kid("joint"), t.kid("actuator")) else {
            dropped.push(format!(
                "transmission {}: not one joint and one actuator",
                t.attr("name").unwrap_or("?")
            ));
            continue;
        };
        let jn = j.attr("name").unwrap_or("");
        let Some(jb) = joint_bind.get(jn) else {
            dropped.push(format!("transmission of no joint {jn}"));
            continue;
        };
        let red = a
            .kid("mechanicalReduction")
            .map(|r| r.text.trim().parse::<f64>().unwrap_or(1.0))
            .unwrap_or(1.0);
        let iface = j.kid("hardwareInterface").map(|h| h.text.trim().to_string());
        trans.push(format!(
            "transmission({jb}, actuator: {}, reduction: {}{})",
            quote(a.attr("name").unwrap_or("actuator")),
            num(red),
            iface.map_or(String::new(), |i| format!(", interface: {}", quote(&i)))
        ));
    }
    if !trans.is_empty() {
        let _ = writeln!(s);
        for t in trans {
            let _ = writeln!(s, "{t}");
        }
    }
    let known: BTreeSet<&str> = ["link", "joint", "material", "transmission"].into();
    for k in &robot.kids {
        if !known.contains(k.name.as_str()) {
            dropped.push(format!("<{}>", k.name));
        }
    }
    if !dropped.is_empty() {
        let _ = writeln!(s);
        for d in &dropped {
            let _ = writeln!(s, "# not imported: {d}");
        }
    }
    debug_assert!(is_robot(&s));
    Ok(Imported {
        links: robot.all("link").count(),
        joints: robot.all("joint").count(),
        text: s,
        dropped,
    })
}
