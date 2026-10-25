//! cargo run --release -p forge-model --example inspect -- <features> <face>
//! Dumps one face of the torture part after the given number of features.
use forge_geom::{Curve, CurveGeom};
use forge_topo::FaceId;
fn main() {
    let args: Vec<usize> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let body = forge_model::torture::part(args[0]).regenerate().unwrap();
    let f = FaceId(args[1] as u32);
    let face = body.face(f);
    println!(
        "face {f:?} same_sense {} surface {:?}",
        face.same_sense, face.surface
    );
    for &l in &face.loops {
        println!(" loop:");
        for &c in &body.lp(l).coedges {
            let co = body.coedge(c);
            let e = body.edge(co.edge);
            let kind = match &e.curve {
                Curve::Line(_) => "line",
                Curve::Circle(_) => "circle",
                Curve::Ellipse(_) => "ellipse",
                Curve::Intersection(_) => "traced",
                _ => "other",
            };
            let other = body.partner(c).map(|o| body.coedge_face(o));
            println!(
                "   {c:?} {:?} fwd {} {kind} other {:?} {:?} -> {:?}",
                co.edge,
                co.forward,
                other,
                body.vertex(body.coedge_start(c)).point,
                body.vertex(body.coedge_end(c)).point
            );
            if let Curve::Circle(k) = &e.curve {
                println!(
                    "      circle centre {:?} r {} range {:?} mid {:?}",
                    k.frame.origin,
                    k.radius,
                    e.range,
                    e.curve.eval(e.range.mid())
                );
            }
        }
    }
    println!("problems: {:?}", forge_topo::check(&body));
}
