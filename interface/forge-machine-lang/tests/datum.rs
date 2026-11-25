//! Drawn = checked at the part mount: on every built-in machine, the part mount's datum (its `at`
//! along its `dir`) is the top of the real part the work rests on, exactly, so a part is drawn
//! resting on the table where the simulation and the collision check put it (CAM-SIM: the VF-2's
//! table was drawn 1 mm under its datum). The envelope is the part's bounds grown by a margin,
//! except on that face. Every built-in's mount is checked; the excused are named below with why.

/// Horizontal machining centres whose record draws the pallet as a vertical plate (its face at
/// y 0, normal +y) while the part mount sits at the plate's centre along +z with a -z spindle: the
/// datum is inside the pallet. A record-modelling fault (the HMC's work axis), not a drawing
/// offset: left for the records' owner, listed so it is neither hidden nor counted as fine.
const HMC_PALLET_DATUM_INSIDE: [&str; 17] = [
    "brother-speedio-h550xd1",
    "dmg-mori-nhx-4000",
    "dmg-mori-nhx-5000",
    "dn-solutions-nhp-4000",
    "dn-solutions-nhp-5000",
    "dn-solutions-nhp-6300",
    "dn-solutions-nhp-6300-2nd",
    "hyundai-wia-hs4000-ii",
    "hyundai-wia-hs5000-ii",
    "kitamura-mycenter-hx400ig",
    "makino-a51nx",
    "makino-a61nx",
    "mazak-hcn-5000",
    "mazak-hcn-4000",
    "mazak-hcn-6000",
    "okuma-ma-500hii",
    "okuma-ma-600hiii",
];
use forge_machine::def::MountKind;
use std::path::Path;

#[test]
fn the_part_mount_datum_is_the_real_tables_top() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-machine");
    let mut bad = Vec::new();
    let mut seen = 0;
    for (m, _) in forge_machine::library::builtin_files() {
        let parts = forge_machine_lang::check_parts(&m, &dir, 0.01).unwrap();
        for mt in m.mounts.iter().filter(|x| x.kind == MountKind::Part) {
            // the bodies riding the mount's link: the work rests on the highest along `dir`
            let on: Vec<_> = m
                .bodies
                .iter()
                .filter(|b| b.link == mt.link)
                .filter_map(|b| parts.iter().find(|c| c.body == b.name))
                .collect();
            if on.is_empty() {
                continue;
            }
            seen += 1;
            let along = |p: [f64; 3]| p[0] * mt.dir[0] + p[1] * mt.dir[1] + p[2] * mt.dir[2];
            // the top of a box's bounds along an axis-aligned dir: the hi or lo corner's
            let (top, part) = on
                .iter()
                .map(|c| (along(c.hi).max(along(c.lo)), c.part.as_str()))
                .fold((f64::NEG_INFINITY, ""), |a, b| if b.0 > a.0 { b } else { a });
            let datum = along(mt.at);
            let excused = HMC_PALLET_DATUM_INSIDE.contains(&m.name.as_str()) && mt.name == "pallet";
            if excused {
                // still inside: if a record is fixed, take it off the list
                assert!(
                    top - datum > 1.0,
                    "{}: fixed, take it off HMC_PALLET_DATUM_INSIDE",
                    m.name
                );
                continue;
            }
            if (top - datum).abs() > 1e-6 {
                bad.push(format!(
                    "{} mount `{}`: datum {datum} along {:?}, the real part's top {top} ({:+.3} mm) {part}",
                    m.name,
                    mt.name,
                    mt.dir,
                    top - datum
                ));
            }
        }
    }
    assert!(seen > 50, "{seen} mounts checked");
    assert!(
        bad.is_empty(),
        "{} mounts off their part:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
