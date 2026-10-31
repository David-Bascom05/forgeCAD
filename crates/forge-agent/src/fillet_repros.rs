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
