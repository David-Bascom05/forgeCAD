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
    let seeded: Vec<(u64, &(String, Part))> =
        parts.iter().enumerate().map(|(i, p)| (i as u64 + 1, p)).collect();
    suite::each(&seeded, |&(seed, (name, part))| {
        if let Some(hit) = f(part, seed) {
            panic!(
                "{prop} fails on {name} (seed {seed}): {}: {}",
                hit.kind, hit.detail
            );
        }
    });
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

// Ruled lofts cut by a cylinder: the curve on the wall comes to a tip, so a plane across it cuts two
// arms (Newton from a far seed landed on the other, as exact as the right one), and one chain step
// can hold the whole turn at the tip.
#[test]
fn mass_agrees_with_the_mesh_on_a_curve_with_two_arms() {
    let parts = [
        (
            "two arms",
            "arm: loft profile at 1.92387 2.39903 0 circle 0 0 20.2127 profile at 2.77072 2.03649 66.9086 circle 0 0 23.9307
             extrude cut 76.9086 z 0 0 1 x 1 0 0 at 17.0662172828 19.6484941013 -5 circle 0 0 1.69944
             extrude cut 213.949 z 0.986371786648 -0.164531755306 0 x 0 0 1 at -105.516428921 17.6006689353 11.8710917543 circle 0 0 5.42322
             extrude cut 213.949 z -0.590787687949 0.806675132166 -0.0156569126432 x 0 -0.0194055372993 -0.999811694832 at 63.1990978722 -86.2935055498 19.826670494 circle 0 0 2.62392",
        ),
        (
            "no region",
            "arm: loft profile at -2.76075 -0.743971 0 circle 0 0 5.58991 profile at 1.86503 0.567644 56.3615 circle 0 0 10.7266 profile at -1.45221 -2.28102 112.723 circle 0 0 12.0474
             extrude cut 122.723 z 0 0 1 x 1 0 0 at -11.4885874662 -1.52046134588 -5 circle 0 0 2.05112
             extrude cut 122.723 z 0 0 1 x 1 0 0 at 7.88027351596 8.6001202425 -5 circle 0 0 1.84516",
        ),
        (
            "turn in a chain step",
            "arm: loft profile at 1.71584 2.90262 0 circle 0 0 22.4298 profile at -0.877333 -0.276794 47.8258 circle 0 0 10.8143 profile at 2.41955 -0.408761 95.6516 circle 0 0 17.746
             extrude cut 219.311 z 0.16714244831 -0.985932757328 0 x 0 0 1 at -18.3281298161 108.113191765 22.4474540782 circle 0 0 4.05556
             extrude add 105.652 z 0 0 1 x 1 0 0 at 17.0008755596 5.12124836525 -5 circle 0 0 1.14856",
        ),
    ];
    for (name, src) in parts {
        let lines: Vec<&str> = src.lines().map(str::trim).collect();
        let m = forge_agent::model::Model::new(&lines.join("\n"));
        let part = Part {
            features: m.features.clone(),
        };
        assert!(part.regenerate().is_ok(), "{name} builds");
        assert!(
            props::mass_mesh(&part, 1).is_none(),
            "{name}: {:?}",
            props::mass_mesh(&part, 1)
        );
        // and through the n-ary road
        let body = part.regenerate_nary(1).expect("n-ary builds");
        let m = forge_topo::mass::mass(&body.compact()).expect("a mass");
        let seq = forge_topo::mass::mass(&part.regenerate().unwrap().compact()).expect("a mass");
        assert!(
            (m.volume - seq.volume).abs() < 1e-6 * seq.volume,
            "{name}: {} vs {}",
            m.volume,
            seq.volume
        );
    }
}

#[test]
fn every_fillet_holds_against_the_part_before_it() {
    let corpus: Vec<(String, Part)> = forge_model::fillets::cases()
        .into_iter()
        .map(|(n, p, _)| (n.to_string(), p))
        .collect();
    holds("blend", props::blend, &corpus);
}

/// A check never seen to fail proves nothing: each of the blend checks against a body made wrong
/// in the way it is there to see.
#[test]
fn the_ridge_check_tells_a_ball_on_a_ridge_from_one_on_a_true_rim() {
    use forge_proof::blends::ridges;
    let body = |src: &str| {
        let m = forge_agent::model::Model::new(src);
        assert!(
            m.error.is_none(),
            "{src}: {:?}",
            m.error.as_ref().map(|f| f.msg.clone())
        );
        m.body().expect("a body").clone()
    };
    // a ring of a shallow dome: wall, then short flats rising inward. An arc of radius 2 from the
    // wall ends on the first ridge, with the ball's centre under the top: the next flat runs into it
    let (wx, wy) = (31.7f64, 11.5f64);
    let ridge = (31.26f64, 11.53f64);
    let cy = ridge.1 - (4.0f64 - (ridge.0 - (wx - 2.0)) * (ridge.0 - (wx - 2.0))).sqrt();
    let cx = wx - 2.0;
    let rest = |from: (f64, f64)| {
        format!(
            "line {} {} 30 11.6 line 30 11.6 25 11.6 line 25 11.6 25 0 line 25 0 {wx} 0",
            from.0, from.1
        )
    };
    let sharp = format!(
        "revolve z 0 0 1 x 1 0 0 at 0 0 0 axis 0 0 0 1 loop line {wx} 0 {wx} {wy} line {wx} {wy} {} {} {}\n",
        ridge.0,
        ridge.1,
        rest(ridge)
    );
    let a1 = forge_math::dmath::atan2(ridge.1 - cy, ridge.0 - cx).to_degrees();
    let rounded = format!(
        "revolve z 0 0 1 x 1 0 0 at 0 0 0 axis 0 0 0 1 loop line {wx} 0 {wx} {cy} arc {cx} {cy} 2 0 {a1} {}\n",
        rest(ridge)
    );
    let (before, after) = (body(&sharp), body(&rounded));
    assert!(
        !ridges(&before, &after).is_empty(),
        "a ridge the next flat covers"
    );
    // past the wall's far side: a plate 1 thick, ball 2, its far face falling away
    let plate = body("b: extrude 20 rect 0 0 1 30\n");
    let m = forge_agent::model::Model::new("b: extrude 20 rect 0 0 1 30\nfillet 2 (edges near 0 15 20)\n");
    let rim = m.body().expect("a ball past the wall").clone();
    assert!(ridges(&plate, &rim).is_empty(), "a true rim");
}

#[test]
fn the_blend_checks_fail_what_they_should() {
    use forge_math::Point3;
    use forge_ops::{FilletEdge, FilletOptions, Radius, fillet, fillet_with};
    use forge_proof::blends::Radii;
    let m = forge_agent::model::Model::new("b: extrude 40 rect 0 0 100 80\n");
    let before = m.body().expect("a block").clone();
    let edge = |p: Point3| {
        before
            .edge_ids()
            .find(|&e| {
                use forge_geom::CurveGeom;
                let x = before.edge(e);
                x.curve.eval(x.range.mid()).distance(p) < 1e-9
            })
            .expect("the edge")
    };
    let (a, b) = (
        edge(Point3::new(50.0, 80.0, 40.0)),
        edge(Point3::new(100.0, 40.0, 40.0)),
    );
    let pick = |r: f64| {
        [a, b].map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
    };
    let good = fillet(&before, &pick(5.0), 1).expect("a mitre");
    let held = forge_proof::blends::audit(&before, &good, &Radii::List(vec![5.0]), 1);
    assert_eq!(held.unsupported, 0);
    assert_eq!(forge_proof::blends::points(&before, &good, 400, 1).wrong, 0);
    assert!(forge_proof::blends::needless_patches(&before, &good).is_empty());
    // held to the wrong radius, the balls stand off their supports
    assert!(forge_proof::blends::audit(&before, &good, &Radii::List(vec![5.01]), 1).unsupported > 0);
    // a patched corner where two blends meet beside a sharp edge: seen as a patch, and by points
    let v = before.edge(a).start;
    let at = if before.vertex(v).point.distance(Point3::new(100.0, 80.0, 40.0)) < 1e-9 {
        v
    } else {
        before.edge(a).end
    };
    let opts = FilletOptions {
        setbacks: vec![(at, None, 6.0)],
        ..FilletOptions::default()
    };
    let patched = fillet_with(&before, &pick(5.0), &opts, 1).expect("a patch");
    assert!(!forge_proof::blends::needless_patches(&before, &patched).is_empty());
    assert!(forge_proof::blends::points(&before, &patched, 2000, 1).wrong > 0);
    // a blend from another part entirely: points the round did not change are claimed by nothing
    let other = fillet(&before, &pick(9.0), 1).expect("another mitre");
    assert!(forge_proof::blends::audit(&before, &other, &Radii::List(vec![5.0]), 1).unsupported > 0);
}

/// A thin blade with round corners running out over a disc's rim. Rounds at its foot wider than the
/// flat at its narrow end mitre there, some resting on an edge of the blade carried on past its end by
/// less than the radius. The audit says so as such, and still refuses another radius.
#[test]
fn audit_tells_a_mitre_on_a_rim_carried_on() {
    use forge_bool::{Op, boolean};
    use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3};
    use forge_ops::{
        Axis2, FilletEdge, Profile, ProfileLoop, Radius, Segment, extrude, fillet, fillet_kind, revolve,
    };
    use forge_proof::blends::Radii;
    use std::f64::consts::{PI, TAU};
    let line = |a: (f64, f64), b: (f64, f64), id: u64| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let hub = revolve(
        &Profile {
            plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![
                line((0.0, -5.0), (84.0, -5.0), 0),
                line((84.0, -5.0), (84.0, 2.0), 1),
                line((84.0, 2.0), (30.0, 20.0), 2),
                line((30.0, 20.0), (0.0, 20.0), 3),
                line((0.0, 20.0), (0.0, -5.0), 4),
            ])],
        },
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        TAU,
        1,
    )
    .expect("hub");
    let (a, b, c) = (28.4, 0.4, 1.6);
    let arc = |x: f64, y: f64, start: f64, id| {
        (
            Segment::Arc {
                center: Point2::new(x, y),
                radius: c,
                start,
                sweep: PI / 2.0,
            },
            id,
        )
    };
    let outline = ProfileLoop::new(vec![
        line((-a, -2.0), (a, -2.0), 0),
        arc(a, -b, -PI / 2.0, 1),
        line((30.0, -b), (30.0, b), 2),
        arc(a, b, 0.0, 3),
        line((a, 2.0), (-a, 2.0), 4),
        arc(-a, b, PI / 2.0, 5),
        line((-30.0, b), (-30.0, -b), 6),
        arc(-a, -b, PI, 7),
    ]);
    let x = Vec3::new(1.0, 1.0, 0.0);
    let centre = Point3::ORIGIN + x.normalize().expect("unit").get() * 55.0;
    let blade = extrude(
        &Profile {
            plane: Frame::from_zx(centre, UnitVec3::Z, x).expect("frame"),
            loops: vec![outline],
        },
        60.0,
        2,
    )
    .expect("blade");
    let before = boolean(&hub, &blade, Op::Union, 3).expect("union");
    let root: Vec<FilletEdge> = before
        .edge_ids()
        .filter(|&e| fillet_kind(&before, e) == Some(false))
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(1.5),
        })
        .collect();
    let done = fillet(&before, &root, 9).expect("rounds");
    let held = forge_proof::blends::audit(&before, &done, &Radii::List(vec![1.5]), 1);
    assert_eq!(held.unsupported, 0);
    assert!(held.carried > 0);
    assert!(held.worst_carried < 0.5);
    assert!(forge_proof::blends::audit(&before, &done, &Radii::List(vec![1.51]), 1).unsupported > 0);
}
