//! The same drawing (a part's drawing sheet, and lines, arcs, an ellipse, a spline, a polyline,
//! a point, a solid and text) as the ODA converter writes it in each version: every object decodes
//! and encodes back to its bits, and each file, written and read again, is the same drawing. The
//! written bytes' hash pins the writer: native and wasm builds must agree.

use forge_dwg::file;
use forge_dwg::object::{Classes, encode_object, read_object};
use forge_dwg::types::Data;

const VERSIONS: [&str; 6] = ["2000", "2004", "2007", "2010", "2013", "2018"];

fn load(v: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/data/mix_{v}.dwg", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The same length and the same first `n` bits (writers pad with whatever bits they had).
fn same_bits(a: &[u8], b: &[u8], n: usize) -> bool {
    a.len() == b.len() && (0..n).all(|k| (a[k / 8] ^ b[k / 8]) & (0x80 >> (k % 8)) == 0)
}

#[test]
fn every_object_decodes_and_encodes_back() {
    for v in VERSIONS {
        let buf = load(v);
        let s = file::sections(&buf).unwrap();
        let classes = file::r2000::classes(&s.classes, s.ver, s.maint).unwrap();
        let list: Vec<(String, bool)> = classes.iter().map(|c| (c.dxf.text(), c.item == 0x1F2)).collect();
        let cl = Classes { list: &list };
        let map = file::r2000::read_map(&s.handles, 0, s.handles.len()).unwrap();
        let mut decoded = 0;
        for (h, at) in map {
            let o = read_object(&s.objects, at, s.ver, &cl, None)
                .unwrap_or_else(|e| panic!("{v} {h:#x}: {}", e.0));
            if matches!(o.data, Data::Raw(_)) {
                continue;
            }
            let mut fresh = o.clone();
            fresh.raw = None;
            let (b, n) = encode_object(&fresh, s.ver).unwrap();
            assert!(
                same_bits(&b, o.raw.as_ref().unwrap(), n),
                "{v} {h:#x} re-encodes differently"
            );
            decoded += 1;
        }
        assert!(decoded > 150, "{v}: {decoded} decoded");
    }
}

#[test]
fn files_write_and_read_back() {
    for v in VERSIONS {
        let d = file::read(&load(v)).unwrap();
        assert!(d.warnings.is_empty(), "{v}: {:?}", d.warnings);
        let out = match file::write(&d) {
            Ok(b) => b,
            Err(e) if v == "2007" && e.0.contains("not written yet") => continue,
            Err(e) => panic!("{v}: {}", e.0),
        };
        let e = file::read(&out).unwrap();
        assert_eq!(d.header, e.header, "{v} header");
        assert_eq!(d.classes, e.classes, "{v} classes");
        assert_eq!(d.objects.len(), e.objects.len(), "{v}");
        for ((h, a), (_, b)) in d.objects.iter().zip(&e.objects) {
            assert!(
                a.data == b.data && a.ent == b.ent && a.handle == b.handle,
                "{v} {h:#x}"
            );
        }
    }
}

/// FNV-1a.
fn hash(b: &[u8]) -> u64 {
    b.iter().fold(0xCBF2_9CE4_8422_2325, |h, &x| {
        (h ^ x as u64).wrapping_mul(0x0100_0000_01B3)
    })
}

#[test]
fn written_bytes_are_the_same_on_every_target() {
    let mut all = vec![];
    for v in VERSIONS {
        let d = file::read(&load(v)).unwrap();
        if let Ok(out) = file::write(&d) {
            all.push(format!("{v}:{:016x}", hash(&out)));
        }
    }
    let got = all.join(" ");
    let want = include_str!("data/written.hashes").trim();
    if std::env::var("DWG_BLESS").is_ok() {
        std::fs::write(
            format!("{}/tests/data/written.hashes", env!("CARGO_MANIFEST_DIR")),
            &got,
        )
        .unwrap();
        return;
    }
    assert_eq!(got, want);
}

#[test]
fn edits_write_and_undo() {
    use forge_dwg::edit::History;
    use forge_dwg::types::{Circle, Line};
    for v in VERSIONS {
        let orig = file::read(&load(v)).unwrap();
        let mut d = orig.clone();
        let mut hist = History::default();
        hist.begin();
        let layer = d.add_layer(&mut hist, "FORGE_EDIT", 1).unwrap();
        let ms = d.model_space();
        let l = d
            .add_entity(
                &mut hist,
                ms,
                layer,
                Data::Line(Line {
                    end: [10.0, 20.0, 0.0],
                    ..Line::default()
                }),
            )
            .unwrap();
        d.add_entity(
            &mut hist,
            ms,
            layer,
            Data::Circle(Circle {
                radius: 3.0,
                ..Circle::default()
            }),
        )
        .unwrap();
        let victim = *d
            .objects
            .iter()
            .find(|(h, o)| **h != l && matches!(o.data, Data::Line(_)) && d.block_of(**h) == Some(ms))
            .unwrap()
            .0;
        d.erase(&mut hist, victim).unwrap();
        hist.end();
        let lines = |d: &file::Drawing| {
            d.objects
                .values()
                .filter(|o| matches!(o.data, Data::Line(_)))
                .count()
        };
        match file::write(&d) {
            Ok(out) => {
                let e = file::read(&out).unwrap();
                assert_eq!(lines(&e), lines(&orig), "{v}: one line in, one out");
                assert!(e.layer("forge_edit").is_some(), "{v}");
                assert!(!e.objects.contains_key(&victim), "{v}");
                // the space lists exactly its entities
                if let Some(Data::BlockHeader(bh)) = e.objects.get(&ms).map(|o| &o.data)
                    && !bh.owned.is_empty()
                {
                    assert!(bh.owned.iter().all(|h| e.objects.contains_key(&h.handle)), "{v}");
                }
            }
            Err(x) if v == "2007" && x.0.contains("not written yet") => {}
            Err(x) => panic!("{v}: {}", x.0),
        }
        assert!(hist.undo(&mut d));
        assert!(
            d.objects.keys().eq(orig.objects.keys()) && d.header == orig.header,
            "{v} undo"
        );
        assert!(hist.redo(&mut d));
        assert!(
            d.objects.contains_key(&l) && !d.objects.contains_key(&victim),
            "{v} redo"
        );
    }
}
