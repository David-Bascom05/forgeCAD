pub fn guarded(x: i32) -> i32 {
    // a comment with a { brace
    let s = "a } string";
    if x > 0 { x } else { s.len() as i32 }
}

#[test]
fn samples() {}

#[test]
#[ignore]
fn ignored_mutation() {}

fn not_a_test() {}
