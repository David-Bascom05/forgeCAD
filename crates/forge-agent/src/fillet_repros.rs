//! Fillets on real parts that once failed: each builds valid, sequentially and n-ary.

use crate::model::Model;
use forge_model::Part;

fn valid(src: &str) {
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
    }
}

#[test]
fn shell_after_a_variable_fillet() {
    // the blend's run-outs past its ends are tighter than the shell is thick, the face is not;
    // its offset springs meet the offset top and sides tangentially
    valid("extrude 14 rect 0 -15 170 30\nfillet 4..3 (edges top line along +x)\nshell 1.6 (faces bottom)\n");
}

#[test]
fn round_a_tapered_arm_with_its_tip() {
    // the sides and the tip arc are one tangent chain round the tip, so it comes back beside the
    // root corner it leaves: its middle is halfway along it, not across
    valid(
        "extrude 14 loop line 0 -15 169.905858 -15.999723 arc 170 0 16 -90.337123 180.674246 line 169.905858 15.999723 0 15 line 0 15 0 -15\nfillet 2 (edges top)\n",
    );
}

#[test]
fn block_flush_on_a_disc() {
    // a conrod bolt boss: the end faces split by the union join before the blends run out over
    // them. Radius 3 would touch the boss's side 3.04 above its top (the ball centre sits at
    // z = sqrt(34^2 - 23^2) = 25.04 > 22): that refuses, and says so
    let part = |r: &str| {
        format!(
            "big: extrude sym 22 xz circle 0 0 31\nb: extrude sym 22 xz rect 26 -24 16 46\nfillet {r} (edges between b big concave)\n"
        )
    };
    valid(&part("1.5"));
    let m = Model::new(&part("3"));
    let msg = m.error.map(|f| f.msg).unwrap_or_default();
    assert!(msg.contains("Overflow"), "{msg}");
}

#[test]
fn variable_fillet_round_a_tube_into_a_cone() {
    // the joint's loop is two traced edges; their end tangents at the seam differed by 3e-6 rad,
    // so the chain was left open and the blends met at the seam with different sections
    valid(
        "reach = 385\nstack = 575\nhta = 73 deg\nsta = 73.5 deg\nhtl = 160\nstl = 540\nhdx = cos(hta)\nhdz = -sin(hta)\ns1x = -(stl-35)*cos(sta)\ns1z = (stl-35)*sin(sta)\nh1x = reach + 25*hdx\nh1z = stack + 25*hdz\nttl = sqrt((h1x-s1x)^2 + (h1z-s1z)^2)\nht: revolve xz at reach 0 stack axis 0 0 hdx hdz poly 0 0 25*sin(hta) 25*cos(hta) htl*hdx+30*sin(hta) -htl*sin(hta)+30*cos(hta) htl*hdx -htl*sin(hta)\ntt: extrude ttl z (h1x-s1x)/ttl 0 (h1z-s1z)/ttl at s1x 0 s1z circle 0 0 14.3\nfillet 4..2.5 (edges between tt ht concave)\n",
    );
}

#[test]
fn concave_pad_fillet_ending_on_a_drafted_tube() {
    // the blend's spine is a hyperbola (a plane cut through the offset cone): its nearest-point
    // search over a corner's wide range sampled past where it could be near
    valid(
        "tube: extrude 60 circle 0 5 14\ndraft -8 xy at 0 0 0 (faces cyl)\npad: extrude 5 yz at 8 -40 10 rect 0 0 80 40\nfillet 2 (edges between pad tube concave)\n",
    );
}

#[test]
fn face_fillet_and_full_round_after_a_union() {
    // the union leaves dead entities: the strip compacts the body, and the faces it was given are
    // found again after compacting, not read by their old ids
    valid(
        "base: extrude 10 rect -40 -30 80 60\nup: extrude 50 xy at 0 0 10 rect -40 -30 12 60\nfacefillet 6 (faces of up normal +x) to (faces of base normal +z)\n",
    );
    valid(
        "base: extrude 10 rect -40 -30 80 60\nrib: extrude 40 xy at 0 0 10 rect -30 -4 60 8\nfullround (faces of rib normal -y) (faces of rib normal +z) (faces of rib normal +y)\n",
    );
}

#[test]
fn tab_root_fillet_on_a_thin_wall() {
    // the run-out on the cylinder sits a radius above the tab's top, past the end of its 1.5 high
    // side edge: the edge is carried on as far as that, not only half its own length
    valid(
        "ring: extrude 50 circle 0 0 69.5 circle 0 0 68\ntab: extrude 1.5 rect -88 -10 19.25 20\nfillet 1 (edges between tab ring concave circle)\n",
    );
}

#[test]
fn second_pad_fillet() {
    // pad b's top lies on pad a's top plane but far from it: it is b's face, so the first line
    // rounds only pad a, and the second finds pad b's edges unrounded
    let src = "box: extrude 95 rect -120 -65 240 130\na: extrude 16 rect -96 -93 44 34\nb: extrude 16 rect 52 -93 44 34\nfillet 5 (edges between a box concave)\nfillet 5 (edges between b box concave)\n";
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let first = m.select("(edges between a box concave)", Some(3)).unwrap().1;
    assert_eq!(first.len(), 3);
    // n-ary names pad b's edges apart from seq until the n-ary naming fix (R30) lands
    assert!(forge_topo::check(m.body().unwrap()).is_empty());
}

#[test]
fn mirrored_cuts_after_a_variable_fillet() {
    // a pattern's copies go through one n-ary boolean, whose rays classify the stay bore's pieces:
    // one grazed the seat tube by the blend and met the blend's balls past its arc, a point not
    // on the face that counted as a crossing
    valid(
        "reach = 385\nstack = 575\nhta = 73 deg\nsta = 73.5 deg\nstl = 540\ncsl = 410\ndrop = 70\nsp = 130\nhdx = cos(hta)\nhdz = -sin(hta)\nax = -sqrt(csl^2 - drop^2)\naz = drop\ns1x = -(stl-45)*cos(sta)\ns1z = (stl-45)*sin(sta)\nh1x = reach + 25*hdx\nh1z = stack + 25*hdz\nttl = sqrt((h1x-s1x)^2 + (h1z-s1z)^2)\nc0x = -17.5\nc0y = 22\nc1x = ax + 6\nc1y = sp/2 - 4\ncsx = c1x - c0x\ncsy = c1y - c0y\ncsz = az\ncl = sqrt(csx^2 + csy^2 + csz^2)\nq0x = -(stl-80)*cos(sta)\nq0z = (stl-80)*sin(sta)\nq0y = 10\nq1x = ax + 10\nq1z = az + 14\nq1y = sp/2 - 4\nssx = q1x - q0x\nssy = q1y - q0y\nssz = q1z - q0z\nsl = sqrt(ssx^2 + ssy^2 + ssz^2)\nst: extrude stl z -cos(sta) 0 sin(sta) circle 0 0 15.9\ntt: extrude ttl z (h1x-s1x)/ttl 0 (h1z-s1z)/ttl at s1x 0 s1z circle 0 0 14.3\nfillet 0:3,0.5:4.5,1:3 (edges between tt st concave)\ncsb: loft cut profile z csx/cl csy/cl csz/cl at c0x+12*csx/cl c0y+12*csy/cl 12*csz/cl circle 0 0 9.9 profile z csx/cl csy/cl csz/cl at c1x c1y az circle 0 0 5.5\nssb: loft cut profile z ssx/sl ssy/sl ssz/sl at q0x q0y q0z circle 0 0 7 profile z ssx/sl ssy/sl ssz/sl at q1x q1y q1z circle 0 0 5\npattern csb ssb mirror xz\n",
    );
}

#[test]
fn law_direction_does_not_follow_names() {
    // other ids give the edges and vertices other names; the laws start where the geometry says
    let points = |src: &str| {
        let m = Model::new(src);
        assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
        let b = m.body().unwrap();
        let mut ps: Vec<[i64; 3]> = b
            .vertex_ids()
            .map(|v| b.vertex(v).point)
            .map(|p| [p.x, p.y, p.z].map(|x| (x * 1e6).round() as i64))
            .collect();
        ps.sort();
        ps
    };
    for law in ["1..3", "0:1,0.4:2,1:1.5"] {
        let src = |id: &str| {
            format!(
                "base: {id}extrude 8 rect 0 0 20 10\nfillet {law} (edges front top) (edges back bottom)\n"
            )
        };
        let a = points(&src(""));
        for id in ["id 7 ", "id 123456 "] {
            assert_eq!(a, points(&src(id)), "{law} {id}");
        }
    }
}

#[test]
fn ring_round_wider_than_its_ring_is_refused() {
    // a conrod (the I-beam shank, pocketed) whose small end, 20 thick and of radius 15, is rounded
    // over its rim: the full round's ball (radius 10) rolls round a centre circle of radius 5,
    // which works only up to (5 + 10) / 2
    let m = Model::new(
        "c = 150\nrb = 31\nrs = 15\ntb = 22\nts = 20\nside: sketch xz\n  line -14 rb-6 -8 c-rs+5\n  line l1.b 8 c-rs+5\n  line l2.b 14 rb-6\n  line l3.b l1.a\n  line o 0 c cons\n  ver l5\n  sym l1.a l3.b l5\n  sym l1.b l3.a l5\n  vdist o l4.a rb-6\n  vdist o l2.a c-rs+5\n  dist l4 28\n  dist l2 16\nend\nshank: extrude sym ts side\nbig: extrude sym tb xz circle 0 0 rb\nsmall: extrude sym ts xz circle 0 c rs\nbolts: extrude sym tb xz rect 26 -24 16 46\npattern bolts mirror yz\nweb: sketch xz at 0 ts/2 0\n  line -9.5 rb+2 -4.5 c-rs-4\n  line l1.b 4.5 c-rs-4\n  line l2.b 9.5 rb+2\n  line l3.b l1.a\nend\npocket: extrude cut 7.5 web\npattern pocket mirror xz\nfillet 6 (edges between shank big concave line) (edges between shank small concave)\nfullround (faces normal +y near 0 10 165) (faces cyl near 0 0 165) (faces normal -y near 0 -10 165)\n",
    );
    let msg = m.error.map(|f| f.msg).unwrap_or_default();
    assert!(msg.contains("radius under 7.5"), "{msg}");
}

#[test]
fn fillet_rolling_onto_a_corner_round() {
    // a pad 0.31 past the wall's tangent line onto a corner round: the r 3 ball between the pad's
    // side and the wall rests on the round, never the wall, so the blend is rolled on the round and
    // the wall's 0.31 strip goes. Its section, the region between the pad side, the ball and the
    // round, is 1.463135 mm^2 (the ball centre at x = -110.69, 15 from the round's axis), over 16
    let part = "box: extrude 95 rect -120 -65 240 130\nfillet 12 (edges of box shorter 100 near -120 -65 50)\nextrude 16 rect -107.69 -93 44 40\n";
    let src = format!("{part}fillet 3 (edges concave near -107.69 -65 8)\n");
    valid(&src);
    let q = forge_mesh::Quality {
        chord: 1e-3,
        max_angle: 0.2,
    };
    let volume = |s: &str| {
        let b = Model::new(s).body().expect("built").clone();
        let fine = forge_mesh::tessellate(&b, &q).expect("meshes").volume();
        let coarse = forge_mesh::tessellate(&b, &forge_mesh::Quality { chord: 4e-3, ..q })
            .expect("meshes")
            .volume();
        fine + (fine - coarse) / 3.0
    };
    let added = volume(&src) - volume(part);
    assert!((added - 23.41016).abs() < 1e-2, "{added}");
    // drafted walls: the round is a cone, the wall a drafted plane
    valid(
        "box: extrude 95 rect -120 -65 240 130\ndraft -2 xy at 0 0 95 (faces of box normal -x) (faces of box normal -y)\nfillet 12 (edges of box shorter 100 near -120 -65 50)\nextrude 16 rect -104 -93 44 40\nfillet 3 (edges concave near -104 -65 8)\n",
    );
}

#[test]
fn fillet_rolling_onto_a_corner_round_meets_the_pad_top_blend() {
    // the full R2 repro: every concave edge, so the blend resting on the round meets the pad
    // top's blend at the pad's top corner. The corner sees the round between the blend and the
    // wall, the smooth edge between them cut where the pad top's blend meets it
    for (draft, x) in [
        ("", -107.69),
        (
            "draft -2 xy at 0 0 95 (faces of box normal -x) (faces of box normal -y)\n",
            -104.0,
        ),
    ] {
        valid(&format!(
            "box: extrude 95 rect -120 -65 240 130\n{draft}fillet 12 (edges of box shorter 100 near -120 -65 50)\nextrude 16 rect {x} -93 44 40\nfillet 3 (edges concave)\n"
        ));
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
        format!(
            "box: extrude 95 rect -120 -65 240 130\nfillet 12 (edges of box shorter 100 near -120 -65 50)\npad: extrude 16 rect {x} -93 44 40\ndraft {draft} xy at 0 0 0 (faces of pad normal -x)\nfillet 3 (edges concave{which})\n"
        )
    };
    for (x, draft) in [(-104.0, -10), (-106.0, 10)] {
        for which in [" near -105 -65 8", ""] {
            valid(&part(x, draft, which));
        }
    }
    let b = Model::new(&part(-104.0, -10, " near -105 -65 8"))
        .body()
        .expect("built")
        .clone();
    let walls = b
        .faces
        .iter()
        .filter(|f| {
            f.alive
                && matches!(&f.surface, forge_geom::Surface::Plane(p)
                    if p.frame.z().get().y.abs() > 0.99 && (p.frame.origin.y + 65.0).abs() < 1e-9)
        })
        .count();
    // the wall and the sliver
    assert_eq!(walls, 2);
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
    // edge's two coedges were stored in, which seq and n-ary builds store differently). Corner
    // patches are left out: their sides' order still follows storage (corner.rs)
    let m = Model::new(
        "box: extrude 95 rect -120 -65 240 130\ndraft -2 xy at 0 0 95 (faces of box normal -y)\nfoot: extrude 16 loop line -96 -55 -96 -85 arc -88 -85 8 180 90 line -88 -93 -60 -93 arc -60 -85 8 270 90 line -52 -85 -52 -55 line -52 -55 -96 -55\nfillet 5 (edges between foot box concave)\n",
    );
    assert!(m.error.is_none(), "{:?}", m.error.map(|f| f.msg));
    let i = m.features.len() - 1;
    let forge_model::Feature::Fillet { edges, .. } = &m.features[i] else {
        panic!("a fillet");
    };
    let base = m.body_after(i).expect("built").compact();
    let shape = |b: &forge_topo::Body| {
        let fe: Vec<_> = edges
            .iter()
            .map(|(n, r)| forge_ops::FilletEdge {
                edge: b.edge_by_name(*n).expect("edge"),
                radius: r.clone(),
            })
            .collect();
        let out = forge_ops::fillet(b, &fe, 99).expect("rounds");
        let mut faces: Vec<String> = out
            .face_ids()
            .map(|f| format!("{:?}", out.face(f).surface))
            .filter(|s| !s.contains("HolePatch"))
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
    assert!(shape(&base) == shape(&reversed_storage(&base)));
}
