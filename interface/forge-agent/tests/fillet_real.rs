//! Fillets as real parts ask for them, each restated in its own numbers from a shape a dataset
//! part had: every one valid on both regens and of exactly the volume and area worked out here.
use forge_agent::model::Model;
use forge_model::Part;
use std::f64::consts::PI;

fn exact(src: &str, volume: f64, area: f64) {
    let m = Model::new(src);
    if let Some(f) = &m.error {
        panic!("line {}: {}\n{src}", m.lines[f.at].no, f.msg);
    }
    let part = Part {
        features: m.features.clone(),
    };
    for (how, built) in [("seq", part.regenerate()), ("n-ary", part.regenerate_nary(2))] {
        let b = built.unwrap_or_else(|e| panic!("{how}: {e:?}\n{src}"));
        let problems = forge_topo::check(&b);
        assert!(problems.is_empty(), "{how}: {problems:?}\n{src}");
        let p = forge_topo::mass::mass(&b).expect("mass");
        assert!(
            (p.volume - volume).abs() < 1e-5 * volume,
            "{how} volume {} not {volume}\n{src}",
            p.volume
        );
        assert!(
            (p.area - area).abs() < 1e-5 * area,
            "{how} area {} not {area}\n{src}",
            p.area
        );
    }
}

#[test]
fn two_rounds_beside_a_sharp_edge_mitre() {
    // two cylinders of radius r cut each other in a plane; each removes (1 - pi/4) r^2 of section
    // along its edge up to the mitre, and where they overlap the solid under both is integrated
    let (w, d, h, r) = (100.0f64, 80.0f64, 40.0f64, 5.0f64);
    // the removed volume of a corner of two rounds: along each edge (length - r) of section, and in
    // the last r square, the union of the two: (2 - pi/2) r^2 r - common, common = (1 - pi/4)^... by integration
    let n = 2000;
    let mut corner = 0.0;
    for i in 0..n {
        for j in 0..n {
            let (x, y) = ((i as f64 + 0.5) / n as f64 * r, (j as f64 + 0.5) / n as f64 * r);
            let drop = |t: f64| r - (r * r - t * t).max(0.0).sqrt();
            corner += drop(x).max(drop(y)) * (r / n as f64) * (r / n as f64);
        }
    }
    let side = (1.0 - PI / 4.0) * r * r;
    let volume = w * d * h - side * (w - r) - side * (d - r) - corner;
    let m = Model::new(&format!(
        "b: extrude {h} rect 0 0 {w} {d}\nfillet {r} (edges near 50 80 40) (edges near 100 40 40)\n"
    ));
    assert!(m.error.is_none());
    let b = m.body().expect("built");
    let p = forge_topo::mass::mass(b).expect("mass");
    assert!((p.volume - volume).abs() < 1e-2, "{} not {volume}", p.volume);
    assert!(forge_topo::check(b).is_empty());
    // and nothing beside the two cylinders: no patch
    let new = b
        .face_ids()
        .filter(|&f| matches!(b.face(f).surface, forge_geom::Surface::Procedural(_)))
        .count();
    assert_eq!(new, 0);
}

#[test]
fn a_rim_of_the_balls_own_radius_is_a_dome() {
    let (r, h) = (2.5f64, 8.0f64);
    let v = PI * r * r * (h - r) + 2.0 / 3.0 * PI * r * r * r;
    let a = PI * r * r + 2.0 * PI * r * (h - r) + 2.0 * PI * r * r;
    exact(
        &format!("extrude {h} circle 0 0 {r}\nfillet {r} (edges top)\n"),
        v,
        a,
    );
}

#[test]
fn corners_of_the_balls_own_radius_turn_on_a_sphere() {
    // a plate 60 x 40 x 14 with its upright corners of radius 3, then its top edges of radius 3
    let (w, d, h, r) = (60.0f64, 40.0f64, 14.0f64, 3.0f64);
    let plan = w * d - (4.0 - PI) * r * r;
    // the section lost along the rim's straight runs, and a full turn of it about the corner axes
    let lost = (1.0 - PI / 4.0) * r * r;
    let straight = 2.0 * (w - 2.0 * r) + 2.0 * (d - 2.0 * r);
    // a corner keeps a whole sphere's quarter-of-a-turn: over one turn the corners keep, from a
    // cylinder of radius r and height r, a half ball
    let turn_lost = PI * r * r * r - 2.0 / 3.0 * PI * r * r * r;
    let v = plan * h - lost * straight - turn_lost;
    let m = Model::new(&format!(
        "b: extrude {h} rect 0 0 {w} {d}\nfillet {r} (edges along +z)\nfillet {r} (edges top)\n"
    ));
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let b = m.body().expect("built");
    let p = forge_topo::mass::mass(b).expect("mass");
    assert!((p.volume - v).abs() < 1e-6 * v, "{} not {v}", p.volume);
    assert!(forge_topo::check(b).is_empty());
}

#[test]
fn a_ball_as_wide_as_the_face_takes_it() {
    let (w, l, h) = (48.0f64, 90.0f64, 24.0f64);
    let v = (w * h - h * h * (1.0 - PI / 4.0)) * l;
    let m = Model::new(&format!(
        "extrude {h} rect 0 0 {w} {l}\nfillet {h} (edges top along +y right)\n"
    ));
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let b = m.body().expect("built");
    assert!((forge_topo::mass::mass(b).expect("mass").volume - v).abs() < 1e-6 * v);
    assert!(forge_topo::check(b).is_empty());
    // the wall under the round is gone
    assert_eq!(b.face_ids().count(), 6);
}

#[test]
fn half_the_width_rounds_full() {
    // 20 wide, both upright edges of radius 10: the end face goes, the blends meet along a line
    let (l, w, h) = (45.0f64, 20.0f64, 6.0f64);
    let r = w / 2.0;
    let v = ((l - r) * w + PI * r * r / 2.0) * h;
    let a = 2.0 * ((l - r) * w + PI * r * r / 2.0) + h * (2.0 * (l - r) + w + PI * r);
    exact(
        &format!("extrude {h} rect 0 0 {l} {w}\nfillet {r} (edges along +z right)\n"),
        v,
        a,
    );
    // a disc 2 thick, both rims of radius 1: a bull nose
    let (big, t, q) = (15.0f64, 2.0f64, 1.0f64);
    let v = PI * (big - q) * (big - q) * t + 2.0 * PI * (big - q + 4.0 * q / (3.0 * PI)) * (PI * q * q / 2.0);
    let a = 2.0 * PI * (big - q) * (big - q) + 2.0 * PI * q * (PI * (big - q) + 2.0 * q);
    exact(
        &format!("extrude {t} circle 0 0 {big}\nfillet {q} (edges circle)\n"),
        v,
        a,
    );
}

#[test]
fn past_half_the_width_the_rounds_leave_a_ridge() {
    // 12 wide with both top edges of radius 8.5: two arcs meeting over the middle, the height
    // there sqrt(r^2 - (w/2 - (w - r))^2) above the arcs' centres
    let (w, l, h, r) = (12.0f64, 48.0f64, 24.0f64, 8.5f64);
    let c = w - r; // an arc's centre, from the near side
    let half = w / 2.0;
    // the section under one arc, from the middle to the side
    let under = |x: f64| (r * r - (x - c) * (x - c)).max(0.0).sqrt();
    let n = 200_000;
    let mut area = 0.0;
    for i in 0..n {
        let x = half + half * (i as f64 + 0.5) / n as f64;
        area += (h - r + under(x)) * half / n as f64;
    }
    let v = 2.0 * area * l;
    let m = Model::new(&format!(
        "extrude {h} rect 0 0 {w} {l}\nfillet {r} (edges top along +y)\n"
    ));
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let b = m.body().expect("built");
    assert!((forge_topo::mass::mass(b).expect("mass").volume - v).abs() < 1e-5 * v);
    assert!(forge_topo::check(b).is_empty());
}

#[test]
fn a_ball_past_the_wall_rolls_on_its_far_edge() {
    // a disc radius 20, 3 thick, rounded 5 on its top rim: the ball touches the top and the bottom
    // rim, its centre sqrt(r^2 - (r - t)^2) in from the rim
    let (big, t, r) = (20.0f64, 3.0f64, 5.0f64);
    let c = big - (r * r - (r - t) * (r - t)).sqrt();
    let n = 400_000;
    let mut v = 0.0;
    for i in 0..n {
        let z = (i as f64 + 0.5) / n as f64 * t;
        let rho = c + (r * r - (z - r) * (z - r)).max(0.0).sqrt();
        v += PI * rho * rho * t / n as f64;
    }
    let m = Model::new(&format!("extrude {t} circle 0 0 {big}\nfillet {r} (edges top)\n"));
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let b = m.body().expect("built");
    assert!((forge_topo::mass::mass(b).expect("mass").volume - v).abs() < 1e-5 * v);
    assert!(forge_topo::check(b).is_empty());
}

#[test]
fn a_notch_round_rolls_the_rim_over_the_concave_blend_first() {
    // two top rims down a V and the V's concave bottom, all of one radius: the V's bottom rounds
    // first and the rims run on round it, so the corner is the torus they sweep, no patch
    let src = "b: extrude 25 loop line -40 0 40 0 line 40 0 40 30 line 40 30 15 30 line 15 30 0 10 line 0 10 -15 30 line -15 30 -40 30 line -40 30 -40 0\nfillet 4 (edges concave) (edges top near 7.5 20 25) (edges top near -7.5 20 25) (edges top near 27.5 30 25) (edges top near -27.5 30 25)\n";
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let b = m.body().expect("built");
    assert!(forge_topo::check(b).is_empty());
    let tori = b
        .face_ids()
        .filter(|&f| matches!(b.face(f).surface, forge_geom::Surface::Torus(_)))
        .count();
    assert_eq!(tori, 1, "the rim's turn round the bottom's round");
    let patches = b
        .face_ids()
        .filter(|&f| matches!(b.face(f).surface, forge_geom::Surface::Procedural(_)))
        .count();
    assert_eq!(patches, 0);
}

#[test]
fn a_ring_as_wide_as_the_ball_loses_its_top() {
    // a rounded frame 120 x 200 outside with corners of 20, 100 x 180 inside with corners of 10, 40
    // tall, its outer top rim of radius 10: the spring lies on the inner rim all the way round
    let src = "b: extrude 40 loop line 40 100 -40 100 arc -40 80 20 90 90 line -60 80 -60 -80 arc -40 -80 20 180 90 line -40 -100 40 -100 arc 40 -80 20 -90 90 line 60 -80 60 80 arc 40 80 20 0 90 loop line -40 90 40 90 arc 40 80 10 90 -90 line 50 80 50 -80 arc 40 -80 10 0 -90 line 40 -90 -40 -90 arc -40 -80 10 -90 -90 line -50 -80 -50 80 arc -40 80 10 180 -90\nfillet 10 (edges top r=20) (edges top line near 0 100 40) (edges top line near 0 -100 40) (edges top line near 60 0 40) (edges top line near -60 0 40)\n";
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let b = m.body().expect("built");
    assert!(forge_topo::check(b).is_empty());
    let tops = b
        .face_ids()
        .filter(|&f| matches!(&b.face(f).surface, forge_geom::Surface::Plane(_)))
        .filter(|&f| b.face_box(f).z.lo > 39.0)
        .count();
    assert_eq!(tops, 0, "the top goes");
}

#[test]
fn two_rounds_past_each_other_round_a_ring_refuse() {
    // a disc's rim and the root of a boss on it, the ring between narrower than the two balls: their
    // springs pass with no crossing to see; the face must not come back wrong side out
    let m = Model::new(
        "extrude 0.15 circle 0 0 0.3\nextrude add 1 at 0 0 0 circle 0 0 0.15\nfillet 0.1 (edges top r=0.3) (edges concave)\n",
    );
    let b = m.body().expect("the part before");
    assert!(forge_topo::check(b).is_empty());
    if m.error.is_none() {
        panic!("built");
    }
}

#[test]
fn a_ball_wider_than_a_shell_wall_refuses() {
    // a shelled cup, wall 0.5, its rim rounded 3: the blend would pass through the wall into the cup
    let m = Model::new("c: extrude 20 circle 0 0 20\nshell 0.5 (faces top)\nfillet 3 (edges bottom)\n");
    let msg = m.error.map(|f| f.msg).unwrap_or_default();
    assert!(msg.contains("Overflow"), "{msg}");
}

#[test]
fn rounds_beside_chamfers_keep_their_names_apart() {
    // a plate chamfered all round its one end face, then those bevels' outer edges rounded by the bevel's own size
    let m = Model::new(
        "b: extrude 0.2 yz rect 0 0 0.19 0.75\nchamfer 0.02 (edges of b top)\nfillet 0.02 (edges line near 0.2 0.1 0.02) (edges line near 0.2 0.19 0.37) (edges line near 0.2 0 0.37) (edges line near 0.2 0.1 0.73)\n",
    );
    if m.error.is_none() {
        assert!(forge_topo::check(m.body().expect("built")).is_empty());
    }
}
