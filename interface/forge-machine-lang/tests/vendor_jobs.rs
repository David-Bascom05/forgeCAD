//! One sample job simulated per architecture the vendor library adds: the posted program drives the
//! joints, the tip lands where the words say in the part frame, and the machine, tool, stock and
//! fixture stay clear (apart from the cutting edge in the stock).
use forge_machine::collide::{self, Role, Verdict};
use forge_machine::def::{MachineDef, Prim};
use forge_machine::gcode::Control;
use forge_machine::kin;
use forge_machine::sim::{self, Cut, Setup, SimTool};
use std::path::Path;

fn machine(vendor: &str, name: &str) -> MachineDef {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../kernel/forge-machine/data/machines")
        .join(format!("{vendor}.toml"));
    let src = std::fs::read_to_string(&p).unwrap();
    MachineDef::parse_all(&src)
        .unwrap()
        .into_iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("no {name} in {}", p.display()))
}

fn end_mill(t: u32, l: f64, r: f64) -> SimTool {
    SimTool {
        t,
        name: format!("flat {}", 2.0 * r),
        tip: [0.0, 0.0, -l],
        body: vec![(r, 0.0, 20.0), (r + 5.0, 20.0, l)],
        cut: Cut::Mill { r, corner: 0.0 },
    }
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6)
}

/// The machine's and tool's bodies against each other, the stock and the fixture: nothing but the
/// cutting edge may touch the stock.
fn assert_clear(s: &Setup, tl: &sim::Timeline, stock: Prim, fixture: Prim) {
    let work = [
        collide::part_body(s, "stock", &[stock], Role::Stock),
        collide::part_body(s, "fixture", &[fixture], Role::Fixture),
    ];
    let r = collide::check(s, tl, &work);
    let bad: Vec<_> = r
        .contacts
        .iter()
        .filter(|c| !matches!(c.verdict, Verdict::Clear))
        .filter(|c| !(c.a.ends_with("cutter") && c.b == "stock" || c.b.ends_with("cutter") && c.a == "stock"))
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
}

/// A/C trunnion (A about X carries C): a 40 mm block on a 60 mm riser (so the 140 mm table stays
/// below the work when tilted), its top faced at A = 0, then tilted A90 so its +Y face looks up at
/// the spindle and that face cut 1 mm deep.
#[test]
fn ac_trunnion_3_plus_2_on_brother_m200x3() {
    let m = machine("brother", "brother-speedio-m200x3");
    let c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    assert_eq!(c.letters, vec!['X', 'Y', 'Z', 'A', 'C']);
    let gauge = m.mount("spindle").unwrap().at[2];
    let pivot = m.joints.iter().find(|j| j.name == "A").unwrap().pivot[2];
    let mut s = Setup::new(&m, c).unwrap();
    // Z word 0 on the table top: tip z = gauge + q_Z - H and q_Z = word + wcs + H
    s.wcs = vec![(54, vec![0.0, 0.0, -gauge, 0.0, 0.0])];
    s.tools = vec![end_mill(1, 120.0, 5.0)];
    s.h = vec![(1, 120.0)];
    // the +Y face (y = 20) after A90 sits at y = pivot - 20 ... worked out from the pivot
    let fy = -(80.0 - pivot); // after +90 about +x: (y, z) -> (-z, y) about the pivot
    let fz = pivot + 20.0;
    let prog = format!(
        "G90 G17 G21\nT1 M06\nG54\nG43 H1\nS8000 M03\nG0 X-30. Y0 A0 C0\nZ105.\nZ99.5\nG1 X30. F600.\nG0 Z160.\nG0 A90.\nG0 X0 Y{fy:.3} Z{:.3}\nG1 Z{:.3} F300.\nG0 Z120.\nM05\nM30\n",
        fz + 5.0,
        fz - 1.0
    );
    let tl = sim::simulate(&prog, &s).unwrap();
    assert!(tl.violations.is_empty(), "{:?}", tl.violations);
    let last_cut = tl
        .segments
        .iter()
        .rev()
        .find(|g| g.kind == sim::SegKind::Feed)
        .unwrap();
    let (tip, _) = kin::tip_in_part(&m, &last_cut.q1, "spindle", "table", 120.0).unwrap();
    assert!(close(tip, [0.0, 19.0, 80.0]), "{tip:?}");
    assert_clear(
        &s,
        &tl,
        Prim::Box {
            min: [-20.0, -20.0, 60.0],
            max: [20.0, 20.0, 100.0],
        },
        Prim::Box {
            min: [-15.0, -15.0, 0.0],
            max: [15.0, 15.0, 60.0],
        },
    );
}

/// HMC (horizontal spindle along -z, B under the pallet): a 100 x 200 x 100 block on the pallet, its +Z face
/// (toward the spindle) faced at B0, then B90 turns its -X face to the spindle and that face is cut
/// 1 mm deep.
#[test]
fn hmc_index_and_face_on_makino_a51nx() {
    let m = machine("makino", "makino-a51nx");
    let c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    assert_eq!(c.letters, vec!['X', 'Y', 'Z', 'B']);
    let nose = m.mount("spindle").unwrap().at;
    let mut s = Setup::new(&m, c).unwrap();
    // Y word 0 on the pallet top (the spindle's centre is nose[1] above it at Y = 0); Z word 0 at
    // the pallet centre (tip z = nose[2] + q_Z - H, q_Z = word + wcs + H)
    s.wcs = vec![(54, vec![0.0, -nose[1], -nose[2], 0.0])];
    s.tools = vec![end_mill(1, 250.0, 5.0)];
    s.h = vec![(1, 250.0)];
    let prog = "G90 G17 G21\nT1 M06\nG54\nG43 H1\nS8000 M03\nG0 B0\nG0 X-70. Y120.\nZ60.\nZ49.5\nG1 X70. F1000.\nG0 Z300.\nB90.\nG0 X0 Y120.\nZ60.\nG1 Z49. F300.\nG0 Z300.\nM05\nM30\n";
    // a 250 mm tool: the spindle housing (radius 120, estimate) stays off the 400 mm pallet
    let tl = sim::simulate(prog, &s).unwrap();
    assert!(tl.violations.is_empty(), "{:?}", tl.violations);
    let last_cut = tl
        .segments
        .iter()
        .rev()
        .find(|g| g.kind == sim::SegKind::Feed)
        .unwrap();
    let (tip, _) = kin::tip_in_part(&m, &last_cut.q1, "spindle", "pallet", 250.0).unwrap();
    // B90 about +y takes the part's -X face to +z: the tip 1 mm inside x = -50, at the face centre
    assert!(close(tip, [-49.0, 120.0, 0.0]), "{tip:?}");
    assert_clear(
        &s,
        &tl,
        Prim::Box {
            min: [-50.0, 0.0, -50.0],
            max: [50.0, 200.0, 50.0],
        },
        Prim::Box {
            min: [-50.0, 0.0, -70.0],
            max: [50.0, 20.0, -55.0],
        },
    );
}
