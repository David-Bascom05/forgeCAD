//! Why a part's fillet is refused: filletwhy FILE.fc. Takes the refused fillet to pieces: each edge
//! alone, then the smallest set of edges still refused, with the geometry round what it blames.
use forge_agent::model::Model;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_model::Feature;
use forge_ops::{FilletEdge, FilletError, fillet};
use forge_topo::{Body, EdgeId, FaceId, VertexId};

fn surf(s: &Surface) -> String {
    match s {
        Surface::Plane(_) => "plane".into(),
        Surface::Cylinder(c) => format!("cyl r{:.4}", c.radius),
        Surface::Cone(_) => "cone".into(),
        Surface::Sphere(c) => format!("sphere r{:.4}", c.radius),
        Surface::Torus(t) => format!("torus {:.4}/{:.4}", t.major, t.minor),
        Surface::Procedural(_) => "proc".into(),
    }
}
fn curve(c: &Curve) -> &'static str {
    match c {
        Curve::Line(_) => "line",
        Curve::Circle(_) => "circle",
        Curve::Procedural(_) => "proc",
        _ => "other",
    }
}
fn face(b: &Body, f: FaceId) -> String {
    format!("f{}[{}]", f.index(), surf(&b.face(f).surface))
}
fn edge(b: &Body, e: EdgeId) -> String {
    let x = b.edge(e);
    let (p, q) = (b.vertex(x.start).point, b.vertex(x.end).point);
    let fs: Vec<String> = x.coedges.iter().map(|&c| face(b, b.coedge_face(c))).collect();
    let mid = x.curve.eval(x.range.mid());
    let n: Vec<_> = x
        .coedges
        .iter()
        .filter_map(|&c| {
            let f = b.coedge_face(c);
            b.face_normal(f, b.face(f).surface.project(mid))
        })
        .collect();
    let ang = if n.len() == 2 {
        forge_math::dmath::atan2(n[0].cross(n[1]).length(), n[0].dot(n[1].get())).to_degrees()
    } else {
        -1.0
    };
    format!(
        "e{} {} v{}({:.3},{:.3},{:.3})-v{}({:.3},{:.3},{:.3}) len~{:.4} {} ang{:.2}",
        e.index(),
        curve(&x.curve),
        x.start.index(),
        p.x,
        p.y,
        p.z,
        x.end.index(),
        q.x,
        q.y,
        q.z,
        p.distance(mid) + mid.distance(q),
        fs.join(" "),
        ang
    )
}
fn vertex(b: &Body, v: VertexId, picked: &[EdgeId]) {
    println!("  vertex v{} {:?}", v.index(), b.vertex(v).point);
    for e in b.edge_ids() {
        let x = b.edge(e);
        if x.start == v || x.end == v {
            println!(
                "    {}{}",
                if picked.contains(&e) { "* " } else { "  " },
                edge(b, e)
            );
        }
    }
}
fn blame(b: &Body, err: &FilletError, picked: &[EdgeId]) {
    match err {
        FilletError::Corner { vertex: v } if v.index() < b.vertices.len() => vertex(b, *v, picked),
        FilletError::Overflow { edge: e }
        | FilletError::NoBlend { edge: e }
        | FilletError::Tangency { edge: e }
        | FilletError::Smooth { edge: e }
        | FilletError::BadRadius { edge: e }
            if e.index() < b.edges.len() =>
        {
            println!("  edge {}", edge(b, *e));
            let x = b.edge(*e);
            vertex(b, x.start, picked);
            vertex(b, x.end, picked);
        }
        _ => {}
    }
}

fn kind(e: &FilletError) -> String {
    format!("{e:?}").split(' ').next().unwrap_or("").to_string()
}

fn main() {
    let file = std::env::args().nth(1).expect("a part file");
    let src = std::fs::read_to_string(&file).expect("readable");
    let m = Model::new(&src);
    let Some(f) = &m.error else {
        println!("built");
        return;
    };
    println!("refused: line {}: {}", m.lines[f.at].no, f.msg);
    let Some(Feature::Fillet { edges, .. }) = &m.refused else {
        println!("not a fillet");
        return;
    };
    let Some(body) = m.body().map(|b| if b.dead > 0 { b.compact() } else { b.clone() }) else {
        return;
    };
    let picks: Vec<FilletEdge> = edges
        .iter()
        .filter_map(|(n, r)| {
            let e = body.edge_ids().find(|&e| body.edge(e).name == *n)?;
            Some(FilletEdge {
                edge: e,
                radius: r.clone(),
            })
        })
        .collect();
    println!(
        "{} of {} picks resolved; faces {} edges {}",
        picks.len(),
        edges.len(),
        body.face_ids().count(),
        body.edge_ids().count()
    );
    println!(
        "radii {:?}",
        picks.iter().map(|p| &p.radius).take(4).collect::<Vec<_>>()
    );
    let ids: Vec<EdgeId> = picks.iter().map(|p| p.edge).collect();
    let whole = fillet(&body, &picks, 999);
    println!("whole: {:?}", whole.as_ref().err());
    let mut alone_bad = 0;
    for p in &picks {
        if let Err(e) = fillet(&body, std::slice::from_ref(p), 999) {
            alone_bad += 1;
            if alone_bad <= 3 {
                println!("alone {:?} {:?}", p.radius, e);
                blame(&body, &e, &ids);
            }
        }
    }
    println!("{alone_bad} edges fail alone");
    // the smallest set still refused
    let mut set = picks.clone();
    if let Err(w) = &whole {
        let want = kind(w);
        let mut i = 0;
        while i < set.len() {
            let mut t = set.clone();
            t.remove(i);
            if !t.is_empty() && fillet(&body, &t, 999).err().is_some_and(|x| kind(&x) == want) {
                set = t;
            } else {
                i += 1;
            }
        }
        let e = fillet(&body, &set, 999).err();
        println!("smallest refused set: {} edges: {:?}", set.len(), e);
        let sid: Vec<EdgeId> = set.iter().map(|p| p.edge).collect();
        for s in &set {
            println!("  * {:?} {}", s.radius, edge(&body, s.edge));
        }
        if let Some(e) = e {
            blame(&body, &e, &sid);
        }
    }
}
