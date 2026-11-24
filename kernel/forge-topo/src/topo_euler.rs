//! `check_shell_euler`'s test, as `topo.rs`'s are (and translated with them): in a module of its
//! own so that adding it left the code of the other tests as it was (rustc places generic code by
//! module).

use crate::topo_shell::ShellTopo;

/// Where `shell_euler` puts what it finds.
pub trait EulerSink {
    /// Shell `s`'s V − E + 2F − L is `value`: odd, or above 2.
    fn shell_euler(&mut self, s: usize, value: i64);
}

/// `check_shell_euler`: per shell, its V − E + 2F − L, the edges and vertices counted once each
/// (stamped with the shell: the same counts as sorting and dedup, without the sorts), the faces and
/// loops as listed; a value that is odd or above 2 is a problem. Read only once `links` found
/// nothing and every edge has its two uses and every loop closes: every index it follows is then
/// in range, except a shell's face ids (`shell_faces` reports those later): a face id past the
/// faces counts as a face with no loops, as the model reads it, and is not followed.
pub fn shell_euler<T: ShellTopo, S: EulerSink>(t: &T, out: &mut S) {
    let ns = t.n_shells();
    let n_faces = t.n_faces();
    let mut edge_at: Vec<u32> = vec![u32::MAX; t.n_edges()];
    let mut vert_at: Vec<u32> = vec![u32::MAX; t.n_vertices()];
    let mut s = 0;
    while s < ns {
        let stamp = s as u32;
        let mut f: i64 = 0;
        let mut l: i64 = 0;
        let mut e: i64 = 0;
        let mut v: i64 = 0;
        let nf = t.shell_n_faces(s);
        let mut i = 0;
        while i < nf {
            let face = t.shell_face(s, i);
            f += 1;
            let nl = if face < n_faces { t.face_n_loops(face) } else { 0 };
            let mut j = 0;
            while j < nl {
                let lp = t.face_loop(face, j);
                l += 1;
                let first = t.loop_first(lp);
                let len = t.loop_len(lp);
                let mut k = 0;
                while k < len {
                    let ed = t.coedge_edge(first + k);
                    if edge_at[ed] != stamp {
                        edge_at[ed] = stamp;
                        e += 1;
                    }
                    let (a, b) = t.edge_ends(ed);
                    if vert_at[a] != stamp {
                        vert_at[a] = stamp;
                        v += 1;
                    }
                    if vert_at[b] != stamp {
                        vert_at[b] = stamp;
                        v += 1;
                    }
                    k += 1;
                }
                j += 1;
            }
            i += 1;
        }
        let value = v - e + 2 * f - l;
        if value % 2 != 0 || value > 2 {
            out.shell_euler(s, value);
        }
        s += 1;
    }
}
