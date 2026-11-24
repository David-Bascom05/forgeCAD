//! Shared by the URDF tests: a strict little XML reader, forward kinematics of a URDF read back
//! (independent of the exporter's own frames), and urdfdom's `check_urdf` where installed.
#![allow(dead_code)]

use forge_lang::urdf::Exported;
use forge_math::{Frame, dmath};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ---- a small XML reader: well-formedness, and the tree --------------------------------------

#[derive(Debug, Clone)]
pub struct El {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub kids: Vec<El>,
}

impl El {
    pub fn attr(&self, k: &str) -> &str {
        self.attrs.get(k).map_or("", String::as_str)
    }
    pub fn kid(&self, n: &str) -> Option<&El> {
        self.kids.iter().find(|k| k.name == n)
    }
    pub fn all<'a>(&'a self, n: &'a str) -> impl Iterator<Item = &'a El> + 'a {
        self.kids.iter().filter(move |k| k.name == n)
    }
}

/// Parses the document strictly: one root, every tag closed in order, attributes quoted and not
/// repeated, comments without `--`, only the five predefined entities.
pub fn parse(xml: &str) -> El {
    let b = xml.as_bytes();
    let mut i = 0;
    let mut stack: Vec<El> = vec![El {
        name: "#doc".into(),
        attrs: BTreeMap::new(),
        kids: vec![],
    }];
    while i < b.len() {
        if b[i] != b'<' {
            let j = xml[i..].find('<').map_or(b.len(), |k| i + k);
            let text = &xml[i..j];
            assert!(
                text.trim().is_empty() || stack.len() > 1,
                "text outside elements: {text:?}"
            );
            i = j;
            continue;
        }
        if xml[i..].starts_with("<?xml") {
            assert_eq!(i, 0, "the declaration comes first");
            i += xml[i..].find("?>").expect("declaration closed") + 2;
            continue;
        }
        if xml[i..].starts_with("<!--") {
            let end = xml[i + 4..].find("-->").expect("comment closed") + i + 4;
            assert!(!xml[i + 4..end].contains("--"), "-- inside a comment");
            i = end + 3;
            continue;
        }
        let end = xml[i..].find('>').expect("tag closed") + i;
        let tag = &xml[i + 1..end];
        i = end + 1;
        if let Some(name) = tag.strip_prefix('/') {
            let el = stack.pop().expect("an open element");
            assert_eq!(el.name, name.trim(), "closing tag order");
            stack.last_mut().expect("a parent").kids.push(el);
            continue;
        }
        let selfclose = tag.ends_with('/');
        let tag = tag.trim_end_matches('/');
        let (name, mut rest) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        assert!(
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "tag name {name:?}"
        );
        let mut attrs = BTreeMap::new();
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            let eq = rest.find('=').expect("attribute =");
            let k = rest[..eq].trim().to_string();
            assert!(rest[eq + 1..].starts_with('"'), "attribute {k} quoted");
            let close = rest[eq + 2..].find('"').expect("quote closed") + eq + 2;
            let v = &rest[eq + 2..close];
            assert!(!v.contains('<'), "< in an attribute");
            let mut s = v;
            while let Some(a) = s.find('&') {
                let t = &s[a..];
                assert!(
                    ["&amp;", "&quot;", "&lt;", "&gt;", "&apos;"]
                        .iter()
                        .any(|e| t.starts_with(e)),
                    "entity in {v:?}"
                );
                s = &t[1..];
            }
            assert!(
                attrs.insert(k.clone(), v.to_string()).is_none(),
                "attribute {k} twice"
            );
            rest = &rest[close + 1..];
        }
        let el = El {
            name: name.to_string(),
            attrs,
            kids: vec![],
        };
        if selfclose {
            stack.last_mut().expect("a parent").kids.push(el);
        } else {
            stack.push(el);
        }
    }
    assert_eq!(stack.len(), 1, "unclosed elements");
    let mut doc = stack.pop().expect("the document");
    assert_eq!(doc.kids.len(), 1, "one root element");
    doc.kids.pop().expect("the root")
}

pub fn floats(s: &str) -> Vec<f64> {
    s.split_whitespace()
        .map(|x| x.parse::<f64>().expect("a number"))
        .collect()
}

// ---- forward kinematics, independent of the exporter's own frames ---------------------------

pub type M4 = [[f64; 4]; 4];

pub fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}

pub fn rot_axis(u: [f64; 3], t: f64) -> M4 {
    let (s, c) = dmath::sin_cos(t);
    let (x, y, z) = (u[0], u[1], u[2]);
    let k = 1.0 - c;
    [
        [c + x * x * k, x * y * k - z * s, x * z * k + y * s, 0.0],
        [y * x * k + z * s, c + y * y * k, y * z * k - x * s, 0.0],
        [z * x * k - y * s, z * y * k + x * s, c + z * z * k, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn trans(v: [f64; 3]) -> M4 {
    [
        [1.0, 0.0, 0.0, v[0]],
        [0.0, 1.0, 0.0, v[1]],
        [0.0, 0.0, 1.0, v[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn origin_of(el: Option<&El>) -> M4 {
    let Some(o) = el else { return trans([0.0; 3]) };
    let xyz = floats(o.attr("xyz"));
    let rpy = floats(o.attr("rpy"));
    let r = mul(
        &mul(
            &rot_axis([0.0, 0.0, 1.0], rpy[2]),
            &rot_axis([0.0, 1.0, 0.0], rpy[1]),
        ),
        &rot_axis([1.0, 0.0, 0.0], rpy[0]),
    );
    mul(&trans([xyz[0], xyz[1], xyz[2]]), &r)
}

pub fn frame_m4(f: &Frame) -> M4 {
    let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
    [
        [x.x, y.x, z.x, o.x / 1000.0],
        [x.y, y.y, z.y, o.y / 1000.0],
        [x.z, y.z, z.z, o.z / 1000.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Every link's world placement at the export's positions.
pub fn forward(robot: &El, e: &Exported) -> BTreeMap<String, M4> {
    let pos: BTreeMap<String, Vec<f64>> = e
        .joints
        .iter()
        .map(|j| (j.name.clone(), j.position.clone()))
        .collect();
    forward_at(robot, &pos)
}

/// Every link's world placement with these joint positions (URDF units; a joint not given: 0).
pub fn forward_at(robot: &El, pos: &BTreeMap<String, Vec<f64>>) -> BTreeMap<String, M4> {
    let joints: Vec<&El> = robot.all("joint").collect();
    let children: Vec<&str> = joints
        .iter()
        .map(|j| j.kid("child").expect("child").attr("link"))
        .collect();
    let links: Vec<&str> = robot.all("link").map(|l| l.attr("name")).collect();
    let roots: Vec<&str> = links.iter().copied().filter(|l| !children.contains(l)).collect();
    assert_eq!(roots.len(), 1, "one root link: {roots:?}");
    let mut world = BTreeMap::from([(roots[0].to_string(), trans([0.0; 3]))]);
    let mut changed = true;
    while changed {
        changed = false;
        for j in &joints {
            let (p, c) = (
                j.kid("parent").expect("parent").attr("link"),
                j.kid("child").expect("child").attr("link"),
            );
            if world.contains_key(c) || !world.contains_key(p) {
                continue;
            }
            let q: Vec<f64> = pos.get(j.attr("name")).cloned().unwrap_or_else(|| vec![0.0; 3]);
            let q = if q.is_empty() { vec![0.0; 3] } else { q };
            let axis = j.kid("axis").map_or([1.0, 0.0, 0.0], |a| {
                let v = floats(a.attr("xyz"));
                [v[0], v[1], v[2]]
            });
            let m = match j.attr("type") {
                "revolute" | "continuous" => rot_axis(axis, q[0]),
                "prismatic" => trans(axis.map(|a| a * q[0])),
                "planar" => mul(&trans([q[0], q[1], 0.0]), &rot_axis([0.0, 0.0, 1.0], q[2])),
                "fixed" | "floating" => trans([0.0; 3]),
                t => panic!("joint type {t}"),
            };
            let w = mul(&mul(&world[p], &origin_of(j.kid("origin"))), &m);
            world.insert(c.to_string(), w);
            changed = true;
        }
    }
    assert_eq!(world.len(), links.len(), "every link reached");
    world
}

/// `check_urdf` on the export, where urdfdom is installed.
pub fn check_urdf(name: &str, e: &Exported) {
    let Ok(out) = std::process::Command::new("check_urdf").arg("--help").output() else {
        eprintln!("check_urdf not installed: skipped");
        return;
    };
    let _ = out;
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("urdf-check-{name}"));
    let _ = std::fs::create_dir_all(&d);
    let f = d.join(format!("{}.urdf", e.name));
    std::fs::write(&f, &e.urdf).expect("write");
    let r = std::process::Command::new("check_urdf")
        .arg(&f)
        .output()
        .expect("check_urdf runs");
    let text = String::from_utf8_lossy(&r.stdout).to_string() + &String::from_utf8_lossy(&r.stderr);
    assert!(
        r.status.success() && text.contains("Successfully Parsed"),
        "check_urdf {name}:\n{text}"
    );
}
