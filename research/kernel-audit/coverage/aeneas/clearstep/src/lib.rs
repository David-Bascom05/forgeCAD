//! `interior_point` and `clear_step` (kernel/forge-bool/src/boolean.rs:6150-6207 at 888fc195),
//! made float-free: every float test is passed in as a table, so what is left is exactly the
//! kernel's control flow.
//!
//! A half-edge is `(piece, fwd)`; a region's loops are `loops`. The tables:
//!
//! * `small[p]` = `half_chord(&pieces[p]) < 0.01` (6155, 6159: the kernel's `find` keeps the first
//!   half-edge of the first loop whose chord is `>= 0.01`, the negation, since a chord is a
//!   non-NaN float, being at least 1e-3 by `half_chord`'s `max`).
//! * `nsteps[p]`: the number of steps `clear_step` tries off a half-edge of piece `p` (6186-6201).
//!   The step starts at `s0 = min(0.01 * chord, 0.05)` and is halved while `>= 2 LINEAR`; halving
//!   a normal float is exact, so the steps tried are exactly `s0 * 2^-k` for `k < nsteps[p]`, and
//!   their number depends only on the piece's chord (the chord passed is `half_chord` of the
//!   half-edge's own piece both at 6163, where `chord` is that of the chosen `h`, and at 6169).
//! * `probed[p][f][k]`: `probe_at(body, s.from, (p, f), pieces, Some(s0 * 2^-k))` is `Some`
//!   (6189; its `None` goes through both `?`, making `interior_point` return `None`).
//! * `clear[p][f][k][o]`: piece `o` is at least that step from that probe (6190-6196, the circle
//!   case `||`-ed in). The kernel skips `o == h.0` by piece index, so a piece used twice in a
//!   region (a spur: both directions) is skipped both times; `clear_all` keeps that.
//! * `fallback_ok[p][f]`: `probe(body, s.from, (p, f), pieces)` is `Some` (6174).
//!
//! `interior_point` returns `NoPoint` (the kernel's `None`), `Step(p, f, k)` (the probe `k` steps
//! down off half-edge `(p, f)`), or `Fallback(p, f)` (the default-step probe off the chosen `h`).
//! The tables are argued, not proven, to decide the kernel's tests; every float computation
//! upstream of a test is in the table.

pub enum Step {
    Fail,
    Found(usize),
    Exhausted,
}

pub enum Res {
    NoPoint,
    Step(usize, bool, usize),
    Fallback(usize, bool),
}

pub fn fi(f: bool) -> usize {
    if f {
        1
    } else {
        0
    }
}

/// 6190-6196: every piece of the region but the half-edge's own is clear of the probe.
pub fn clear_all(loops: &Vec<Vec<(usize, bool)>>, row: &Vec<bool>, h0: usize) -> bool {
    let mut ok = true;
    let mut i = 0;
    while i < loops.len() {
        let l = &loops[i];
        let mut j = 0;
        while j < l.len() {
            let o = l[j].0;
            if o != h0 && !row[o] {
                ok = false;
            }
            j += 1;
        }
        i += 1;
    }
    ok
}

/// 6179-6203: the first step, from the largest down, whose probe is clear of the region.
pub fn clear_step(
    loops: &Vec<Vec<(usize, bool)>>,
    nsteps: &Vec<usize>,
    probed: &Vec<Vec<Vec<bool>>>,
    clear: &Vec<Vec<Vec<Vec<bool>>>>,
    p: usize,
    f: bool,
) -> Step {
    let n = nsteps[p];
    let fx = fi(f);
    let mut res = Step::Exhausted;
    let mut stop = false;
    let mut k = 0;
    while k < n && !stop {
        if !probed[p][fx][k] {
            res = Step::Fail;
            stop = true;
        } else if clear_all(loops, &clear[p][fx][k], p) {
            res = Step::Found(k);
            stop = true;
        }
        k += 1;
    }
    res
}

/// 6153-6162: the first half-edge of the first loop, or the first of that loop whose chord is not
/// small when its own is.
pub fn pick(first: &Vec<(usize, bool)>, small: &Vec<bool>) -> (usize, bool) {
    let mut h = first[0];
    if small[h.0] {
        let mut found = false;
        let mut j = 0;
        while j < first.len() && !found {
            if !small[first[j].0] {
                h = first[j];
                found = true;
            }
            j += 1;
        }
    }
    h
}

/// 6166-6173 within one loop: `clear_step` off each half-edge but `h`, until one fails or finds.
pub fn scan_one(
    loops: &Vec<Vec<(usize, bool)>>,
    l: &Vec<(usize, bool)>,
    nsteps: &Vec<usize>,
    probed: &Vec<Vec<Vec<bool>>>,
    clear: &Vec<Vec<Vec<Vec<bool>>>>,
    h0: usize,
    h1: bool,
) -> Res2 {
    let mut out = Res2::Go;
    let mut j = 0;
    while j < l.len() && out.is_go() {
        let o = l[j];
        if o.0 != h0 || o.1 != h1 {
            out = match clear_step(loops, nsteps, probed, clear, o.0, o.1) {
                Step::Fail => Res2::Fail,
                Step::Found(k) => Res2::Found(o.0, o.1, k),
                Step::Exhausted => Res2::Go,
            };
        }
        j += 1;
    }
    out
}

/// 6166-6173: every loop in turn.
pub fn scan(
    loops: &Vec<Vec<(usize, bool)>>,
    nsteps: &Vec<usize>,
    probed: &Vec<Vec<Vec<bool>>>,
    clear: &Vec<Vec<Vec<Vec<bool>>>>,
    h0: usize,
    h1: bool,
) -> Res2 {
    let mut out = Res2::Go;
    let mut i = 0;
    while i < loops.len() && out.is_go() {
        out = scan_one(loops, &loops[i], nsteps, probed, clear, h0, h1);
        i += 1;
    }
    out
}

/// The second pass's outcome: go on, a probe failed (None), or a step found.
pub enum Res2 {
    Go,
    Fail,
    Found(usize, bool, usize),
}

impl Res2 {
    pub fn is_go(&self) -> bool {
        match self {
            Res2::Go => true,
            _ => false,
        }
    }
}

/// 6150-6175.
pub fn interior_point(
    loops: &Vec<Vec<(usize, bool)>>,
    small: &Vec<bool>,
    nsteps: &Vec<usize>,
    probed: &Vec<Vec<Vec<bool>>>,
    clear: &Vec<Vec<Vec<Vec<bool>>>>,
    fallback_ok: &Vec<Vec<bool>>,
) -> Res {
    if loops.len() == 0 {
        return Res::NoPoint;
    }
    if loops[0].len() == 0 {
        return Res::NoPoint;
    }
    let h = pick(&loops[0], small);
    match clear_step(loops, nsteps, probed, clear, h.0, h.1) {
        Step::Fail => Res::NoPoint,
        Step::Found(k) => Res::Step(h.0, h.1, k),
        Step::Exhausted => match scan(loops, nsteps, probed, clear, h.0, h.1) {
            Res2::Fail => Res::NoPoint,
            Res2::Found(p, f, k) => Res::Step(p, f, k),
            Res2::Go => {
                if fallback_ok[h.0][fi(h.1)] {
                    Res::Fallback(h.0, h.1)
                } else {
                    Res::NoPoint
                }
            }
        },
    }
}
