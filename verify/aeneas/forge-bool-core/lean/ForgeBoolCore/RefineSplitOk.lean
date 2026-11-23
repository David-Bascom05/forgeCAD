import ForgeBoolCore.Specs
open Aeneas Aeneas.Std Result

/-!
# Group S: `split_ok` is the Lean `splitOk`

`sort_triples` is the bottom-up merge sort of `sort_u32` on `(cell, body, face)` triples, ordered by
`le3` (lexicographic, `SRt`). `split_ok` checks each piece, sorts the triples, and refuses if two
neighbours share cell and body but not face. A sorted list has such a neighbour pair exactly when
some two of its entries do (`s_adj_iff`), which is the Lean `splitOk`'s inner `all`.

The sort's arithmetic is bounded by `2·n ≤ usize::MAX` (not `3·n`): `lo + 2·width < 2·n`, since
`lo` is a multiple of `2·width` below `n`. `LedgerWF.room` gives `2·pieces < u32::MAX`.
-/

namespace forge_bool_core

open ForgeBoolCore.Sort

abbrev STri := U32 × U32 × U32

/-- `le3`, on values. -/
def SRt (a b : STri) : Prop :=
  a.1.val < b.1.val ∨ (a.1.val = b.1.val ∧
    (a.2.1.val < b.2.1.val ∨ (a.2.1.val = b.2.1.val ∧ a.2.2.val ≤ b.2.2.val)))

instance (a b : STri) : Decidable (SRt a b) := by unfold SRt; infer_instance

theorem s_rt_trans : ∀ a b c, SRt a b → SRt b c → SRt a c := by
  intro a b c h1 h2; unfold SRt at *; omega

theorem s_rt_total (a b : STri) (h : ¬ SRt a b) : SRt b a := by
  unfold SRt at *; omega

theorem s_get_some {l : List STri} {i : Nat} (h : i < l.length) : l[i]? = some l[i] :=
  List.getElem?_eq_getElem h

theorem s_le3_spec (a b : STri) : sort.le3 a b ⦃ r => r = decide (SRt a b) ⦄ := by
  obtain ⟨a1, a2, a3⟩ := a
  obtain ⟨b1, b2, b3⟩ := b
  show (if a1 < b1 then ok true else if a1 = b1 then (if a2 < b2 then ok true
    else if a2 = b2 then ok (decide (a3 ≤ b3)) else ok false) else ok false) ⦃ r => _ ⦄
  have e2 : (a1 = b1) ↔ a1.val = b1.val := by scalar_tac
  have e4 : (a2 = b2) ↔ a2.val = b2.val := by scalar_tac
  simp only [UScalar.lt_equiv, UScalar.le_equiv, e2, e4, SRt]
  by_cases h1 : a1.val < b1.val <;> by_cases h2 : a1.val = b1.val <;>
    by_cases h3 : a2.val < b2.val <;> by_cases h4 : a2.val = b2.val <;>
    simp [h1, h2, h3, h4]

/-- No condition on the values (the merge reads no cells). -/
def s_IdsIn (_ : List STri) : Prop := True

theorem s_merge_spec (src D0 dst : alloc.vec.Vec STri)
    (lo mid hi i j k : Usize) (hm : MInv SRt src.val D0.val dst.val lo.val mid.val hi.val i.val j.val k.val)
    (hsl : (seg src.val lo.val mid.val).Pairwise SRt)
    (hsr : (seg src.val mid.val hi.val).Pairwise SRt) (hids : s_IdsIn src.val) :
    sort.sort_triples_loop0_loop0_loop0 src dst mid hi i j k ⦃ r =>
      ∃ out, r.val = D0.val.take lo.val ++ out ++ D0.val.drop hi.val ∧
        out.Perm (seg src.val lo.val hi.val) ∧ out.Pairwise SRt ⦄ := by
  unfold sort.sort_triples_loop0_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec STri × Usize × Usize × Usize) => hi.val - q.2.2.2.val)
    (fun q => MInv SRt src.val D0.val q.1.val lo.val mid.val hi.val q.2.1.val q.2.2.1.val q.2.2.2.val)
  · rintro ⟨d, a, b, c⟩ hinv
    unfold sort.sort_triples_loop0_loop0_loop0.body
    simp only at hinv ⊢
    have hinv' := hinv
    obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk, out, heq, hperm, _, _⟩ := hinv'
    have hdl : d.val.length = src.val.length := by
      have := hperm.length_eq
      rw [List.length_append, seg_length _ hlo (by omega), seg_length _ hmj (by omega)] at this
      rw [heq]; simp; omega
    split
    · rename_i hkh
      have hc : c.val < hi.val := by scalar_tac
      have hcd : c.val < d.val.length := by omega
      split
      · rename_i hamid
        have ha : a.val < mid.val := by scalar_tac
        split
        · rename_i hbhi
          have hb : hi.val ≤ b.val := by scalar_tac
          step as ⟨y, hy⟩
          step as ⟨w, hw, hback⟩
          obtain ⟨w1, back⟩ := w
          simp only at hw hback ⊢
          step as ⟨a1, ha1⟩
          step as ⟨c1, hc1⟩
          refine ⟨?_, by scalar_tac⟩
          rw [hback, alloc.vec.Vec.set_val_eq, ha1, hc1, hy]
          exact minv_left s_rt_trans hinv hsl ha (s_get_some (by omega)) (fun h => absurd h (by omega))
        · rename_i hbhi
          have hb : b.val < hi.val := by scalar_tac
          step as ⟨x1, hx1⟩
          step as ⟨y1, hy1⟩
          step with s_le3_spec as ⟨bb, hbb⟩
          split
          · rename_i hle
            have hle' : SRt src.val[a.val] src.val[b.val] := by
              rw [← hx1, ← hy1]; subst hbb; simpa using hle
            step as ⟨w, hw, hback⟩
            obtain ⟨w1, back⟩ := w
            simp only at hw hback ⊢
            step as ⟨a1, ha1⟩
            step as ⟨c1, hc1⟩
            refine ⟨?_, by scalar_tac⟩
            rw [hback, alloc.vec.Vec.set_val_eq, ha1, hc1, hx1]
            refine minv_left s_rt_trans hinv hsl ha (s_get_some (by omega)) (fun _ z hz => ?_)
            have hz' : z = src.val[b.val] := by rw [s_get_some (by omega)] at hz; exact (Option.some.inj hz).symm
            subst hz'; exact hle'
          · rename_i hgt
            have hgt' : SRt src.val[b.val] src.val[a.val] := by
              rw [← hx1, ← hy1]; subst hbb; exact s_rt_total _ _ (by simpa using hgt)
            step as ⟨w, hw, hback⟩
            obtain ⟨w1, back⟩ := w
            simp only at hw hback ⊢
            step as ⟨b1, hb1⟩
            step as ⟨c1, hc1⟩
            refine ⟨?_, by scalar_tac⟩
            rw [hback, alloc.vec.Vec.set_val_eq, hb1, hc1, hy1]
            refine minv_right s_rt_trans hinv hsr hb (s_get_some (by omega)) (fun _ z hz => ?_)
            have hz' : z = src.val[a.val] := by rw [s_get_some (by omega)] at hz; exact (Option.some.inj hz).symm
            subst hz'; exact hgt'
      · rename_i hamid
        have ha : mid.val ≤ a.val := by scalar_tac
        have hb : b.val < hi.val := by omega
        step as ⟨y, hy⟩
        step as ⟨w, hw, hback⟩
        obtain ⟨w1, back⟩ := w
        simp only at hw hback ⊢
        step as ⟨b1, hb1⟩
        step as ⟨c1, hc1⟩
        refine ⟨?_, by scalar_tac⟩
        rw [hback, alloc.vec.Vec.set_val_eq, hb1, hc1, hy]
        exact minv_right s_rt_trans hinv hsr hb (s_get_some (by omega)) (fun h => absurd h (by omega))
    · rename_i hkh
      simp only [WP.spec_ok]
      exact minv_done hinv (by scalar_tac)
  · exact hm

/-- The middle loop: blocks of width `2w` merged up to `lo`. -/
def S_MidInv (src D0 dst : List STri) (w lo : Nat) : Prop :=
  lo ≤ src.length ∧ ((∃ b, lo = b * (2 * w)) ∨ lo = src.length) ∧
  ∃ out, dst = out ++ D0.drop lo ∧ out.length = lo ∧ out.Perm (src.take lo) ∧ Blocks (SRt) (2 * w) out

theorem s_drop_of_out {out D0 : List STri} {lo hi : Nat} (hl : out.length = lo) (h : lo ≤ hi) :
    (out ++ D0.drop lo).drop hi = D0.drop hi := by
  rw [List.drop_append, List.drop_of_length_le (by omega), List.nil_append, List.drop_drop, hl]
  congr 1; omega

/-- One block's merge, from the middle loop's invariant to the next. -/
theorem s_mid_step (n : Usize) (src D0 d : alloc.vec.Vec STri)
    (w l mid hi : Usize) (hn : n.val = src.val.length) (hD0 : D0.val.length = n.val) (hw : 0 < w.val)
    (hb : Blocks (SRt) w.val src.val) (hids : s_IdsIn src.val)
    (hlt : l.val < n.val) (b : Nat) (hbl : l.val = b * (2 * w.val))
    (hmidv : mid.val = min (l.val + w.val) n.val) (hhiv : hi.val = min (l.val + 2 * w.val) n.val)
    (out : List STri) (hdeq : d.val = out ++ D0.val.drop l.val) (hol : out.length = l.val)
    (hperm : out.Perm (src.val.take l.val)) (hblk : Blocks (SRt) (2 * w.val) out) :
    (do
      let dst1 ← sort.sort_triples_loop0_loop0_loop0 src d mid hi l mid l
      ok (ControlFlow.cont (dst1, hi))) ⦃ r =>
      match r with
      | ControlFlow.done (y : alloc.vec.Vec STri) =>
        y.val.Perm src.val ∧ Blocks (SRt) (2 * w.val) y.val ∧ y.val.length = n.val
      | ControlFlow.cont (x' : alloc.vec.Vec STri × Usize) =>
        S_MidInv src.val D0.val x'.1.val w.val x'.2.val ∧ n.val - x'.2.val < n.val - l.val ⦄ := by
  have hdl : d.val.length = src.val.length := by rw [hdeq]; simp; omega
  have hlm : l.val ≤ mid.val := by rw [hmidv]; omega
  have hmh : mid.val ≤ hi.val := by rw [hmidv, hhiv]; omega
  have hhn : hi.val ≤ src.val.length := by rw [hhiv]; omega
  -- the two runs
  have hrl : (seg src.val l.val mid.val).Pairwise (SRt) := by
    have := run_of_blocks hb (2 * b)
    rw [show 2 * b * w.val = l.val by rw [hbl]; rw [Nat.mul_comm 2 b, Nat.mul_assoc],
      show (2 * b + 1) * w.val = l.val + w.val by rw [hbl, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc],
      ← hn, ← hmidv] at this
    exact this
  have hrr : (seg src.val mid.val hi.val).Pairwise (SRt) := by
    by_cases hc : l.val + w.val < n.val
    · have hm' : mid.val = (2 * b + 1) * w.val := by
        rw [hmidv, Nat.min_eq_left (by omega), hbl, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc]
      have := run_of_blocks hb (2 * b + 1)
      rw [← hm', show (2 * b + 1 + 1) * w.val = l.val + 2 * w.val by
        rw [hbl, Nat.add_mul, Nat.add_mul, Nat.one_mul, Nat.mul_comm 2 b, Nat.mul_assoc]; omega, ← hn, ← hhiv] at this
      exact this
    · have : mid.val = hi.val := by rw [hmidv, hhiv]; omega
      rw [this, seg_self]; exact List.Pairwise.nil
  have hstart := minv_start (R := SRt) src.val d.val hlm hmh hhn hdl
  apply WP.spec_bind (s_merge_spec src d d l mid hi l mid l hstart hrl hrr hids)
  intro d1 hd1
  obtain ⟨m, hd1, hmp, hms⟩ := hd1
  simp only [WP.spec_ok]
  refine ⟨⟨hhn, ?_, out ++ m, ?_, ?_, ?_, ?_⟩, ?_⟩
  · by_cases hc : l.val + 2 * w.val < n.val
    · left; exact ⟨b + 1, by rw [hhiv, Nat.min_eq_left (by omega), hbl, Nat.add_mul, Nat.one_mul]⟩
    · right; rw [hhiv, ← hn, Nat.min_eq_right (by omega)]
  · rw [hd1, hdeq]
    have e1 : (out ++ D0.val.drop l.val).take l.val = out := by
      rw [List.take_append_of_le_length (by omega), List.take_of_length_le (by omega)]
    rw [e1, s_drop_of_out (D0 := D0.val) hol (show l.val ≤ hi.val by omega)]
  · have := hmp.length_eq; rw [seg_length _ (by omega) hhn] at this; simp; omega
  · rw [take_eq_seg, seg_append src.val (Nat.zero_le _) (by omega : l.val ≤ hi.val), ← take_eq_seg]
    exact hperm.append hmp
  · apply blocks_append (by omega) hblk b (by rw [hol, hbl]) hms
    have := hmp.length_eq; rw [seg_length _ (by omega) hhn] at this; rw [this, hhiv]; omega
  · rw [hhiv]; omega

theorem s_mid_loop_spec (n : Usize) (src D0 dst : alloc.vec.Vec STri)
    (w lo : Usize) (hn : n.val = src.val.length) (hD0 : D0.val.length = n.val) (hw : 0 < w.val)
    (hwn : w.val < n.val) (hmax : 2 * n.val ≤ Usize.max) (hb : Blocks (SRt) w.val src.val)
    (hids : s_IdsIn src.val) (hinv : S_MidInv src.val D0.val dst.val w.val lo.val) :
    sort.sort_triples_loop0_loop0 n src dst w lo ⦃ r =>
      r.val.Perm src.val ∧ Blocks (SRt) (2 * w.val) r.val ∧ r.val.length = n.val ⦄ := by
  unfold sort.sort_triples_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec STri × Usize) => n.val - q.2.val)
    (fun q => S_MidInv src.val D0.val q.1.val w.val q.2.val)
  · rintro ⟨d, l⟩ hm
    unfold sort.sort_triples_loop0_loop0.body
    simp only at hm ⊢
    obtain ⟨hln, hal, out, hdeq, hol, hperm, hblk⟩ := hm
    split
    · rename_i hlt
      have hlt' : l.val < n.val := by scalar_tac
      obtain ⟨b, hbl⟩ : ∃ b, l.val = b * (2 * w.val) := by
        rcases hal with h | h
        · exact h
        · omega
      have h1 : l.val + w.val ≤ Usize.max := by omega
      step as ⟨i, hi⟩
      have h2 : 2 * w.val ≤ Usize.max := by omega
      have hl2 : l.val = 0 ∨ 2 * w.val ≤ l.val := by
        cases b with
        | zero => left; rw [hbl]; simp
        | succ b => right; rw [hbl]; exact Nat.le_mul_of_pos_left _ (Nat.succ_pos _)
      have h3 : l.val + 2 * w.val ≤ Usize.max := by omega
      by_cases hin : i < n
      · simp only [hin, ite_true, Std.bind_ok]
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        have hmv : i.val = min (l.val + w.val) n.val := by
          have : i.val < n.val := by scalar_tac
          rw [Nat.min_eq_left (by omega)]; omega
        by_cases hin2 : i2 < n
        · simp only [hin2, ite_true]
          step as ⟨i3, hi3⟩
          refine WP.spec_mono (s_mid_step n src D0 d w l i i3 hn hD0 hw hb hids hlt' b hbl hmv
            (by have : i2.val < n.val := hin2
                rw [Nat.min_eq_left (by omega)]; omega) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
        · simp only [hin2, ite_false, Std.bind_ok]
          refine WP.spec_mono (s_mid_step n src D0 d w l i n hn hD0 hw hb hids hlt' b hbl hmv
            (by have : ¬i2.val < n.val := hin2
                rw [Nat.min_eq_right (by omega)]) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
      · simp only [hin, ite_false, Std.bind_ok]
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        have hmv : n.val = min (l.val + w.val) n.val := by
          have : ¬i.val < n.val := by scalar_tac
          rw [Nat.min_eq_right (by omega)]
        by_cases hin2 : i2 < n
        · simp only [hin2, ite_true]
          step as ⟨i3, hi3⟩
          refine WP.spec_mono (s_mid_step n src D0 d w l n i3 hn hD0 hw hb hids hlt' b hbl hmv
            (by have : i2.val < n.val := hin2
                rw [Nat.min_eq_left (by omega)]; omega) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
        · simp only [hin2, ite_false, Std.bind_ok]
          refine WP.spec_mono (s_mid_step n src D0 d w l n n hn hD0 hw hb hids hlt' b hbl hmv
            (by have : ¬i2.val < n.val := hin2
                rw [Nat.min_eq_right (by omega)]) out hdeq hol hperm hblk) ?_
          intro r hr; cases r <;> exact hr
    · rename_i hge
      simp only [WP.spec_ok]
      have hl : l.val = n.val := by scalar_tac
      rw [hl] at hdeq hol hperm
      have hdrop : D0.val.drop n.val = [] := List.drop_of_length_le (by omega)
      rw [hdeq, hdrop, List.append_nil]
      refine ⟨?_, hblk, by omega⟩
      rw [hn, List.take_of_length_le (Nat.le_refl _)] at hperm; exact hperm
  · exact hinv

theorem s_idsIn_perm {l l' : List STri} (h : l.Perm l') (hl : s_IdsIn l') :
    s_IdsIn l := trivial

theorem s_outer_loop_spec (n : Usize) (v0 : List STri)
    (src dst : alloc.vec.Vec STri) (w : Usize) (hn : n.val = src.val.length) (hdn : dst.val.length = n.val)
    (hw : 0 < w.val) (hmax : 2 * n.val ≤ Usize.max) (hp : src.val.Perm v0)
    (hb : Blocks (SRt) w.val src.val) (hids : s_IdsIn v0) :
    sort.sort_triples_loop0 n src dst w ⦃ r => r.val.Perm v0 ∧ r.val.Pairwise (SRt) ⦄ := by
  unfold sort.sort_triples_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec STri × alloc.vec.Vec STri × Usize) => n.val - q.2.2.val)
    (fun q => q.1.val.length = n.val ∧ q.2.1.val.length = n.val ∧ 0 < q.2.2.val ∧
      q.1.val.Perm v0 ∧ Blocks (SRt) q.2.2.val q.1.val)
  · rintro ⟨s, d, ww⟩ ⟨hsn, hdn', hww, hsp, hsb⟩
    unfold sort.sort_triples_loop0.body
    simp only at hsn hdn' hww hsp hsb ⊢
    split
    · rename_i hlt
      have hlt' : ww.val < n.val := by scalar_tac
      have hstart : S_MidInv s.val d.val d.val ww.val 0 :=
        ⟨Nat.zero_le _, Or.inl ⟨0, by simp⟩, [], by simp, rfl, by simp, fun b => by simp [seg]⟩
      step with s_mid_loop_spec n s d d ww 0#usize (by omega) (by omega) hww hlt' hmax hsb
        (s_idsIn_perm hsp hids) hstart as ⟨d1, hd1p, hd1b, hd1l⟩
      step as ⟨w1, hw1⟩
      refine ⟨hd1l, by omega, by omega, hd1p.trans hsp, by rw [hw1, Nat.mul_comm]; exact hd1b, by omega⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : n.val ≤ ww.val := by scalar_tac
      exact ⟨hsp, blocks_full hsb (by omega)⟩
  · exact ⟨hn.symm, hdn, hw, hp, hb⟩


theorem s_clone_spec (v : alloc.vec.Vec STri) :
    alloc.vec.CloneVec.clone (BuiltinClone STri) v ⦃ r => r.val = v.val ⦄ := by
  unfold alloc.vec.CloneVec.clone
  have hc : ∀ x ∈ v.slice.val, (BuiltinClone STri).clone x = ok x := fun _ _ => rfl
  step as ⟨s, hs⟩
  rw [← slice_val v, ← hs]; rfl

/-- **`sort_triples` sorts**: a permutation of its input, ascending. -/
theorem s_sort_triples_spec (v : alloc.vec.Vec STri) (hmax : 2 * v.val.length ≤ Usize.max) :
    sort.sort_triples v ⦃ r => r.val.Perm v.val ∧ r.val.Pairwise SRt ⦄ := by
  unfold sort.sort_triples
  simp only
  split
  · rename_i h
    simp only [WP.spec_ok]
    exact ⟨List.Perm.refl _, pairwise_of_len_le_one (by scalar_tac)⟩
  · rename_i h
    step with s_clone_spec v as ⟨src, hsrc⟩
    apply s_outer_loop_spec _ v.val src src 1#usize (by simp [hsrc]) (by simp [hsrc]) (by simp)
      (by simpa using hmax) (by rw [hsrc]) (by simp; exact blocks_one _) trivial

/-! ## Sorted triples: a clash between neighbours iff a clash anywhere -/

/-- Same cell, same body, another face. -/
def SConf (x y : STri) : Prop := x.1.val = y.1.val ∧ x.2.1.val = y.2.1.val ∧ x.2.2.val ≠ y.2.2.val

def SNoAdj (t : List STri) : Prop := ∀ i (h : i + 1 < t.length), ¬ SConf (t[i]'(by omega)) t[i+1]

def SNoPair (t : List STri) : Prop :=
  ∀ x ∈ t, ∀ y ∈ t, x.1.val = y.1.val → x.2.1.val = y.2.1.val → x.2.2.val = y.2.2.val

theorem s_adj_iff (t : List STri) (hs : t.Pairwise SRt) : SNoAdj t ↔ SNoPair t := by
  constructor
  · intro h
    induction t with
    | nil => intro x hx; simp at hx
    | cons a rest ih =>
      have hrest : SNoAdj rest := fun i hi => h (i + 1) (by simp; omega)
      have ih' := ih (List.pairwise_cons.mp hs).2 hrest
      have key : ∀ y ∈ rest, a.1.val = y.1.val → a.2.1.val = y.2.1.val → a.2.2.val = y.2.2.val := by
        intro y hy h1 h2
        cases rest with
        | nil => simp at hy
        | cons b rest' =>
          have hab : SRt a b := (List.pairwise_cons.mp hs).1 b (by simp)
          have hby : SRt b y ∨ b = y := by
            rcases List.mem_cons.mp hy with e | e
            · exact Or.inr e.symm
            · exact Or.inl ((List.pairwise_cons.mp (List.pairwise_cons.mp hs).2).1 y e)
          have h0 := h 0 (by simp)
          simp only [List.getElem_cons_zero, List.getElem_cons_succ, SConf] at h0
          have hb1 : b.1.val = a.1.val ∧ b.2.1.val = a.2.1.val := by
            rcases hby with hby | hby
            · unfold SRt at hab hby; omega
            · subst hby; omega
          have hfb : a.2.2.val = b.2.2.val := by omega
          have := ih' b (by simp) y hy (by omega) (by omega)
          omega
      intro x hx y hy h1 h2
      rcases List.mem_cons.mp hx with ex | ex <;> rcases List.mem_cons.mp hy with ey | ey
      · subst ex; subst ey; rfl
      · subst ex; exact key y ey h1 h2
      · subst ey; exact (key x ex h1.symm h2.symm).symm
      · exact ih' x ex y ey h1 h2
  · intro h i hi ⟨h1, h2, h3⟩
    exact h3 (h _ (List.getElem_mem _) _ (List.getElem_mem _) h1 h2)

/-! ## `cls_of` -/

def sClsF (l : Ledger) (f : Nat) : Nat := ((l.face_class.val.getD f (0#u32 : U32)) : U32).val

theorem s_cls_of_spec (l : Ledger) (f : FaceId) : cls_of l f ⦃ r => r.val = sClsF l (f : U32).val ⦄ := by
  unfold cls_of
  step as ⟨i, hi⟩
  have hiv : i.val = (f : U32).val := by rw [hi]; scalar_tac
  split
  · rename_i hlt
    have hl : (f : U32).val < l.face_class.val.length := by scalar_tac
    step as ⟨i2, hi2⟩
    have : i2.val = (f : U32).val := by rw [hi2]; scalar_tac
    step as ⟨r, hr⟩
    rw [hr]; simp [sClsF, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, this]
  · rename_i hge
    have hl : l.face_class.val.length ≤ (f : U32).val := by scalar_tac
    simp only [WP.spec_ok]
    simp [sClsF, List.getD_eq_getElem?_getD, List.getElem?_eq_none hl]

/-! ## Loop 0: check each piece, collect its triple -/

def sTn (l : Ledger) (p : Nat × Nat) : Nat × Nat × Nat := (p.1, bodyF l p.2, p.2)
def sTv (x : STri) : Nat × Nat × Nat := (x.1.val, x.2.1.val, x.2.2.val)

/-- One piece's check, as the loop makes it. -/
def SChk (l : Ledger) (ext : Ext) (p : Nat × Nat) : Prop :=
  p.1 < (allCells l ext).length ∧ (covE l ext p.1).isFacet = true ∧
    (covE l ext p.1).zeroIdx = sClsF l p.2 ∧ bodyF l p.2 < l.nb.val

def SInv0 (l : Ledger) (ext : Ext) (ok : Bool) (tr : List STri) (k : Nat) : Prop :=
  k ≤ (pcs l).length ∧
  (ok = true → tr.map sTv = ((pcs l).take k).map (sTn l) ∧ ∀ p ∈ (pcs l).take k, SChk l ext p) ∧
  (ok = false → ∃ p ∈ pcs l, ¬ SChk l ext p)

theorem s_pcs_len (l : Ledger) : (pcs l).length = l.pieces.val.length := by simp [pcs]

theorem s_pcs_get (l : Ledger) (k : Nat) (hk : k < l.pieces.val.length) :
    (pcs l)[k]'(by rw [s_pcs_len]; exact hk) = ((l.pieces.val[k].1 : U32).val, (l.pieces.val[k].2 : U32).val) := by
  simp [pcs]

theorem s_inv0_step (l : Ledger) (ext : Ext) (tr : List STri) (k : Nat) (c f b : U32)
    (hinv : SInv0 l ext true tr k) (hk : k < l.pieces.val.length)
    (hpc : (pcs l)[k]'(by rw [s_pcs_len]; exact hk) = (c.val, f.val)) (hb : b.val = bodyF l f.val)
    (ok3 : Bool) (hok3 : ok3 = true ↔ SChk l ext (c.val, f.val)) :
    SInv0 l ext ok3 (tr ++ [(c, b, f)]) (k + 1) := by
  obtain ⟨hkl, hT, _⟩ := hinv
  obtain ⟨hmap, hall⟩ := hT rfl
  have htk : (pcs l).take (k + 1) = (pcs l).take k ++ [(c.val, f.val)] := by
    rw [List.take_add_one, List.getElem?_eq_getElem (by rw [s_pcs_len]; exact hk), hpc]; rfl
  refine ⟨by rw [s_pcs_len]; omega, fun h => ⟨?_, ?_⟩, fun h => ⟨(c.val, f.val), ?_, ?_⟩⟩
  · rw [htk, List.map_append, List.map_append, hmap]; simp [sTv, sTn, hb]
  · rw [htk]; intro p hp
    rcases List.mem_append.mp hp with hp | hp
    · exact hall p hp
    · simp at hp; subst hp; exact hok3.mp h
  · rw [← hpc]; exact List.getElem_mem _
  · intro hc; rw [hok3.mpr hc] at h; cases h

theorem s_loop0_spec (l : Ledger) (ext : Ext) (hsmall : (allCells l ext).length < U32.max)
    (hpl : l.pieces.val.length < U32.max) (ok : Bool) (tr : alloc.vec.Vec STri) (k : Usize)
    (hinv : SInv0 l ext ok tr.val k.val) :
    split_ok_loop0 l ext ok tr k ⦃ r =>
      (r.1 = true → r.2.val.map sTv = (pcs l).map (sTn l) ∧ ∀ p ∈ pcs l, SChk l ext p) ∧
      (r.1 = false → ∃ p ∈ pcs l, ¬ SChk l ext p) ⦄ := by
  unfold split_ok_loop0
  apply loop.spec_decr_nat
    (fun (q : Bool × alloc.vec.Vec STri × Usize) => (pcs l).length - q.2.2.val)
    (fun q => SInv0 l ext q.1 q.2.1.val q.2.2.val)
  · rintro ⟨o, t, kk⟩ hi
    unfold split_ok_loop0.body
    simp only at hi ⊢
    have hi' := hi
    obtain ⟨hkl, hT, hF⟩ := hi'
    rw [s_pcs_len] at hkl
    split
    · rename_i ho
      subst ho
      split
      · rename_i hlt
        have hk : kk.val < l.pieces.val.length := by scalar_tac
        have hpc := s_pcs_get l kk.val hk
        step as ⟨c, f, hcf⟩
        have hpc' : (pcs l)[kk.val]'(by rw [s_pcs_len]; exact hk) = (c.val, f.val) := by
          rw [hpc, ← hcf]
        have hmem : (c.val, f.val) ∈ pcs l := by rw [← hpc']; exact List.getElem_mem _
        obtain ⟨hm, _⟩ := hT rfl
        have htl : t.val.length = kk.val := by
          have := congrArg List.length hm; simp at this; rw [this, s_pcs_len]; omega
        step as ⟨i1, hi1⟩
        step with ncells_spec l ext (by scalar_tac) as ⟨i2, hi2⟩
        split
        · rename_i hge
          step as ⟨k1, hk1⟩
          refine ⟨⟨by rw [s_pcs_len]; scalar_tac, fun h => absurd h (by simp), fun _ => ⟨_, hmem, fun h => ?_⟩⟩,
            by rw [s_pcs_len]; scalar_tac⟩
          have := h.1; simp only at this; scalar_tac
        · rename_i hlt2
          have hc : c.val < (allCells l ext).length := by scalar_tac
          step with cov_spec l ext c hc as ⟨x, hx⟩
          have hcx : covOf x = covE l ext c.val := by simp [covOf, covE, hx]
          step with body_of_spec as ⟨b, hb⟩
          step with is_facet_spec as ⟨b1, hb1⟩
          rw [hcx] at hb1
          split
          · rename_i hb1t
            step with zero_idx_spec as ⟨i3, hi3⟩
            step with s_cls_of_spec as ⟨i4, hi4⟩
            step as ⟨i5, hi5⟩
            rw [hcx] at hi3
            split
            · rename_i heq
              step with alloc.vec.Vec.push_spec as ⟨t2, ht2⟩
              step as ⟨k1, hk1⟩
              refine ⟨?_, by rw [s_pcs_len]; scalar_tac⟩
              rw [ht2, hk1]
              refine s_inv0_step l ext t.val kk.val c f b hi hk hpc' hb _ ?_
              have e1 : i3.val = i5.val := by rw [heq]
              have e2 : i5.val = i4.val := by scalar_tac
              simp only [SChk, decide_eq_true_eq]
              constructor
              · intro h; refine ⟨hc, by rw [← hb1, hb1t], by omega, by scalar_tac⟩
              · intro h; scalar_tac
            · rename_i hne
              step with alloc.vec.Vec.push_spec as ⟨t2, ht2⟩
              step as ⟨k1, hk1⟩
              refine ⟨?_, by rw [s_pcs_len]; scalar_tac⟩
              rw [ht2, hk1]
              refine s_inv0_step l ext t.val kk.val c f b hi hk hpc' hb _ ?_
              simp only [SChk, Bool.false_eq_true, false_iff]
              intro h; apply hne
              have e2 : i5.val = i4.val := by scalar_tac
              scalar_tac
          · rename_i hb1f
            step with alloc.vec.Vec.push_spec as ⟨t2, ht2⟩
            step as ⟨k1, hk1⟩
            refine ⟨?_, by rw [s_pcs_len]; scalar_tac⟩
            rw [ht2, hk1]
            refine s_inv0_step l ext t.val kk.val c f b hi hk hpc' hb _ ?_
            simp only [SChk, Bool.false_eq_true, false_iff]
            intro h; apply hb1f; rw [hb1]; exact h.2.1
      · rename_i hge
        simp only [WP.spec_ok]
        have hk : kk.val = (pcs l).length := by rw [s_pcs_len]; scalar_tac
        obtain ⟨hm, ha⟩ := hT rfl
        rw [hk, List.take_length] at hm ha
        exact ⟨fun _ => ⟨hm, ha⟩, fun h => by cases h⟩
    · rename_i ho
      simp only [WP.spec_ok]
      have ho' : o = false := by simpa using ho
      exact ⟨fun h => absurd h ho, fun _ => hF ho'⟩
  · exact hinv

/-! ## Loop 1: neighbours in the sorted triples -/

def SNoAdjTo (t : List STri) (k : Nat) : Prop :=
  ∀ i (h : i + 1 < t.length), i + 1 < k → ¬ SConf (t[i]'(by omega)) t[i+1]

theorem s_noadj_succ (t : List STri) (k : Nat) (hk : 1 ≤ k) (hkl : k < t.length) :
    SNoAdjTo t (k + 1) ↔ SNoAdjTo t k ∧ ¬ SConf (t[k - 1]'(by omega)) t[k] := by
  constructor
  · intro h
    refine ⟨fun i hi hik => h i hi (by omega), ?_⟩
    have := h (k - 1) (by omega) (by omega)
    simp only [show k - 1 + 1 = k by omega] at this
    exact this
  · rintro ⟨h, hk'⟩ i hi hik
    by_cases e : i + 1 < k
    · exact h i hi e
    · have : i = k - 1 := by omega
      subst this
      simp only [show k - 1 + 1 = k by omega]
      exact hk'

theorem s_loop1_spec (ok0 : Bool) (t : alloc.vec.Vec STri) (ok : Bool) (k : Usize)
    (hk1 : 1 ≤ k.val) (hkl : k.val ≤ max 1 t.val.length)
    (hinv : ok = true ↔ ok0 = true ∧ SNoAdjTo t.val k.val) :
    split_ok_loop1 ok t k ⦃ r => r = true ↔ ok0 = true ∧ SNoAdj t.val ⦄ := by
  unfold split_ok_loop1
  apply loop.spec_decr_nat
    (fun (q : Bool × Usize) => t.val.length - q.2.val)
    (fun q => 1 ≤ q.2.val ∧ q.2.val ≤ max 1 t.val.length ∧
      (q.1 = true ↔ ok0 = true ∧ SNoAdjTo t.val q.2.val))
  · rintro ⟨o, kk⟩ ⟨h1, h2, h3⟩
    unfold split_ok_loop1.body
    simp only at h1 h2 h3 ⊢
    split
    · rename_i ho
      split
      · rename_i hlt
        have hlt' : kk.val < t.val.length := by scalar_tac
        have hcur := h3.mp ho
        have hsucc := s_noadj_succ t.val kk.val h1 hlt'
        step as ⟨k0, hk0⟩
        step as ⟨c0, b0, f0, h0⟩
        step as ⟨c1, b1, f1, h1'⟩
        have hk0v : k0.val = kk.val - 1 := by scalar_tac
        have e0 : t.val[kk.val - 1]'(by omega) = (c0, b0, f0) := by
          rw [h0]; congr 1; exact hk0v.symm
        have e1 : t.val[kk.val] = (c1, b1, f1) := by rw [h1']
        rw [e0, e1] at hsucc
        simp only [SConf] at hsucc
        split
        · rename_i hc
          subst hc
          split
          · rename_i hb
            subst hb
            step as ⟨k1, hk1⟩
            refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
            rw [hk1, show kk.val + 1 = kk.val + 1 from rfl, hsucc]
            have : (f0 != f1) = true ↔ f0.val ≠ f1.val := by
              simp only [bne_iff_ne, ne_eq]; constructor <;> intro h e <;> apply h <;> scalar_tac
            simp only [decide_eq_true_eq, this]
            constructor
            · intro h; exact ⟨hcur.1, hcur.2, fun ⟨_, _, e⟩ => by
                cases hf : (f0 != f1) <;> simp_all⟩
            · intro h; cases hf : (f0 != f1) <;> simp_all
          · rename_i hb
            step as ⟨k1, hk1⟩
            refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
            rw [hk1, hsucc]
            have : b0.val ≠ b1.val := fun e => hb (by scalar_tac)
            simp only [Bool.false_eq_true, not_false_eq_true, decide_true, true_iff]
            exact ⟨hcur.1, hcur.2, fun ⟨_, e, _⟩ => this e⟩
        · rename_i hc
          step as ⟨k1, hk1⟩
          refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
          rw [hk1, hsucc]
          have : c0.val ≠ c1.val := fun e => hc (by scalar_tac)
          simp only [Bool.false_eq_true, not_false_eq_true, decide_true, true_iff]
          exact ⟨hcur.1, hcur.2, fun ⟨e, _, _⟩ => this e⟩
      · rename_i hge
        simp only [WP.spec_ok]
        have hge' : t.val.length ≤ kk.val := by scalar_tac
        rw [h3]
        constructor
        · rintro ⟨a, b⟩; exact ⟨a, fun i hi => b i hi (by omega)⟩
        · rintro ⟨a, b⟩; exact ⟨a, fun i hi _ => b i hi⟩
    · rename_i ho
      simp only [WP.spec_ok]
      have ho' : o = false := by simpa using ho
      subst ho'
      simp only [Bool.false_eq_true, false_iff] at h3 ⊢
      rintro ⟨a, b⟩; exact h3 ⟨a, fun i hi _ => b i hi⟩
  · exact ⟨hk1, hkl, hinv⟩

/-! ## The Lean `splitOk`, read on ids -/

def SNoPairP (l : Ledger) : Prop :=
  ∀ p ∈ pcs l, ∀ q ∈ pcs l, p.1 = q.1 → bodyF l p.2 = bodyF l q.2 → p.2 = q.2

theorem s_bodyOf (l : Ledger) (f : Nat) : (toL l).bodyOf f = bodyF l f := by
  simp only [ForgeExact.DecideOnce.Ledger.bodyOf, toL, bodyF, List.getD_eq_getElem?_getD,
    List.getElem?_map]
  cases l.face_body.val[f]? <;> rfl

theorem s_clsOf (l : Ledger) (f : Nat) : (toL l).clsOf f = sClsF l f := by
  simp only [ForgeExact.DecideOnce.Ledger.clsOf, toL, sClsF, List.getD_eq_getElem?_getD,
    List.getElem?_map]
  cases l.face_class.val[f]? <;> rfl

theorem s_lean_iff (l : Ledger) (ext : Ext) (hwf : LedgerWF l) (hok : ExtOK l ext) :
    (toL l).splitOk = true ↔ (∀ p ∈ pcs l, SChk l ext p) ∧ SNoPairP l := by
  have hid : ∀ p ∈ pcs l, p.1 < l.cells.val.length := fun p hp => (hwf.pieces p hp).1
  have hcov : ∀ p ∈ pcs l, covE l ext p.1 = cellL l p.1 := fun p hp => by
    rw [cellL_covAt l ext (hid p hp)]; rfl
  have hall : ∀ p ∈ pcs l, p.1 < (allCells l ext).length := fun p hp => by
    rw [allCells_length]; have := hid p hp; omega
  have ep : (toL l).pieces = (pcs l).map fun q => (cellL l q.1, q.2) := rfl
  have en : (toL l).nb = l.nb.val := rfl
  simp only [ForgeExact.DecideOnce.Ledger.splitOk, ep, en, s_bodyOf, s_clsOf, List.all_eq_true,
    List.forall_mem_map, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq, Bool.or_eq_true,
    Bool.not_eq_true', Bool.and_eq_false_imp, beq_eq_false_iff_ne]
  constructor
  · intro h
    refine ⟨fun p hp => ?_, fun p hp q hq e1 e2 => ?_⟩
    · obtain ⟨⟨⟨h1, h2⟩, h3⟩, _⟩ := h p hp
      exact ⟨hall p hp, by rw [hcov p hp]; exact h1, by rw [hcov p hp]; exact h2, h3⟩
    · obtain ⟨_, h4⟩ := h p hp
      rcases h4 q hq with h5 | h5
      · exact absurd e2.symm (h5 (by rw [e1]))
      · exact h5.symm
  · rintro ⟨h, hn⟩ p hp
    obtain ⟨_, h1, h2, h3⟩ := h _ hp
    rw [hcov _ hp] at h1 h2
    refine ⟨⟨⟨h1, h2⟩, h3⟩, fun q hq => ?_⟩
    by_cases ec : cellL l q.1 = cellL l p.1
    · have := cellL_inj l ext hok (hid _ hq) (hid _ hp) ec
      by_cases ed : bodyF l q.2 = bodyF l p.2
      · exact Or.inr (hn _ hq _ hp this ed)
      · exact Or.inl (fun _ => ed)
    · exact Or.inl (fun e => absurd e ec)

theorem s_pair_iff (l : Ledger) (tr : List STri) (hm : tr.map sTv = (pcs l).map (sTn l)) :
    SNoPair tr ↔ SNoPairP l := by
  have key : ∀ (P : Nat × Nat × Nat → Prop), (∀ x ∈ tr, P (sTv x)) ↔ (∀ p ∈ pcs l, P (sTn l p)) := by
    intro P
    rw [← List.forall_mem_map (f := sTv), hm, List.forall_mem_map]
  unfold SNoPair SNoPairP
  refine (key (fun u => ∀ y ∈ tr, u.1 = y.1.val → u.2.1 = y.2.1.val → u.2.2 = y.2.2.val)).trans ?_
  apply forall_congr'; intro p; apply forall_congr'; intro _
  refine (key (fun v => (sTn l p).1 = v.1 → (sTn l p).2.1 = v.2.1 → (sTn l p).2.2 = v.2.2)).trans ?_
  simp [sTn]

/-! ## `split_ok` -/

theorem split_ok_spec_proof : SplitOkSpec := by
  intro l ext hwf hok _hext
  unfold split_ok
  have hsmall := hok.small
  have hroom := hwf.room
  have hpl : l.pieces.val.length < U32.max := by omega
  step with s_loop0_spec l ext hsmall hpl true (alloc.vec.Vec.new STri) 0#usize
    ⟨by simp, fun _ => ⟨by simp, by simp⟩, fun h => absurd h (by simp)⟩ as ⟨ok1, tr, hT, hF⟩
  split
  · rename_i ho
    obtain ⟨hm, hall⟩ := hT ho
    have htl : tr.val.length = l.pieces.val.length := by
      have := congrArg List.length hm; simp at this; rw [this, s_pcs_len]
    step with s_sort_triples_spec tr (by have := Usize.bounds_eq; scalar_tac) as ⟨T, hTp, hTs⟩
    subst ho
    have hinv1 : true = true ↔ true = true ∧ SNoAdjTo T.val (1#usize).val := by
      simp only [true_and, true_iff]
      intro i hi hik; simp at hik
    refine WP.spec_mono (s_loop1_spec true T true 1#usize (by simp) (by simp) hinv1) ?_
    intro r hr
    rw [Bool.eq_iff_iff, s_lean_iff l ext hwf hok, hr, s_adj_iff T.val hTs]
    have : SNoPair T.val ↔ SNoPair tr.val := by
      unfold SNoPair
      constructor
      · intro h x hx y hy; exact h x (hTp.mem_iff.mpr hx) y (hTp.mem_iff.mpr hy)
      · intro h x hx y hy; exact h x (hTp.mem_iff.mp hx) y (hTp.mem_iff.mp hy)
    rw [this, s_pair_iff l tr.val hm]
    exact ⟨fun h => ⟨hall, h.2⟩, fun h => ⟨rfl, h.2⟩⟩
  · rename_i ho
    have ho' : ok1 = false := by simpa using ho
    obtain ⟨p, hp, hnp⟩ := hF ho'
    subst ho'
    simp only [Std.bind_ok]
    refine WP.spec_mono (s_loop1_spec false tr false 1#usize (by simp) (by simp) (by simp)) ?_
    intro r hr
    rw [Bool.eq_iff_iff, s_lean_iff l ext hwf hok]
    simp only [Bool.false_eq_true, false_and, iff_false] at hr
    simp only [hr, Bool.false_eq_true, false_iff, not_and]
    intro h; exact absurd (h p hp) hnp

end forge_bool_core
