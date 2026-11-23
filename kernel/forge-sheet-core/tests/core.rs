//! The core against brute force: classes as connected components, chains as walks, owners.
use forge_math::rng::Rng;
use forge_sheet_core::{ChainError, chain, classes, owner};

fn components(n: usize, pairs: &[(u32, u32)]) -> Vec<usize> {
    // the least member of each point's component, by repeated relaxation
    let mut least: Vec<usize> = (0..n).collect();
    loop {
        let mut changed = false;
        for &(a, b) in pairs {
            let m = least[a as usize].min(least[b as usize]);
            for x in [a as usize, b as usize] {
                if least[x] != m {
                    least[x] = m;
                    changed = true;
                }
            }
        }
        if !changed {
            return least;
        }
    }
}

#[test]
fn classes_are_components_numbered_by_least_member() {
    let mut rng = Rng::new(7);
    for _ in 0..500 {
        let n = 1 + rng.below(40);
        let pairs: Vec<(u32, u32)> = (0..rng.below(2 * n))
            .map(|_| (rng.below(n) as u32, rng.below(n) as u32))
            .collect();
        let c = classes(n, &pairs);
        let least = components(n, &pairs);
        let mut roots: Vec<usize> = least.clone();
        roots.sort();
        roots.dedup();
        for i in 0..n {
            assert_eq!(c[i] as usize, roots.iter().position(|&r| r == least[i]).unwrap());
        }
    }
}

#[test]
fn chains() {
    // two squares
    let from = [0, 1, 2, 3, 4, 5, 6, 7];
    let to = [1, 2, 3, 0, 5, 6, 7, 4];
    let l = chain(8, &from, &to).unwrap();
    assert_eq!(l.starts, vec![0, 4, 8]);
    assert_eq!(l.pieces, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    // a vertex left twice and reached once
    assert!(matches!(
        chain(3, &[0, 0, 1], &[1, 2, 0]),
        Err(ChainError::Unchained { vertex: 0, .. })
    ));
    // a figure eight: vertex 0 twice in and out
    match chain(3, &[0, 1, 0, 2], &[1, 0, 2, 0]) {
        Err(ChainError::Pinched { pinches }) => assert_eq!(pinches, vec![(0, 1, 3)]),
        r => panic!("{r:?}"),
    }
}

#[test]
fn owners() {
    // 0 holds 1 holds the hole: the owner is 1
    let holds = [true, true];
    let nest = [false, false, true, false]; // nest[o*2+p]: p holds o's vertex
    assert_eq!(owner(2, &holds, &nest), Some(1));
    assert_eq!(owner(2, &[false, false], &nest), None);
}

#[test]
fn place_then_inverse() {
    use forge_sheet_core::place::{Place, apply, inverse, then};
    let (s, c) = forge_math::dmath::sin_cos(0.7);
    let a = Place {
        ox: 3.0,
        oy: -2.0,
        xx: c,
        xy: s,
    };
    let b = Place {
        ox: -1.5,
        oy: 4.25,
        xx: 0.6,
        xy: 0.8,
    };
    let ab = then(a, b);
    // (a ∘ b)(p) = a(b(p))
    let p = (1.25, -0.5);
    let (bx, by) = apply(b, p.0, p.1);
    let (x1, y1) = apply(a, bx, by);
    let (x2, y2) = apply(ab, p.0, p.1);
    assert!((x1 - x2).abs() < 1e-12 && (y1 - y2).abs() < 1e-12);
    // a⁻¹ ∘ a = id
    let id = then(inverse(a), a);
    assert!(id.ox.abs() < 1e-12 && id.oy.abs() < 1e-12 && (id.xx - 1.0).abs() < 1e-12 && id.xy.abs() < 1e-12);
}
