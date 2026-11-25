//! What machine makers' files do that the standard does not allow, read as the maker meant
//! (Haas's VF-2 model, SolidWorks 2021: research/machines/model-sources.md): a vertex reference
//! naming an ORIENTED_EDGE, and planes and lines placed 500 m out (just past the model box).
use forge_math::{Frame, Point2};
use forge_ops::{Profile, extrude};
use forge_step::{Limits, Options, export, import};

fn block_text() -> String {
    let b = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(0.0, 0.0), 10.0, 20.0, 0)],
        },
        5.0,
        1,
    )
    .unwrap();
    export(&b, &Options::default()).unwrap().text
}

/// The file's records as (id, text).
fn records(t: &str) -> Vec<(u32, String)> {
    t.split(';')
        .filter_map(|r| {
            let r = r.trim();
            let (id, rest) = r.strip_prefix('#')?.split_once('=')?;
            Some((id.trim().parse().ok()?, rest.trim().to_string()))
        })
        .collect()
}

fn volume(t: &str) -> f64 {
    let got = import(t.as_bytes(), &Limits::default()).expect("reads");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    assert_eq!(got.solids.len(), 1);
    forge_topo::mass::mass(&got.solids[0].body).unwrap().volume
}

#[test]
fn a_vertex_named_by_another_entity_is_where_its_edges_meet() {
    let t = block_text();
    let rs = records(&t);
    let v = rs.iter().find(|r| r.1.starts_with("VERTEX_POINT")).unwrap().0;
    let oe = rs.iter().find(|r| r.1.starts_with("ORIENTED_EDGE")).unwrap().0;
    // every edge's reference to vertex v made a reference to an oriented edge, as SolidWorks 2021
    // wrote them; the vertex itself left unused
    let bad: String = t
        .split_inclusive(';')
        .map(|r| {
            if r.contains("EDGE_CURVE") {
                let from = format!("#{v},");
                let to = format!("#{oe},");
                r.replace(&from, &to)
                    .replace(&format!("#{v})"), &format!("#{oe})"))
            } else {
                r.to_string()
            }
        })
        .collect();
    assert_ne!(bad, t);
    assert!((volume(&bad) - 1000.0).abs() < 1e-6);
}

#[test]
fn a_plane_and_a_line_placed_far_out_are_moved_along_themselves() {
    let t = block_text();
    let line_of = |id: u32| {
        t.lines()
            .find(|l| l.starts_with(&format!("#{id}=")))
            .unwrap()
            .to_string()
    };
    let num = |s: &str| -> u32 { s.trim().trim_start_matches('#').parse().unwrap() };
    let args = |l: &str| -> Vec<String> {
        let inner = &l[l.find('(').unwrap() + 1..l.rfind(')').unwrap()];
        inner.split(',').map(|x| x.trim().to_string()).collect()
    };
    let coords = |l: &str| -> Vec<f64> {
        let inner = &l[l.rfind('(').unwrap() + 1..l.find(')').unwrap()];
        inner.split(',').map(|x| x.trim().parse().unwrap()).collect()
    };
    let shifted = |l: &str, d: &[f64]| -> String {
        let c = coords(l);
        let head = &l[..l.rfind('(').unwrap()];
        format!(
            "{head}({:?},{:?},{:?}));",
            c[0] + 6e5 * d[0],
            c[1] + 6e5 * d[1],
            c[2] + 6e5 * d[2]
        )
    };
    let mut bad = t.clone();
    // the first plane's origin 500 m out within the plane; the first line's along itself
    let plane = t.lines().find(|l| l.contains("=PLANE(")).unwrap();
    let place = line_of(num(&args(plane)[1]));
    let a = args(&place);
    let (o, x) = (line_of(num(&a[1])), coords(&line_of(num(&a[3]))));
    bad = bad.replace(&o, &shifted(&o, &x));
    let line = t.lines().find(|l| l.contains("=LINE(")).expect("a line");
    let la = args(line);
    let p0 = line_of(num(&la[1]));
    let dir = coords(&line_of(num(&args(&line_of(num(&la[2])))[1])));
    // the line's point may be shared with a vertex: give the line a point of its own
    let fresh = 9_000_000;
    bad = bad.replace(line, &line.replacen(&la[1], &format!("#{fresh}"), 1));
    bad = bad.replace(
        "ENDSEC;\nEND-ISO",
        &format!(
            "{}\nENDSEC;\nEND-ISO",
            shifted(&p0, &dir).replacen(&p0[..p0.find('=').unwrap()], &format!("#{fresh}"), 1)
        ),
    );
    assert!(bad.contains(&format!("#{fresh}=")), "{bad}");
    assert_ne!(bad, t);
    assert!((volume(&bad) - 1000.0).abs() < 1e-6);
    // without the move, the file would be refused
    let o2 = shifted(&o, &[0.0, 0.0, 1.0]);
    let off = t.replace(&o, &o2);
    let got = import(off.as_bytes(), &Limits::default()).unwrap();
    assert!(!got.refusals.is_empty());
}
