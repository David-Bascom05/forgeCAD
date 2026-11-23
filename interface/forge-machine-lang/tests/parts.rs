//! Every built-in machine body's real part builds and lies in its collision envelope.
use std::path::Path;

#[test]
fn builtin_parts_lie_in_their_envelopes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-machine");
    let mut n = 0;
    for m in forge_machine::library::builtin() {
        assert!(
            m.bodies.iter().all(|b| b.part.is_some()),
            "{}: a body without its part",
            m.name
        );
        for c in forge_machine_lang::check_parts(&m, &dir, 0.01).unwrap() {
            assert!(
                c.outside.is_empty(),
                "{} {} ({}): {} of {} vertices outside, worst {:?}",
                m.name,
                c.body,
                c.part,
                c.outside.len(),
                c.vertices,
                c.outside.iter().map(|x| x.1).fold(0.0, f64::max)
            );
            assert!(
                c.volume > 0.0 && c.volume <= c.envelope * (1.0 + 1e-9),
                "{} {}: volume {} envelope {}",
                m.name,
                c.body,
                c.volume,
                c.envelope
            );
            // a real part fills a good part of its envelope (the VMC column is cored: 38%)
            assert!(
                c.volume >= 0.25 * c.envelope,
                "{} {}: volume {} of envelope {}",
                m.name,
                c.body,
                c.volume,
                c.envelope
            );
            n += 1;
        }
    }
    assert_eq!(n, 19);
}
