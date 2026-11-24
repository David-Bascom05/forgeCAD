/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeGeom.Refine

/-!
# The real knot insertion, against its binary64 model (L4)

This is about the Aeneas translation of `insert_knot` itself, in `kernel/forge-geom/src/spline.rs`.

`insert_knot_spec`: whenever its indices are in range, the shipped insertion returns `ok`. The
knots it returns are `U.insertIdx (s + 1) k`. The poles are `insQ`:
- the first `s - p + 1` poles;
- then `lerp P[i-1] P[i] α_i` for `i` in `s - p + 1 ..= s - m`;
- then the poles from `s - m` on.

Here `α_i` is `(k - U[i]) / (U[i+p] - U[i])` in binary64, or 0 where that denominator is not
positive. `s` is the span: the count of knots at most k, less one, capped at the last pole when k is
at or past the last knot. `m` is the multiplicity of k.

In exact arithmetic this is Boehm's insertion (research/spline-loops/lean `SplineSpec.insPoles`,
`insert_curve`), with the ratios of the last `m` poles, which are 0 there, taken as 0. That link is
not yet a theorem here.

One hypothesis comes from the model, not the Rust: Aeneas's `Vec.insert` refuses index = length
(Rust allows it), so the spec asks `s + 1 < U.length`. That always holds for a spline, which has
`U.length = P.length + p + 1` and `s < P.length`.
-/

open Aeneas Aeneas.Std Result
open forge_geom

namespace ForgeGeom

abbrev H := Array F64 4#usize

/-- How many of the knots are at most k (`u[j] <= k` in binary64). -/
def leCount (U : List F64) (k : F64) : ℕ := (U.filter (fun x => F64.Ops.le x k)).length

/-- How many of the knots equal k (`u[j] <= k && u[j] == k`). -/
def multCount (U : List F64) (k : F64) : ℕ :=
  (U.filter (fun x => F64.Ops.le x k && F64.Ops.eq x k)).length

theorem leCount_take_succ (U : List F64) (k : F64) (j : ℕ) (hj : j < U.length) :
    leCount (U.take (j + 1)) k = leCount (U.take j) k + if F64.Ops.le U[j] k then 1 else 0 := by
  simp only [leCount, List.take_add_one, List.getElem?_eq_getElem hj, Option.toList_some,
    List.filter_append, List.length_append]
  rw [List.filter_singleton]; cases F64.Ops.le U[j] k <;> rfl

theorem multCount_take_succ (U : List F64) (k : F64) (j : ℕ) (hj : j < U.length) :
    multCount (U.take (j + 1)) k = multCount (U.take j) k +
      if F64.Ops.le U[j] k && F64.Ops.eq U[j] k then 1 else 0 := by
  simp only [multCount, List.take_add_one, List.getElem?_eq_getElem hj, Option.toList_some,
    List.filter_append, List.length_append]
  rw [List.filter_singleton]; cases (F64.Ops.le U[j] k && F64.Ops.eq U[j] k) <;> rfl

theorem leCount_le (U : List F64) (k : F64) : leCount U k ≤ U.length := List.length_filter_le _ _

/-- The counting loop. -/
theorem count_spec (u : alloc.vec.Vec F64) (k : F64) (le mult j : Std.Usize)
    (hj : j.val ≤ u.length) (hle : le.val = leCount (u.val.take j.val) k)
    (hm : mult.val = multCount (u.val.take j.val) k) :
    spline.insert_knot_loop0 u k le mult j ⦃ le' mult' =>
      le'.val = leCount u.val k ∧ mult'.val = multCount u.val k ⦄ := by
  unfold spline.insert_knot_loop0 spline.insert_knot_loop0.body
  apply loop.spec_decr_nat (fun x => u.length - x.2.2.val)
    (fun x => x.2.2.val ≤ u.length ∧ x.1.val = leCount (u.val.take x.2.2.val) k ∧
      x.2.1.val = multCount (u.val.take x.2.2.val) k)
  · rintro ⟨l0, m0, j0⟩ ⟨hj0, hl0, hm0⟩
    simp only at hj0 hl0 hm0 ⊢
    have hlm : l0.val ≤ j0.val := by
      rw [hl0]; have := leCount_le (u.val.take j0.val) k; simp at this; omega
    have hmm : m0.val ≤ j0.val := by
      rw [hm0]; have := List.length_filter_le (fun x => F64.Ops.le x k && F64.Ops.eq x k)
        (u.val.take j0.val); simp only [List.length_take] at this; simp only [multCount]; omega
    split
    · rename_i hlt
      have hjl : j0.val < u.val.length := by scalar_tac
      step*
      have hL := leCount_take_succ u.val k j0.val hjl
      have hM := multCount_take_succ u.val k j0.val hjl
      rw [← fl_post] at hL hM
      split
      · rename_i hb
        step*
        split
        · rename_i he
          step*
        · rename_i he
          step*
          simp only [hb, he, Bool.and_false, ite_true] at hL hM
          refine ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac⟩
      · rename_i hb
        step*
        simp only [hb, Bool.false_and, ite_false] at hL hM
        refine ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac⟩
    · simp only [WP.spec_ok, WP.uncurry']
      have : j0.val = u.val.length := by scalar_tac
      rw [hl0, hm0, this]
      simp
  · exact ⟨hj, hle, hm⟩

/-- The insertion ratio at pole i in binary64: `(k - U[i]) / (U[i+p] - U[i])`, 0 on an empty span. -/
def alphaF (U : List F64) (d : ℕ) (k : F64) (i : ℕ) : F64 :=
  let den := F64.Ops.sub U[i + d]! U[i]!
  if F64.Ops.gt den (F64.Ops.ofBits 0) then F64.Ops.div (F64.Ops.sub k U[i]!) den
  else F64.Ops.ofBits 0

/-- The new pole i: `lerp P[i-1] P[i] α_i`. -/
def newPt (U : List F64) (P : List H) (d : ℕ) (k : F64) (i : ℕ) : H :=
  lerpA P[i - 1]! P[i]! (alphaF U d k i)

/-- The lerp loop: poles lo ..= s - m. -/
theorem lerps_spec (u : alloc.vec.Vec F64) (p : alloc.vec.Vec H) (degree : Std.Usize) (k : F64)
    (mult s : Std.Usize) (q : alloc.vec.Vec H) (i : Std.Usize) (lo : ℕ) (Q0 : List H)
    (hlo : lo ≤ i.val) (hhi : i.val ≤ max lo (s.val - mult.val + 1)) (hms : mult.val ≤ s.val)
    (hb : ∀ j, lo ≤ j → j ≤ s.val - mult.val → j + degree.val < u.length ∧ j < p.length ∧ 1 ≤ j)
    (hq : q.val = Q0 ++ (List.range' lo (i.val - lo)).map (newPt u.val p.val degree.val k))
    (hsz : Q0.length + (s.val - mult.val + 1 - lo) < Usize.max) :
    spline.insert_knot_loop1 u p degree k mult s q i ⦃ q' =>
      q'.val = Q0 ++ (List.range' lo (s.val - mult.val + 1 - lo)).map
        (newPt u.val p.val degree.val k) ⦄ := by
  unfold spline.insert_knot_loop1 spline.insert_knot_loop1.body
  apply loop.spec_decr_nat (fun x => s.val - mult.val + 1 - x.2.val)
    (fun x => lo ≤ x.2.val ∧ x.2.val ≤ max lo (s.val - mult.val + 1) ∧
      x.1.val = Q0 ++ (List.range' lo (x.2.val - lo)).map (newPt u.val p.val degree.val k))
  · rintro ⟨q0, i0⟩ ⟨hlo0, hhi0, hq0⟩
    simp only at hlo0 hhi0 hq0 ⊢
    step*
    · obtain ⟨hd, hp, h1⟩ := hb i0.val hlo0 (by scalar_tac)
      have hU1 : fl = u.val[i0.val + degree.val]! := by
        rw [fl_post, getElem!_pos u.val (i0.val + degree.val) (by scalar_tac)]; simp only [i2_post]
      have hU0 : fl1 = u.val[i0.val]! := by
        rw [fl1_post, getElem!_pos u.val i0.val (by scalar_tac)]
      simp only [lift, bind_ok]
      split
      all_goals (
        step*
        case h =>
          have hql := congrArg List.length hq0
          simp only [List.length_append, List.length_map, List.length_range'] at hql
          scalar_tac
        have hP1 : a1 = p.val[i0.val - 1]! := by
          rw [a1_post, getElem!_pos p.val (i0.val - 1) (by scalar_tac)]; simp only [i3_post]
        have hP0 : a2 = p.val[i0.val]! := by
          rw [a2_post, getElem!_pos p.val i0.val (by scalar_tac)]
        have hnew : a3 = newPt u.val p.val degree.val k i0.val := by
          rw [a3_post, hP1, hP0]; unfold newPt alphaF; rw [← hU1, ← hU0]; simp_all
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        rw [q1_post, hq0, hnew, show i4.val - lo = (i0.val - lo) + 1 by scalar_tac,
          List.range'_concat, List.map_append, List.append_assoc]
        simp only [List.map_cons, List.map_nil]
        congr 4
        omega)
    · rw [hq0, show i0.val - lo = s.val - mult.val + 1 - lo by scalar_tac]
  · exact ⟨hlo, hhi, hq⟩

theorem mapM_id {α : Type} (f : α → Result α) (hf : ∀ x, f x = ok x) (l : List α) :
    List.mapM_with_length f l = ok ⟨l, rfl⟩ := by
  induction l with
  | nil => rfl
  | cons a t ih => simp [List.mapM_with_length, ih, hf, pure]

theorem mapM_ok (l : List F64) :
    List.mapM_with_length F64.Insts.CoreCloneClone.clone l = ok ⟨l, rfl⟩ :=
  mapM_id _ (fun _ => rfl) l

theorem clone_H (a : H) :
    core.array.CloneArray.clone F64.Insts.CoreCloneClone a = ok a := by
  simp [core.array.CloneArray.clone, Array.clone, List.clone, mapM_ok]

/-- `extend_from_slice` of poles appends them (each clone is the pole itself). -/
@[step]
theorem efs_spec (v : alloc.vec.Vec H) (s : Slice H) (h : v.length + s.length ≤ Usize.max) :
    alloc.vec.Vec.extend_from_slice (core.clone.CloneArray 4#usize F64.Insts.CoreCloneClone) v s
      ⦃ v' => v'.val = v.val ++ s.val ⦄ := by
  unfold alloc.vec.Vec.extend_from_slice
  rw [dif_pos h]
  simp [List.clone, mapM_id _ clone_H, WP.spec_ok]

/-- The span of k: the knots at most k, less one, capped at the last pole when k is at or past the
last knot. -/
def spanK (U : List F64) (P : List H) (k : F64) : ℕ :=
  if F64.Ops.ge k U[U.length - 1]! then min (leCount U k - 1) (P.length - 1) else leCount U k - 1

/-- The poles after inserting k (`insert_knot`'s `q`). -/
def insQ (U : List F64) (P : List H) (d : ℕ) (k : F64) : List H :=
  let s := spanK U P k
  let m := multCount U k
  P.take (s - d + 1) ++ (List.range' (s - d + 1) (s - m + 1 - (s - d + 1))).map (newPt U P d k)
    ++ P.drop (s - m)

/-- **The shipped `insert_knot` is the binary64 Boehm insertion**, whenever its indices are in
range: the knots get k after the span, and the poles are `insQ`. -/
theorem insert_knot_spec (u : alloc.vec.Vec F64) (p : alloc.vec.Vec H) (degree : Std.Usize)
    (k : F64)
    (hU : 0 < u.length) (hle : 1 ≤ leCount u.val k) (hP : 0 < p.length)
    (hd : degree.val ≤ spanK u.val p.val k) (hm : multCount u.val k ≤ spanK u.val p.val k)
    (hb : ∀ j, spanK u.val p.val k - degree.val + 1 ≤ j → j ≤ spanK u.val p.val k - multCount u.val k →
      j + degree.val < u.length ∧ j < p.length)
    (hdrop : spanK u.val p.val k - multCount u.val k ≤ p.length)
    (htake : spanK u.val p.val k - degree.val + 1 ≤ p.length)
    (hins : spanK u.val p.val k + 1 < u.length)
    (hsz : 2 * p.length + u.length + 2 < Usize.max) :
    spline.insert_knot u p degree k ⦃ u' q' =>
      u'.val = u.val.insertIdx (spanK u.val p.val k + 1) k ∧
      q'.val = insQ u.val p.val degree.val k ⦄ := by
  unfold spline.insert_knot
  step with count_spec u k 0#usize 0#usize 0#usize (by simp) (by simp [leCount]) (by simp [multCount])
  step*
  have hfl : fl = u.val[u.val.length - 1]! := by
    rw [fl_post, getElem!_pos u.val (u.val.length - 1) (by scalar_tac)]; simp only [i1_post]; rfl
  by_cases hge : F64.Ops.ge k fl
  · simp only [hge, ite_true]
    step
    generalize hS : core.cmp.impls.OrdUsize.min s s1 = S
    have hsp : S.val = spanK u.val p.val k := by
      rw [← hS, core.cmp.impls.OrdUsize.min_val]
      simp only [spanK, ← hfl, hge, ite_true, s_post, le_post, s1_post]
      scalar_tac
    simp only [← hsp, ← le_post1] at hb hdrop htake hins hd hm
    step*
    case h =>
      have h0 : (alloc.vec.Vec.with_capacity (Std.Array F64 4#usize) i3).length = 0 := rfl
      rw [h0]; scalar_tac
    have hq1 : q1.val = p.val.take (S.val - degree.val + 1) := by
      rw [q1_post, s2_post]
      simp [alloc.vec.Vec.with_capacity, alloc.vec.Vec.new, List.slice, i5_post, i4_post]
    step with lerps_spec u p degree k mult S q1 i6 (S.val - degree.val + 1)
      (p.val.take (S.val - degree.val + 1))
    case hhi => scalar_tac
    case hq => rw [hq1, show i6.val - (S.val - degree.val + 1) = 0 by scalar_tac]; simp
    case hsz => simp only [List.length_take]; scalar_tac
    step*
    case h =>
      have hl := congrArg List.length q2_post
      simp only [List.length_append, List.length_map, List.length_range', List.length_take] at hl
      scalar_tac
    refine ⟨by rw [u1_post, ← hsp]; congr 1, ?_⟩
    rw [q3_post, q2_post, s3_post]
    simp only [insQ, ← hsp, ← le_post1, List.append_assoc]
    rw [show i7.val = S.val - mult.val by scalar_tac]
  · simp only [hge, Bool.false_eq_true, ite_false]
    have hsp : s.val = spanK u.val p.val k := by
      simp only [spanK, ← hfl, hge, Bool.false_eq_true, ite_false, s_post, le_post]
    simp only [← hsp, ← le_post1] at hb hdrop htake hins hd hm
    step*
    case h =>
      have h0 : (alloc.vec.Vec.with_capacity (Std.Array F64 4#usize) i3).length = 0 := rfl
      rw [h0]; scalar_tac
    have hq1 : q1.val = p.val.take (s.val - degree.val + 1) := by
      rw [q1_post, s2_post]
      simp [alloc.vec.Vec.with_capacity, alloc.vec.Vec.new, List.slice, i5_post, i4_post]
    step with lerps_spec u p degree k mult s q1 i6 (s.val - degree.val + 1)
      (p.val.take (s.val - degree.val + 1))
    case hhi => scalar_tac
    case hq => rw [hq1, show i6.val - (s.val - degree.val + 1) = 0 by scalar_tac]; simp
    case hsz => simp only [List.length_take]; scalar_tac
    step*
    case h =>
      have hl := congrArg List.length q2_post
      simp only [List.length_append, List.length_map, List.length_range', List.length_take] at hl
      scalar_tac
    refine ⟨by rw [u1_post, ← hsp]; congr 1, ?_⟩
    rw [q3_post, q2_post, s3_post]
    simp only [insQ, ← hsp, ← le_post1, List.append_assoc]
    rw [show i7.val = s.val - mult.val by scalar_tac]

/-- **`insQ` position by position**: pole i is `P[i]` up to `s - p`, `newPt i` (the lerp) up to
`s - m`, and `P[i-1]` after. This is `SplineSpec.kernelPoles` with each operation the binary64
one, and `SplineSpec.kernelPoles_eq` proves that formula is Boehm's `insPoles` in exact
arithmetic. -/
theorem insQ_get (U : List F64) (P : List H) (d : ℕ) (k : F64)
    (hmp : multCount U k ≤ d) (hd : d ≤ spanK U P k)
    (htake : spanK U P k - d + 1 ≤ P.length) (i : ℕ) (hi : i < P.length + 1) :
    (insQ U P d k)[i]! =
      if i ≤ spanK U P k - d then P[i]!
      else if i ≤ spanK U P k - multCount U k then newPt U P d k i
      else P[i - 1]! := by
  simp only [insQ]
  generalize spanK U P k = s at *
  generalize multCount U k = m at *
  have hl1 : (P.take (s - d + 1)).length = s - d + 1 := by simp; omega
  have hl2 : ((List.range' (s - d + 1) (s - m + 1 - (s - d + 1))).map (newPt U P d k)).length
      = d - m := by simp; omega
  simp only [List.getElem!_eq_getElem?_getD, List.append_assoc]
  by_cases ha : i ≤ s - d
  · rw [List.getElem?_append_left (by omega)]
    simp [ha, List.getElem?_take, show i < s - d + 1 by omega]
  · rw [List.getElem?_append_right (by omega), hl1]
    simp only [ha, ite_false]
    by_cases hb : i ≤ s - m
    · rw [List.getElem?_append_left (by omega)]
      simp only [hb, ite_true, List.getElem?_map]
      rw [List.getElem?_range' (by omega)]
      simp only [Option.map_some, Option.getD_some]
      congr 1; omega
    · rw [List.getElem?_append_right (by omega), hl2]
      simp only [hb, ite_false, List.getElem?_drop]
      congr 2; omega

end ForgeGeom
