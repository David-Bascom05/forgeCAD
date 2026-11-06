//! The same drawing in each version makes the same scene: as many items, the same extents.

use forge_dwg::scene::{BoxFont, Options, Prim, Space, build};

#[test]
fn every_version_the_same_scene() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut seen = vec![];
    for v in ["2000", "2004", "2007", "2010", "2013", "2018"] {
        let path = dir.join(format!("mix_{v}.dwg"));
        let d = forge_dwg::file::read(&std::fs::read(&path).unwrap()).unwrap();
        let s = build(&d, Space::Model, &BoxFont, &Options::default());
        assert!(s.items.len() > 100, "{v}: {} items", s.items.len());
        let points: usize = s
            .items
            .iter()
            .map(|it| match &it.prim {
                Prim::Path { pts, .. } | Prim::Segments { pts } => pts.len(),
                Prim::Fill { rings } | Prim::Mask { rings } => rings.iter().map(Vec::len).sum(),
            })
            .sum();
        seen.push((v, s.items.len(), points, s.extents.unwrap()));
    }
    let (_, n, p, (lo, hi)) = seen[0];
    for (v, n2, p2, (lo2, hi2)) in &seen[1..] {
        assert_eq!(*n2, n, "{v}: item count");
        assert_eq!(*p2, p, "{v}: point count");
        for k in 0..2 {
            assert!(
                (lo2[k] - lo[k]).abs() < 1e-9 && (hi2[k] - hi[k]).abs() < 1e-9,
                "{v}: extents"
            );
        }
    }
}

#[test]
fn layouts_build() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let d = forge_dwg::file::read(&std::fs::read(dir.join("mix_2018.dwg")).unwrap()).unwrap();
    for (h, o) in &d.objects {
        if let forge_dwg::types::Data::Layout(_) = &o.data {
            // a layout with nothing on it is an empty scene, not a failure
            let s = build(&d, Space::Layout(*h), &BoxFont, &Options::default());
            assert_eq!(s.items.is_empty(), s.extents.is_none());
        }
    }
}

/// Every number of a scene, hashed: native and wasm builds must agree on it.
fn scene_hash(s: &forge_dwg::scene::Scene) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        for b in x.to_le_bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    };
    for it in &s.items {
        eat(it.entity as u64);
        eat(it.layer as u64);
        eat(u32::from_le_bytes([it.rgb[0], it.rgb[1], it.rgb[2], 0]) as u64);
        eat(it.weight.to_bits() as u64);
        let mut pts = |v: &[[f64; 2]]| {
            for p in v {
                eat(p[0].to_bits());
                eat(p[1].to_bits());
            }
        };
        match &it.prim {
            Prim::Path { pts: v, closed } => {
                pts(v);
                eat(*closed as u64);
            }
            Prim::Segments { pts: v } => pts(v),
            Prim::Fill { rings } | Prim::Mask { rings } => rings.iter().for_each(|r| pts(r)),
        }
    }
    h
}

#[test]
fn the_same_on_every_target() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let d = forge_dwg::file::read(&std::fs::read(dir.join("mix_2018.dwg")).unwrap()).unwrap();
    let s = build(&d, Space::Model, &BoxFont, &Options::default());
    assert_eq!(format!("{:016x}", scene_hash(&s)), "624fb32b956b5f8d");
}
