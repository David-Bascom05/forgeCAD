//! The G-code interpreter's core (L4 of research/PROOF-STANDARD.md): one block of parsed words,
//! in integer units, against the control's modal state. It is verify/forge-machine's `step`
//! (ForgeMachine/Interp.lean), written for Aeneas (research/aeneas/STYLE.md): no floats, no
//! closures, no iterators, no `break` and no `return` inside a loop, no `Ordering`, no struct
//! holding a borrow. kernel/forge-machine's gcode.rs is the shell: it reads the text into words,
//! handles the words that do not move the axes (tools, spindle, coolant, offsets, G53, dwell) and
//! calls `step` for the rest.
//!
//! A refusal is `Err` with its reason; the spec's `none`.

#![forbid(unsafe_code)]
// The Aeneas style (research/aeneas/STYLE.md) writes some things the long way.
#![allow(clippy::vec_init_then_push, clippy::redundant_pattern_matching, clippy::len_zero, clippy::manual_unwrap_or_default, clippy::manual_unwrap_or)]

/// Which axis (index into a position) is X, Y and Z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axes {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

/// An arc's centre: absolute plane coordinates (from I J K, which are incremental from the start),
/// or the radius word (negative for more than half a turn).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Centre {
    Abs(i64, i64),
    R(i64),
}

/// The words of one block that the core reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The motion code: 0 1 2 3, 80, or a cycle 81 82 83 84 85 73.
    pub g: Option<u32>,
    /// G90 (true) or G91 (false).
    pub absolute: Option<bool>,
    /// G98 (true) or G99 (false).
    pub retract_initial: Option<bool>,
    /// G17 G18 G19.
    pub plane: Option<u32>,
    /// G28.
    pub home: bool,
    /// The axis words, one per axis.
    pub ax: Vec<Option<i64>>,
    pub i: Option<i64>,
    pub j: Option<i64>,
    pub k: Option<i64>,
    pub r: Option<i64>,
    pub q: Option<i64>,
    /// A cycle's dwell P, milliseconds.
    pub p: Option<i64>,
    pub f: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cyc {
    pub g: u32,
    pub z: i64,
    pub r: i64,
    pub q: i64,
    pub dwell: i64,
    /// The height the cycle started from (G98's return level).
    pub initial: i64,
}

/// The modal state between blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub pos: Vec<Option<i64>>,
    pub absolute: bool,
    /// 0 1 2 3, or 80 while a cycle is active.
    pub mode: Option<u32>,
    pub plane: u32,
    pub feed: Option<i64>,
    pub retract_initial: bool,
    pub cyc: Option<Cyc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Rapid(Vec<Option<i64>>),
    Feed(Vec<Option<i64>>, i64),
    /// To, the plane (17 18 19), the centre, clockwise, a whole turn, the feed.
    Arc(Vec<Option<i64>>, u32, Centre, bool, bool, i64),
    /// G28: the intermediate point, then these axes to their reference.
    Home(Vec<Option<i64>>, Vec<usize>),
    /// A cycle's dwell, milliseconds.
    Dwell(i64),
}

/// Why a block is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refuse {
    /// An incremental word for an axis whose position is unknown.
    IncrementalFromUnknown,
    /// G28 with no axis word.
    HomeNoAxis,
    /// A motion code the core does not know.
    UnknownMotion,
    CycleNotG17,
    CycleNoZ,
    CycleNoR,
    CycleIncremental,
    /// The height a cycle starts from is unknown.
    CycleUnknownHeight,
    /// A cycle needs bottom Z ≤ R ≤ the initial height.
    CycleOrder,
    CycleNegativePeck,
    /// A Z word in a cycle's repeat block (it would change the bottom).
    CycleZRepeat,
    /// A hole's X, Y or the feed unknown.
    HoleUnknown,
    /// A feed move or arc with no feed yet.
    NoFeed,
    /// An arc from or to an unknown position.
    ArcUnknown,
    ArcR0,
    /// A whole turn written with R.
    ArcFullR,
    ArcRAndCentre,
    ArcNoCentre,
    /// Axis words with no motion mode.
    NoMotion,
}

/// A copy of a position (index loop: Aeneas has no model of `Vec::clone`'s element clones).
pub fn copy(p: &[Option<i64>]) -> Vec<Option<i64>> {
    let mut out: Vec<Option<i64>> = Vec::with_capacity(p.len());
    let mut k = 0;
    while k < p.len() {
        let v = p[k];
        out.push(v);
        k += 1;
    }
    out
}

/// A list of one event.
pub fn one(e: Ev) -> Vec<Ev> {
    let mut v = Vec::new();
    v.push(e);
    v
}

/// One axis's target, and whether it is an incremental word from an unknown value.
pub fn target_one(absolute: bool, p: Option<i64>, w: Option<i64>) -> (Option<i64>, bool) {
    match w {
        None => (p, false),
        Some(d) => {
            if absolute {
                (Some(d), false)
            } else {
                match p {
                    Some(v) => (Some(v + d), false),
                    None => (None, true),
                }
            }
        }
    }
}

/// The target of a block's axis words: absolute words replace; incremental ones add to a known
/// value.
pub fn target(absolute: bool, pos: &[Option<i64>], ax: &[Option<i64>]) -> Result<Vec<Option<i64>>, Refuse> {
    let mut out: Vec<Option<i64>> = Vec::with_capacity(pos.len());
    let mut bad = false;
    let mut k = 0;
    while k < pos.len() {
        let w = if k < ax.len() { ax[k] } else { None };
        let pk = pos[k];
        let (v, unknown) = target_one(absolute, pk, w);
        if unknown {
            bad = true;
        }
        out.push(v);
        k += 1;
    }
    if bad {
        Err(Refuse::IncrementalFromUnknown)
    } else {
        Ok(out)
    }
}

/// `p` with X, Y and Z set.
pub fn put(a: &Axes, p: &[Option<i64>], x: i64, y: i64, h: i64) -> Vec<Option<i64>> {
    let mut out: Vec<Option<i64>> = Vec::with_capacity(p.len());
    let mut k = 0;
    while k < p.len() {
        let v = if k == a.z {
            Some(h)
        } else if k == a.y {
            Some(y)
        } else if k == a.x {
            Some(x)
        } else {
            p[k]
        };
        out.push(v);
        k += 1;
    }
    out
}

/// G2 or G3.
pub fn is_arc(g: u32) -> bool {
    g == 2 || g == 3
}

/// A canned-cycle motion code: 81 82 83 84 85 73.
pub fn is_cycle(g: u32) -> bool {
    g == 81 || g == 82 || g == 83 || g == 84 || g == 85 || g == 73
}

/// A pecking cycle: 83 (back to R each peck) or 73 (chip break).
pub fn is_peck(g: u32) -> bool {
    g == 83 || g == 73
}

/// A cycle that feeds back out to R: 85 (bore) and 84 (tap).
pub fn feeds_out(g: u32) -> bool {
    g == 85 || g == 84
}

/// One hole of a cycle at (x, y) from the current height `cur`: to (x, y) at `cur`, to R, the cut,
/// back to `back`. `pc` and `lift` are the peck clearance and chip-break lift in integer units.
#[allow(clippy::too_many_arguments)]
pub fn hole(
    a: &Axes,
    base: &[Option<i64>],
    c: &Cyc,
    cur: i64,
    back: i64,
    pc: i64,
    lift: i64,
    f: i64,
    x: i64,
    y: i64,
    out: &mut Vec<Ev>,
) {
    out.push(Ev::Rapid(put(a, base, x, y, cur)));
    out.push(Ev::Rapid(put(a, base, x, y, c.r)));
    if is_peck(c.g) {
        let step = if c.q > 0 { c.q } else { c.r - c.z };
        if step > 0 {
            let mut depth = c.r;
            while depth > c.z {
                let next = if depth - step > c.z { depth - step } else { c.z };
                out.push(Ev::Feed(put(a, base, x, y, next), f));
                depth = next;
                if depth > c.z {
                    if c.g == 83 {
                        out.push(Ev::Rapid(put(a, base, x, y, c.r)));
                        out.push(Ev::Rapid(put(a, base, x, y, depth + pc)));
                    } else {
                        out.push(Ev::Rapid(put(a, base, x, y, depth + lift)));
                    }
                }
            }
        }
    } else {
        out.push(Ev::Feed(put(a, base, x, y, c.z), f));
        if c.g == 82 && c.dwell > 0 {
            out.push(Ev::Dwell(c.dwell));
        }
        if feeds_out(c.g) {
            out.push(Ev::Feed(put(a, base, x, y, c.r), f));
        }
    }
    out.push(Ev::Rapid(put(a, base, x, y, back)));
}

/// A word's value, or 0 without one.
fn or0(w: Option<i64>) -> i64 {
    match w {
        Some(v) => v,
        None => 0,
    }
}

/// A modal word, or the state's value without one.
fn or_word(w: Option<i64>, d: Option<i64>) -> Option<i64> {
    match w {
        Some(v) => Some(v),
        None => d,
    }
}

/// A modal flag, or the state's value without one.
fn or_bool(w: Option<bool>, d: bool) -> bool {
    match w {
        Some(v) => v,
        None => d,
    }
}

/// A modal plane, or the state's plane without one.
fn or_plane(w: Option<u32>, d: u32) -> u32 {
    match w {
        Some(v) => v,
        None => d,
    }
}

/// Whether a word is there.
fn has(w: Option<i64>) -> bool {
    match w {
        Some(_) => true,
        None => false,
    }
}

/// Whether a motion code is there.
fn has_g(g: Option<u32>) -> bool {
    match g {
        Some(_) => true,
        None => false,
    }
}

/// `any` or a word there.
fn any_word(any: bool, w: Option<i64>) -> bool {
    match w {
        Some(_) => true,
        None => any,
    }
}

fn word(ax: &[Option<i64>], k: usize) -> Option<i64> {
    if k < ax.len() { ax[k] } else { None }
}

fn at(p: &[Option<i64>], k: usize) -> Option<i64> {
    if k < p.len() { p[k] } else { None }
}

/// The motion-code group: the state after it, before any move.
fn motion_code(a: &Axes, st: &State, b: &Block, s1: State) -> Result<State, Refuse> {
    match b.g {
        None => Ok(s1),
        Some(g) => {
            if g <= 3 {
                Ok(State {
                    mode: Some(g),
                    cyc: None,
                    ..s1
                })
            } else if g == 80 {
                Ok(State {
                    mode: None,
                    cyc: None,
                    ..s1
                })
            } else if is_cycle(g) {
                if s1.plane != 17 {
                    Err(Refuse::CycleNotG17)
                } else {
                    match (word(&b.ax, a.z), b.r, at(&st.pos, a.z)) {
                        (None, _, _) => Err(Refuse::CycleNoZ),
                        (_, None, _) => Err(Refuse::CycleNoR),
                        (_, _, None) => Err(Refuse::CycleUnknownHeight),
                        (Some(z), Some(r), Some(init)) => {
                            let q = or0(b.q);
                            let dwell = or0(b.p);
                            if !s1.absolute {
                                Err(Refuse::CycleIncremental)
                            } else if z > r || r > init {
                                Err(Refuse::CycleOrder)
                            } else if q < 0 {
                                Err(Refuse::CycleNegativePeck)
                            } else {
                                Ok(State {
                                    mode: Some(80),
                                    cyc: Some(Cyc {
                                        g,
                                        z,
                                        r,
                                        q,
                                        dwell,
                                        initial: init,
                                    }),
                                    ..s1
                                })
                            }
                        }
                    }
                }
            } else {
                Err(Refuse::UnknownMotion)
            }
        }
    }
}

/// The plane's two axes and its centre words: G17 X Y (I J), G18 Z X (K I), G19 Y Z (J K).
fn plane_axes(a: &Axes, plane: u32, b: &Block) -> (usize, usize, Option<i64>, Option<i64>) {
    if plane == 17 {
        (a.x, a.y, b.i, b.j)
    } else if plane == 18 {
        (a.z, a.x, b.k, b.i)
    } else {
        (a.y, a.z, b.j, b.k)
    }
}

fn arc(
    a: &Axes,
    st: &State,
    b: &Block,
    s2: State,
    dst: Vec<Option<i64>>,
    g: u32,
) -> Result<(Vec<Ev>, State), Refuse> {
    let (pa, pb, wa, wb) = plane_axes(a, s2.plane, b);
    match (
        at(&st.pos, pa),
        at(&st.pos, pb),
        at(&dst, pa),
        at(&dst, pb),
        s2.feed,
    ) {
        (Some(sa), Some(sb), Some(ea), Some(eb), Some(f)) => {
            let full = ea == sa && eb == sb;
            let centre = match (b.r, wa, wb) {
                (Some(r), None, None) => {
                    if r == 0 {
                        Err(Refuse::ArcR0)
                    } else if full {
                        Err(Refuse::ArcFullR)
                    } else {
                        Ok(Centre::R(r))
                    }
                }
                (Some(_), _, _) => Err(Refuse::ArcRAndCentre),
                (None, None, None) => Err(Refuse::ArcNoCentre),
                (None, ia, ib) => {
                    let i = or0(ia);
                    let j = or0(ib);
                    Ok(Centre::Abs(sa + i, sb + j))
                }
            }?;
            let ev = Ev::Arc(copy(&dst), s2.plane, centre, g == 2, full, f);
            Ok((one(ev), State { pos: dst, ..s2 }))
        }
        (Some(_), Some(_), Some(_), Some(_), None) => Err(Refuse::NoFeed),
        _ => Err(Refuse::ArcUnknown),
    }
}

/// One block: its events and the state after it.
pub fn step(a: &Axes, pc: i64, lift: i64, st: &State, b: &Block) -> Result<(Vec<Ev>, State), Refuse> {
    let feed = or_word(b.f, st.feed);
    let absolute = or_bool(b.absolute, st.absolute);
    let ri = or_bool(b.retract_initial, st.retract_initial);
    let plane = or_plane(b.plane, st.plane);
    let s1 = State {
        pos: copy(&st.pos),
        absolute,
        mode: st.mode,
        plane,
        feed,
        retract_initial: ri,
        cyc: st.cyc,
    };
    let mut any = false;
    let mut k = 0;
    while k < b.ax.len() {
        any = any_word(any, b.ax[k]);
        k += 1;
    }
    if b.home {
        let via = target(absolute, &st.pos, &b.ax)?;
        let mut axes: Vec<usize> = Vec::new();
        let mut pos: Vec<Option<i64>> = Vec::with_capacity(via.len());
        let mut k = 0;
        while k < via.len() {
            if has(word(&b.ax, k)) {
                axes.push(k);
                pos.push(None);
            } else {
                pos.push(via[k]);
            }
            k += 1;
        }
        if axes.len() == 0 {
            return Err(Refuse::HomeNoAxis);
        }
        return Ok((one(Ev::Home(via, axes)), State { pos, ..s1 }));
    }
    let s2 = motion_code(a, st, b, s1)?;
    if !any {
        return Ok((Vec::new(), s2));
    }
    let dst = target(absolute, &st.pos, &b.ax)?;
    match (s2.mode, s2.cyc) {
        (Some(80), Some(c)) => {
            if has(word(&b.ax, a.z)) && !has_g(b.g) {
                return Err(Refuse::CycleZRepeat);
            }
            match (at(&dst, a.x), at(&dst, a.y), at(&st.pos, a.z), feed) {
                (Some(x), Some(y), Some(cur), Some(f)) => {
                    let back = if ri { c.initial } else { c.r };
                    let mut out = Vec::new();
                    hole(a, &dst, &c, cur, back, pc, lift, f, x, y, &mut out);
                    let pos = put(a, &dst, x, y, back);
                    Ok((out, State { pos, ..s2 }))
                }
                _ => Err(Refuse::HoleUnknown),
            }
        }
        (Some(0), _) => Ok((one(Ev::Rapid(copy(&dst))), State { pos: dst, ..s2 })),
        (Some(1), _) => match feed {
            Some(f) => Ok((one(Ev::Feed(copy(&dst), f)), State { pos: dst, ..s2 })),
            None => Err(Refuse::NoFeed),
        },
        (Some(g), _) => {
            if is_arc(g) {
                arc(a, st, b, s2, dst, g)
            } else {
                Err(Refuse::NoMotion)
            }
        }
        (None, _) => Err(Refuse::NoMotion),
    }
}
