//! `check_shell_faces`'s tests, as `topo.rs`'s are (and translated with them): in a module of their
//! own so that adding them left the code of `topo.rs`'s tests as it was (rustc places generic
//! code by module).

use crate::topo::{Sink, Topo};

/// What `shell_faces` reads besides: the shells.
pub trait ShellTopo: Topo {
    /// The shell face `f` names.
    fn face_shell(&self, f: usize) -> usize;
    fn n_shells(&self) -> usize;
    /// How many faces shell `s` lists, and its `i`th.
    fn shell_n_faces(&self, s: usize) -> usize;
    fn shell_face(&self, s: usize, i: usize) -> usize;
}

/// The face coedge `c` bounds.
#[inline]
fn coedge_face<T: Topo>(t: &T, c: usize) -> usize {
    t.loop_face(t.coedge_loop(c))
}

/// `check_shell_faces`: the shells (each lists its faces once, each live face is in the one shell
/// it names, the faces at an edge share a shell; a shell at fault is named once, the shell count
/// naming a face whose shell is out of range), then the sound shells whose faces are not all
/// joined through edges. Read only once `links` found nothing: every index it follows is then in
/// range.
pub fn shell_faces<T: ShellTopo, S: Sink>(t: &T, out: &mut S) {
    let ns = t.n_shells();
    let nf = t.n_faces();
    let ne = t.n_edges();
    let mut bad: Vec<bool> = vec![false; ns + 1];
    // how often each face's shell lists it, up to 2
    let mut listed: Vec<u32> = vec![0; nf];
    let mut s = 0;
    while s < ns {
        let n = t.shell_n_faces(s);
        if n == 0 {
            bad[s] = true;
        }
        let mut i = 0;
        while i < n {
            let f = t.shell_face(s, i);
            if f >= nf || !t.face_alive(f) || t.face_shell(f) != s {
                bad[s] = true;
            } else if listed[f] < 2 {
                listed[f] += 1;
            }
            i += 1;
        }
        s += 1;
    }
    let mut f = 0;
    while f < nf {
        if t.face_alive(f) && listed[f] != 1 {
            let at = t.face_shell(f);
            bad[if at < ns { at } else { ns }] = true;
        }
        f += 1;
    }
    let mut e = 0;
    while e < ne {
        if t.edge_alive(e) {
            let n = t.edge_n_uses(e);
            if n > 0 {
                let s0 = t.face_shell(coedge_face(t, t.edge_use(e, 0)));
                let mut split = false;
                // (from the second: the first shares its own shell)
                let mut i = 1;
                while !split && i < n {
                    split = t.face_shell(coedge_face(t, t.edge_use(e, i))) != s0;
                    i += 1;
                }
                if split {
                    bad[if s0 < ns { s0 } else { ns }] = true;
                }
            }
        }
        e += 1;
    }
    let mut s = 0;
    while s <= ns {
        if bad[s] {
            out.shell_faces(s);
        }
        s += 1;
    }
    // joined: every face of a sound shell reached from its first through edges (`seen` holds the
    // shell a face was last reached from, so each shell's flood is its own). The stack is
    // `stack[..sp]`, its vector grown only past its longest yet.
    // (`listed`'s buffer, done with: one allocation fewer; u32 stamps, as a body's shell ids are)
    let mut seen = listed;
    let mut k = 0;
    while k < nf {
        seen[k] = u32::MAX;
        k += 1;
    }
    // (u32, as a body's face ids are: its allocations then are the ones `check` always made)
    let mut stack: Vec<u32> = Vec::new();
    let mut s = 0;
    while s < ns {
        if !bad[s] {
            let f0 = t.shell_face(s, 0);
            if stack.is_empty() {
                stack.push(f0 as u32);
            } else {
                stack[0] = f0 as u32;
            }
            let mut sp = 1;
            seen[f0] = s as u32;
            while sp > 0 {
                sp -= 1;
                let f = stack[sp] as usize;
                let nlf = t.face_n_loops(f);
                let mut i = 0;
                while i < nlf {
                    let l = t.face_loop(f, i);
                    let end = t.loop_first(l) + t.loop_len(l);
                    let mut c = t.loop_first(l);
                    while c < end {
                        let e = t.coedge_edge(c);
                        let nu = t.edge_n_uses(e);
                        let mut j = 0;
                        while j < nu {
                            let g = coedge_face(t, t.edge_use(e, j));
                            if seen[g] != s as u32 {
                                seen[g] = s as u32;
                                if sp < stack.len() {
                                    stack[sp] = g as u32;
                                } else {
                                    stack.push(g as u32);
                                }
                                sp += 1;
                            }
                            j += 1;
                        }
                        c += 1;
                    }
                    i += 1;
                }
            }
            let n = t.shell_n_faces(s);
            let mut joined = true;
            let mut i = 0;
            while joined && i < n {
                joined = seen[t.shell_face(s, i)] == s as u32;
                i += 1;
            }
            if !joined {
                out.shell_disconnected(s);
            }
        }
        s += 1;
    }
}
