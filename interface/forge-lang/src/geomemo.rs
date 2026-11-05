//! The assembly checker's geometry answers (`forge_asm::check::Learned`: mesh distances and
//! narrow-phase tests, by mesh content and relative pose) kept on disk between runs, so a run
//! after an edit tests only the pairs whose meshes or poses it changed. Kept as batches, one a
//! run, merged into one once there are many.

use crate::store::{self, In, Out};
use forge_asm::check::{ExactFail, Learned, learned, teach};
use forge_math::Point3;

const KIND: &str = "geometry";
const MERGE_AT: usize = 32;

/// Loads what earlier runs worked out, once a process.
pub fn recall() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for (_, bytes) in store::all(KIND) {
            if let Some(l) = decode(&bytes) {
                teach(&l);
            }
        }
    });
}

/// Keeps what this run worked out since the last call.
pub fn keep() {
    let l = learned();
    if l.gaps.is_empty() && l.narrows.is_empty() {
        return;
    }
    let key = store::key(KIND, |h| {
        h.word(std::process::id() as u64);
        h.word(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64),
        );
        h.word(l.gaps.len() as u64);
    });
    store::put(KIND, key, &encode(&l));
    let all = store::all(KIND);
    if all.len() > MERGE_AT {
        let mut merged = Learned::default();
        for (_, b) in &all {
            if let Some(x) = decode(b) {
                merged.gaps.extend(x.gaps);
                merged.narrows.extend(x.narrows);
                merged.volumes.extend(x.volumes);
            }
        }
        merged.gaps.sort_by_key(|x| x.0);
        merged.gaps.dedup_by(|a, b| a.0 == b.0);
        merged.narrows.sort_by_key(|x| x.0);
        merged.narrows.dedup_by(|a, b| a.0 == b.0);
        merged.volumes.sort_by_key(|x| x.0);
        merged.volumes.dedup_by(|a, b| a.0 == b.0);
        let mkey = store::key(KIND, |h| h.str("merged"));
        store::put(KIND, mkey, &encode(&merged));
        for (k, _) in all {
            if k != mkey {
                store::forget(KIND, k);
            }
        }
    }
}

fn pose(o: &mut Out, p: &[u64; 9]) {
    for w in p {
        o.u64(*w);
    }
}

fn read_pose(i: &mut In) -> Option<[u64; 9]> {
    let mut p = [0u64; 9];
    for w in &mut p {
        *w = i.u64()?;
    }
    Some(p)
}

fn point(o: &mut Out, p: Point3) {
    o.f64(p.x);
    o.f64(p.y);
    o.f64(p.z);
}

fn read_point(i: &mut In) -> Option<Point3> {
    Some(Point3::new(i.f64()?, i.f64()?, i.f64()?))
}

fn encode(l: &Learned) -> Vec<u8> {
    let mut o = Out::default();
    o.u64(l.gaps.len() as u64);
    for ((a, b, p, lim), g) in &l.gaps {
        o.u128(*a);
        o.u128(*b);
        pose(&mut o, p);
        o.u64(*lim);
        o.bool(g.is_some());
        if let Some((d, p, q)) = g {
            o.f64(*d);
            point(&mut o, *p);
            point(&mut o, *q);
        }
    }
    o.u64(l.narrows.len() as u64);
    for ((a, b, p), n) in &l.narrows {
        o.u128(*a);
        o.u128(*b);
        pose(&mut o, p);
        o.0.push(*n);
    }
    o.u64(l.volumes.len() as u64);
    for ((a, b, p), v) in &l.volumes {
        o.u128(*a);
        o.u128(*b);
        pose(&mut o, p);
        match v {
            Ok(x) => {
                o.0.push(0);
                o.f64(*x);
            }
            Err(ExactFail::NoSolid) => o.0.push(1),
            Err(ExactFail::Boolean(why)) => {
                o.0.push(2);
                o.str(why);
            }
        }
    }
    o.0
}

fn decode(b: &[u8]) -> Option<Learned> {
    let mut i = In(b);
    let mut l = Learned::default();
    for _ in 0..i.count(113)? {
        let key = (i.u128()?, i.u128()?, read_pose(&mut i)?, i.u64()?);
        let g = if i.bool()? {
            Some((i.f64()?, read_point(&mut i)?, read_point(&mut i)?))
        } else {
            None
        };
        l.gaps.push((key, g));
    }
    for _ in 0..i.count(105)? {
        let key = (i.u128()?, i.u128()?, read_pose(&mut i)?);
        let n = *i.0.first()?;
        i.0 = &i.0[1..];
        l.narrows.push((key, n));
    }
    for _ in 0..i.count(105)? {
        let key = (i.u128()?, i.u128()?, read_pose(&mut i)?);
        let tag = *i.0.first()?;
        i.0 = &i.0[1..];
        let v = match tag {
            0 => Ok(i.f64()?),
            1 => Err(ExactFail::NoSolid),
            2 => Err(ExactFail::Boolean(i.str()?.to_string())),
            _ => return None,
        };
        l.volumes.push((key, v));
    }
    i.done().then_some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learned_answers_read_back_bit_for_bit() {
        let l = Learned {
            gaps: vec![
                (
                    (1, 2, [3; 9], 4),
                    Some((0.1, Point3::new(1.0, -0.0, 2.5), Point3::new(0.0, 1e-300, 3.0))),
                ),
                ((5, 6, [7; 9], 8), None),
            ],
            narrows: vec![((9, 10, [11; 9]), 2)],
            volumes: vec![
                ((1, 1, [0; 9]), Ok(2.5)),
                ((2, 2, [0; 9]), Err(ExactFail::NoSolid)),
                ((3, 3, [0; 9]), Err(ExactFail::Boolean("why".into()))),
            ],
        };
        assert_eq!(decode(&encode(&l)), Some(l.clone()));
        let e = encode(&l);
        assert_eq!(decode(&e[..e.len() - 1]), None);
    }
}
