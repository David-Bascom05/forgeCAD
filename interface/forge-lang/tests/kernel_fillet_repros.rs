//! Fillets on real parts that once failed, as Forge parts: each builds valid, sequentially and
//! n-ary.
mod common;

use forge_model::Part;
use forge_topo::Body;

/// The part's history as the kernel takes it: its features and their ids.
fn history(src: &str) -> (Part, Vec<u64>) {
    let (r, cache) = common::run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    let (f, ids) = cache.history();
    (Part { features: f.to_vec() }, ids.to_vec())
}

/// The finished part, its build free of errors.
fn body(src: &str) -> Body {
    let (r, _) = common::run(src);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    r.body.expect("a part")
}

/// The kernel's refusal of the part, as the kernel names it.
fn kernel_refusal(src: &str) -> String {
    let (r, _) = common::run(src);
    r.errors
        .iter()
        .find_map(|e| match e {
            forge_lang::error::Error::Build { error, .. } => Some(format!("{error:?}")),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no kernel refusal: {}", forge_lang::report::text(&r, src)))
}

/// Builds, and the kernel regenerates it valid sequentially and in n-ary booleans on two threads.
fn valid(src: &str) {
    let (part, ids) = history(src);
    let seq = part.regenerate_ids(Some(&ids)).map(|b| vec![b]);
    let nary = part
        .regenerate_nary_solids_ids(2, Some(&ids))
        .map(|s| s.into_iter().map(|s| s.body).collect::<Vec<_>>());
    for (how, built) in [("seq", seq), ("n-ary", nary)] {
        for b in built.unwrap_or_else(|e| panic!("{how}: {e:?}\n{src}")) {
            let problems = forge_topo::check(&b);
            assert!(problems.is_empty(), "{how}: {problems:?}\n{src}");
        }
    }
}

#[test]
fn shell_after_a_variable_fillet() {
    // the blend's run-outs past its ends are tighter than the shell is thick, the face is not;
    // its offset springs meet the offset top and sides tangentially
    valid(
        r#"
sketch s(on: xy) {
  r = rect(corner: (0, -15), size: (170, 30))
}
bar = extrude(s, 14)
round = fillet(edges(of: bar.end, along: X), vary(4, 3))
cup = shell(1.6, open: bar.start)
"#,
    );
}

#[test]
fn round_a_tapered_arm_with_its_tip() {
    // the sides and the tip arc are one tangent chain round the tip, so it comes back beside the
    // root corner it leaves: its middle is halfway along it, not across
    valid(
        r#"
sketch s(on: xy) {
  a = pt(0, -15)
  b = pt(169.905858, -15.999723)
  c = pt(170, 0)
  d = pt(169.905858, 15.999723)
  e = pt(0, 15)
  l1 = line(a, b)
  q = arc(center: c, from: b, to: d)
  l2 = line(d, e)
  l3 = line(e, a)
}
arm = extrude(s, 14)
round = fillet(edges(of: arm.end), 2)
"#,
    );
}

#[test]
fn block_flush_on_a_disc() {
    // a conrod bolt boss: the end faces split by the union join before the blends run out over
    // them. Radius 3 would touch the boss's side 3.04 above its top (the ball centre sits at
    // z = sqrt(34^2 - 23^2) = 25.04 > 22): that refuses, and says so
    let part = |r: &str| {
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
sketch bg(on: XZ) {
  c = circle((0, 0), 31)
}
big = extrude(bg, sym(22))
sketch bk(on: XZ) {
  r = rect(corner: (26, -24), size: (16, 46))
}
b = extrude(bk, sym(22))
root = fillet(edges(between: (b, big), crease: "concave"), R)
"#
        .replace('R', r)
    };
    valid(&part("1.5"));
    let msg = kernel_refusal(&part("3"));
    assert!(msg.contains("Overflow"), "{msg}");
}

#[test]
fn variable_fillet_round_a_tube_into_a_cone() {
    // the joint's loop is two traced edges; their end tangents at the seam differed by 3e-6 rad,
    // so the chain was left open and the blends met at the seam with different sections
    valid(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
shared reach = 385
shared stack = 575
shared hta = 73deg
shared sta = 73.5deg
shared htl = 160
shared stl = 540
shared hdx = cos(hta)
shared hdz = 0 - sin(hta)
shared s1x = 0 - (stl - 35) * cos(sta)
shared s1z = (stl - 35) * sin(sta)
shared h1x = reach + 25mm * hdx
shared h1z = stack + 25mm * hdz
shared ttl = sqrt((h1x - s1x) ^ 2 + (h1z - s1z) ^ 2)
sketch hs(on: XZ) {
  ax = line((reach, stack), (reach + 10mm * hdx, stack + 10mm * hdz), construction: true)
  o = poly([(reach, stack), (reach + 25mm * sin(hta), stack + 25mm * cos(hta)), (reach + htl * hdx + 30mm * sin(hta), stack - htl * sin(hta) + 30mm * cos(hta)), (reach + htl * hdx, stack - htl * sin(hta))])
  tl = line((s1x, s1z), (h1x, h1z), construction: true)
}
ht = revolve(hs.o, axis: hs.ax)
sketch ts(on: plane(at: hs.tl.a, normal: hs.tl.dir)) {
  c = circle((0, 0), 14.3)
}
tt = extrude(ts, ttl)
joint = fillet(edges(between: (tt, ht), crease: "concave"), vary(4, 2.5))
"#,
    );
}

#[test]
fn concave_pad_fillet_ending_on_a_drafted_tube() {
    // the blend's spine is a hyperbola (a plane cut through the offset cone): its nearest-point
    // search over a corner's wide range sampled past where it could be near
    valid(
        r#"
sketch ts(on: xy) {
  c = circle((0, 5), 14)
}
tube = extrude(ts, 60)
dr = draft(tube.side(ts.c), angle: -8deg, neutral: ts.plane)
sketch ps(on: offset(plane(at: ts.origin, normal: yz.normal, x: -xz.normal), 8)) {
  r = rect(corner: (-40, 10), size: (80, 40))
}
pad = extrude(ps, 5)
root = fillet(edges(between: (pad, tube), crease: "concave"), 2)
"#,
    );
}

#[test]
fn face_fillet_and_full_round_after_a_union() {
    // the union leaves dead entities: the strip compacts the body, and the faces it was given are
    // found again after compacting, not read by their old ids
    valid(
        r#"
sketch bs(on: xy) {
  r = rect(corner: (-40, -30), size: (80, 60))
}
base = extrude(bs, 10)
sketch us(on: base.end) {
  r = rect(corner: (-40, -30), size: (12, 60))
}
up = extrude(us, 50)
ff = facefillet(up.side(us.r[1]), base.end, 6)
"#,
    );
    valid(
        r#"
sketch bs(on: xy) {
  r = rect(corner: (-40, -30), size: (80, 60))
}
base = extrude(bs, 10)
sketch rs(on: base.end) {
  r = rect(corner: (-30, -4), size: (60, 8))
}
rb = extrude(rs, 40)
fr = fullround(side: rb.side(rs.r[0]), middle: rb.end, other: rb.side(rs.r[2]))
"#,
    );
}

#[test]
fn tab_root_fillet_on_a_thin_wall() {
    // the run-out on the cylinder sits a radius above the tab's top, past the end of its 1.5 high
    // side edge: the edge is carried on as far as that, not only half its own length
    valid(
        r#"
sketch rs(on: xy) {
  o = circle((0, 0), 69.5)
  i = circle((0, 0), 68)
}
ring = extrude(rs, 50)
sketch ts(on: rs.plane) {
  r = rect(corner: (-88, -10), size: (19.25, 20))
}
tab = extrude(ts, 1.5)
root = fillet(edges(between: (tab, ring), crease: "concave", shape: "circle"), 1)
"#,
    );
}

#[test]
fn second_pad_fillet() {
    // pad b's top lies on pad a's top plane but far from it: it is b's face, so the first fillet
    // rounds only pad a, and the second finds pad b's edges unrounded
    let src = r#"
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
sketch pa(on: foot.plane) {
  r = rect(corner: (-96, -93), size: (44, 34))
}
a = extrude(pa, 16)
sketch pb(on: foot.plane) {
  r = rect(corner: (52, -93), size: (44, 34))
}
b = extrude(pb, 16)
ra = fillet(edges(between: (a, box), crease: "concave"), 5)
rb = fillet(edges(between: (b, box), crease: "concave"), 5)
"#;
    let (part, _) = history(src);
    let first = part
        .features
        .iter()
        .find_map(|f| match f {
            forge_model::Feature::Fillet { edges, .. } => Some(edges.len()),
            _ => None,
        })
        .expect("a fillet");
    assert_eq!(first, 3);
    // n-ary names pad b's edges apart from seq until the n-ary naming fix (R30) lands
    assert!(forge_topo::check(&body(src)).is_empty());
}

#[test]
fn mirrored_cuts_after_a_variable_fillet() {
    // a pattern's copies go through one n-ary boolean, whose rays classify the stay bore's pieces:
    // one grazed the seat tube by the blend and met the blend's balls past its arc, a point not
    // on the face that counted as a crossing
    valid(
        r#"
XZ = plane(at: xy.origin, normal: xz.normal, x: yz.normal)
shared reach = 385
shared stack = 575
shared hta = 73deg
shared sta = 73.5deg
shared stl = 540
shared csl = 410
shared drop = 70
shared sp = 130
shared hdx = cos(hta)
shared hdz = 0 - sin(hta)
shared ax = 0 - sqrt(csl ^ 2 - drop ^ 2)
shared az = drop
shared s1x = 0 - (stl - 45) * cos(sta)
shared s1z = (stl - 45) * sin(sta)
shared h1x = reach + 25mm * hdx
shared h1z = stack + 25mm * hdz
shared ttl = sqrt((h1x - s1x) ^ 2 + (h1z - s1z) ^ 2)
shared c0x = -17.5
shared c0y = 22
shared c1x = ax + 6
shared c1y = sp / 2 - 4
shared csx = c1x - c0x
shared csy = c1y - c0y
shared csz = az
shared cl = sqrt(csx ^ 2 + csy ^ 2 + csz ^ 2)
shared q0x = 0 - (stl - 80) * cos(sta)
shared q0z = (stl - 80) * sin(sta)
shared q0y = 10
shared q1x = ax + 10
shared q1z = az + 14
shared q1y = sp / 2 - 4
sketch g(on: XZ) {
  sl = line((0, 0), (0 - 10mm * cos(sta), 10mm * sin(sta)), construction: true)
  tl = line((s1x, s1z), (h1x, h1z), construction: true)
}
sketch sts(on: plane(at: g.sl.a, normal: g.sl.dir)) {
  c = circle((0, 0), 15.9)
}
st = extrude(sts, stl)
sketch tts(on: plane(at: g.tl.a, normal: g.tl.dir)) {
  c = circle((0, 0), 14.3)
}
tt = extrude(tts, ttl)
joint = fillet(edges(between: (tt, st), crease: "concave"), vary([(0, 3), (ratio(0.5), 4.5), (1, 3)]))
sketch c0(on: offset(XZ, 0 - c0y)) {
  p = pt(c0x, 0)
}
sketch c1(on: offset(XZ, 0 - c1y)) {
  p = pt(c1x, az)
}
sketch ca(on: offset(XZ, 0 - (c0y + 12mm * csy / cl))) {
  p = pt(c0x + 12mm * csx / cl, 12mm * csz / cl)
}
sketch cs1(on: plane(at: ca.p, normal: axis(c0.p, c1.p))) {
  c = circle((0, 0), 9.9)
}
sketch cs2(on: plane(at: c1.p, normal: axis(c0.p, c1.p))) {
  c = circle((0, 0), 5.5)
}
csb = loft([cs1, cs2], cut: true)
sketch q0(on: offset(XZ, 0 - q0y)) {
  p = pt(q0x, q0z)
}
sketch q1(on: offset(XZ, 0 - q1y)) {
  p = pt(q1x, q1z)
}
sketch ss1(on: plane(at: q0.p, normal: axis(q0.p, q1.p))) {
  c = circle((0, 0), 7)
}
sketch ss2(on: plane(at: q1.p, normal: axis(q0.p, q1.p))) {
  c = circle((0, 0), 5)
}
ssb = loft([ss1, ss2], cut: true)
stays = pattern(of: [csb, ssb], layout: mirror(on: XZ))
"#,
    );
}

#[test]
fn law_direction_does_not_follow_names() {
    // other ids give the edges and vertices other names; the laws start where the geometry says
    let points = |src: &str| {
        let b = body(src);
        let mut ps: Vec<[i64; 3]> = b
            .vertex_ids()
            .map(|v| b.vertex(v).point)
            .map(|p| [p.x, p.y, p.z].map(|x| (x * 1e6).round() as i64))
            .collect();
        ps.sort();
        ps
    };
    for law in ["vary(1, 3)", "vary([(0, 1), (ratio(0.4), 2), (1, 1.5)])"] {
        // the block's name is its id, so its edges' and vertices' names
        let src = |id: &str| {
            format!(
                "sketch s(on: xy) {{\n  r = rect(corner: (0, 0), size: (20, 10))\n}}\n{id} = extrude(s, 8)\nround = fillet(edges(between: ({id}.end, {id}.side(s.r[0]))) + edges(between: ({id}.start, {id}.side(s.r[2]))), {law})\n"
            )
        };
        let a = points(&src("base"));
        for id in ["b7", "block123456"] {
            assert_eq!(a, points(&src(id)), "{law} {id}");
        }
    }
}

#[test]
fn ring_round_wider_than_its_ring_is_refused() {
    // a conrod (the I-beam shank, pocketed) whose small end, 20 thick and of radius 15, is rounded
    // over its rim: the full round's ball (radius 10) rolls round a centre circle of radius 5,
    // which works only up to (5 + 10) / 2
    let msg = kernel_refusal(
        r#"
shared cc = 150
shared rb = 31
shared rs = 15
shared tb = 22
shared ts = 20
sketch side(on: xz) {
  o = poly([(-14, rb - 6), (-8, cc - rs + 5), (8, cc - rs + 5), (14, rb - 6)])
  ax = line((0, 0), (0, cc), construction: true)
}
shank = extrude(side.o, sym(ts))
sketch bg(on: side.plane) {
  c = circle((0, 0), rb)
}
big = extrude(bg, sym(tb))
sketch sm(on: side.plane) {
  c = circle((0, cc), rs)
}
small = extrude(sm, sym(ts))
sketch bk(on: side.plane) {
  r = rect(corner: (26, -24), size: (16, 46))
}
bolts = extrude(bk, sym(tb))
boltsm = pattern(of: bolts, layout: mirror(on: plane(at: side.origin, normal: side.o[1].dir, x: side.ax.dir)))
sketch web(on: offset(side.plane, 0 - ts / 2)) {
  o = poly([(-9.5, rb + 2), (-4.5, cc - rs - 4), (4.5, cc - rs - 4), (9.5, rb + 2)])
}
pocket = cut(web, -7.5)
pocketm = pattern(of: pocket, layout: mirror(on: side.plane))
roots = fillet(edges(between: (shank, big), crease: "concave", shape: "line") + edges(between: (shank, small), crease: "concave"), 6)
ring = fullround(side: small.start, middle: small.side(sm.c), other: small.end)
"#,
    );
    assert!(msg.contains("TooLarge") && msg.contains("max: 7.5"), "{msg}");
}

/// The box, its corner rounded 12, `draft` before it, a pad 44 x 40 at `x` and the pad's
/// concave edges `which` rounded by `r`.
fn rolling(draft: bool, x: f64, which: &str, r: &str) -> String {
    let dr = if draft {
        "dr = draft(box.side(foot.r[3]) + box.side(foot.r[0]), angle: -2deg, neutral: offset(foot.plane, 95))\n"
    } else {
        ""
    };
    let edges = match which {
        "one" => "edges(between: (pad.side(ps.r[3]), box), crease: \"concave\")",
        _ => "edges(of: pad, crease: \"concave\")",
    };
    let round = if r.is_empty() {
        String::new()
    } else {
        format!("root = fillet({edges}, {r})\n")
    };
    format!(
        "sketch foot(on: xy) {{\n  r = rect(corner: (-120, -65), size: (240, 130))\n}}\nbox = extrude(foot, 95)\n{dr}corner = fillet(edges(between: (box.side(foot.r[0]), box.side(foot.r[3]))), 12)\nsketch ps(on: foot.plane) {{\n  r = rect(corner: ({x:?}, -93), size: (44, 40))\n}}\npad = extrude(ps, 16)\n{round}"
    )
}

#[test]
fn fillet_rolling_onto_a_corner_round() {
    // a pad 0.31 past the wall's tangent line onto a corner round: the r 3 ball between the pad's
    // side and the wall rests on the round, never the wall, so the blend is rolled on the round and
    // the wall's 0.31 strip goes. Its section, the region between the pad side, the ball and the
    // round, is 1.463135 mm^2 (the ball centre at x = -110.69, 15 from the round's axis), over 16
    let part = rolling(false, -107.69, "one", "");
    let src = rolling(false, -107.69, "one", "3");
    valid(&src);
    let q = forge_mesh::Quality {
        chord: 1e-3,
        max_angle: 0.2,
    };
    let volume = |s: &str| {
        let b = body(s);
        let fine = forge_mesh::tessellate(&b, &q).expect("meshes").volume();
        let coarse = forge_mesh::tessellate(&b, &forge_mesh::Quality { chord: 4e-3, ..q })
            .expect("meshes")
            .volume();
        fine + (fine - coarse) / 3.0
    };
    let added = volume(&src) - volume(&part);
    assert!((added - 23.41016).abs() < 1e-2, "{added}");
    // drafted walls: the round is a cone, the wall a drafted plane
    valid(&rolling(true, -104.0, "one", "3"));
}

#[test]
fn fillet_rolling_onto_a_corner_round_meets_the_pad_top_blend() {
    // the full R2 repro: every concave edge, so the blend resting on the round meets the pad
    // top's blend at the pad's top corner. The corner sees the round between the blend and the
    // wall, the smooth edge between them cut where the pad top's blend meets it
    for (draft, x) in [(false, -107.69), (true, -104.0)] {
        valid(&rolling(draft, x, "all", "3"));
    }
}

#[test]
fn fillet_moving_onto_a_corner_round_partway() {
    // the pad's side drafted 10 degrees: along the edge the ball's contact with the wall crosses
    // the round's tangent line, so the edge is split there and its pieces' blends (on the wall,
    // on the round) share the section through the crossing. Ball on the round above: the strip
    // left below between the round and the wall's blend is a face of its own. Below: the round's
    // tangent line ends at the section
    let part = |x: f64, draft: i32, which: &str| {
        rolling(false, x, which, "")
            + &format!("pd = draft(pad.side(ps.r[3]), angle: {draft}deg, neutral: foot.plane)\n")
            + match which {
                "one" => "root = fillet(edges(between: (pad.side(ps.r[3]), box), crease: \"concave\"), 3)\n",
                _ => "root = fillet(edges(of: pad, crease: \"concave\"), 3)\n",
            }
    };
    for (x, draft) in [(-104.0, -10), (-106.0, 10)] {
        for which in ["one", "all"] {
            valid(&part(x, draft, which));
        }
    }
    let b = body(&part(-104.0, -10, "one"));
    let walls = b
        .face_ids()
        .filter(|&f| {
            matches!(&b.face(f).surface, forge_geom::Surface::Plane(p)
                    if p.frame.z().get().y.abs() > 0.99 && (p.frame.origin.y + 65.0).abs() < 1e-9)
        })
        .count();
    // the wall and the sliver
    assert_eq!(walls, 2);
}

#[test]
fn variable_fillet_rolling_onto_a_corner_round() {
    // a radius from 2 to 4 up the pad's side: the ball rests on the round all along, the
    // variable blend rolled on the round as the constant one is; alone and meeting the pad
    // top's blend, walls drafted or not
    for (draft, x) in [(false, -107.69), (true, -104.0)] {
        for which in ["one", "all"] {
            valid(&rolling(draft, x, which, "vary(2, 4)"));
        }
    }
}

/// `b` with every list stored the other way round: vertices, edges, faces, each face's loops, and
/// each loop starting at its middle coedge.
fn reversed_storage(b: &forge_topo::Body) -> forge_topo::Body {
    let b = b.compact();
    let mut out = forge_topo::Body::new();
    let mut vmap = vec![None; b.vertices.len()];
    for v in b.vertex_ids().collect::<Vec<_>>().into_iter().rev() {
        let x = b.vertex(v);
        vmap[v.index()] = Some(out.add_tolerant_vertex(x.point, x.tol, x.name));
    }
    let mut emap = vec![None; b.edges.len()];
    for e in b.edge_ids().collect::<Vec<_>>().into_iter().rev() {
        let x = b.edge(e);
        let (s, t) = (vmap[x.start.index()].unwrap(), vmap[x.end.index()].unwrap());
        emap[e.index()] = Some(out.add_edge(x.curve.clone(), x.range, s, t, x.name));
    }
    let mut smap = vec![None; b.shells.len()];
    for f in b.face_ids().collect::<Vec<_>>().into_iter().rev() {
        let face = b.face(f);
        let shell = *smap[face.shell.index()].get_or_insert_with(|| out.add_shell());
        let nf = out.add_face(shell, face.surface.clone(), face.same_sense, face.name);
        for &l in face.loops.iter().rev() {
            let cs: Vec<_> = b.lp(l).coedges().collect();
            let k = cs.len() / 2;
            let uses: Vec<_> = cs[k..]
                .iter()
                .chain(&cs[..k])
                .map(|&c| (emap[b.coedge(c).edge.index()].unwrap(), b.coedge(c).forward))
                .collect();
            out.add_loop(nf, &uses);
        }
    }
    out
}

#[test]
fn fillet_does_not_follow_storage_order() {
    // a gearbox foot on a drafted wall: the same body stored the other way round must round to
    // the same faces and vertices (the blends' surfaces once took their frames from the order an
    // edge's two coedges were stored in, which seq and n-ary builds store differently), and to the
    // same corner patches with the edges listed the other way round too (their sides once started
    // at the corner of the edge listed or stored first)
    let (part, ids) = history(
        r#"
sketch foot(on: xy) {
  r = rect(corner: (-120, -65), size: (240, 130))
}
box = extrude(foot, 95)
dr = draft(box.side(foot.r[0]), angle: -2deg, neutral: offset(foot.plane, 95))
sketch fs(on: foot.plane) {
  path o(start: (-96, -55)) {
    line(to: (-96, -85))
    tarc(to: (-88, -93))
    line(to: (-60, -93))
    tarc(to: (-52, -85))
    line(to: (-52, -55))
    close()
  }
}
pad = extrude(fs.o, 16)
root = fillet(edges(between: (pad, box), crease: "concave"), 5)
"#,
    );
    let i = part.features.len() - 1;
    let forge_model::Feature::Fillet { edges, .. } = &part.features[i] else {
        panic!("a fillet");
    };
    // the part as the kernel had it before the fillet
    let h = forge_model::History::new(&part.features, Some(&ids));
    let mut stage = None;
    for k in 0..i {
        stage = forge_model::step(h, k, stage).expect("builds");
    }
    let base = stage.expect("built").compact();
    let shape = |b: &forge_topo::Body, flip: bool| {
        let mut fe: Vec<_> = edges
            .iter()
            .map(|(n, r)| forge_ops::FilletEdge {
                edge: b.edge_by_name(*n).expect("edge"),
                radius: r.clone(),
            })
            .collect();
        if flip {
            fe.reverse();
        }
        let out = forge_ops::fillet(b, &fe, 99).expect("rounds");
        let mut faces: Vec<String> = out
            .face_ids()
            // a zero's sign may follow the order a cross product took its factors in
            .map(|f| {
                format!("{:?}", out.face(f).surface)
                    .replace("-0.0,", "0.0,")
                    .replace("-0.0 ", "0.0 ")
            })
            .collect();
        faces.sort();
        let mut verts: Vec<[i64; 3]> = out
            .vertex_ids()
            .map(|v| out.vertex(v).point)
            .map(|p| [p.x, p.y, p.z].map(|x| (x * 1e9).round() as i64))
            .collect();
        verts.sort();
        (faces, verts)
    };
    assert!(shape(&base, false) == shape(&reversed_storage(&base), true));
}

/// A plate's top edge rounded where a through hole's rim is tangent to the round's spring on the
/// top (CADFS 00092585 restated). The ball's feet stay in both faces all along, so by the local
/// rolling ball it rounds: the top face is left pinched where the rim touches the spring.
fn spring_touching_a_hole(t: f64, r: f64, d: f64) -> String {
    format!(
        r#"
shared L = 40
shared T = {t}
shared R = {r}
shared D = {d}
param round.r = R
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (L, L))
}}
plate = extrude(plan, T)
sketch spot(on: plate.end) {{
  c = circle((L - R - D / 2, L / 2), D / 2)
}}
drill = cut(spot.c, thru)
round = fillet(edges(between: (plate.end, plate.side(plan.outline[1]))), r)
"#
    )
}

#[test]
fn fillet_spring_tangent_to_a_hole_rim() {
    // the probe of the spring's side was at its middle, the very point the rim touches
    valid(&spring_touching_a_hole(8.0, 4.0, 6.0));
    // as thick as the radius: the side face goes whole too
    valid(&spring_touching_a_hole(5.0, 5.0, 5.0));
}
