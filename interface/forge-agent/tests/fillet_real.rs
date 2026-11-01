//! Fillets as real parts ask for them, each restated in its own numbers from a shape a dataset
//! part had: every one valid on both regens and of exactly the volume and area worked out here.
use forge_agent::model::Model;
use forge_math::dmath;
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

const ROUNDED_CORNER: &str = "b: extrude 10 z 0 0 1 x 1 0 0 at 0 0 0 loop line 0 0 40 0 arc 40 5 5 -90 90 line 45 5 45 30 line 45 30 0 30 line 0 30 0 0";

fn with_line(line: &str) -> String {
    format!("{ROUNDED_CORNER}\n{line}\n")
}

#[test]
fn only_a_straight_edge_runs_out_into_the_rounded_corner_beside_it() {
    // the plate's section is 45 x 30 less a corner of radius 5
    let (r, base) = (2.0f64, 45.0 * 30.0 - (1.0 - PI / 4.0) * 25.0);
    // along the 40 of the straight edge, the section the round takes off
    let along = 40.0 * (1.0 - PI / 4.0) * r * r;
    // past it the round's cylinder runs straight on, over the corner's cylinder until it leaves it:
    // over y from 0 to r and z from the round's arc to the top, x from 40 to the corner's circle
    let n = 4000;
    let h = r / n as f64;
    let beyond: f64 = (0..n)
        .map(|i| {
            let y = (i as f64 + 0.5) * h;
            let under = r - (r * r - (r - y) * (r - y)).sqrt();
            under * (25.0 - (5.0 - y) * (5.0 - y)).sqrt()
        })
        .sum::<f64>()
        * h;
    for at in ["10", "0"] {
        let src = with_line(&format!("fillet 2 only (edges near 20 0 {at})"));
        let m = Model::new(&src);
        if let Some(f) = &m.error {
            panic!("line {}: {}\n{src}", m.lines[f.at].no, f.msg);
        }
        let part = Part {
            features: m.features.clone(),
        };
        for (how, built) in [("seq", part.regenerate()), ("n-ary", part.regenerate_nary(2))] {
            let b = built.unwrap_or_else(|e| panic!("{how}: {e:?}\n{src}"));
            assert_eq!(forge_topo::check(&b), vec![], "{how}\n{src}");
            assert_eq!(b.face_ids().count(), 8, "{how}: the seven and the round");
            let v = forge_topo::mass::mass(&b).expect("mass").volume;
            let want = 10.0 * base - along - beyond;
            assert!((v - want).abs() < 1e-4 * want, "{how} {at}: {v} not {want}");
        }
    }
}

#[test]
fn only_changes_nothing_without_it_and_prints_back() {
    let plain = Model::new(&with_line("fillet 2 (edges near 20 0 10)"));
    assert!(plain.error.is_none());
    let part = Part {
        features: plain.features.clone(),
    };
    // the whole chain: the straight, the corner, the far wall's top
    let b = part.regenerate().expect("chain");
    assert_eq!(b.face_ids().count(), 10);
    let src = with_line("fillet 2 only (edges near 20 0 10)");
    let m = Model::new(&src);
    assert!(matches!(
        m.features.last(),
        Some(forge_model::Feature::Fillet { only: true, .. })
    ));
    let back = forge_doc::text::print(&Part {
        features: m.features.clone(),
    });
    assert!(back.contains("fillet 2 only"), "{back}");
}

#[test]
fn a_free_end_with_nothing_to_run_out_into_says_so() {
    // past the corner's arc the round stays in the material, and the wall it should end on stays
    // tangent to it
    for line in [
        "fillet 2 only (edges near 43.5 1.5 10)",
        "fillet 2 only (edges near 20 0 10) (edges near 43.5 1.5 10)",
        // the corner's far side closer than the ball can leave it
        "fillet 4.5 only (edges near 20 0 10)",
    ] {
        let m = Model::new(&with_line(line));
        let f = m.error.as_ref().unwrap_or_else(|| panic!("{line} built"));
        assert!(f.msg.contains("tangent chain"), "{line}: {}", f.msg);
    }
}

#[test]
fn a_ball_wider_than_a_short_face_does_not_rest_on_a_ridge_the_next_face_covers() {
    // a top of short flats, each a little steeper: the ball on the corner's wall reaches past the
    // first few, and the ridge between them lies outside it
    let mut top = String::new();
    let mut at = (20.0f64, 10.0f64);
    for k in 1..=20 {
        let next = (20.0 - k as f64, 10.0 + 0.05 * k as f64 - 0.00125 * (k * k) as f64);
        top += &format!(" line {} {} {} {}", at.0, at.1, next.0, next.1);
        at = next;
    }
    let src = format!(
        "b: extrude 10 z 0 0 1 x 1 0 0 at 0 0 0 loop line 0 0 20 0 line 20 0 20 10{top} line {} {} 0 0\nfillet 3 (edges near 20 10 5)\n",
        at.0, at.1
    );
    let m = Model::new(&src);
    let f = m.error.as_ref().expect("refuses, not a blend through the top");
    assert!(f.msg.contains("fillet"), "{}", f.msg);
}

#[test]
fn two_rounds_crossing_on_a_face_become_one_of_the_faces_beyond() {
    // a lobe of radius big between two hollows of radius a: a ball on either of its edges would
    // reach past the other's tangent point, and one ball rests on both hollows instead
    let (d, a, r, h, base) = (3.518f64, 2.218f64, 1.5625f64, 10.0f64, -6.0f64);
    let drop = 3.37f64;
    let big = ((d - a) * (d - a) + drop * drop).sqrt();
    let start = dmath::atan2(drop, d - a).to_degrees();
    let src = format!(
        "b: extrude {h} z 0 0 1 x 1 0 0 at 0 0 0 loop line {md} {base} {d} {base} line {d} {base} {d} {low} arc {d} 0 {a} -90 -90 arc 0 {mdrop} {big} {start} {sweep} arc {md} 0 {a} 0 -90 line {md} {low} {md} {base}\nfillet {r} (edges near {e} 0 5) (edges near {me} 0 5)\n",
        md = -d,
        low = -a,
        mdrop = -drop,
        sweep = 180.0 - 2.0 * start,
        e = d - a,
        me = a - d,
    );
    // the ball's centre, on the middle line, a + r from each hollow's centre
    let yc = -((a + r) * (a + r) - d * d).sqrt();
    let out = dmath::atan2(-yc, d).to_degrees();
    let mut pts = vec![(-d, base), (d, base)];
    let n = 20000;
    let arc = |pts: &mut Vec<(f64, f64)>, c: (f64, f64), rad: f64, from: f64, to: f64| {
        for i in 0..=n {
            let t = (from + (to - from) * i as f64 / n as f64).to_radians();
            pts.push((c.0 + rad * dmath::cos(t), c.1 + rad * dmath::sin(t)));
        }
    };
    arc(&mut pts, (d, 0.0), a, -90.0, -180.0 + out);
    arc(&mut pts, (0.0, yc), r, out, 180.0 - out);
    arc(&mut pts, (-d, 0.0), a, -out, -90.0);
    let area: f64 = (0..pts.len())
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            p.0 * q.1 - q.0 * p.1
        })
        .sum::<f64>()
        * 0.5;
    let m = Model::new(&src);
    if let Some(f) = &m.error {
        panic!("line {}: {}\n{src}", m.lines[f.at].no, f.msg);
    }
    let part = Part {
        features: m.features.clone(),
    };
    for (how, built) in [("seq", part.regenerate()), ("n-ary", part.regenerate_nary(2))] {
        let b = built.unwrap_or_else(|e| panic!("{how}: {e:?}\n{src}"));
        assert_eq!(forge_topo::check(&b), vec![], "{how}\n{src}");
        // ends, base, two sides, two hollows, the one round; the lobe is gone
        assert_eq!(b.face_ids().count(), 8, "{how}\n{src}");
        let v = forge_topo::mass::mass(&b).expect("mass").volume;
        assert!(
            (v - h * area).abs() < 1e-5 * v,
            "{how}: {v} not {}\n{src}",
            h * area
        );
    }
}

#[test]
fn a_round_foot_of_a_boss_is_not_taken_for_a_blend_through_the_part() {
    // a stadium boss on a stadium plate, its foot rounded all round: the half rings' vertices sit
    // together at their ends, and the mean of them lies off the round
    let (r, br, bl, bh) = (3.125f64, 9.375f64, 37.5f64, 14.0625f64);
    let (pr, pl, ph) = (18.65f64, 37.3f64, 9.375f64);
    let src = format!(
        "extrude {ph} z 1 0 0 x 0 1 0 at 0 -18.75 -18.75 loop arc {pr} 0 {pr} 180 180 line {pl} 0 {pl} {pl} arc {pr} {pl} {pr} 0 180 line 0 {pl} 0 0\nextrude add {bh} z 1 0 0 x 0 1 0 at {ph} -{br} -18.75 loop arc {br} 0 {br} 180 180 line {d} 0 {d} {bl} arc {br} {bl} {br} 0 180 line 0 {bl} 0 0\nfillet {r} (edges near {ph} 0 28.125) (edges near {ph} {br} 0) (edges near {ph} 0 -28.125) (edges near {ph} -{br} 0)\n",
        d = 2.0 * br,
    );
    let section = (1.0 - PI / 4.0) * r * r;
    // a spandrel's centroid stands r(10 - 3 pi)/(12 - 3 pi) out from its corner
    let out = r * (10.0 - 3.0 * PI) / (12.0 - 3.0 * PI);
    let added = 2.0 * bl * section + 2.0 * PI * (br + out) * section;
    let plate = ph * (pl * pl + PI * pr * pr);
    let boss = bh * (2.0 * br * bl + PI * br * br);
    let m = Model::new(&src);
    if let Some(f) = &m.error {
        panic!("line {}: {}\n{src}", m.lines[f.at].no, f.msg);
    }
    let part = Part {
        features: m.features.clone(),
    };
    for (how, built) in [("seq", part.regenerate()), ("n-ary", part.regenerate_nary(2))] {
        let b = built.unwrap_or_else(|e| panic!("{how}: {e:?}\n{src}"));
        assert_eq!(forge_topo::check(&b), vec![], "{how}\n{src}");
        let v = forge_topo::mass::mass(&b).expect("mass").volume;
        let want = plate + boss + added;
        assert!((v - want).abs() < 1e-5 * want, "{how}: {v} not {want}\n{src}");
    }
}

#[test]
fn a_round_ending_where_its_arc_is_tangent_to_a_wall_keeps_to_its_own_arc() {
    // a boss low against a taller plate, the foot edge beside the wall rounded, then the arc that
    // leaves on the boss's top: the blend's torus meets the wall in a loop through their touch point,
    // and the cut there is its short way, up from the touch point to the top
    let src = "extrude 12 rect 0 0 100 50\nextrude add 8 rect 40 50 20 20\nfillet 1 (edges near 40 50 4)\nfillet 0.5 (edges circle near 39.3 50.7 8)\n";
    let (r0, r) = (1.0f64, 0.5f64);
    let base = 100.0 * 50.0 * 12.0 + 20.0 * 20.0 * 8.0 + (1.0 - PI / 4.0) * r0 * r0 * 8.0;
    let along = 19.0 * (1.0 - PI / 4.0) * r * r;
    // over the arc: at radius u round its axis, from the line's end to the wall
    let n = 200_000;
    let w = r / n as f64;
    let round: f64 = (0..n)
        .map(|i| {
            let u = r0 + (i as f64 + 0.5) * w;
            let under = r - (r * r - (r0 + r - u) * (r0 + r - u)).sqrt();
            under * u * dmath::asin(r0 / u)
        })
        .sum::<f64>()
        * w;
    let want = base - along - round;
    let m = Model::new(src);
    if let Some(f) = &m.error {
        panic!("line {}: {}", m.lines[f.at].no, f.msg);
    }
    let part = Part {
        features: m.features.clone(),
    };
    for (how, built) in [("seq", part.regenerate()), ("n-ary", part.regenerate_nary(2))] {
        let b = built.unwrap_or_else(|e| panic!("{how}: {e:?}"));
        assert_eq!(forge_topo::check(&b), vec![], "{how}");
        let v = forge_topo::mass::mass(&b).expect("mass").volume;
        assert!((v - want).abs() < 1e-5, "{how}: {v} not {want}");
    }
}

#[test]
fn a_round_whose_edge_crosses_a_holes_rim_refuses() {
    // the top edge rounded by 5.1, its spring 5.1 in: a hole's rim comes to 5 of that edge, and would
    // stay a whole loop of the top face, across the spring
    let src = "extrude 10 rect 0 0 60 40\nextrude cut 4 z 0 0 1 x 1 0 0 at 0 0 6 circle 3 7 2\nfillet 5.003 (edges near 30 0 10)\n";
    let m = Model::new(src);
    let f = m.error.as_ref().expect("refuses");
    assert_eq!(m.lines[f.at].no, 3, "{}", f.msg);
    assert!(f.msg.contains("fillet"), "{}", f.msg);
    // clear of the rim, it builds
    let m = Model::new(
        "extrude 10 rect 0 0 60 40\nextrude cut 4 z 0 0 1 x 1 0 0 at 0 0 6 circle 3 7 2\nfillet 4.5 (edges near 30 0 10)\n",
    );
    assert!(m.error.is_none());
    let part = Part {
        features: m.features.clone(),
    };
    for built in [part.regenerate(), part.regenerate_nary(2)] {
        assert_eq!(forge_topo::check(&built.expect("builds")), vec![]);
    }
}

#[test]
fn a_convex_and_a_concave_round_passing_on_a_narrow_step_refuse() {
    // a boss on a disc: the step between is 15 and the two rounds of 9.5 need 19. Their boolean is a
    // solid with the step's flat facing down under an overhang, valid and not what was asked
    let lines = "extrude 15 circle 0 0 30\nextrude add 60 z 0 0 1 x 1 0 0 at 0 0 15 circle 0 0 15\n";
    let refused = Model::new(&format!(
        "{lines}fillet 9.5 (edges circle r=30 near 30 0 15) (edges circle r=15 near 15 0 15)\n"
    ));
    let f = refused.error.as_ref().expect("refuses");
    assert_eq!(refused.lines[f.at].no, 3, "{}", f.msg);
    // with room they build: each ring a torus, the flat still up
    let m = Model::new(&format!(
        "{lines}fillet 7 (edges circle r=30 near 30 0 15) (edges circle r=15 near 15 0 15)\n"
    ));
    assert!(m.error.is_none());
    let part = Part {
        features: m.features.clone(),
    };
    for built in [part.regenerate(), part.regenerate_nary(2)] {
        assert_eq!(forge_topo::check(&built.expect("builds")), vec![]);
    }
}
