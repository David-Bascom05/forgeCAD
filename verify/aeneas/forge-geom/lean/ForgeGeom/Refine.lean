/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeGeom.Funs
import SplineFP

/-!
# The real spline evaluation, against verify/ieee754 (L4)

About the Aeneas translation of `kernel/forge-geom` itself (`ForgeGeom/Funs.lean`, checked by
`tools/aeneas/translate.sh forge-geom --check`):
- `lerp_eq`: the shipped `lerp` returns `ok`, each coordinate `SplineFP.lerp` (`a + (b - a) * s`
  in binary64), so `SplineFP.lerp_err_M` bounds it;
- `decasteljau_spec`: the shipped `decasteljau` (both loops, every degree ≤ 15) returns `ok`,
  exactly `dc`: level by level, point k becomes `lerpA` of points k and k+1, each coordinate
  `SplineFP.lerp`, so `SplineFP.dc_step` bounds each level.
-/

open Aeneas Aeneas.Std Result
open forge_geom

namespace ForgeGeom

/-- The model lerp on homogeneous points: `SplineFP.lerp` coordinate by coordinate. -/
def lerpH (a b : List F64) (s : F64) : List F64 :=
  List.zipWith (fun x y => SplineFP.lerp x y s) a b

/-- **The shipped `lerp` is the binary64 lerp**, coordinate by coordinate. -/
theorem lerp_eq (a0 a1 a2 a3 b0 b1 b2 b3 s : F64) :
    spline.lerp (Array.make 4#usize [a0, a1, a2, a3]) (Array.make 4#usize [b0, b1, b2, b3]) s
      = ok (Array.make 4#usize [SplineFP.lerp a0 b0 s, SplineFP.lerp a1 b1 s,
          SplineFP.lerp a2 b2 s, SplineFP.lerp a3 b3 s]) := by
  simp [spline.lerp, SplineFP.lerp, lift, Array.index_usize, F64.Ops.add, F64.Ops.sub, F64.Ops.mul]

/-- The model lerp of two homogeneous points. -/
def lerpA (a b : Array F64 4#usize) (s : F64) : Array F64 4#usize :=
  Array.from (List.zipWith (fun x y => SplineFP.lerp x y s) a.val b.val) (by
    simp [Array.length_eq])

/-- A point of four coordinates is the list of them. -/
theorem four (a : Array F64 4#usize) :
    ∃ a0 a1 a2 a3, a.val = [a0, a1, a2, a3] :=
  List.length_eq_four.1 (by simpa using Array.length_eq a)

@[step]
theorem lerp_spec (a b : Array F64 4#usize) (s : F64) :
    spline.lerp a b s ⦃ r => r = lerpA a b s ⦄ := by
  obtain ⟨a0, a1, a2, a3, ha⟩ := four a
  obtain ⟨b0, b1, b2, b3, hb⟩ := four b
  unfold spline.lerp
  simp [Array.index_usize, ha, hb, lift, WP.spec_ok, lerpA, Array.ext_iff,
    Array.from_val, Array.make_val, SplineFP.lerp, F64.Ops.add, F64.Ops.sub, F64.Ops.mul]
  rfl

instance : Inhabited F64 := ⟨F64.Ops.ofBits 0⟩

/-- The model level: points below r the lerp of themselves and the next, the rest as they were. -/
def level (w : Array (Array F64 4#usize) 16#usize) (r : ℕ) (s : F64) (k : ℕ) : Array F64 4#usize :=
  if k < r then lerpA w.val[k]! w.val[k + 1]! s else w.val[k]!

/-- The inner loop from j: what is left of the level. -/
theorem inner_spec (s : F64) (r : Std.Usize) (w w0 : Array (Array F64 4#usize) 16#usize)
    (j : Std.Usize) (hr : r.val ≤ 15) (hj : j.val ≤ r.val)
    (hdone : ∀ k, k < j.val → w.val[k]! = level w0 r.val s k)
    (hrest : ∀ k, j.val ≤ k → k < 16 → w.val[k]! = w0.val[k]!) :
    spline.decasteljau_loop0_loop0 w s r j ⦃ res => ∀ k, k < 16 → res.val[k]! = level w0 r.val s k ⦄ := by
  unfold spline.decasteljau_loop0_loop0 spline.decasteljau_loop0_loop0.body
  apply loop.spec_decr_nat (fun x => r.val - x.2.val)
    (fun x => x.2.val ≤ r.val ∧ (∀ k, k < x.2.val → x.1.val[k]! = level w0 r.val s k) ∧
      (∀ k, x.2.val ≤ k → k < 16 → x.1.val[k]! = w0.val[k]!))
  · rintro ⟨v, j0⟩ ⟨hi, hd, hre⟩
    simp only at hi hd hre ⊢
    split
    · step*
      have hlen : v.val.length = 16 := by simpa using Array.length_eq v
      have hj0 : j0.val < 16 := by scalar_tac
      have hj1 : j0.val + 1 < 16 := by scalar_tac
      have hset : a3.val = v.val.set j0.val a2 := by subst a3_post; simp [Array.from_val]
      have ha2 : a2 = lerpA w0.val[j0.val]! w0.val[j0.val + 1]! s := by
        rw [a2_post, a_post, a1_post, ← getElem!_pos v.val j0.val (by omega),
          ← getElem!_pos v.val i.val (by scalar_tac), hre j0.val le_rfl hj0, i_post,
          hre (j0.val + 1) (by omega) hj1]
      refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
      · intro k hk
        rw [hset]
        by_cases hkj : k = j0.val
        · subst hkj
          rw [getElem!_pos _ _ (by simp [hlen]; omega), List.getElem_set_self, ha2]
          simp [level, show j0.val < r.val by scalar_tac]
        · rw [getElem!_pos _ _ (by simp [hlen]; scalar_tac), List.getElem_set_ne (Ne.symm hkj),
            ← getElem!_pos _ _ (by rw [hlen]; scalar_tac)]
          exact hd k (by scalar_tac)
      · intro k hk1 hk2
        rw [hset, getElem!_pos _ _ (by simp [hlen]; omega), List.getElem_set_ne (by scalar_tac),
          ← getElem!_pos _ _ (by rw [hlen]; omega)]
        exact hre k (by scalar_tac) hk2
    · simp only [WP.spec_ok]
      intro k hk
      have hir : j0.val = r.val := by scalar_tac
      by_cases hkr : k < r.val
      · exact hd k (by omega)
      · rw [hre k (by omega) hk]; simp [level, hkr]
  · exact ⟨hj, hdone, hrest⟩

/-- The control points of `decasteljau`: 16 homogeneous points. -/
abbrev Pts := Array (Array F64 4#usize) 16#usize

/-- One de Casteljau level as an array: point k is `level w r s k`. -/
def levelArr (w : Pts) (r : Nat) (s : F64) :
    Pts :=
  Array.from ((List.range 16).map (level w r s)) (by simp)

/-- The whole of de Casteljau from level r down: the model the Rust is proven to compute. -/
def dc (s : F64) : Nat → Pts → Pts
  | 0, w => w
  | r + 1, w => dc s r (levelArr w (r + 1) s)

theorem arr_ext16 (a : Pts) (b : Pts)
    (h : ∀ k, k < 16 → a.val[k]! = b.val[k]!) : a = b := by
  apply Aeneas.Std.Array.ext
  apply List.ext_getElem (by simp)
  intro k h1 h2
  have := h k (by simpa using h1)
  simpa [List.getElem?_eq_getElem h1, List.getElem?_eq_getElem h2] using this

theorem levelArr_of (w : Pts) (v : Pts) (r : Nat) (s : F64)
    (h : ∀ k, k < 16 → v.val[k]! = level w r s k) : v = levelArr w r s := by
  apply arr_ext16
  intro k hk
  rw [h k hk]
  simp only [levelArr, Array.from_val]
  rw [getElem!_pos _ _ (by simp; omega)]
  simp [List.getElem_map, List.getElem_range]

/-- The outer loop: the shipped `decasteljau` computes `dc`. -/
theorem outer_spec (s : F64) (w : Pts) (r : Std.Usize)
    (hr : r.val ≤ 15) :
    spline.decasteljau_loop0 w s r ⦃ res => res = dc s r.val w ⦄ := by
  unfold spline.decasteljau_loop0 spline.decasteljau_loop0.body
  apply loop.spec_decr_nat (fun x => x.2.val)
    (fun x => x.2.val ≤ 15 ∧ dc s x.2.val x.1 = dc s r.val w)
  · rintro ⟨v, q⟩ ⟨hq, hdc⟩
    simp only at hq hdc ⊢
    split
    · have hin := inner_spec s q v v 0#usize hq (by simp) (by simp) (by intro k _ _; rfl)
      step*
      have hv : w1 = levelArr v q.val s := levelArr_of v w1 q.val s w1_post
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [← hdc, hv]
      obtain ⟨n, hn⟩ : ∃ n, q.val = n + 1 := ⟨q.val - 1, by scalar_tac⟩
      have : r1.val = n := by scalar_tac
      rw [this, hn, dc]
    · simp only [WP.spec_ok]
      have : q.val = 0 := by scalar_tac
      rw [← hdc, this, dc]
  · exact ⟨hr, rfl⟩

/-- `spline::decasteljau` as shipped, for every degree the kernel allows. -/
theorem decasteljau_spec (w : Pts) (degree : Std.Usize) (s : F64)
    (hd : degree.val ≤ 15) :
    spline.decasteljau w degree s ⦃ res => res = dc s degree.val w ⦄ :=
  outer_spec s w degree hd

end ForgeGeom
