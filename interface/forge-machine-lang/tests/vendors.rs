//! Every vendor machine file (kernel/forge-machine/data/machines/*.toml) parses and validates, every
//! body's part builds and lies in its collision envelope, and the machine's own bodies stay clear at
//! home and with each joint at each end of its travel.
use forge_machine::collide;
use forge_machine::def::MachineDef;
use std::path::Path;

fn vendor_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-machine/data/machines");
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return out;
    };
    for e in rd {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "toml") {
            out.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
        }
    }
    out.sort();
    out
}

#[test]
fn vendor_machines_parse_build_and_stay_clear() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-machine");
    let mut n = 0;
    for (file, src) in vendor_files() {
        let ms = MachineDef::parse_all(&src).unwrap_or_else(|e| panic!("{file}: {e}"));
        for m in ms {
            m.validate().unwrap_or_else(|e| panic!("{file} {}: {e}", m.name));
            assert!(m.source.starts_with("http"), "{}: source must be a URL", m.name);
            assert!(
                m.bodies.iter().all(|b| b.part.is_some()),
                "{}: a body without its part",
                m.name
            );
            for c in forge_machine_lang::check_parts(&m, &root, 0.01).unwrap() {
                assert!(
                    c.outside.is_empty(),
                    "{} {}: {} vertices outside",
                    m.name,
                    c.body,
                    c.outside.len()
                );
                assert!(
                    c.volume > 0.0 && c.volume <= c.envelope * (1.0 + 1e-9),
                    "{} {}",
                    m.name,
                    c.body
                );
            }
            let home: Vec<f64> = m.joints.iter().map(|j| j.home).collect();
            let c = collide::check_pose(&m, &home);
            assert!(c.is_empty(), "{} at home: {c:?}", m.name);
            for j in 0..m.joints.len() {
                for end in 0..2 {
                    let mut q = home.clone();
                    q[j] = m.joints[j].range[end];
                    let c = collide::check_pose(&m, &q);
                    assert!(
                        c.is_empty(),
                        "{} with {} at {}: {c:?}",
                        m.name,
                        m.joints[j].name,
                        q[j]
                    );
                }
            }
            n += 1;
        }
    }
    eprintln!("{n} vendor machines checked");
}
