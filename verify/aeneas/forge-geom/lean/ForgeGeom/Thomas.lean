/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeGeom.Refine

/-!
# The real Thomas solve, against its binary64 model (L4)

About the Aeneas translation of `kernel/forge-geom/src/spline.rs`'s `tridiagonal` itself:
`tridiagonal_spec`: on any rows (n ≥ 1) the shipped solve returns `ok`, n points, point k being
`fback rows n k`. `fsweep` and `fback` are `SplineSpec.sweep` and `SplineSpec.backSub`
(research/spline-loops/lean/SplineSpec/Thomas.lean) with each field operation the binary64 one, in
the order the Rust does them. So the exact algorithm `thomas_solves` proves correct is what ships,
rounding aside; the rounding is research/spline-loops/fp's.
-/

open Aeneas Aeneas.Std Result
open forge_geom

namespace ForgeGeom

/-- A row `(a, b, c, d)`: `a x_(i-1) + b x_i + c x_(i+1) = d`. -/
abbrev Row := F64 × F64 × F64 × forge_math.vec.Vec3

abbrev V3 := forge_math.vec.Vec3

instance : Inhabited V3 := ⟨{ x := default, y := default, z := default }⟩

/-- `Vec3 * f64`, `Vec3 - Vec3`, `Vec3 / f64` in binary64, field by field (forge-math's `vec_ops!`). -/
def vmul (v : V3) (s : F64) : V3 := { x := F64.Ops.mul v.x s, y := F64.Ops.mul v.y s, z := F64.Ops.mul v.z s }
def vsub (v o : V3) : V3 := { x := F64.Ops.sub v.x o.x, y := F64.Ops.sub v.y o.y, z := F64.Ops.sub v.z o.z }
def vdiv (v : V3) (s : F64) : V3 := { x := F64.Ops.div v.x s, y := F64.Ops.div v.y s, z := F64.Ops.div v.z s }

/-- The forward sweep in binary64: `(c'_i, d'_i)`. -/
def fsweep (R : List Row) : ℕ → F64 × V3
  | 0 => (F64.Ops.div R[0]!.2.2.1 R[0]!.2.1, vdiv R[0]!.2.2.2 R[0]!.2.1)
  | i + 1 =>
    let m := F64.Ops.sub R[i + 1]!.2.1 (F64.Ops.mul R[i + 1]!.1 (fsweep R i).1)
    (F64.Ops.div R[i + 1]!.2.2.1 m, vdiv (vsub R[i + 1]!.2.2.2 (vmul (fsweep R i).2 R[i + 1]!.1)) m)

/-- The back substitution in binary64: `x_(n-1) = d'_(n-1)`, `x_k = d'_k - x_(k+1) c'_k`. -/
def fback (R : List Row) (n : ℕ) : ℕ → V3
  | k =>
    if h : k + 1 < n then vsub (fsweep R k).2 (vmul (fback R n (k + 1)) (fsweep R k).1)
    else (fsweep R k).2
  termination_by k => n - k

theorem vset_get {α : Type} [Inhabited α] (v : alloc.vec.Vec α) (i : Std.Usize) (x : α) (k : ℕ)
    (hi : i.val < v.length) : (v.set i x).val[k]! = if k = i.val then x else v.val[k]! := by
  rw [alloc.vec.Vec.set_val_eq]
  have hl : i.val < v.val.length := by simpa using hi
  by_cases h : k = i.val
  · subst h; simp [hl]
  · simp [h, List.getElem!_eq_getElem?_getD, List.getElem?_set, Ne.symm h]

/-- The forward loop: rows 1.. of the sweep, into c and d. -/
theorem sweep_spec (rows : Slice Row) (it : core.ops.range.Range Std.Usize)
    (c : alloc.vec.Vec F64) (d : alloc.vec.Vec V3)
    (he : it.end.val = rows.length) (h1 : 1 ≤ it.start.val) (hs : it.start.val ≤ rows.length)
    (hc : c.length = rows.length) (hd : d.length = rows.length)
    (hk : ∀ k, k < it.start.val → c.val[k]! = (fsweep rows.val k).1 ∧ d.val[k]! = (fsweep rows.val k).2) :
    spline.tridiagonal_loop0 it rows c d ⦃ c' d' => c'.length = rows.length ∧ d'.length = rows.length ∧
      ∀ k, k < rows.length → c'.val[k]! = (fsweep rows.val k).1 ∧ d'.val[k]! = (fsweep rows.val k).2 ⦄ := by
  unfold spline.tridiagonal_loop0 spline.tridiagonal_loop0.body
  apply loop.spec_decr_nat (fun x => rows.length - x.1.start.val)
    (fun x => x.1.end.val = rows.length ∧ 1 ≤ x.1.start.val ∧ x.1.start.val ≤ rows.length ∧
      x.2.1.length = rows.length ∧ x.2.2.length = rows.length ∧
      ∀ k, k < x.1.start.val → x.2.1.val[k]! = (fsweep rows.val k).1 ∧
        x.2.2.val[k]! = (fsweep rows.val k).2)
  · rintro ⟨it0, c0, d0⟩ ⟨he0, h10, hs0, hc0, hd0, hk0⟩
    simp only at he0 h10 hs0 hc0 hd0 hk0 ⊢
    step*
    · split_ifs at o_post with hlt
      · simp_all
      · refine ⟨hc0, hd0, fun k hk => hk0 k (by omega)⟩
    · split_ifs at o_post with hlt
      · obtain ⟨h, _⟩ := o_post; simp_all
      · simp_all
    · split_ifs at o_post with hlt
      swap; · simp_all
      obtain ⟨ho, hst⟩ := o_post
      have hi : i = it0.start := by simp_all
      subst hi
      obtain ⟨ta, tb, tc, td⟩ := t
      step*
      simp only [lift, forge_math.vec.Vec3.Insts.CoreOpsArithMulF64Vec3.mul,
        forge_math.vec.Vec3.Insts.CoreOpsArithSubVec3Vec3.sub,
        forge_math.vec.Vec3.Insts.CoreOpsArithDivF64Vec3.div, bind_ok]
      step*
      obtain ⟨j, hj⟩ : ∃ j, it0.start.val = j + 1 := ⟨it0.start.val - 1, by omega⟩
      have hlen : it0.start.val < rows.length := by omega
      have hrow : rows.val[j + 1]! = (ta, tb, tc, td) := by
        rw [← hj, getElem!_pos rows.val it0.start.val (by simpa using hlen), t_post]
      obtain ⟨hcj, hdj⟩ := hk0 j (by omega)
      have hi1 : i1.val = j := by omega
      have hfl : fl = (fsweep rows.val j).1 := by
        rw [fl_post, ← hcj, getElem!_pos c0.val j (by scalar_tac)]; simp only [hi1]
      have hv : v = (fsweep rows.val j).2 := by
        rw [v_post, ← hdj, getElem!_pos d0.val j (by scalar_tac)]; simp only [hi1]
      have key : fsweep rows.val (j + 1) =
          (F64.Ops.div tc (F64.Ops.sub tb (F64.Ops.mul ta fl)),
            { x := F64.Ops.div (F64.Ops.sub td.x (F64.Ops.mul v.x ta)) (F64.Ops.sub tb (F64.Ops.mul ta fl)),
              y := F64.Ops.div (F64.Ops.sub td.y (F64.Ops.mul v.y ta)) (F64.Ops.sub tb (F64.Ops.mul ta fl)),
              z := F64.Ops.div (F64.Ops.sub td.z (F64.Ops.mul v.z ta)) (F64.Ops.sub tb (F64.Ops.mul ta fl)) }) := by
        rw [hfl, hv]; simp only [fsweep, hrow, vdiv, vsub, vmul]
      subst index_mut_back_post index_mut_back1_post
      refine ⟨by simp_all, by omega, by omega, by simp_all, by simp_all, ?_, by omega⟩
      intro k hk
      rw [vset_get _ _ _ _ (by scalar_tac), vset_get _ _ _ _ (by scalar_tac)]
      by_cases hkj : k = it0.start.val
      · simp [hkj, hj, key]
      · simp only [hkj, ite_false]; exact hk0 k (by omega)
  · exact ⟨he, h1, hs, hc, hd, hk⟩

/-- The backward loop: from row i down, each `d_k - d_(k+1) c_k`. -/
theorem back_spec (R : List Row) (n : ℕ) (c : alloc.vec.Vec F64) (d : alloc.vec.Vec V3)
    (i : Std.Usize) (hin : i.val + 1 ≤ n) (hc : c.length = n) (hd : d.length = n)
    (hcv : ∀ k, k < n → c.val[k]! = (fsweep R k).1)
    (hdv : ∀ k, k < n → d.val[k]! = if k < i.val then (fsweep R k).2 else fback R n k) :
    spline.tridiagonal_loop1 c d i ⦃ d' => d'.length = n ∧ ∀ k, k < n → d'.val[k]! = fback R n k ⦄ := by
  unfold spline.tridiagonal_loop1 spline.tridiagonal_loop1.body
  apply loop.spec_decr_nat (fun x => x.2.val)
    (fun x => x.2.val + 1 ≤ n ∧ x.1.length = n ∧
      ∀ k, k < n → x.1.val[k]! = if k < x.2.val then (fsweep R k).2 else fback R n k)
  · rintro ⟨d0, j⟩ ⟨hj, hd0, hdv0⟩
    simp only at hj hd0 hdv0 ⊢
    split
    · step*
      simp only [forge_math.vec.Vec3.Insts.CoreOpsArithMulF64Vec3.mul,
        forge_math.vec.Vec3.Insts.CoreOpsArithSubVec3Vec3.sub, bind_ok]
      step*
      have hi2 : i2.val = i1.val + 1 := by scalar_tac
      have hv : v = (fsweep R i1.val).2 := by
        have := hdv0 i1.val (by scalar_tac)
        rw [getElem!_pos d0.val i1.val (by scalar_tac)] at this
        rw [v_post, this]; simp only [show i1.val < j.val by scalar_tac, ite_true]
      have hv1 : v1 = fback R n (i1.val + 1) := by
        have := hdv0 (i1.val + 1) (by scalar_tac)
        rw [getElem!_pos d0.val (i1.val + 1) (by scalar_tac)] at this
        rw [v1_post]; simp only [hi2]; rw [this]
        simp only [show ¬ (i1.val + 1 < j.val) by scalar_tac, ite_false]
      have hfl : fl = (fsweep R i1.val).1 := by
        have := hcv i1.val (by scalar_tac)
        rw [getElem!_pos c.val i1.val (by scalar_tac)] at this
        rw [fl_post, this]
      subst index_mut_back_post
      refine ⟨by scalar_tac, by simp_all, ?_, by scalar_tac⟩
      intro k hk
      rw [vset_get _ _ _ _ (by scalar_tac)]
      by_cases hki : k = i1.val
      · simp only [hki, ite_true, show ¬ (i1.val < i1.val) from lt_irrefl _, ite_false]
        rw [fback, dif_pos (by scalar_tac), hv, hv1, hfl]
        rfl
      · simp only [hki, ite_false]
        rw [hdv0 k hk]
        by_cases hkl : k < i1.val
        · simp [hkl, show k < j.val by scalar_tac]
        · simp [hkl, show ¬ k < j.val by scalar_tac]
    · simp only [WP.spec_ok]
      refine ⟨hd0, fun k hk => ?_⟩
      rw [hdv0 k hk]
      simp [show ¬ k < j.val by scalar_tac]
  · exact ⟨hin, hd, hdv⟩

/-- **The shipped `tridiagonal` is the binary64 Thomas algorithm**: on any n ≥ 1 rows it returns
`ok`, n points, point k being `fback rows n k`. -/
theorem tridiagonal_spec (rows : Slice Row) (hn : 0 < rows.length) :
    spline.tridiagonal rows ⦃ x => x.length = rows.length ∧
      ∀ k, k < rows.length → x.val[k]! = fback rows.val rows.length k ⦄ := by
  unfold spline.tridiagonal
  simp only [forge_math.vec.Vec3.ZERO, forge_math.vec.Vec3.Insts.CoreOpsArithDivF64Vec3.div,
    lift, bind_ok]
  step*
  · rfl
  · rfl
  obtain ⟨ta, tb, tc, td⟩ := t
  step*
  subst index_mut_back_post index_mut_back1_post
  have hrow0 : rows.val[0]! = (ta, tb, tc, td) := by
    rw [getElem!_pos rows.val 0 (by simpa using hn), t_post]
  step with sweep_spec
  case hk =>
    intro k hk
    have hk0 : k = 0 := by simp at hk; omega
    subst hk0
    rw [vset_get _ _ _ _ (by scalar_tac), vset_get _ _ _ _ (by scalar_tac)]
    simp only [ite_true, fsweep, hrow0, vdiv]
    exact ⟨rfl, rfl⟩
  all_goals try simp_all
  step*
  apply WP.spec_mono (back_spec rows.val rows.length c2 d2 i (by scalar_tac) (by scalar_tac)
    (by scalar_tac) ?_ ?_)
  · rintro x ⟨hx1, hx2⟩
    refine ⟨by simpa using hx1, fun k hk => ?_⟩
    have := hx2 k hk
    simpa [List.getElem!_eq_getElem?_getD] using this
  · intro k hk
    have := (c2_post2 k (by simpa using hk)).1
    rw [getElem!_pos c2.val k (by scalar_tac)]; exact this
  · intro k hk
    have := (c2_post2 k (by simpa using hk)).2
    rw [getElem!_pos d2.val k (by scalar_tac), this]
    by_cases hki : k < i.val
    · simp only [hki, ite_true]
    · simp only [hki, ite_false]
      rw [fback, dif_neg (by scalar_tac)]

end ForgeGeom
