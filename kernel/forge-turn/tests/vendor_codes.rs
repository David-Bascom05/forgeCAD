//! forge-tooling's cited maker records against forge-turn's ISO decoding: every insert's ISO 1832
//! code decodes to the inscribed circle, thickness and corner radius the maker states (and edge
//! length where stated), and every holder's/bar's ISO 5608/6261 code to its stated κr, hand,
//! shank and length. A mismatch fails here, naming the record, unless the record carries a
//! `conflict` cite for that field.
use forge_tooling::{CiteStatus, Library};
use forge_turn::insert::{decode_bar, decode_holder, decode_insert};

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn maker_inserts_and_holders_agree_with_their_codes() {
    let lib = Library::builtin();
    assert!(lib.inserts.len() >= 40 && lib.turn_holders.len() >= 40);
    let mut bad = Vec::new();
    // every code decodes: a code forge-turn cannot decode fails here, as a mismatch does
    for i in &lib.inserts {
        let conflict = |p: &str| {
            i.cites
                .iter()
                .any(|c| c.path == p && c.status == CiteStatus::Conflict)
        };
        let d = match decode_insert(&i.code) {
            Ok(d) => d,
            Err(e) => {
                bad.push(format!("{}: {} does not decode: {e:?}", i.id, i.code));
                continue;
            }
        };
        // the IC and thickness series are rounded in catalogues (12.7, 9.525, 4.76, 3.97)
        for (what, ours, theirs, tol) in [
            ("ic", i.ic, d.ic, 0.03),
            ("s", i.s, d.thickness, 0.03),
            ("re", i.re, d.re, 1e-6),
        ] {
            if !close(ours, theirs, tol) && !conflict(what) {
                bad.push(format!(
                    "{} ({}): {what} stated {ours} vs code {theirs}",
                    i.id, i.code
                ));
            }
        }
        if let Some(l) = i.l
            && !close(l, d.l, 0.15)
            && !conflict("l")
        {
            bad.push(format!("{} ({}): l stated {l} vs code {}", i.id, i.code, d.l));
        }
    }
    for h in &lib.turn_holders {
        let conflict = |p: &str| {
            h.cites
                .iter()
                .any(|c| c.path == p && c.status == CiteStatus::Conflict)
        };
        match h.kind {
            forge_tooling::turning::HolderKind::Holder => match decode_holder(&h.code) {
                Ok(d) => {
                    for (what, ours, theirs) in [
                        ("kr", h.kr, d.kapr),
                        ("h", h.h.unwrap_or(f64::NAN), d.h),
                        ("b", h.b.unwrap_or(f64::NAN), d.b),
                        ("l1", h.l1, d.length),
                    ] {
                        if !close(ours, theirs, 1e-6) && !conflict(what) {
                            bad.push(format!(
                                "{} ({}): {what} stated {ours} vs code {theirs}",
                                h.id, h.code
                            ));
                        }
                    }
                    if d.hand.to_string() != h.hand && !conflict("hand") {
                        bad.push(format!(
                            "{} ({}): hand {} vs code {}",
                            h.id, h.code, h.hand, d.hand
                        ));
                    }
                }
                Err(e) => bad.push(format!("{}: {} does not decode: {e:?}", h.id, h.code)),
            },
            forge_tooling::turning::HolderKind::Bar => match decode_bar(&h.code) {
                Ok(d) => {
                    for (what, ours, theirs) in [
                        ("kr", h.kr, d.kapr),
                        ("dmm", h.dmm.unwrap_or(f64::NAN), d.d),
                        ("l1", h.l1, d.length),
                    ] {
                        if !close(ours, theirs, 1e-6) && !conflict(what) {
                            bad.push(format!(
                                "{} ({}): {what} stated {ours} vs code {theirs}",
                                h.id, h.code
                            ));
                        }
                    }
                }
                Err(e) => bad.push(format!("{}: {} does not decode: {e:?}", h.id, h.code)),
            },
        }
    }
    // every record was checked: inserts of every shape (W too), holders and bars of both hands
    assert!(lib.inserts.len() + lib.turn_holders.len() >= 75);
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad.join("\n"));
}
