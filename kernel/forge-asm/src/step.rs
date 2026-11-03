//! STEP export of an assembly's structure (prior-art §9): each part written once by the kernel's
//! writer, each assembly document a product with a shape representation, and each use of a part or
//! subassembly a next_assembly_usage_occurrence placed by a representation relationship with an
//! item defined transformation (PDM Usage Guide 4.3, §4.1-4.4). Reference and suppressed instances
//! are left out. Poses are each level's solved poses.

use crate::model::{Assembly, Component, DocId, Parts};
use forge_math::Frame;
use forge_topo::Body;
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum StepError {
    /// A part with no kernel body to write.
    NoBody { doc: DocId, body: u64 },
    /// The kernel's writer refused a part's body.
    Part {
        doc: DocId,
        body: u64,
        why: forge_step::ExportError,
    },
    /// A subassembly document that is missing, or that contains itself.
    NoAssembly { doc: DocId },
}

/// Ids of one written product: its product definition and its shape representation.
#[derive(Clone, Copy)]
struct Product {
    pd: u64,
    rep: u64,
}

struct Out {
    data: String,
    next: u64,
}

impl Out {
    fn put(&mut self, text: &str) -> u64 {
        let id = self.next;
        self.next += 1;
        let _ = writeln!(self.data, "#{id}={text};");
        id
    }
}

fn real(x: f64) -> String {
    // the shortest text that reads back to the same f64, with the point part 21 asks for
    let s = format!("{x:?}");
    match s.split_once('e') {
        Some((m, e)) if m.contains('.') => format!("{m}E{e}"),
        Some((m, e)) => format!("{m}.E{e}"),
        None if s.contains('.') => s,
        None => format!("{s}."),
    }
}

fn quoted(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '_' })
        .collect::<String>()
        .replace('\'', "''")
}

/// A part's own file, its entity ids moved up by `offset`; returns the text and the ids of its
/// product definition and its B-rep representation.
fn renumbered(text: &str, offset: u64) -> (String, u64, u64, u64) {
    let data = text
        .split_once("DATA;\n")
        .and_then(|(_, d)| d.rsplit_once("ENDSEC;"))
        .map_or("", |(d, _)| d);
    let mut out = String::with_capacity(data.len() + data.len() / 8);
    let (mut pd, mut rep, mut max) = (0, 0, 0);
    for line in data.lines() {
        let mut s = String::with_capacity(line.len() + 8);
        let mut quote = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                quote = !quote;
            }
            if c == '#' && !quote {
                let mut n = 0u64;
                while let Some(d) = chars.peek().and_then(|d| d.to_digit(10)) {
                    n = n * 10 + d as u64;
                    chars.next();
                }
                let _ = write!(s, "#{}", n + offset);
                max = max.max(n);
                continue;
            }
            s.push(c);
        }
        let id = || {
            s.split_once('=')
                .and_then(|(i, _)| i.trim_start_matches('#').parse().ok())
                .unwrap_or(0)
        };
        if s.contains("=PRODUCT_DEFINITION(") {
            pd = id();
        }
        if s.contains("=ADVANCED_BREP_SHAPE_REPRESENTATION(") {
            rep = id();
        }
        out.push_str(&s);
        out.push('\n');
    }
    (out, pd, rep, max + offset)
}

/// The assembly as a STEP AP214 file. `bodies` holds each part's kernel body by (document, body).
pub fn export<P: Parts>(
    asm: &Assembly,
    name: &str,
    parts: &P,
    bodies: &BTreeMap<(DocId, u64), Body>,
) -> Result<String, StepError> {
    let mut o = Out {
        data: String::new(),
        next: 1,
    };
    // parts used anywhere, each written once
    let mut used: BTreeMap<(DocId, u64), ()> = BTreeMap::new();
    let mut subs: Vec<DocId> = Vec::new();
    collect(asm, parts, &mut used, &mut subs, &mut Vec::new())?;
    let mut products: BTreeMap<(DocId, u64), Product> = BTreeMap::new();
    for key in used.keys() {
        let body = bodies.get(key).ok_or_else(|| StepError::NoBody {
            doc: key.0.clone(),
            body: key.1,
        })?;
        let opt = forge_step::Options {
            name: format!("{}#{}", key.0.0, key.1),
            ..forge_step::Options::default()
        };
        let text = forge_step::export(body, &opt)
            .map_err(|why| StepError::Part {
                doc: key.0.clone(),
                body: key.1,
                why,
            })?
            .text;
        let (data, pd, rep, last) = renumbered(&text, o.next - 1);
        o.data.push_str(&data);
        o.next = last + 1;
        products.insert(key.clone(), Product { pd, rep });
    }

    // the assemblies' own context, units in mm
    let mm = o.put("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))");
    let rad = o.put("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))");
    let sr = o.put("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())");
    let unc = o.put(&format!(
        "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{mm},'distance_accuracy_value','confusion accuracy')",
        real(forge_math::tol::LINEAR)
    ));
    let ctx = o.put(&format!(
        "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{unc}))\
         GLOBAL_UNIT_ASSIGNED_CONTEXT((#{mm},#{rad},#{sr}))REPRESENTATION_CONTEXT('',''))"
    ));
    let app = o.put("APPLICATION_CONTEXT('automotive design')");
    o.put(&format!(
        "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2001,#{app})"
    ));
    let pc = o.put(&format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
    let pdc = o.put(&format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{app},'design')"
    ));
    let axis = |o: &mut Out, f: &Frame| {
        let p = o.put(&format!(
            "CARTESIAN_POINT('',({},{},{}))",
            real(f.origin.x),
            real(f.origin.y),
            real(f.origin.z)
        ));
        let (z, x) = (f.z().get(), f.x().get());
        let dz = o.put(&format!(
            "DIRECTION('',({},{},{}))",
            real(z.x),
            real(z.y),
            real(z.z)
        ));
        let dx = o.put(&format!(
            "DIRECTION('',({},{},{}))",
            real(x.x),
            real(x.y),
            real(x.z)
        ));
        o.put(&format!("AXIS2_PLACEMENT_3D('',#{p},#{dz},#{dx})"))
    };
    let origin = axis(&mut o, &Frame::WORLD);

    // each assembly document, children before parents
    let mut levels: Vec<(DocId, &Assembly)> = Vec::new();
    for d in &subs {
        levels.push((
            d.clone(),
            parts
                .assembly(d)
                .ok_or(StepError::NoAssembly { doc: d.clone() })?,
        ));
    }
    levels.push((DocId(name.into()), asm));
    let mut assemblies: BTreeMap<DocId, Product> = BTreeMap::new();
    for (doc, a) in levels {
        let sol = crate::solve(a, parts, &crate::Options { threads: 1 });
        let n = quoted(&doc.0);
        let prod = o.put(&format!("PRODUCT('{n}','{n}','',(#{pc}))"));
        let form = o.put(&format!("PRODUCT_DEFINITION_FORMATION('','',#{prod})"));
        let pd = o.put(&format!("PRODUCT_DEFINITION('design','',#{form},#{pdc})"));
        let pds = o.put(&format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"));
        let mut kids = Vec::new();
        let mut places = vec![origin];
        let mut insts: Vec<_> = a
            .instances
            .iter()
            .filter(|i| !i.suppressed && !i.reference)
            .collect();
        insts.sort_by_key(|i| i.id);
        for i in insts {
            let child = match &i.of {
                Component::Part(p) => products[&(p.doc.clone(), p.body)],
                Component::Assembly(d) => assemblies[d],
            };
            let pose = sol.pose(i.id).unwrap_or(i.pose);
            let at = axis(&mut o, &pose);
            places.push(at);
            kids.push((i.id, child, at));
        }
        let list: Vec<String> = places.iter().map(|x| format!("#{x}")).collect();
        let rep = o.put(&format!(
            "SHAPE_REPRESENTATION('{n}',({}),#{ctx})",
            list.join(",")
        ));
        o.put(&format!("SHAPE_DEFINITION_REPRESENTATION(#{pds},#{rep})"));
        for (id, child, at) in kids {
            let nauo = o.put(&format!(
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE('{}','{}','',#{pd},#{},$)",
                id.0, id.0, child.pd
            ));
            let npds = o.put(&format!("PRODUCT_DEFINITION_SHAPE('','',#{nauo})"));
            let idt = o.put(&format!("ITEM_DEFINED_TRANSFORMATION('','',#{origin},#{at})"));
            let rr = o.put(&format!(
                "(REPRESENTATION_RELATIONSHIP('','',#{},#{rep})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{idt})SHAPE_REPRESENTATION_RELATIONSHIP())",
                child.rep
            ));
            o.put(&format!("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{rr},#{npds})"));
        }
        assemblies.insert(doc, Product { pd, rep });
    }

    Ok(format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ForgeCAD assembly'),'2;1');\n\
         FILE_NAME('{}','2000-01-01T00:00:00',(''),(''),'ForgeCAD','ForgeCAD','');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN {{ 1 0 10303 214 1 1 1 1 }}'));\nENDSEC;\nDATA;\n{}ENDSEC;\nEND-ISO-10303-21;\n",
        quoted(name),
        o.data
    ))
}

/// Parts and subassembly documents used below `asm`, each subassembly after those it uses.
fn collect<P: Parts>(
    asm: &Assembly,
    parts: &P,
    used: &mut BTreeMap<(DocId, u64), ()>,
    subs: &mut Vec<DocId>,
    stack: &mut Vec<DocId>,
) -> Result<(), StepError> {
    let mut insts: Vec<_> = asm
        .instances
        .iter()
        .filter(|i| !i.suppressed && !i.reference)
        .collect();
    insts.sort_by_key(|i| i.id);
    for i in insts {
        match &i.of {
            Component::Part(p) => {
                used.insert((p.doc.clone(), p.body), ());
            }
            Component::Assembly(d) => {
                if stack.contains(d) {
                    return Err(StepError::NoAssembly { doc: d.clone() });
                }
                let a = parts
                    .assembly(d)
                    .ok_or(StepError::NoAssembly { doc: d.clone() })?;
                if !subs.contains(d) {
                    stack.push(d.clone());
                    collect(a, parts, used, subs, stack)?;
                    stack.pop();
                    subs.push(d.clone());
                }
            }
        }
    }
    Ok(())
}
