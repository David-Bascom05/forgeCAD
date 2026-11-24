//! Several channels at once (a Swiss lathe, a twin-turret lathe, a mill-turn's sub-spindle): each
//! channel's program on its own letters (Z2, C2 ...), synchronised at waits (`Ev::Wait`), and the
//! bodies of different channels checked against each other over every timing the bounds allow.
//!
//! Time is known only within bounds: a move takes at least its `t_min` and at most its exact-stop
//! `t_stop` (the trapezoid under the declared rates and accelerations); with a moving joint whose
//! acceleration or rate is unknown there is no upper bound. Each channel's clock is therefore an
//! interval [earliest, latest]; a wait releases its channels at the latest of their earliest
//! arrivals and the latest of their latest ones. Each segment is two pieces: its motion, under
//! way at some time in [earliest start, latest end], and its rest at the end pose until the latest
//! start of the channel's next segment (the last rests to the end); the start pose rests until the
//! first segment. Two pieces of different channels are checked against each other, each over its
//! whole motion and independently of the other (collide::check_two), when their windows meet and
//! no wait orders them (a piece before a wait both channels share never meets one after it). That
//! is sound for every timing inside the bounds; it may report a contact a real timing avoids. A
//! window with no upper bound meets every later one, and the run says so.
use crate::collide::{self, CBody, Report, Verdict};
use crate::def::MachineDef;
use crate::sim::{self, Happening, SegKind, Segment, Setup, SimError, Timeline};

/// One channel: its number (the joint letters' digit; 1 for none), setup and program.
pub struct Channel<'s, 'a> {
    pub ch: u8,
    pub setup: &'s Setup<'a>,
    pub program: &'s str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrossContact {
    pub a_channel: u8,
    pub a_line: usize,
    pub b_channel: u8,
    pub b_line: usize,
    pub a: String,
    pub b: String,
    pub verdict: Verdict,
}

#[derive(Debug)]
pub struct Multi {
    /// Per channel, as run alone.
    pub timelines: Vec<Timeline>,
    /// Per channel and segment: [earliest start, latest end of rest] (s; may be infinite).
    pub windows: Vec<Vec<[f64; 2]>>,
    /// Per channel: its own bodies (and the static ones) along its timeline.
    pub own: Vec<Report>,
    /// Bodies of two channels.
    pub cross: Vec<CrossContact>,
    /// The whole run: [earliest end, latest end].
    pub t: [f64; 2],
    /// Where the bounds were open (no upper time): those windows were taken as unbounded.
    pub conservative: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum MultiError {
    Sim {
        ch: u8,
        err: SimError,
    },
    /// No channel can go on: each is held at a wait the others do not reach (channel, line, wait).
    Deadlock(Vec<(u8, usize, u32)>),
    /// A body moved by joints of two channels: not modelled.
    TwoChannels(String),
}

enum Item {
    Seg(usize),
    Wait(u32, usize),
}

/// A motion (q0 → q1) or a rest (q0 = q1) of one channel, with when it can happen and the waits
/// around it.
struct Piece {
    seg: Segment,
    line: usize,
    window: [f64; 2],
    /// Releases the channel passed before it.
    after: Vec<usize>,
    /// The first release it reaches after it, if any.
    before: Option<usize>,
}

/// A segment's latest duration: its exact-stop time when every joint it moves has a known
/// acceleration and rate, else none.
fn upper(def: &MachineDef, sg: &Segment) -> f64 {
    for (j, jt) in def.joints.iter().enumerate() {
        if sg.q0[j] != sg.q1[j] {
            let rate = match sg.kind {
                SegKind::Rapid => jt.rapid,
                _ => jt.max_feed,
            };
            if jt.accel.is_none() || rate.is_none() {
                return f64::INFINITY;
            }
        }
    }
    sg.t_stop.max(sg.t_min)
}

pub fn run(def: &MachineDef, channels: &[Channel], work: &[CBody]) -> Result<Multi, MultiError> {
    let n = channels.len();
    let mut timelines = Vec::new();
    for c in channels {
        timelines.push(sim::simulate(c.program, c.setup).map_err(|err| MultiError::Sim { ch: c.ch, err })?);
    }
    // each channel's items in program order: segments and waits
    let items: Vec<Vec<Item>> = timelines
        .iter()
        .map(|tl| {
            let mut v: Vec<(usize, u8, Item)> = tl
                .segments
                .iter()
                .enumerate()
                .map(|(i, sg)| (sg.line, 0, Item::Seg(i)))
                .collect();
            for m in &tl.marks {
                if let Happening::Wait { n } = m.what {
                    v.push((m.line, 1, Item::Wait(n, m.line)));
                }
            }
            v.sort_by_key(|(l, k, _)| (*l, *k));
            v.into_iter().map(|(_, _, it)| it).collect()
        })
        .collect();
    // walk the channels with interval clocks
    let mut at = vec![0usize; n];
    let mut clock = vec![[0.0f64, 0.0f64]; n];
    let mut starts: Vec<Vec<[f64; 2]>> = timelines
        .iter()
        .map(|tl| vec![[0.0; 2]; tl.segments.len()])
        .collect();
    let mut conservative = Vec::new();
    // per channel: pieces so far (rests filled in when the next start is known)
    let mut pieces: Vec<Vec<Piece>> = (0..n).map(|_| Vec::new()).collect();
    let mut passed: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut release_id = 0usize;
    // the start pose rests until the first move
    let rest = |sg: &Segment, at_end: bool| -> Segment {
        let q = if at_end { sg.q1.clone() } else { sg.q0.clone() };
        Segment {
            q0: q.clone(),
            q1: q,
            kind: SegKind::Rapid,
            path: sim::Path::Line,
            ..sg.clone()
        }
    };
    for c in 0..n {
        if let Some(first) = timelines[c].segments.first() {
            pieces[c].push(Piece {
                seg: rest(first, false),
                line: 0,
                window: [0.0, f64::NAN],
                after: Vec::new(),
                before: None,
            });
        }
    }
    loop {
        // run every channel to its next wait (or its end)
        for c in 0..n {
            while let Some(Item::Seg(i)) = items[c].get(at[c]) {
                let tl = &timelines[c];
                let sg = &tl.segments[*i];
                // the time since the previous segment's end (dwells, tool changes): the same lower
                // and upper
                let gap = if *i == 0 {
                    sg.t0
                } else {
                    let p = &tl.segments[i - 1];
                    (sg.t0 - (p.t0 + p.t_min)).max(0.0)
                };
                clock[c] = [clock[c][0] + gap, clock[c][1] + gap];
                starts[c][*i] = clock[c];
                // the previous rest ends at the latest start of this one
                if let Some(p) = pieces[c].last_mut()
                    && p.window[1].is_nan()
                {
                    p.window[1] = clock[c][1];
                }
                let up = upper(def, sg);
                if up.is_infinite()
                    && !conservative
                        .iter()
                        .any(|s: &String| s.starts_with(&format!("channel {}:", channels[c].ch)))
                {
                    conservative.push(format!(
                        "channel {}: from line {} a move has no upper time (a joint without accel or rate): its windows are open, checked against every later move of the other channels",
                        channels[c].ch, sg.line
                    ));
                }
                let begin = clock[c];
                clock[c] = [clock[c][0] + sg.t_min, clock[c][1] + up];
                pieces[c].push(Piece {
                    seg: sg.clone(),
                    line: sg.line,
                    window: [begin[0], clock[c][1]],
                    after: passed[c].clone(),
                    before: None,
                });
                pieces[c].push(Piece {
                    seg: rest(sg, true),
                    line: sg.line,
                    window: [clock[c][0], f64::NAN],
                    after: passed[c].clone(),
                    before: None,
                });
                at[c] += 1;
            }
        }
        if (0..n).all(|c| at[c] >= items[c].len()) {
            break;
        }
        // release a wait every channel that still has it is held at
        let held = |c: usize| match items[c].get(at[c]) {
            Some(Item::Wait(w, _)) => Some(*w),
            _ => None,
        };
        let mut released = false;
        for c in 0..n {
            let Some(w) = held(c) else { continue };
            let involved: Vec<usize> = (0..n)
                .filter(|&d| {
                    items[d][at[d]..]
                        .iter()
                        .any(|it| matches!(it, Item::Wait(x, _) if *x == w))
                })
                .collect();
            if involved.iter().all(|&d| held(d) == Some(w)) {
                let lo = involved.iter().map(|&d| clock[d][0]).fold(0.0, f64::max);
                let hi = involved.iter().map(|&d| clock[d][1]).fold(0.0, f64::max);
                for &d in &involved {
                    clock[d] = [lo, hi];
                    at[d] += 1;
                    // everything this channel did so far ends before this release
                    for p in pieces[d].iter_mut().filter(|p| p.before.is_none()) {
                        p.before = Some(release_id);
                    }
                    passed[d].push(release_id);
                }
                release_id += 1;
                released = true;
                break;
            }
        }
        if !released {
            let stuck = (0..n)
                .filter_map(|c| match items[c].get(at[c]) {
                    Some(Item::Wait(w, line)) => Some((channels[c].ch, *line, *w)),
                    _ => None,
                })
                .collect();
            return Err(MultiError::Deadlock(stuck));
        }
    }
    for ps in pieces.iter_mut() {
        for p in ps.iter_mut() {
            if p.window[1].is_nan() {
                p.window[1] = f64::INFINITY;
            }
        }
    }
    // a segment's window: its earliest start to the latest start of the next one (it rests at its
    // end pose until then); the last one rests to the end of everything
    let windows: Vec<Vec<[f64; 2]>> = starts
        .iter()
        .map(|st| {
            (0..st.len())
                .map(|i| {
                    [
                        st[i][0],
                        if i + 1 < st.len() {
                            st[i + 1][1]
                        } else {
                            f64::INFINITY
                        },
                    ]
                })
                .collect()
        })
        .collect();
    let t = [
        clock.iter().map(|c| c[0]).fold(0.0, f64::max),
        clock.iter().map(|c| c[1]).fold(0.0, f64::max),
    ];
    // which channel moves each body
    let only = |b: &CBody| -> Result<Option<u8>, MultiError> {
        match collide::body_channels(def, b).as_slice() {
            [] => Ok(None),
            [c] => Ok(Some(*c)),
            _ => Err(MultiError::TwoChannels(b.name.clone())),
        }
    };
    let machine = collide::machine_bodies(def);
    for b in machine.iter().chain(work) {
        only(b)?;
    }
    // each channel's own pairs: bodies static or moved by it alone
    let mut own = Vec::new();
    for (c, ch) in channels.iter().enumerate() {
        let mine = |b: &CBody| matches!(only(b), Ok(None)) || matches!(only(b), Ok(Some(x)) if x == ch.ch);
        own.push(collide::check_where(ch.setup, &timelines[c], work, &|a, b| {
            mine(a) && mine(b)
        }));
    }
    // bodies of two channels, over every pair of pieces that can be under way together
    let mut cross = Vec::new();
    for ca in 0..n {
        for cb in ca + 1..n {
            let (cha, chb) = (channels[ca].ch, channels[cb].ch);
            let ja = collide::channel_joints(def, cha);
            for pa in &pieces[ca] {
                for pb in &pieces[cb] {
                    let (wa, wb) = (pa.window, pb.window);
                    if wa[0] > wb[1] || wb[0] > wa[1] {
                        continue;
                    }
                    let ordered = |x: &Piece, y: &Piece| x.before.is_some_and(|r| y.after.contains(&r));
                    if ordered(pa, pb) || ordered(pb, pa) {
                        continue;
                    }
                    let (sa, sb) = (&pa.seg, &pb.seg);
                    let ta: Vec<CBody> = sa
                        .tool
                        .map(|t| collide::tool_bodies(channels[ca].setup, t))
                        .unwrap_or_default();
                    let tb: Vec<CBody> = sb
                        .tool
                        .map(|t| collide::tool_bodies(channels[cb].setup, t))
                        .unwrap_or_default();
                    let on = |b: &CBody, ch: u8| matches!(only(b), Ok(Some(x)) if x == ch);
                    let bodies_a: Vec<&CBody> = machine
                        .iter()
                        .chain(work)
                        .filter(|b| on(b, cha))
                        .chain(ta.iter())
                        .collect();
                    let bodies_b: Vec<&CBody> = machine
                        .iter()
                        .chain(work)
                        .filter(|b| on(b, chb))
                        .chain(tb.iter())
                        .collect();
                    for x in &bodies_a {
                        for y in &bodies_b {
                            let kind = if ta.iter().any(|t| t.name == x.name) {
                                sa.kind
                            } else if tb.iter().any(|t| t.name == y.name) {
                                sb.kind
                            } else {
                                SegKind::Rapid
                            };
                            let Some(cl) = collide::pair_rule(def, x, y, kind) else {
                                continue;
                            };
                            let mut spans = 0;
                            let v = collide::check_two(def, sa, sb, &ja, x, y, cl, &mut spans);
                            if v != Verdict::Clear
                                && !cross.iter().any(|k: &CrossContact| {
                                    k.a_line == pa.line
                                        && k.b_line == pb.line
                                        && k.a == x.name
                                        && k.b == y.name
                                })
                            {
                                cross.push(CrossContact {
                                    a_channel: cha,
                                    a_line: pa.line,
                                    b_channel: chb,
                                    b_line: pb.line,
                                    a: x.name.clone(),
                                    b: y.name.clone(),
                                    verdict: v,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(Multi {
        timelines,
        windows,
        own,
        cross,
        t,
        conservative,
    })
}
