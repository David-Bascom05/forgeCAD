//! A machine definition as text, in the format docs/cam-machine.md gives (the layout of the
//! built-in files: one key per line, records in the order the reader keeps them).
//!
//! The text is canonical: `write(parse(write(m))) == write(m)` byte for byte, and
//! `parse(write(m)) == m` for every `m` the reader produced (tests/roundtrip.rs runs both over
//! the built-in library). Comments and key order of a hand-written file are not kept: the
//! builder owns the files it writes.
use forge_machine::def::{
    Changer, Cite, CiteStatus, JointType, MachineDef, MountKind, Prim, Reset, Side, SpindleKind,
};
use std::fmt::Write;

/// A number as the reader reads it back exactly: Rust's shortest round-trip decimal, no
/// exponent; −0 is written 0 (the reader's values compare equal).
pub fn num(x: f64) -> String {
    if x == 0.0 { "0".into() } else { format!("{x}") }
}

fn vec(v: &[f64]) -> String {
    let parts: Vec<String> = v.iter().map(|x| num(*x)).collect();
    format!("[{}]", parts.join(", "))
}

/// A string literal the reader takes back unchanged. The reader knows the escapes `\" \\ \n
/// \t` only; any other control character has no spelling and is refused (`Err` names it).
pub fn string(s: &str) -> Result<String, String> {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if c.is_control() => {
                return Err(format!(
                    "{s:?}: the character U+{:04X} cannot be written",
                    c as u32
                ));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    Ok(o)
}

fn strings(v: &[String]) -> Result<String, String> {
    let parts = v.iter().map(|s| string(s)).collect::<Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", parts.join(", ")))
}

struct Out {
    s: String,
}

impl Out {
    fn line(&mut self, l: &str) {
        self.s.push_str(l);
        self.s.push('\n');
    }
    fn text(&mut self, k: &str, v: &str) -> Result<(), String> {
        let v = string(v)?;
        writeln!(self.s, "{k} = {v}").unwrap();
        Ok(())
    }
    fn num(&mut self, k: &str, v: f64) {
        writeln!(self.s, "{k} = {}", num(v)).unwrap();
    }
    fn vec(&mut self, k: &str, v: &[f64]) {
        writeln!(self.s, "{k} = {}", vec(v)).unwrap();
    }
}

pub fn side_name(s: Side) -> &'static str {
    match s {
        Side::Head => "head",
        Side::Table => "table",
    }
}
pub fn type_name(t: JointType) -> &'static str {
    match t {
        JointType::Linear => "linear",
        JointType::Rotary => "rotary",
    }
}
pub fn reset_name(r: Reset) -> &'static str {
    match r {
        Reset::Never => "never",
        Reset::OpStart => "op-start",
        Reset::Rewind => "rewind",
        Reset::Both => "both",
    }
}
pub fn spindle_kind_name(k: SpindleKind) -> &'static str {
    match k {
        SpindleKind::Milling => "milling",
        SpindleKind::Turning => "turning",
    }
}
pub fn mount_kind_name(k: MountKind) -> &'static str {
    match k {
        MountKind::Tool => "tool",
        MountKind::Part => "part",
    }
}
pub fn status_name(s: CiteStatus) -> &'static str {
    match s {
        CiteStatus::Published => "published",
        CiteStatus::Estimate => "estimate",
        CiteStatus::Unknown => "unknown",
        CiteStatus::Conflict => "conflict",
    }
}

/// One `[[machine]]` record. Err: a string the reader could not take back (a control character
/// in a name or note).
pub fn machine(m: &MachineDef) -> Result<String, String> {
    let mut o = Out { s: String::new() };
    o.line("[[machine]]");
    o.text("name", &m.name)?;
    o.text("kind", m.kind.name())?;
    o.text("control", &m.control)?;
    if let Some(l) = &m.length_joint {
        o.text("length_joint", l)?;
    }
    if !m.coolants.is_empty() {
        writeln!(o.s, "coolants = {}", strings(&m.coolants)?).unwrap();
    }
    o.num("clearance", m.clearance);
    if m.real {
        o.line("real = true");
    }
    o.text("source", &m.source)?;
    if !m.taken.is_empty() {
        o.text("taken", &m.taken)?;
    }
    if !m.note.is_empty() {
        o.text("note", &m.note)?;
    }
    for j in &m.joints {
        o.line("");
        o.line("[[machine.joint]]");
        o.text("name", &j.name)?;
        if j.letter != j.name {
            o.text("letter", &j.letter)?;
        }
        o.text("parent", &j.parent)?;
        o.text("side", side_name(j.side))?;
        o.text("type", type_name(j.ty))?;
        o.vec("axis", &j.axis);
        if j.ty == JointType::Rotary {
            o.vec("pivot", &j.pivot);
        }
        o.vec("range", &j.range);
        if j.home != 0.0 {
            o.num("home", j.home);
        }
        o.num("rapid", j.rapid);
        o.num("max_feed", j.max_feed);
        if let Some(a) = j.accel {
            o.num("accel", a);
        }
        if j.cyclic {
            o.line("cyclic = true");
        }
        if j.tcp {
            o.line("tcp = true");
        }
        if j.preference != 0 {
            o.num("preference", j.preference as f64);
        }
        if j.reset != Reset::Never {
            o.text("reset", reset_name(j.reset))?;
        }
        if j.diameter {
            o.line("diameter = true");
        }
    }
    for s in &m.spindles {
        o.line("");
        o.line("[[machine.spindle]]");
        o.text("name", &s.name)?;
        o.text("link", &s.link)?;
        o.text("kind", spindle_kind_name(s.kind))?;
        o.vec("axis", &s.axis);
        o.vec("nose", &s.nose);
        o.vec("rpm", &s.rpm);
        if !s.power.is_empty() {
            o.vec("curve_rpm", &s.power.iter().map(|p| p[0]).collect::<Vec<_>>());
            o.vec("curve_kw", &s.power.iter().map(|p| p[1]).collect::<Vec<_>>());
        }
        o.vec("max", &s.max);
        if !s.interface.is_empty() {
            o.text("interface", &s.interface)?;
        }
        if let Some(c) = &s.c_joint {
            o.text("c_joint", c)?;
        }
    }
    for t in &m.turrets {
        o.line("");
        o.line("[[machine.turret]]");
        o.text("name", &t.name)?;
        o.text("link", &t.link)?;
        o.num("stations", t.stations as f64);
        o.num("index_s", t.index_s);
        o.vec("center", &t.center);
        o.vec("index_axis", &t.index_axis);
        if let Some(l) = &t.live {
            o.text("live", l)?;
        }
    }
    if let Some(Changer {
        pockets,
        change_s,
        max_tool_kg,
    }) = &m.changer
    {
        o.line("");
        o.line("[[machine.changer]]");
        o.num("pockets", *pockets as f64);
        o.num("change_s", *change_s);
        o.num("max_tool_kg", *max_tool_kg);
    }
    for mt in &m.mounts {
        o.line("");
        o.line("[[machine.mount]]");
        o.text("name", &mt.name)?;
        o.text("kind", mount_kind_name(mt.kind))?;
        o.text("link", &mt.link)?;
        o.vec("at", &mt.at);
        o.vec("dir", &mt.dir);
        if mt.kind == MountKind::Part {
            o.vec("xdir", &mt.xdir);
        }
    }
    for b in &m.bodies {
        o.line("");
        o.line("[[machine.body]]");
        o.text("name", &b.name)?;
        o.text("link", &b.link)?;
        if let Some(p) = &b.part {
            o.text("part", p)?;
        }
        if !b.ignore.is_empty() {
            writeln!(o.s, "ignore = {}", strings(&b.ignore)?).unwrap();
        }
        // the reader keeps boxes before cylinders
        for p in &b.prims {
            if let Prim::Box { min, max } = p {
                o.line("[[machine.body.box]]");
                o.vec("min", min);
                o.vec("max", max);
            }
        }
        for p in &b.prims {
            if let Prim::Cyl { base, axis, r, h } = p {
                o.line("[[machine.body.cyl]]");
                o.vec("base", base);
                o.vec("axis", axis);
                o.num("r", *r);
                o.num("h", *h);
            }
        }
    }
    for c in &m.cites {
        o.line("");
        cite(&mut o, c)?;
    }
    Ok(o.s)
}

fn cite(o: &mut Out, c: &Cite) -> Result<(), String> {
    o.line("[[machine.cite]]");
    o.text("path", &c.path)?;
    o.text("status", status_name(c.status))?;
    for (k, v) in [
        ("source", &c.source),
        ("taken", &c.taken),
        ("value", &c.value),
        ("basis", &c.basis),
        ("note", &c.note),
    ] {
        if !v.is_empty() {
            o.text(k, v)?;
        }
    }
    Ok(())
}

/// A file of machines: a header comment, then each record, a blank line between.
pub fn file(header: &str, machines: &[MachineDef]) -> Result<String, String> {
    let mut s = String::new();
    for l in header.lines() {
        s.push_str("# ");
        s.push_str(l);
        s.push('\n');
    }
    for m in machines {
        s.push('\n');
        s.push_str(&machine(m)?);
    }
    Ok(s)
}
