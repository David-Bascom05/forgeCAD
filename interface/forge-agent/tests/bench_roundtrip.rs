//! Every real-part benchmark out to a STEP file and back: nothing refused, each solid valid and of
//! the volume and area it went out with. A part of ours its own reader will not take back is how the
//! budget on a file's surfaces was first found too small.
use forge_agent::bench;

fn round_trip(name: &str) {
    let solids = bench::part(name)
        .regenerate_solids()
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    for s in &solids {
        let out = forge_topo::mass::mass(&s.body).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let e = forge_step::export(&s.body, &forge_step::Options::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let back = forge_step::import(e.text.as_bytes(), &forge_step::Limits::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(back.refusals.is_empty(), "{name}: {:?}", back.refusals);
        let (mut v, mut a) = (0.0, 0.0);
        for b in &back.solids {
            let p = forge_topo::check(&b.body);
            assert!(p.is_empty(), "{name}: {p:?}");
            // as the reader measured it, to a millionth, a thousand times finer than the check
            // below (to a billionth, a file's splines took most of a minute a part)
            let m = match &b.mass {
                Some(m) => m.clone(),
                None => forge_topo::mass::mass_rel(&b.body, 1e-6).unwrap_or_else(|e| panic!("{name}: {e:?}")),
            };
            (v, a) = (v + m.volume, a + m.area);
        }
        assert!(
            (v - out.volume).abs() <= 1e-3 * out.volume,
            "{name}: volume {v} went out {}",
            out.volume
        );
        assert!(
            (a - out.area).abs() <= 1e-3 * out.area,
            "{name}: area {a} went out {}",
            out.area
        );
        // what the file says of itself holds
        assert!(back.stats.curve_off_surface <= e.uncertainty, "{name}");
    }
}

#[test]
fn bracket() {
    round_trip("bracket");
}

#[test]
fn clamp() {
    round_trip("clamp");
}

#[test]
fn conrod() {
    round_trip("conrod");
}

#[test]
fn crank() {
    round_trip("crank");
}

#[test]
fn frame() {
    round_trip("frame");
}

#[test]
fn gearbox() {
    round_trip("gearbox");
}

#[test]
fn guard() {
    round_trip("guard");
}

#[test]
fn heatsink() {
    round_trip("heatsink");
}

#[test]
fn hub() {
    round_trip("hub");
}

#[test]
fn insert() {
    round_trip("insert");
}

#[test]
fn jack() {
    round_trip("jack");
}

#[test]
fn manifold() {
    round_trip("manifold");
}

#[test]
fn quadarm() {
    round_trip("quadarm");
}

#[test]
fn valve() {
    round_trip("valve");
}

#[test]
fn volute() {
    round_trip("volute");
}

/// A budget that runs out anywhere is a refusal by name or a read, never a panic, on a real part.
#[test]
fn a_budget_run_out_refuses() {
    let solids = bench::part("crank").regenerate_solids().expect("builds");
    let e = forge_step::export(&solids[0].body, &forge_step::Options::default()).expect("writes");
    for k in [4, 8, 12, 16, 20] {
        let lim = forge_step::Limits {
            max_evals: 1 << k,
            ..forge_step::Limits::default()
        };
        let got = forge_step::import(e.text.as_bytes(), &lim).expect("reads");
        assert!(
            !got.solids.is_empty() || got.refusals[0].why.contains("too long"),
            "{k}: {:?}",
            got.refusals
        );
    }
}
