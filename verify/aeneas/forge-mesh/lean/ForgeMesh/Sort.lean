/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMesh.Funs

/-!
# `sort_keys` returns

The CDT's own sort (insertion sorts of runs of 16, merged bottom-up through a scratch vector)
returns `ok`, the vector as long as it was, for fewer than 2^62 keys: every index in bounds,
every loop ending (each by `loop.spec_decr_nat`, a measure and an invariant). That it sorts,
and permutes, is to come; `Ends` needs only this.
-/

open Aeneas Aeneas.Std Result
namespace forge_mesh

/-- `sink`'s loop returns, the vector as long, stopping at a place between s and where it
started. -/
@[step] theorem sink_loop_spec (v : alloc.vec.Vec Std.U64) (s : Std.Usize) (x : Std.U64) (j : Std.Usize)
    (hs : s.val ≤ j.val) (hj : j.val < v.length) :
    cdt.sink_loop v s x j true ⦃ r => r.1.length = v.length ∧ s.val ≤ r.2.val ∧ r.2.val ≤ j.val ⦄ := by
  unfold cdt.sink_loop
  apply loop.spec_decr_nat
    (fun (a : alloc.vec.Vec Std.U64 × Std.Usize × Bool) => 2 * a.2.1.val + (if a.2.2 then 1 else 0))
    (fun a => a.1.length = v.length ∧ s.val ≤ a.2.1.val ∧ a.2.1.val ≤ j.val ∧ a.2.1.val < v.length)
  · rintro ⟨w, k, m⟩ ⟨hl, h1, h2, h3⟩
    simp only at hl h1 h2 h3 ⊢
    unfold cdt.sink_loop.body
    split <;> (try split) <;> (try step*) <;> simp_all <;> omega
  · simp_all

end forge_mesh

namespace forge_mesh

/-- One merge of two runs, src[s..m] and src[m..e] into dst[s..e], returns, dst as long. -/
@[step]
theorem merge_inner_spec (src dst : Slice Std.U64) (m e i j k : Std.Usize)
    (h2 : i.val ≤ m.val) (h3 : m.val ≤ j.val) (h4 : j.val ≤ e.val)
    (h5 : e.val ≤ src.length) (h6 : e.val ≤ dst.length) (h7 : k.val + m.val = i.val + j.val) :
    cdt.merge_runs_loop0_loop0 src dst m e i j k ⦃ r => r.length = dst.length ⦄ := by
  unfold cdt.merge_runs_loop0_loop0
  apply loop.spec_decr_nat
    (fun (a : Slice Std.U64 × Std.Usize × Std.Usize × Std.Usize) => e.val - a.2.2.2.val)
    (fun a => a.1.length = dst.length ∧ i.val ≤ a.2.1.val ∧ a.2.1.val ≤ m.val ∧
      m.val ≤ a.2.2.1.val ∧ a.2.2.1.val ≤ e.val ∧ a.2.2.2.val + m.val = a.2.1.val + a.2.2.1.val)
  · rintro ⟨d, a, b, c⟩ ⟨hl, g1, g2, g3, g4, g5⟩
    simp only at hl g1 g2 g3 g4 g5 ⊢
    clear h2 h3 h4 h7
    unfold cdt.merge_runs_loop0_loop0.body
    split
    · split <;> (try split) <;> (try split) <;> step* <;> (try split) <;> (try step*) <;> simp_all <;> scalar_tac
    · simp_all
  · simp_all

end forge_mesh

namespace forge_mesh

/-- A merge pass over src[..n] into dst returns, dst as long. -/
@[step] theorem merge_runs_spec (src dst : Slice Std.U64) (n w : Std.Usize)
    (hw : 1 ≤ w.val) (hwn : w.val ≤ n.val) (hn : 2 * n.val ≤ Usize.max)
    (h5 : n.val ≤ src.length) (h6 : n.val ≤ dst.length) :
    cdt.merge_runs src dst n w ⦃ r => r.length = dst.length ⦄ := by
  unfold cdt.merge_runs cdt.merge_runs_loop0
  apply loop.spec_decr_nat
    (fun (a : Slice Std.U64 × Std.Usize) => n.val - a.2.val)
    (fun a => a.1.length = dst.length ∧ a.2.val ≤ n.val)
  · rintro ⟨d, s⟩ ⟨hl, g1⟩
    simp only at hl g1 ⊢
    unfold cdt.merge_runs_loop0.body
    split
    · step*
      all_goals (split <;> step* <;> (try split) <;> step* <;> (try simp_all) <;> scalar_tac)
    · simp_all
  · simp_all

end forge_mesh

namespace forge_mesh

@[step] theorem sink_spec (v : alloc.vec.Vec Std.U64) (s i : Std.Usize)
    (hs : s.val ≤ i.val) (hi : i.val < v.length) :
    cdt.sink v s i ⦃ r => r.length = v.length ⦄ := by
  unfold cdt.sink
  step*
  all_goals simp_all <;> scalar_tac

@[step] theorem sort_runs_inner_spec (v : alloc.vec.Vec Std.U64) (s e i : Std.Usize)
    (hs : s.val < i.val) (he : e.val ≤ v.length) :
    cdt.sort_keys_loop0_loop0 v s e i ⦃ r => r.length = v.length ⦄ := by
  unfold cdt.sort_keys_loop0_loop0
  apply loop.spec_decr_nat
    (fun (a : alloc.vec.Vec Std.U64 × Std.Usize) => e.val - a.2.val)
    (fun a => a.1.length = v.length ∧ s.val < a.2.val)
  · rintro ⟨w, k⟩ ⟨hl, g1⟩
    simp only at hl g1 ⊢
    unfold cdt.sort_keys_loop0_loop0.body
    split <;> step* <;> (try simp_all) <;> scalar_tac
  · simp_all

@[step] theorem sort_runs_spec (v : alloc.vec.Vec Std.U64) (n s : Std.Usize)
    (hn : n.val = v.length) (hm : n.val + 16 ≤ Usize.max) :
    cdt.sort_keys_loop0 v n s ⦃ r => r.length = v.length ⦄ := by
  unfold cdt.sort_keys_loop0
  apply loop.spec_decr_nat
    (fun (a : alloc.vec.Vec Std.U64 × Std.Usize) => n.val - a.2.val)
    (fun a => a.1.length = v.length)
  · rintro ⟨w, k⟩ hl
    simp only at hl ⊢
    unfold cdt.sort_keys_loop0.body
    split <;> step* <;> (try split) <;> step* <;> (try simp_all) <;> scalar_tac
  · simp_all

end forge_mesh

namespace forge_mesh

@[step] theorem merge_passes_spec (v tmp : alloc.vec.Vec Std.U64) (n w : Std.Usize) (in_v : Bool)
    (hn : n.val = v.length) (ht : n.val ≤ tmp.length) (hw : 1 ≤ w.val) (hm : 2 * n.val ≤ Usize.max) :
    cdt.sort_keys_loop1 v tmp n w in_v ⦃ r => r.1.length = v.length ∧ n.val ≤ r.2.1.length ⦄ := by
  unfold cdt.sort_keys_loop1
  apply loop.spec_decr_nat
    (fun (a : alloc.vec.Vec Std.U64 × alloc.vec.Vec Std.U64 × Std.Usize × Bool) => n.val - a.2.2.1.val)
    (fun a => a.1.length = v.length ∧ n.val ≤ a.2.1.length ∧ 1 ≤ a.2.2.1.val)
  · rintro ⟨x, y, k, b⟩ ⟨g1, g2, g3⟩
    simp only at g1 g2 g3 ⊢
    unfold cdt.sort_keys_loop1.body
    simp only [alloc.vec.Vec.deref_mut, alloc.vec.Vec.deref, lift, bind_ok]
    split <;> (try split) <;> step* <;> (try simp_all [alloc.vec.Vec.val]) <;> scalar_tac
  · simp_all

@[step] theorem copy_back_spec (v tmp : alloc.vec.Vec Std.U64) (n i : Std.Usize)
    (hn : n.val ≤ v.length) (ht : n.val ≤ tmp.length) :
    cdt.sort_keys_loop2 v tmp n i ⦃ r => r.length = v.length ⦄ := by
  unfold cdt.sort_keys_loop2
  apply loop.spec_decr_nat
    (fun (a : alloc.vec.Vec Std.U64 × Std.Usize) => n.val - a.2.val)
    (fun a => a.1.length = v.length)
  · rintro ⟨x, k⟩ g1
    simp only at g1 ⊢
    unfold cdt.sort_keys_loop2.body
    split <;> step* <;> (try simp_all) <;> scalar_tac
  · simp_all

/-- `sort_keys` returns, v as long as it was (sorted: to come), for a vector of fewer than
2^62 keys. -/
@[step] theorem sort_keys_spec (v tmp : alloc.vec.Vec Std.U64) (hm : 2 * v.length + 16 ≤ Usize.max) :
    cdt.sort_keys v tmp ⦃ r => r.1.length = v.length ⦄ := by
  unfold cdt.sort_keys
  step*
  all_goals (try split) <;> step* <;> (try split) <;> step* <;> (try simp_all) <;> scalar_tac

end forge_mesh
