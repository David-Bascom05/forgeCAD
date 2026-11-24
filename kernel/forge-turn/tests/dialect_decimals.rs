//! A lathe dialect's decimals are whole numbers from 0 to forge_post::MAX_DECIMALS (6): 10^k is
//! then exact in f64, so a word's scaling is the same bits on every target. More is refused, with
//! the limit named.
use forge_turn::post::{DIALECTS, Dialect, PostError};

fn with(key: &str, value: &str) -> Result<Dialect, PostError> {
    let src = DIALECTS[0].1;
    let line = src
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{key} =")))
        .expect("the key in the dialect");
    Dialect::parse(&src.replacen(line, &format!("{key} = {value}"), 1))
}

#[test]
fn up_to_six_decimals_are_taken() {
    for key in ["decimals", "feed_decimals", "pitch_decimals"] {
        for k in ["0", "6"] {
            assert!(with(key, k).is_ok(), "{key} = {k}");
        }
    }
    assert_eq!(with("pitch_decimals", "6").unwrap().pitch_decimals, 6);
}

#[test]
fn more_than_six_decimals_or_a_fraction_or_a_negative_are_refused_naming_the_limit() {
    for key in ["decimals", "feed_decimals", "pitch_decimals"] {
        for k in ["7", "23", "2.5", "-1"] {
            match with(key, k) {
                Err(PostError::Dialect(why)) => {
                    assert!(
                        why.contains(key) && why.contains("from 0 to 6"),
                        "{key} = {k}: {why}"
                    )
                }
                r => panic!("{key} = {k}: {r:?}"),
            }
        }
    }
}
