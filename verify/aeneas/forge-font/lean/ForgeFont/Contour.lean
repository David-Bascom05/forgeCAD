import ForgeFont.Walk

open Aeneas Aeneas.Std Result ControlFlow
open forge_font

namespace ForgeFont

/-- Point `k` of the glyph on the doubled grid, as the spec's. -/
def ptAt (g : glyf_core.Simple) (k : Nat) : ForgeText.Pt :=
  ⟨2 * (g.xs.val.getD k 0#i32).val, 2 * (g.ys.val.getD k 0#i32).val⟩

def onAt (g : glyf_core.Simple) (k : Nat) : Bool := g.on.val.getD k false

/-- The points `first ..< first + n` with their on-curve flags. -/
def pts (g : glyf_core.Simple) (first n : Nat) : List (ForgeText.Pt × Bool) :=
  (List.range n).map fun i => (ptAt g (first + i), onAt g (first + i))

@[step]
theorem at_spec (g : glyf_core.Simple) (k : Usize) (hx : k.val < g.xs.val.length)
    (hy : k.val < g.ys.val.length) :
    glyf_core.at g k ⦃ p => toPt p = ptAt g k.val ∧ Grid p ⦄ := by
  unfold glyf_core.at
  step*
  have ex : i1.val = i.val := by rw [i1_post]; simp [IScalar.val_mod_pow_greater_numBits]
  have ey : i4.val = i3.val := by rw [i4_post]; simp [IScalar.val_mod_pow_greater_numBits]
  have bx := i.hBounds
  have by_ := i3.hBounds
  simp only [IScalarTy.numBits] at bx by_
  refine ⟨?_, ?_⟩
  · simp [toPt, ptAt, i2_post, i5_post, ex, ey, i_post, i3_post, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem hx, List.getElem?_eq_getElem hy]
  · simp only [Grid, i2_post, i5_post, ex, ey]
    refine ⟨dvd_mul_right _ _, dvd_mul_right _ _, ?_, ?_, ?_, ?_⟩ <;> omega

/-- The search for the first on point: it stops on one, or at the end with none from `k` on. -/
theorem loop0_spec (g : glyf_core.Simple) (first n : Usize)
    (hn : first.val + n.val ≤ g.on.val.length) :
    glyf_core.contour_loop0 g first n 0#usize false ⦃ g' k' found' =>
      g' = g ∧
      (found' = true → k'.val < n.val ∧ onAt g (first.val + k'.val) = true) ∧
      (∀ j, j < (if found' then k'.val else n.val) → onAt g (first.val + j) = false) ⦄ := by
  unfold glyf_core.contour_loop0
  apply loop.spec_decr_nat (fun (s : Usize × Bool) => n.val - s.1.val + if s.2 then 0 else 1)
    (fun s => s.1.val ≤ n.val ∧ (s.2 = true → s.1.val < n.val ∧ onAt g (first.val + s.1.val) = true) ∧
      ∀ j, j < s.1.val → onAt g (first.val + j) = false)
  · rintro ⟨k, f⟩ ⟨hk, hf, hj⟩
    unfold glyf_core.contour_loop0.body
    simp only at hk hf hj
    cases f
    · simp only [Bool.false_eq_true, ↓reduceIte]
      split
      · rename_i hkn
        have hkn' : k.val < n.val := hkn
        have hon : onAt g (first.val + k.val) = g.on.val[first.val + k.val]'(by omega) := by
          simp only [onAt, List.getD_eq_getElem?_getD]
          rw [List.getElem?_eq_getElem (by omega)]; rfl
        step*
        · simp only [i_post] at b_post
          refine ⟨by omega, by simp, ?_, by simp; omega⟩
          intro j hj'
          by_cases hjk : j < k.val
          · exact hj j hjk
          · have : j = k.val := by omega
            subst this; rw [hon, ← b_post]; simpa using ‹¬b = true›
      · rename_i hkn
        have hkn' : ¬ k.val < n.val := hkn
        simp_all
        intro j hj'; exact hj j (by omega)
    · simp_all
  · simp

/-- The points the walk visits from its `i`th: index `(from + t) % n` of the contour for each
`t ≥ i`, all but the last when the start is an on point (it is the start again). -/
def visit (g : glyf_core.Simple) (first n frm : Nat) (found : Bool) (i : Nat) :
    List (ForgeText.Pt × Bool) :=
  ((List.range' i (n - i)).filter (fun t => !(found && t + 1 == n))).map
    (fun t => (ptAt g (first + (frm + t) % n), onAt g (first + (frm + t) % n)))

theorem visit_end (g : glyf_core.Simple) (first n frm : Nat) (found : Bool) :
    visit g first n frm found n = [] := by
  simp [visit]

theorem visit_succ (g : glyf_core.Simple) (first n frm : Nat) (found : Bool) (i : Nat)
    (h : i < n) :
    visit g first n frm found i =
      (if found && i + 1 == n then []
       else [(ptAt g (first + (frm + i) % n), onAt g (first + (frm + i) % n))]) ++
      visit g first n frm found (i + 1) := by
  have : n - i = (n - (i + 1)) + 1 := by omega
  simp only [visit, this, List.range'_succ, List.filter_cons]
  cases found <;> by_cases hc : i + 1 = n <;> simp_all

/-- **The walk loop is the spec's fold** over the points it visits. -/
theorem loop1_spec (g : glyf_core.Simple) (first n : Usize) (found : Bool) (frm : Usize)
    (w : glyf_core.Walk) (hn : 2 ≤ n.val) (hfrm : frm.val ≤ n.val)
    (hlen : first.val + n.val ≤ g.on.val.length) (hx : g.xs.val.length = g.on.val.length)
    (hy : g.ys.val.length = g.on.val.length) (hbig : 2 * n.val ≤ Usize.max)
    (hc : CtrlOk w) (hl : w.out.val.length + n.val < Usize.max) :
    glyf_core.contour_loop1 g first n found frm w 0#usize ⦃ w' =>
      toWalk w' = (visit g first.val n.val frm.val found 0).foldl ForgeText.step (toWalk w) ∧
      CtrlOk w' ∧ w'.out.val.length ≤ w.out.val.length + n.val ⦄ := by
  unfold glyf_core.contour_loop1
  apply loop.spec_decr_nat (fun (s : glyf_core.Simple × glyf_core.Walk × Usize) => n.val - s.2.2.val)
    (fun s => s.1 = g ∧ s.2.2.val ≤ n.val ∧ CtrlOk s.2.1 ∧
      s.2.1.out.val.length + (n.val - s.2.2.val) ≤ w.out.val.length + n.val ∧
      (visit g first.val n.val frm.val found s.2.2.val).foldl ForgeText.step (toWalk s.2.1) =
        (visit g first.val n.val frm.val found 0).foldl ForgeText.step (toWalk w))
  · rintro ⟨g1, w1, i⟩ ⟨rfl, hi, hc1, hl1, hf⟩
    simp only at hi hc1 hl1 hf
    unfold glyf_core.contour_loop1.body
    dsimp only
    split
    · rename_i hin
      have hin' : i.val < n.val := hin
      rw [visit_succ _ _ _ _ _ _ hin'] at hf
      simp only at hlen hx hy
      have hj : ∀ j : Usize, j.val = first.val + (frm.val + i.val) % n.val → j.val < g1.on.val.length := by
        intro j hj; rw [hj]; have := Nat.mod_lt (frm.val + i.val) (by omega : 0 < n.val); omega
      cases found
      · simp only [Bool.false_and, Bool.false_eq_true, ↓reduceIte, List.singleton_append,
          List.foldl_cons] at hf ⊢
        step*
        have hjl : j.val < g1.on.val.length := hj j (by rw [j_post, i2_post, i1_post])
        have hl2 : w1.out.val.length < Usize.max := by omega
        simp only [Bool.false_eq_true, ↓reduceIte]
        step as ⟨p, hp, hpg⟩
        step as ⟨b, hb⟩
        step with step_spec w1 p b hc1 (fun _ => hpg) hl2 as ⟨w2, hw2, hc2, hl3⟩
        step*
        have hjv : j.val = first.val + (frm.val + i.val) % n.val := by rw [j_post, i2_post, i1_post]
        have hbo : b = onAt g1 j.val := by
          rw [hb]; simp [onAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]
        refine ⟨by omega, hc2, by omega, ?_, by omega⟩
        rw [i3_post, hw2, hp, hbo, hjv]; exact hf
      · by_cases hlast : i.val + 1 = n.val
        · simp only [Bool.true_and, hlast, beq_self_eq_true, ↓reduceIte, List.nil_append] at hf
          step*
          simp only [↓reduceIte]
          step as ⟨i3, hi3⟩
          have hd : decide (i3 = n) = true := by
            simp only [decide_eq_true_eq, IScalar.eq_equiv, UScalar.eq_equiv]; scalar_tac
          simp only [hd, ↓reduceIte]
          step*
          refine ⟨by omega, hc1, by omega, ?_, by omega⟩
          rw [i3_post, hlast, visit_end]
          simpa [hlast, visit_end] using hf
        · simp only [Bool.true_and, beq_iff_eq, hlast, decide_false, Bool.false_eq_true, ↓reduceIte,
            List.singleton_append, List.foldl_cons] at hf
          step*
          simp only [↓reduceIte]
          step as ⟨i3, hi3⟩
          have hd : decide (i3 = n) = false := by
            simp only [decide_eq_false_iff_not, UScalar.eq_equiv]; scalar_tac
          simp only [hd, Bool.false_eq_true, ↓reduceIte]
          have hjl : j.val < g1.on.val.length := hj j (by rw [j_post, i2_post, i1_post])
          have hl2 : w1.out.val.length < Usize.max := by omega
          step as ⟨p, hp, hpg⟩
          step as ⟨b, hb⟩
          step with step_spec w1 p b hc1 (fun _ => hpg) hl2 as ⟨w2, hw2, hc2, hl3⟩
          step*
          have hjv : j.val = first.val + (frm.val + i.val) % n.val := by rw [j_post, i2_post, i1_post]
          have hbo : b = onAt g1 j.val := by
            rw [hb]; simp [onAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]
          refine ⟨by omega, hc2, by omega, ?_, by omega⟩
          rw [i3_post, hw2, hp, hbo, hjv]; exact hf
    · rename_i hin
      have hin' : ¬ i.val < n.val := hin
      have : i.val = n.val := by omega
      simp only [WP.spec_ok]
      rw [this, visit_end] at hf
      exact ⟨by simpa using hf, hc1, by omega⟩
  · simp_all

/-! ## The lists the walk goes over, as the spec's rotations -/

theorem pts_length (g : glyf_core.Simple) (first n : Nat) : (pts g first n).length = n := by
  simp [pts]

theorem pts_getElem (g : glyf_core.Simple) (first n t : Nat) (h : t < (pts g first n).length) :
    (pts g first n)[t] = (ptAt g (first + t), onAt g (first + t)) := by
  simp [pts]

/-- Visiting `(m + t) % n` for `t < c` is the rotation by `m`, cut to `c`. -/
theorem rotate_map (ps : List (ForgeText.Pt × Bool)) (f : Nat → ForgeText.Pt × Bool) (m c : Nat)
    (hm : m ≤ ps.length) (hc : c ≤ ps.length) (hpos : 0 < ps.length)
    (hf : ∀ t (h : t < ps.length), f t = ps[t]) :
    (List.range c).map (fun t => f ((m + t) % ps.length)) = (ps.drop m ++ ps.take m).take c := by
  apply List.ext_getElem
  · simp; omega
  · intro t h1 h2
    simp only [List.length_map, List.length_range] at h1
    simp only [List.getElem_map, List.getElem_range, List.getElem_take]
    rw [hf _ (Nat.mod_lt _ hpos), List.getElem_append]
    split
    · rename_i h
      simp only [List.length_drop] at h
      simp only [List.getElem_drop]
      congr 1; exact Nat.mod_eq_of_lt (by omega)
    · rename_i h
      simp only [List.length_drop] at h
      simp only [List.getElem_take, List.length_drop]
      congr 1
      rw [Nat.mod_eq_sub_mod (by omega), Nat.mod_eq_of_lt (by omega)]
      omega

theorem filter_last (n : Nat) :
    (List.range' 0 n).filter (fun t => !(true && t + 1 == n)) = List.range (n - 1) := by
  cases n with
  | zero => simp
  | succ m =>
    rw [List.range'_concat, List.filter_append, List.range_eq_range']
    simp only [Nat.add_sub_cancel]
    rw [List.filter_eq_self.2]
    · simp
    · intro t ht; simp [List.mem_range'] at ht; simp; omega

theorem filter_none (n : Nat) :
    (List.range' 0 n).filter (fun t => !(false && t + 1 == n)) = List.range n := by
  simp [List.range_eq_range']

theorem visit_on (g : glyf_core.Simple) (first n k : Nat) (hk : k < n) :
    visit g first n (k + 1) true 0 =
      (pts g first n).drop (k + 1) ++ (pts g first n).take k := by
  have hl := pts_length g first n
  have e : visit g first n (k + 1) true 0 =
      (List.range (n - 1)).map (fun t => (fun u => (ptAt g (first + u), onAt g (first + u)))
        ((k + 1 + t) % (pts g first n).length)) := by
    simp only [visit, Nat.sub_zero, filter_last, hl]
  rw [e, rotate_map _ _ _ _ (by omega) (by omega) (by omega)
    (fun t h => (pts_getElem g first n t h).symm), List.take_append]
  simp only [List.length_drop, hl, List.take_take]
  rw [List.take_of_length_le (by simp [hl]; omega)]
  congr 2; omega

theorem visit_off (g : glyf_core.Simple) (first n : Nat) (hn : 1 ≤ n) :
    visit g first n 1 false 0 = (pts g first n).drop 1 ++ (pts g first n).take 1 := by
  have hl := pts_length g first n
  have e : visit g first n 1 false 0 =
      (List.range n).map (fun t => (fun u => (ptAt g (first + u), onAt g (first + u)))
        ((1 + t) % (pts g first n).length)) := by
    simp only [visit, Nat.sub_zero, filter_none, hl]
  rw [e, rotate_map _ _ _ _ (by omega) (by omega) (by omega)
    (fun t h => (pts_getElem g first n t h).symm)]
  rw [List.take_of_length_le (by simp [hl]; omega)]

/-! ## The contour -/

/-- **`contour` is the spec's `contourOf`** over the glyph's points `first..=last` (on the
doubled grid), for a well-formed glyph: the indices in range, the three arrays of one length.
So (`ForgeText.contourOf_closes`) every contour the reader makes is closed. -/
theorem contour_spec (g : glyf_core.Simple) (first last : Usize) (hle : first.val ≤ last.val)
    (hlt : last.val < g.on.val.length) (hx : g.xs.val.length = g.on.val.length)
    (hy : g.ys.val.length = g.on.val.length) (hbig : 2 * g.on.val.length + 2 ≤ Usize.max) :
    glyf_core.contour g first last ⦃ v =>
      v.val.map toSeg = ForgeText.contourOf (pts g first.val (last.val + 1 - first.val)) ⦄ := by
  unfold glyf_core.contour
  step*
  · have : n.val < 2 := by scalar_tac
    simp only [alloc.vec.Vec.new, ForgeText.contourOf, pts_length]
    rw [ite_eq_right_iff.2 (by omega)]; rfl
  · have hn2 : 2 ≤ n.val := by scalar_tac
    have hN : n.val = last.val + 1 - first.val := by omega
    have hlen : first.val + n.val ≤ g.on.val.length := by omega
    have hsmall : 2 * n.val ≤ Usize.max := by omega
    step with loop0_spec g first n hlen as ⟨g1, k, found, hg, hfnd, hoff⟩
    subst g1
    rw [← hN]
    have hif : 2 ≤ (pts g first.val n.val).length := by rw [pts_length]; omega
    cases found
    · simp only [Bool.false_eq_true, ↓reduceIte] at hoff ⊢
      step as ⟨p, hp, hpg⟩
      step as ⟨i1, hi1⟩
      step as ⟨p1, hp1, hpg1⟩
      step as ⟨p2, hp2⟩
      step with loop1_spec g first n false 1#usize
        { at_ := p2, ctrl := none, out := alloc.vec.Vec.new glyf_core.SegI } hn2 (by simp; omega)
        hlen hx hy hsmall (by simp [CtrlOk]) (by simp [alloc.vec.Vec.new]; omega) as ⟨w, hw, hcw, hlw⟩
      step with step_spec w p2 true hcw (by simp) (by simp [alloc.vec.Vec.new] at hlw; omega)
        as ⟨w1, hw1, _, _⟩
      have hnone : (pts g first.val n.val).findIdx? (·.2) = none := by
        rw [List.findIdx?_eq_none_iff]
        intro x hx'
        simp only [pts, List.mem_map, List.mem_range] at hx'
        obtain ⟨t, ht, rfl⟩ := hx'
        exact hoff t ht
      have hs : ForgeText.mid ((pts g first.val n.val).getD 0 (⟨0, 0⟩, true)).1
          ((pts g first.val n.val).getD 1 (⟨0, 0⟩, true)).1 = toPt p2 := by
        rw [hp2, hp, hp1, hi1]
        simp [List.getD_eq_getElem?_getD, pts, show 0 < n.val by omega,
          show 1 < n.val by omega]
      show (toWalk w1).out = _
      rw [hw1, hw, visit_off _ _ _ (by omega), ForgeText.contourOf, ite_eq_left_iff.2 (fun h => absurd hif h), hnone]
      simp only [hs, ForgeText.decode, List.foldl_append, List.foldl_cons, List.foldl_nil, toWalk]
      rfl
    · simp only [↓reduceIte] at hfnd hoff ⊢
      obtain ⟨hkn, hkon⟩ := hfnd trivial
      step as ⟨i1, hi1⟩
      step as ⟨p, hp, hpg⟩
      step as ⟨i2, hi2⟩
      step with loop1_spec g first n true i2
        { at_ := p, ctrl := none, out := alloc.vec.Vec.new glyf_core.SegI } hn2 (by omega)
        hlen hx hy hsmall (by simp [CtrlOk]) (by simp [alloc.vec.Vec.new]; omega) as ⟨w, hw, hcw, hlw⟩
      step with step_spec w p true hcw (by simp) (by simp [alloc.vec.Vec.new] at hlw; omega)
        as ⟨w1, hw1, _, _⟩
      have hsome : (pts g first.val n.val).findIdx? (·.2) = some k.val := by
        rw [List.findIdx?_eq_some_iff_getElem]
        refine ⟨by rw [pts_length]; omega, ?_, ?_⟩
        · simp [pts_getElem, hkon]
        · intro j hj; simp [pts_getElem, hoff j hj]
      have hs : ((pts g first.val n.val).getD k.val (⟨0, 0⟩, true)).1 = toPt p := by
        rw [hp, hi1]
        simp [List.getD_eq_getElem?_getD, pts, hkn]
      show (toWalk w1).out = _
      rw [hw1, hw, hi2, visit_on _ _ _ _ hkn, ForgeText.contourOf, ite_eq_left_iff.2 (fun h => absurd hif h), hsome]
      simp only [hs, ForgeText.decode, List.foldl_append, List.foldl_cons, List.foldl_nil, toWalk]
      rfl

/-- **Every contour the shipped reader walks is closed**: its pieces chain from a start and end
back on it (the translation of `contour`, with `ForgeText.contourOf_closes`). -/
theorem contour_closes (g : glyf_core.Simple) (first last : Usize) (hle : first.val ≤ last.val)
    (hlt : last.val < g.on.val.length) (hx : g.xs.val.length = g.on.val.length)
    (hy : g.ys.val.length = g.on.val.length) (hbig : 2 * g.on.val.length + 2 ≤ Usize.max) :
    glyf_core.contour g first last ⦃ v =>
      ∃ s, ForgeText.Chain s (v.val.map toSeg) ∧ ForgeText.endOf s (v.val.map toSeg) = s ⦄ :=
  WP.spec_mono (contour_spec g first last hle hlt hx hy hbig)
    (fun v h => h ▸ ForgeText.contourOf_closes _)

end ForgeFont
