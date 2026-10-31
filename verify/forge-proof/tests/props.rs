//! The quick tier of the properties (the deep one is `examples/meta.rs`): small fuzz histories
//! and a few suite parts, each property on each.
use forge_model::{Part, suite};
use forge_proof::props;

fn fuzz() -> Vec<(String, Part)> {
    (0..4)
        .map(|s| (format!("fuzz{s}"), suite::fuzz(s + 1000, 6)))
        .collect()
}

fn holds(prop: &str, f: props::Property, parts: &[(String, Part)]) {
    for (i, (name, part)) in parts.iter().enumerate() {
        let seed = i as u64 + 1;
        if let Some(hit) = f(part, seed) {
            panic!(
                "{prop} fails on {name} (seed {seed}): {}: {}",
                hit.kind, hit.detail
            );
        }
    }
}

#[test]
fn scaled_by_powers_of_two() {
    let mut parts = fuzz();
    parts.push(("shaft".into(), suite::shaft()));
    holds("scale", props::scale, &parts);
}

/// Quarter turns and mirrors in coordinate planes map every coordinate exactly (even seeds pick
/// those; odd ones an inexact rotation or a tilted plane, which M1 still trips).
#[test]
fn exact_turns_and_mirrors() {
    for (name, part) in fuzz() {
        for seed in [2, 4, 6] {
            for (prop, f) in [
                ("rigid", props::rigid as props::Property),
                ("mirror", props::mirror),
            ] {
                if let Some(hit) = f(&part, seed) {
                    panic!(
                        "{prop} fails on {name} (seed {seed}): {}: {}",
                        hit.kind, hit.detail
                    );
                }
            }
        }
    }
}

#[test]
fn neighbouring_tools_commute() {
    holds("order", props::order, &fuzz());
}

#[test]
fn boolean_identities() {
    holds("boolean", props::identities, &fuzz());
}

#[test]
fn certified() {
    let mut parts = fuzz();
    parts.push(("housing".into(), suite::housing()));
    holds("certify", props::certificate, &parts);
}

#[test]
fn brep_agrees_with_csg_point_by_point() {
    holds("membership", props::membership, &fuzz());
}

/// Every stage moved by a few ulps (even seeds) or up to 1e-12 (odd), then its next feature:
/// valid, with the same topology as from the stage unmoved.
#[test]
fn indifferent_to_ulps() {
    let mut parts = fuzz();
    parts.push(("housing".into(), suite::housing()));
    holds("ulp", props::ulp, &parts);
}

/// The nudge is not vacuous: it moves every vertex off its bits and leaves the body valid.
#[test]
fn nudge_moves_every_vertex() {
    let b = suite::housing().regenerate().expect("builds");
    for seed in [2, 3] {
        let m = props::nudged(&b, seed);
        let moved = b
            .vertices
            .iter()
            .zip(&m.vertices)
            .filter(|(x, y)| x.point != y.point)
            .count();
        assert!(
            moved * 10 >= b.vertices.len() * 9,
            "seed {seed}: {moved} of {} moved",
            b.vertices.len()
        );
        assert!(
            forge_topo::check(&m).is_empty(),
            "seed {seed}: {:?}",
            forge_topo::check(&m)
        );
    }
}

/// U2: a void cut a tolerance short of the top leaves its rim as a hole of the top face, a
/// tolerance off its plane. The notch's split of that face placed the hole by rays whose 3D box
/// test the probe, a few ulps further off, passed beside.
#[test]
fn hole_a_tolerance_off_its_plane() {
    let part = fc("extrude new 10 rect 0 0 60 40
extrude cut 3 at 22 29 7 circle 0 0 3
extrude cut 2.999995 at 11.59574686652088 22.93665566589452 6.999995 circle 0 0 3
extrude cut 3 at 42 12 7 circle 0 0 3
extrude cut 12 at 50 14 -1 rect 0 0 10 8
");
    for seed in [404, 406, 408] {
        if let Some(hit) = props::ulp(&part, seed) {
            panic!("seed {seed}: {}: {}", hit.kind, hit.detail);
        }
    }
}

/// A planar face's first loop is its outer one: the booleans read the face's place from it and
/// keep it when only holes change. A face listing a hole first is flagged, and is what broke
/// later cuts after a local imprint appended a new outer loop last.
#[test]
fn outer_loop_first() {
    let mut b = fc("extrude new 10 rect 0 0 60 40
extrude cut 20 at 20 20 -5 circle 0 0 4
")
    .regenerate()
    .expect("builds");
    assert!(forge_topo::check(&b).is_empty());
    let f = b
        .face_ids()
        .find(|&f| b.face(f).loops.len() == 2)
        .expect("a face with a hole");
    b.faces[f.index()].loops.swap(0, 1);
    assert!(
        forge_topo::check(&b).contains(&forge_topo::Problem::OuterNotFirst { face: f }),
        "{:?}",
        forge_topo::check(&b)
    );
}

/// The oracle is not vacuous: a part's tools against the body without its last cut disagree.
#[test]
fn membership_catches_a_missing_cut() {
    let part = fc("extrude new 20 rect 0 0 100 100
extrude cut 30 at 50 50 -5 circle 0 0 20
");
    let body = fc("extrude new 20 rect 0 0 100 100")
        .regenerate()
        .expect("builds");
    assert!(props::membership_of(&part, &body, 1).is_some());
}

fn block(x: f64, y: f64, z: f64) -> forge_topo::Body {
    let part = Part {
        features: vec![forge_model::Feature::Extrude {
            profile: forge_ops::Profile {
                plane: forge_math::Frame::WORLD.with_origin(forge_math::Point3::new(x, y, z)),
                loops: vec![forge_ops::Profile::rectangle(
                    forge_math::Point2::ORIGIN,
                    10.0,
                    10.0,
                    0,
                )],
            },
            extent: forge_model::Extent::Distance(10.0),
            op: forge_model::Combine::New,
        }],
    };
    let mut b = part.regenerate().expect("a block");
    for f in &mut b.faces {
        f.name = forge_topo::Name::NONE;
    }
    for e in &mut b.edges {
        e.name = forge_topo::Name::NONE;
    }
    for v in &mut b.vertices {
        v.name = forge_topo::Name::NONE;
    }
    b
}

fn kinds(b: &forge_topo::Body) -> Vec<String> {
    forge_proof::cert::certify(b, 0.05)
        .iter()
        .map(|f| f.kind())
        .collect()
}

/// A block turned inside out (every face's normal and loop reversed) passes `check`, whose
/// probes are local, but not ray parity.
#[test]
fn certify_catches_an_inside_out_solid() {
    let a = block(0.0, 0.0, 0.0);
    assert!(kinds(&a).is_empty(), "{:?}", kinds(&a));
    // rebuilt with each face's sense and each loop's way round reversed
    let mut b = forge_topo::Body::new();
    for v in &a.vertices {
        b.add_tolerant_vertex(v.point, v.tol, v.name);
    }
    for e in &a.edges {
        b.add_edge(e.curve.clone(), e.range, e.start, e.end, e.name);
    }
    let shell = b.add_shell();
    for f in a.face_ids() {
        let face = a.face(f);
        let g = b.add_face(shell, face.surface.clone(), !face.same_sense, face.name);
        for &l in &face.loops {
            let uses: Vec<_> = a
                .lp(l)
                .coedges()
                .rev()
                .map(|c| (a.coedge(c).edge, !a.coedge(c).forward))
                .collect();
            b.add_loop(g, &uses);
        }
    }
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    assert!(
        kinds(&b).iter().all(|k| k == "Inverted") && kinds(&b).len() == 6,
        "{:?}",
        kinds(&b)
    );
}

/// Two overlapping blocks in one body: each shell is closed, but their faces cross (check's
/// ShellsOverlap says so too).
#[test]
fn certify_catches_crossing_shells() {
    let mut b = block(0.0, 0.0, 0.0);
    b.append(&block(4.3, 3.7, 5.9));
    assert!(kinds(&b).iter().any(|k| k == "ShellsOverlap"), "{:?}", kinds(&b));
    let k: Vec<String> = forge_proof::cert::geometric(&b, 0.05)
        .iter()
        .map(|f| f.kind())
        .collect();
    assert!(
        k.iter().any(|k| k == "Crossing") && k.iter().any(|k| k == "NotBounding"),
        "{k:?}"
    );
}

/// Two blocks sharing one corner vertex: every edge has two faces, but the vertex has two fans.
#[test]
fn certify_catches_a_pinched_vertex() {
    let mut b = block(0.0, 0.0, 0.0);
    let n = b.vertices.len() as u32;
    b.append(&block(10.0, 10.0, 10.0));
    let corner = |b: &forge_topo::Body, p: forge_math::Point3| {
        (0..b.vertices.len() as u32).find(|&v| b.vertices[v as usize].point.distance(p) < 1e-9)
    };
    let p = forge_math::Point3::new(10.0, 10.0, 10.0);
    let keep = corner(&b, p).expect("the first block's corner");
    assert!(keep < n);
    let gone = (n..b.vertices.len() as u32)
        .find(|&v| b.vertices[v as usize].point.distance(p) < 1e-9)
        .expect("the second block's corner");
    for e in &mut b.edges {
        for end in [&mut e.start, &mut e.end] {
            if end.0 == gone {
                *end = forge_topo::VertexId(keep);
            }
        }
    }
    let probs = forge_topo::certify(&b);
    assert!(
        probs
            .iter()
            .any(|p| matches!(p, forge_topo::Problem::VertexFans { fans: 2, .. })),
        "{probs:?}"
    );
}

fn fc(src: &str) -> Part {
    forge_doc::text::parse(src).expect("parses")
}

/// M1: the same part rotated by an inexact rotation gets one more vertex between the same two
/// faces (forgecad-gate/repros/M1-redundant-vertex-under-rotation.fc).
#[test]
#[ignore = "M1: booleans leave vertices between the same two faces, as rounding falls"]
fn m1_rotation_keeps_topology() {
    let part = fc("extrude new 17 rect 0 0 82 138
extrude add 15.625457658136117 axes 0.6111332437811676 0.7772515422987627 -0.14965359447638588 -0.7694654469826927 0.6277120352052737 0.11790049515662869 0.18557770406283436 0.04310035791786255 0.9816838976483822 at 76.92553480349156 114.11085208111551 9.80044547195848 rrect -7.706843451315201 -3.8327248781946053 15.413686902630403 7.665449756389211 2.6191965533564114
");
    assert_eq!(props::rigid(&part, 1), None);
}

/// M3: two blind cuts sharing an end plane inside a block: seq and n-ary disagree on the faces.
#[test]
#[ignore = "M3: feature by feature leaves the coplanar ends of a void unmerged"]
fn m3_void_ends_seq_matches_nary() {
    let part = fc("extrude new 80 rect 0 0 200 150
extrude cut 13 yz at 6 28 45 circle 0 0 2
extrude cut 9 yz at 6 26 45 circle 0 0 1
");
    assert_eq!(props::backends(&part, 1), None);
}

/// M4: a part split by a cylinder into its difference and intersection, combined again.
#[test]
fn m4_split_halves_combine_valid() {
    let part = fc("a: extrude new 20 rect 0 0 100 100
ball cut 6 -xz at 36 58 22
revolve add -xz at 41 71 20 circle 7 -1 2
extrude cut 17 at 46 60 16 circle 0 0 8
b: extrude new 20 rect 0 0 100 100
ball cut 6 -xz at 36 58 22
revolve add -xz at 41 71 20 circle 7 -1 2
extrude keep 17 at 46 60 16 circle 0 0 8
combine add a b
");
    let body = props::build(&part).expect("builds");
    assert!(
        forge_topo::check(&body).is_empty(),
        "{:?}",
        forge_topo::check(&body)
    );
}

fn valid(src: &str) {
    let body = props::build(&fc(src)).expect("builds");
    assert!(
        forge_topo::check(&body).is_empty(),
        "{:?}",
        forge_topo::check(&body)
    );
}

/// M5: a ball whose poles lie on an edge; with its frame turned over (-xz) it builds.
#[test]
#[ignore = "M5: face split crossed itself"]
fn m5_ball_on_edge_either_frame() {
    valid("extrude new 20 at -20 -20 -20 rect 0 0 40 40\nball add 5 xz at 20 0 0");
}

/// M6: n-ary leaves open edges where a pocket's mouth is 0.00001 above the top.
#[test]
#[ignore = "M6: n-ary open edges near tolerance"]
fn m6_nary_near_tolerance_pocket() {
    let part = fc("extrude new 10 rect 0 0 60 40
extrude cut 3.00001 at 20.829744830901628 31.98774163479715 7.00001 circle 0 0 3");
    assert_eq!(props::backends(&part, 1), None);
}

/// M9: a tilted pad added across a hole, its side plane cut at the hole's seam.
#[test]
#[ignore = "M9: FaceWrongSide on a lens at the hole's seam"]
fn m9_tilted_pad_over_hole() {
    valid("extrude new 15 rect 0 0 120 60
extrude cut 17 at 75 19 -1 circle 0 0 3
extrude add 8.945478516509796 z -0.061633258296996694 0.2939571355937309 0.9538294102748273 x -0.08391472378619189 0.9507366012975499 -0.29842626239168546 at 89.69521759175012 13.908448595830624 3.4058649537841124 rrect -13.632567574771072 -10.946070359275362 27.265135149542143 21.892140718550724 7.854519259083712");
}

/// A cone framed at its apex with a torus on its rim (degen_cone_torus 3, scaled 4 times): the
/// mesh laid the cone out with a floor of 1 mm on its radius, so the part and its scaled copy
/// meshed differently and their areas parted by 0.2%.
#[test]
fn cone_framed_at_its_apex_scales() {
    let (_, g) = forge_model::adversarial::families()
        .into_iter()
        .find(|(f, _)| *f == "degen_cone_torus")
        .expect("the family");
    let part = forge_model::adversarial::canonical(&g(3));
    if let Some(hit) = props::scale(&part, 662) {
        panic!("{}: {}", hit.kind, hit.detail);
    }
}

/// A seam stopped part way up a hole's wall by a torus that eats it: the wall's loop winds past
/// one whole turn, and a point of the wall drawn on the copy outside the loop was called Outside,
/// so a point in the open hole counted one crossing short and came out Inside (fuzz68 at count 20).
#[test]
fn point_in_a_hole_whose_seam_a_torus_cuts_short() {
    let part = fc("extrude new 39 rect 0 0 199 103
extrude cut 35 z 0 0 1 x 0.9887155042247666 0.14980537942799493 0 at 166 88 -4 circle 0 0 4
revolve add -xz at 167.8 97.4 11.1 circle 8.7 0 2.7
extrude add 32 z 0 0 1 x -0.360994102557712 0.9325680982740896 0 at 162 93 7 circle 0 0 7");
    let body = part.regenerate().expect("builds").compact();
    let p = forge_math::Point3::new(167.46767621091539, 88.39294512605494, 2.2139809034473172);
    for body in [body, part.regenerate_nary(1).expect("n-ary").compact()] {
        assert_eq!(
            forge_bool::Index::new(&body).classify(p),
            forge_bool::Where::Outside
        );
        assert_eq!(props::membership_of(&part, &body, 69), None);
    }
}

/// A point of a sphere near its pole: a line in (u, v) from an edge's end is far from the edge
/// there, and the drawn loop said the wrong side, so a ray miscounted.
#[test]
fn point_on_a_ball_near_its_pole() {
    for (seed, count, at) in [(91, 6, 92), (18, 16, 19)] {
        let part = suite::fuzz(seed + 1000, count);
        assert_eq!(props::membership(&part, at), None, "fuzz{seed} count {count}");
    }
}

/// A torus crossing a top with enough holes for a local split: its two circles make a ring and a
/// disc inside it, and the older hole in the disc went to the ring (the first region holding it),
/// which is dropped: the hole's rim was left open and the volume wrong (fuzz19 at count 60).
#[test]
fn a_hole_in_a_disc_inside_a_ring_goes_to_the_disc() {
    let part = fc("extrude new 10.566712767523594 rect 0 0 70.72881866274645 103.24012644860403
revolve cut -xz at 54.636499807943885 52.16417873140876 10.410305724956826 circle 8.351160778816524 0 2.7599878880234967
extrude add 7.466192008905245 z 0 0 1 x 0.9988990041027803 -0.046912467452413 0 at 30.847639674790706 6.694448623656604 5.115359149262295 circle 0 0 2.9515512238302954
extrude cut 6.096829815264994 z 0 0 1 x -0.4926911440690709 0.8702042499068308 0 at 30.777739606003802 25.146464930412808 10.224669555097256 circle 0 0 6.201177187633486
extrude cut 6.878676428117549 axes -0.4943085947740702 0.8533443497235724 -0.16571793483910305 -0.8025304514424799 -0.5212422570478675 -0.29026088950317486 -0.334071680392118 -0.010484763349645184 0.9424893538377441 at 26.85612632367834 47.46664923991346 10.210897973462817 rect -3.105823834349979 -2.0705492228999858 6.211647668699958 4.099687461341972
ball add 4.1911579125771 -xz at 58.404464415959175 98.776984432684 10.12006847653015
ball add 5.307822009781472 -xz at 21.252169984522265 29.55297220289709 5.372937186750471
extrude cut 4.281061686283083 z 0 0 1 x -0.4624780083869633 0.886630752770525 0 at 31.574209806131584 35.7199672865349 9.99707989128916 circle 0 0 3.6656106845435033
revolve add -xz at 31.964188863899732 36.04360694203633 10.700248852150338 circle 5.2713150056640465 0 1.0039475779366047
");
    for b in [part.regenerate(), part.regenerate_nary(1)] {
        let b = b.expect("builds");
        assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    }
    assert_eq!(props::membership(&part, 20), None);
}

#[test]
fn every_feature_takes_effect_or_is_refused() {
    let mut parts = fuzz();
    parts.extend(
        suite::parts()
            .into_iter()
            .filter(|p| p.1.features.len() <= 40)
            .map(|(n, p)| (n.to_string(), p)),
    );
    holds("effect", props::effect, &parts);
}

/// The check itself: a body missing one of a pattern's copies fails it, for a cut and for an add.
#[test]
fn effect_catches_a_skipped_copy() {
    for op in ["cut", "add"] {
        let whole = fc(&format!(
            "extrude new 10 rect 0 0 100 40
h: extrude {op} 5 at 10 20 8 circle 0 0 4
pattern h lin 1 0 0 4 20"
        ));
        let short = fc(&format!(
            "extrude new 10 rect 0 0 100 40
h: extrude {op} 5 at 10 20 8 circle 0 0 4
pattern h lin 1 0 0 3 20"
        ));
        let body = short.regenerate().expect("builds").compact();
        let mut r = forge_math::rng::Rng::new(5);
        assert!(
            props::tools_held(&whole, 2, &body, false, &mut r).is_some(),
            "{op}"
        );
        let all = whole.regenerate().expect("builds").compact();
        assert_eq!(props::tools_held(&whole, 2, &all, false, &mut r), None, "{op}");
    }
}

/// A cut may take a whole body of several and say so; nothing else may lose one.
#[test]
fn effect_lets_a_cut_take_a_whole_body() {
    use forge_model::{BodyFeature, Combine, Feature, Pick};
    let cut = fc("extrude new 10 rect 0 0 100 40\nextrude cut 5 at 10 20 8 circle 0 0 4").features[1].clone();
    let add = fc("extrude new 10 rect 0 0 100 40\nextrude add 5 at 10 20 8 circle 0 0 4").features[1].clone();
    let on = |feature| {
        Feature::Body(Box::new(BodyFeature::In {
            bodies: Pick::All,
            feature,
        }))
    };
    let bodies = |n: usize| vec![forge_topo::Body::new(); n];
    assert_eq!(props::bodies_held(&on(cut.clone()), &bodies(2), &bodies(1)), None);
    assert!(props::bodies_held(&on(add), &bodies(2), &bodies(1)).is_some());
    let combine = |op| {
        Feature::Body(Box::new(BodyFeature::Combine {
            op,
            target: 0,
            tools: vec![1],
            keep: true,
        }))
    };
    assert_eq!(
        props::bodies_held(&combine(Combine::Cut), &bodies(2), &bodies(1)),
        None
    );
    assert!(props::bodies_held(&combine(Combine::Add), &bodies(2), &bodies(1)).is_some());
}
