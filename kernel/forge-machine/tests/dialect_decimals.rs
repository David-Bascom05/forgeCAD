//! A mill or lathe control's dialect: decimals and feed_decimals are whole numbers from 0 to
//! forge_post::MAX_DECIMALS (6), so the simulator's 10^k scaling is exact; more is refused, with
//! the limit named.
use forge_machine::gcode::Control;

fn dialect(decimals: &str, feed_decimals: &str) -> String {
    format!(
        "[dialect]\nname = \"t\"\ndecimals = {decimals}\nfeed_decimals = {feed_decimals}\ndwell = \"G04 P{{ms}}\"\n"
    )
}

#[test]
fn a_dialect_of_up_to_six_decimals_is_taken() {
    let m = forge_machine::library::builtin_named("haas-st-10").unwrap();
    for (d, f) in [("0", "0"), ("6", "6"), ("3", "1")] {
        let c = Control::for_machine(&m, &dialect(d, f), 1).unwrap();
        assert_eq!(
            (c.decimals, c.feed_decimals),
            (d.parse().unwrap(), f.parse().unwrap())
        );
    }
}

#[test]
fn more_than_six_decimals_or_a_fraction_or_a_negative_are_refused_naming_the_limit() {
    let m = forge_machine::library::builtin_named("haas-st-10").unwrap();
    for bad in ["7", "30", "2.5", "-1"] {
        for (src, key) in [
            (dialect(bad, "1"), "decimals"),
            (dialect("3", bad), "feed_decimals"),
        ] {
            let why = Control::for_machine(&m, &src, 1)
                .err()
                .unwrap_or_else(|| panic!("{key} = {bad} taken"));
            assert!(
                why.contains(key) && why.contains("from 0 to 6"),
                "{key} = {bad}: {why}"
            );
        }
    }
}
