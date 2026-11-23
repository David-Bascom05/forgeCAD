import ForgeBoolCore.RefineIncCollapseLean

/-!
# Group M: the inc collapse's Rust loops

`inc::collapse` is v1's `collapse` with topes replaced by rooms. Loops 0, 1, 3, 5 and 6 are v1's own
loops, definitionally, once the inc split is read as a v1 split (`v1S`), so their RefineCollapse
specs apply. Loops 2 and 4 read the facets' rooms as pairs, so they are re-proven here. Loops 7 and 8
are the fill, in two passes.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-! ### Loops that are v1's -/

theorem none_eq : NONE = forge_bool_core.NONE := by
  unfold NONE forge_bool_core.NONE; rfl

/-- The inc metric ledger, read as v1's. -/
def v1M (mt : MetricLedger) : forge_bool_core.MetricLedger := ⟨mt.thin, mt.enclosed⟩

theorem loop0_eq (rp : alloc.vec.Vec U32) (mt : MetricLedger) (nr : Usize) (it : alloc.vec.Vec Bool)
    (any : Bool) (k : Usize) :
    collapse_loop0 rp mt nr it any k = forge_bool_core.collapse_loop0 rp (v1M mt) nr it any k := by
  unfold collapse_loop0 forge_bool_core.collapse_loop0
  congr 1
  funext x
  obtain ⟨a, b, c⟩ := x
  unfold collapse_loop0.body forge_bool_core.collapse_loop0.body
  all_goals (try rw [none_eq])
  all_goals (try rfl)

theorem loop1_eq (rp : alloc.vec.Vec U32) (enc : alloc.vec.Vec RoomId) (nr : Usize) (ie : alloc.vec.Vec Bool)
    (k : Usize) :
    collapse_loop1 rp enc nr ie k = forge_bool_core.collapse_loop1 rp enc nr ie k := by
  unfold collapse_loop1 forge_bool_core.collapse_loop1
  congr 1
  funext x
  obtain ⟨a, b⟩ := x
  unfold collapse_loop1.body forge_bool_core.collapse_loop1.body
  all_goals (try rw [none_eq])
  all_goals (try rfl)

theorem loop3_eq : @collapse_loop3 = @forge_bool_core.collapse_loop3 := by
  funext a b c d e f
  unfold collapse_loop3 forge_bool_core.collapse_loop3
  all_goals (try congr 1)

theorem loop5_eq : @collapse_loop5 = @forge_bool_core.collapse_loop5 := by
  funext a b c
  unfold collapse_loop5 forge_bool_core.collapse_loop5
  congr 1

theorem loop6_eq : @collapse_loop6 = @forge_bool_core.collapse_loop6 := by
  funext a b c d e f g h i
  unfold collapse_loop6 forge_bool_core.collapse_loop6
  congr 1
  funext x
  obtain ⟨a1, b1, c1⟩ := x
  unfold collapse_loop6.body forge_bool_core.collapse_loop6.body
  have : @collapse_loop6_loop0 = @forge_bool_core.collapse_loop6_loop0 := by
    funext a b c d e
    unfold collapse_loop6_loop0 forge_bool_core.collapse_loop6_loop0
    congr 1
  all_goals (try rw [this, none_eq])
  all_goals (try rfl)

/-! ### Loop 2: the unions -/

/-- Facet `f`'s rooms' positions. -/
def ppI (l : Ledger) (s : Split) (f : Nat) : Nat := posR s (plusR l f)
def qqI (l : Ledger) (s : Split) (f : Nat) : Nat := posR s (minusR l f)

theorem ppI_lt (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {f : Nat} (hf : f < nfacets l) :
    ppI l s f < s.rooms.val.length :=
  pos_lt l hw s hs (facet_rooms_get l hw hf).1 (plusR_mem l hf)

theorem qqI_lt (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {f : Nat} (hf : f < nfacets l) :
    qqI l s f < s.rooms.val.length :=
  pos_lt l hw s hs (facet_rooms_get l hw hf).2 (minusR_mem l hf)

theorem kE_inI (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (th : Nat → Bool) (K : Nat)
    (hK : K ≤ nfacets l) :
    ∀ e ∈ KI.kE (ppI l s) (qqI l s) th K, e.1 < s.rooms.val.length ∧ e.2 < s.rooms.val.length := by
  intro e he
  obtain ⟨k, hk, _, _, rfl⟩ := (KI.mem_kE _ _ th K e).mp he
  exact ⟨ppI_lt l hw s hs (by omega), qqI_lt l hw s hs (by omega)⟩

theorem loop2I_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (th : Nat → Bool)
    (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range s.rooms.val.length).map th)
    (uf : union_find.Uf) (f : Usize) (hf : f.val ≤ nfacets l)
    (hu : UInv (KI.kE (ppI l s) (qqI l s) th f.val) s.rooms.val.length uf) :
    collapse_loop2 l.facet_rooms s.room_pos is_thin (alloc.vec.Vec.len l.facet_class) uf f ⦃ r =>
      UInv (KI.kE (ppI l s) (qqI l s) th (nfacets l)) s.rooms.val.length r ⦄ := by
  have hsm := hw.small
  have hfrl := hw.rooms.1
  have hrpl := hs.rinv.2.1
  have hrl := rooms_len_le l s hs
  unfold collapse_loop2
  apply loop.spec_decr_nat (fun (q : union_find.Uf × Usize) => nfacets l - q.2.val)
    (fun q => q.2.val ≤ nfacets l ∧ UInv (KI.kE (ppI l s) (qqI l s) th q.2.val) s.rooms.val.length q.1)
  · rintro ⟨u, j⟩ ⟨hj, hu'⟩
    unfold collapse_loop2.body
    simp only at hj hu' ⊢
    split
    · rename_i hlt
      have hjl : j.val < nfacets l := by simp [nfacets]; scalar_tac
      have hjf : j.val < l.facet_rooms.val.length := by rw [hfrl]; exact hjl
      step as ⟨p, m, hpm⟩
      have hpmg : l.facet_rooms.val.getD j.val (0#u32, 0#u32) = (p, m) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjf]; simp [hpm]
      clear hpm
      have hfr := facet_rooms_get l hw hjl
      have hpv : plusR l j.val = p.val := by unfold plusR; rw [hpmg]
      have hmv : minusR l j.val = m.val := by unfold minusR; rw [hpmg]
      clear hpmg
      have hp2 := ppI_lt l hw s hs hjl
      have hm2 := qqI_lt l hw s hs hjl
      step as ⟨i, hi⟩
      have hiv : i.val = p.val := by scalar_tac
      step as ⟨i1, hi1⟩
      step as ⟨pp, hpp⟩
      have hppv : pp.val = ppI l s j.val := by
        rw [ppI, hpv, posR, ← hiv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
        simp only [Option.getD_some]; rw [← hi1]; scalar_tac
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = m.val := by scalar_tac
      step as ⟨i3, hi3⟩
      step as ⟨mp, hmp⟩
      have hmpv : mp.val = qqI l s j.val := by
        rw [qqI, hmv, posR, ← hi2v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
        simp only [Option.getD_some]; rw [← hi3]; scalar_tac
      have hitl : is_thin.val.length = s.rooms.val.length := by rw [hit]; simp
      have hthr : ∀ t, t < s.rooms.val.length → is_thin.val[t]! = th t := by
        intro t ht; rw [hit]; simp [ht]
      have hkE : KI.kE (ppI l s) (qqI l s) th (j.val + 1) = KI.kE (ppI l s) (qqI l s) th j.val ++
          (if th (ppI l s j.val) && th (qqI l s j.val) then [(ppI l s j.val, qqI l s j.val)] else []) := rfl
      step as ⟨b, hb⟩
      have hbv : b = th (ppI l s j.val) := by
        rw [hb, ← hthr _ hp2, ← hppv, getElem!_pos is_thin.val pp.val (by omega)]
      split
      · rename_i hbt
        step as ⟨b1, hb1⟩
        have hb1v : b1 = th (qqI l s j.val) := by
          rw [hb1, ← hthr _ hm2, ← hmpv, getElem!_pos is_thin.val mp.val (by omega)]
        split
        · rename_i hb1t
          step with union_spec u hu' (kE_inI l hw s hs th j.val (by omega)) (by omega) pp mp (by omega) (by omega)
            as ⟨u1, hu1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hkE, ← hbv, ← hb1v, hbt, hb1t, ← hppv, ← hmpv]; exact hu1
        · rename_i hb1f
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hkE, ← hb1v]; simp [hb1f]; exact hu'
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hj1, hkE, ← hbv]; simp [hbf]; exact hu'
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nfacets l := by unfold nfacets at hj ⊢; scalar_tac
      rw [← this]; exact hu'
  · exact ⟨hf, hu⟩

/-! ### The rims -/

/-- Facet `k` puts `u` on class `r`'s rim: one side thin in class `r`, the other `u`, not thin. -/
def rimHitI (pp qq : Nat → Nat) (th : Nat → Bool) (R : Nat → Nat) (k r u : Nat) : Prop :=
  (th (pp k) = true ∧ th (qq k) = false ∧ R (pp k) = r ∧ qq k = u) ∨
  (th (qq k) = true ∧ th (pp k) = false ∧ R (qq k) = r ∧ pp k = u)

def rimAtI (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (r : Nat) : List Nat :=
  (rim.val.getD r default).val.map (·.val)

/-- The rims after facets `0 .. k-1`. -/
def RimInvI (pp qq : Nat → Nat) (th : Nat → Bool) (R : Nat → Nat) (nt k : Nat) (rim : alloc.vec.Vec (alloc.vec.Vec U32)) : Prop :=
  rim.val.length = nt ∧ ∀ r, r < nt → (rim.val.getD r default).val.length ≤ k ∧
    ∀ u, u ∈ rimAtI rim r ↔ ∃ k', k' < k ∧ rimHitI pp qq th R k' r u

theorem rimAtI_push (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (r : Nat) (hr : r < rim.val.length)
    (v : alloc.vec.Vec U32) (hv : v = rim.val[r]) (x : alloc.vec.Vec U32) (hx : x.val = v.val ++ [y]) (r' : Nat) :
    (((rim.val.set r x).getD r' default).val.map (·.val)) =
      if r' = r then rimAtI rim r ++ [y.val] else rimAtI rim r' := by
  unfold rimAtI
  rw [List.getD_eq_getElem?_getD, List.getElem?_set]
  split
  · rename_i e; subst e
    simp [hr, hx, hv, List.getD_eq_getElem?_getD]
  · rename_i e
    rw [ite_eq_right_iff.mpr (fun h => absurd h.symm e)]; simp [List.getD_eq_getElem?_getD]

/-- One push to class `r0`'s rim keeps the invariant at `k + 1`, when facet `k` hits exactly `(r0, y)`. -/
theorem rimInvI_push {pp qq : Nat → Nat} {th : Nat → Bool} {R : Nat → Nat} {nt k : Nat}
    {rim : alloc.vec.Vec (alloc.vec.Vec U32)} (h : RimInvI pp qq th R nt k rim) {r0 : Nat} (hr0 : r0 < nt)
    {y : U32} (hhit : ∀ r u, rimHitI pp qq th R k r u ↔ (r = r0 ∧ u = y.val))
    (v : alloc.vec.Vec U32) (hv : v = rim.val[r0]'(by rw [h.1]; exact hr0)) (x : alloc.vec.Vec U32)
    (hx : x.val = v.val ++ [y]) (rim' : alloc.vec.Vec (alloc.vec.Vec U32)) (hrim' : rim'.val = rim.val.set r0 x) :
    RimInvI pp qq th R nt (k + 1) rim' := by
  obtain ⟨hl, hrr⟩ := h
  refine ⟨by simp [hrim', hl], fun r hr => ⟨?_, fun u => ?_⟩⟩
  · have h1 := (hrr r hr).1
    simp only [hrim', List.getD_eq_getElem?_getD] at h1 ⊢
    by_cases e : r = r0
    · subst e
      rw [List.getElem?_set_self (by omega)]
      rw [List.getElem?_eq_getElem (by omega)] at h1
      simp only [Option.getD_some, hx, hv] at h1 ⊢
      simp; omega
    · rw [List.getElem?_set_ne (Ne.symm e)]; omega
  · have e := rimAtI_push rim r0 (by omega) v hv x hx r
    unfold rimAtI; rw [hrim', e]
    constructor
    · intro hu
      split at hu
      · rename_i er; subst er
        rcases List.mem_append.mp hu with hu | hu
        · obtain ⟨k', hk', hh⟩ := ((hrr r hr).2 u).mp hu; exact ⟨k', by omega, hh⟩
        · simp at hu; exact ⟨k, by omega, (hhit r u).mpr ⟨rfl, hu⟩⟩
      · obtain ⟨k', hk', hh⟩ := ((hrr r hr).2 u).mp hu; exact ⟨k', by omega, hh⟩
    · rintro ⟨k', hk', hh⟩
      by_cases ek : k' = k
      · subst ek
        obtain ⟨e1, e2⟩ := (hhit r u).mp hh
        subst e1 e2; simp
      · have hu := ((hrr r hr).2 u).mpr ⟨k', by omega, hh⟩
        split
        · rename_i er; subst er; exact List.mem_append_left _ hu
        · exact hu

theorem rimInvI_none {pp qq : Nat → Nat} {th : Nat → Bool} {R : Nat → Nat} {nt k : Nat}
    {rim : alloc.vec.Vec (alloc.vec.Vec U32)} (h : RimInvI pp qq th R nt k rim)
    (hhit : ∀ r u, ¬ rimHitI pp qq th R k r u) : RimInvI pp qq th R nt (k + 1) rim := by
  obtain ⟨hl, hrr⟩ := h
  refine ⟨hl, fun r hr => ⟨by have := (hrr r hr).1; omega, fun u => ?_⟩⟩
  rw [(hrr r hr).2 u]
  constructor
  · rintro ⟨k', hk', hh⟩; exact ⟨k', by omega, hh⟩
  · rintro ⟨k', hk', hh⟩
    by_cases ek : k' = k
    · subst ek; exact absurd hh (hhit r u)
    · exact ⟨k', by omega, hh⟩


theorem loop4I_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (th : Nat → Bool)
    (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range s.rooms.val.length).map th)
    (R : Nat → Nat) (hR : ∀ y, y < s.rooms.val.length → R y < s.rooms.val.length)
    (uf : union_find.Uf) (hu : K.Roots s.rooms.val.length R uf)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (k : Usize) (hk : k.val ≤ nfacets l)
    (hrim : RimInvI (ppI l s) (qqI l s) th R s.rooms.val.length k.val rim) :
    collapse_loop4 l.facet_rooms s.room_pos is_thin (alloc.vec.Vec.len l.facet_class) uf rim k ⦃ r =>
      K.Roots s.rooms.val.length R r.1 ∧ RimInvI (ppI l s) (qqI l s) th R s.rooms.val.length (nfacets l) r.2 ⦄ := by
  have hsm := hw.small
  have hfrl := hw.rooms.1
  have hrpl := hs.rinv.2.1
  have hrl0 := rooms_len_le l s hs
  unfold collapse_loop4
  apply loop.spec_decr_nat
    (fun (q : union_find.Uf × alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => nfacets l - q.2.2.val)
    (fun q => q.2.2.val ≤ nfacets l ∧ K.Roots s.rooms.val.length R q.1 ∧
      RimInvI (ppI l s) (qqI l s) th R s.rooms.val.length q.2.2.val q.2.1)
  · rintro ⟨u, rm, j⟩ ⟨hj, hu, hrm⟩
    unfold collapse_loop4.body
    simp only at hj hu hrm ⊢
    split
    · rename_i hlt
      have hjl : j.val < nfacets l := by simp [nfacets]; scalar_tac
      have hjf : j.val < l.facet_rooms.val.length := by rw [hfrl]; exact hjl
      step as ⟨pr, mr, hpm⟩
      have hpmg : l.facet_rooms.val.getD j.val (0#u32, 0#u32) = (pr, mr) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjf]; simp [hpm]
      clear hpm
      have hpv0 : plusR l j.val = pr.val := by unfold plusR; rw [hpmg]
      have hmv0 : minusR l j.val = mr.val := by unfold minusR; rw [hpmg]
      clear hpmg
      have hfr := facet_rooms_get l hw hjl
      have hp2 := ppI_lt l hw s hs hjl
      have hm2 := qqI_lt l hw s hs hjl
      have hsmall : s.rooms.val.length ≤ U32.max := by have := hsm.2.1; omega
      have hnf : nfacets l < Usize.max := by have := hsm.1; scalar_tac
      step as ⟨i, hi⟩
      have hiv : i.val = pr.val := by scalar_tac
      step as ⟨i1, hi1⟩
      step as ⟨p, hp⟩
      have hpv : p.val = ppI l s j.val := by
        rw [ppI, hpv0, posR, ← hiv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
        simp only [Option.getD_some]; rw [← hi1]; scalar_tac
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = mr.val := by scalar_tac
      step as ⟨i3, hi3⟩
      step as ⟨q, hq⟩
      have hqv : q.val = qqI l s j.val := by
        rw [qqI, hmv0, posR, ← hi2v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
        simp only [Option.getD_some]; rw [← hi3]; scalar_tac
      have hitl : is_thin.val.length = s.rooms.val.length := by rw [hit]; simp
      step as ⟨b, hb⟩
      have hbv : b = th (ppI l s j.val) := by rw [hb, K.read_map hit, hpv]
      have hrl := hrm.1
      -- the hits of facet `j`
      have hitP : th (ppI l s j.val) = true → th (qqI l s j.val) = false →
          ∀ r u, rimHitI (ppI l s) (qqI l s) th R j.val r u ↔ (r = R p.val ∧ u = q.val) := by
        intro h1 h2 r u; unfold rimHitI; rw [hpv, hqv]
        constructor
        · rintro (⟨_, _, e1, e2⟩ | ⟨h3, _⟩)
          · exact ⟨e1.symm, e2.symm⟩
          · rw [h2] at h3; cases h3
        · rintro ⟨e1, e2⟩; exact Or.inl ⟨h1, h2, e1.symm, e2.symm⟩
      have hitM : th (qqI l s j.val) = true → th (ppI l s j.val) = false →
          ∀ r u, rimHitI (ppI l s) (qqI l s) th R j.val r u ↔ (r = R q.val ∧ u = p.val) := by
        intro h1 h2 r u; unfold rimHitI; rw [hpv, hqv]
        constructor
        · rintro (⟨h3, _⟩ | ⟨_, _, e1, e2⟩)
          · rw [h2] at h3; cases h3
          · exact ⟨e1.symm, e2.symm⟩
        · rintro ⟨e1, e2⟩; exact Or.inr ⟨h1, h2, e1.symm, e2.symm⟩
      have hitN : th (ppI l s j.val) = th (qqI l s j.val) → ∀ r u, ¬ rimHitI (ppI l s) (qqI l s) th R j.val r u := by
        intro he r u hh; unfold rimHitI at hh
        rcases hh with ⟨h1, h2, _⟩ | ⟨h1, h2, _⟩
        · rw [he, h2] at h1; cases h1
        · rw [← he, h2] at h1; cases h1
      have hpush : ∀ (rm0 : alloc.vec.Vec (alloc.vec.Vec U32)) (r0 : Nat), RimInvI (ppI l s) (qqI l s) th R s.rooms.val.length j.val rm0 →
          r0 < s.rooms.val.length → (rm0.val.getD r0 default).val.length < Usize.max := by
        intro rm0 r0 h hr0
        have := (h.2 r0 hr0).1; omega
      split
      · rename_i hbt
        step as ⟨b1, hb1⟩
        have hb1v : b1 = th (qqI l s j.val) := by rw [hb1, K.read_map hit, hqv]
        split
        · rename_i hb1t
          step as ⟨b2, hb2⟩
          have hb2v : b2 = th (qqI l s j.val) := by rw [hb2, K.read_map hit, hqv]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
          rw [hj1]
          exact rimInvI_none hrm (hitN (by rw [← hbv, ← hb1v, hbt, hb1t]))
        · rename_i hb1f
          have hthP : th (ppI l s j.val) = true := by rw [← hbv]; exact hbt
          have hthM : th (qqI l s j.val) = false := by rw [← hb1v]; simpa using hb1f
          step with K.roots_find hu p (by omega) as ⟨ru, hr, hu2⟩
          obtain ⟨r, u2⟩ := ru
          simp only at hr hu2 ⊢
          have hrlt : r.val < s.rooms.val.length := by rw [hr]; exact hR _ (by omega)
          step as ⟨w0, hw0a, hback⟩
          obtain ⟨v4, back⟩ := w0
          simp only at hback hw0a ⊢
          subst hback
          step as ⟨i7, hi7⟩
          have hi7v : i7.val = q.val := by rw [hi7]; scalar_tac
          have hv4l : v4.val.length < Usize.max := by
            have := hpush rm r.val hrm hrlt
            rw [hw0a]; simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (show r.val < rm.val.length by omega),
              Option.getD_some] at this; exact this
          step as ⟨v5, hv5⟩
          have hinv1 := rimInvI_push hrm hrlt (y := i7)
            (fun r' u' => by rw [hitP hthP hthM r' u', hr, hi7v]) v4 hw0a v5 hv5 (rm.set r v5)
            (alloc.vec.Vec.set_val_eq ..)
          step as ⟨b2, hb2⟩
          have hb2v : b2 = th (qqI l s j.val) := by rw [hb2, K.read_map hit, hqv]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [hj1]
          exact hinv1
      · rename_i hbf
        have hthP : th (ppI l s j.val) = false := by rw [← hbv]; simpa using hbf
        step as ⟨b2, hb2⟩
        have hb2v : b2 = th (qqI l s j.val) := by rw [hb2, K.read_map hit, hqv]
        split
        · rename_i hb2t
          have hthM : th (qqI l s j.val) = true := by rw [← hb2v]; exact hb2t
          try (rw [if_neg hbf])
          step with K.roots_find hu q (by omega) as ⟨ru, hr, hu2⟩
          obtain ⟨r, u2⟩ := ru
          simp only at hr hu2 ⊢
          have hrlt : r.val < s.rooms.val.length := by rw [hr]; exact hR _ (by omega)
          step as ⟨w0, hw0a, hback⟩
          obtain ⟨v4, back⟩ := w0
          simp only at hback hw0a ⊢
          subst hback
          step as ⟨i7, hi7⟩
          have hi7v : i7.val = p.val := by rw [hi7]; scalar_tac
          have hv4l : v4.val.length < Usize.max := by
            have := hpush rm r.val hrm hrlt
            rw [hw0a]; simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (show r.val < rm.val.length by omega),
              Option.getD_some] at this; exact this
          step as ⟨v5, hv5⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [hj1]
          exact rimInvI_push hrm hrlt (y := i7)
            (fun r' u' => by rw [hitM hthM hthP r' u', hr, hi7v]) v4 hw0a v5 hv5 _ (alloc.vec.Vec.set_val_eq ..)
        · rename_i hb2f
          have hthM : th (qqI l s j.val) = false := by rw [← hb2v]; simpa using hb2f
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
          rw [hj1]
          exact rimInvI_none hrm (hitN (by rw [hthP, hthM]))
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nfacets l := by unfold nfacets at hj ⊢; scalar_tac
      rw [this] at hrm
      exact ⟨hu, hrm⟩
  · exact ⟨hk, hu, hrim⟩



/-! ### Loops 7 and 8: the fill, then the write -/

/-- The fill after positions `0 .. t-1`: a thin position takes its rim's first keep, or its own. -/
def fillI (th : Nat → Bool) (keep : List Bool) (rimL : Nat → List Nat) (R : Nat → Nat) (t u : Nat) : Bool :=
  if decide (u < t) && th u then K.firstK keep (rimL (R u)) u else false

theorem loop7I_spec (keep : Slice Bool) (nt : Usize) (hkl : keep.val.length = nt.val) (th : Nat → Bool)
    (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range nt.val).map th)
    (R : Nat → Nat) (hR : ∀ y, y < nt.val → R y < nt.val)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (hrl : rim.val.length = nt.val)
    (hrb : ∀ r, r < nt.val → ∀ x ∈ (rim.val.getD r default).val, x.val < nt.val)
    (uf : union_find.Uf) (hu : K.Roots nt.val R uf)
    (fill : alloc.vec.Vec Bool) (t : Usize) (ht : t.val ≤ nt.val)
    (hf : fill.val = (List.range nt.val).map (fillI th keep.val (K.rimAt rim) R t.val)) :
    collapse_loop7 keep nt is_thin uf rim fill t ⦃ r =>
      r.val = (List.range nt.val).map (fillI th keep.val (K.rimAt rim) R nt.val) ⦄ := by
  unfold collapse_loop7
  apply loop.spec_decr_nat (fun (q : union_find.Uf × alloc.vec.Vec Bool × Usize) => nt.val - q.2.2.val)
    (fun q => q.2.2.val ≤ nt.val ∧ K.Roots nt.val R q.1 ∧
      q.2.1.val = (List.range nt.val).map (fillI th keep.val (K.rimAt rim) R q.2.2.val))
  · rintro ⟨u, fl, j⟩ ⟨hj, hu, hfl⟩
    unfold collapse_loop7.body
    simp only at hj hu hfl ⊢
    split
    · rename_i hlt
      have hjl : j.val < nt.val := by scalar_tac
      have hitl : is_thin.val.length = nt.val := by rw [hit]; simp
      have hfll : fl.val.length = nt.val := by rw [hfl]; simp
      step as ⟨b, hb⟩
      have hbv : b = th j.val := by rw [hb, K.read_map hit]
      have hoff : ∀ u, u ≠ j.val → fillI th keep.val (K.rimAt rim) R (j.val + 1) u =
          fillI th keep.val (K.rimAt rim) R j.val u := by
        intro u hu; unfold fillI
        have : decide (u < j.val + 1) = decide (u < j.val) := by
          apply decide_eq_decide.mpr; omega
        rw [this]
      split
      · rename_i hbt
        step with K.roots_find hu j hjl as ⟨ru, hr, hu2⟩
        obtain ⟨r, u2⟩ := ru
        try simp only at hr hu2 ⊢
        have hrlt : r.val < nt.val := by rw [hr]; exact hR _ hjl
        step as ⟨rm, hrm⟩
        have hrmv : rm.val.map (·.val) = K.rimAt rim (R j.val) := by
          rw [← hr, K.rimAt_eq _ (by omega), hrm]
        have hnew : fillI th keep.val (K.rimAt rim) R (j.val + 1) j.val =
            K.firstK keep.val (K.rimAt rim (R j.val)) j.val := by
          unfold fillI; rw [← hbv, hbt]; simp
        have hrmb : ∀ x ∈ rm.val, x.val < nt.val := by
          intro x hx
          have := hrb r.val hrlt x
          simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (show r.val < rim.val.length by omega),
            Option.getD_some] at this
          rw [hrm] at hx; exact this hx
        step as ⟨b1, hb1⟩
        have hval : ∀ (b2 : Bool), b2 = K.firstK keep.val (K.rimAt rim (R j.val)) j.val →
            ∀ (fl1 : alloc.vec.Vec Bool), fl1.val = fl.val.set j.val b2 →
            fl1.val = (List.range nt.val).map (fillI th keep.val (K.rimAt rim) R (j.val + 1)) := by
          intro b2 hb2 fl1 hfl1
          rw [hfl1, hfl]
          apply List.ext_getElem (by simp)
          intro u h1 h2
          simp only [List.length_set, List.length_map, List.length_range] at h1
          rw [List.getElem_set]
          simp only [List.getElem_map, List.getElem_range]
          split
          · rename_i e; subst e; rw [hnew, hb2]
          · rename_i e; rw [hoff u (Ne.symm e)]
        split
        · rename_i hb1t
          have hemp : rm.val = [] := by
            have : rm.length = 0 := by rw [← hb1]; exact hb1t
            exact List.eq_nil_of_length_eq_zero (by simpa using this)
          step as ⟨b2, hb2⟩
          have hb2v : b2 = K.firstK keep.val (K.rimAt rim (R j.val)) j.val := by
            rw [← hrmv, hemp]; simp only [List.map_nil, K.firstK, K.kAt]
            rw [hb2, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
          step as ⟨x0, _, hbk⟩
          obtain ⟨x1, bk⟩ := x0
          simp only at hbk ⊢
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [hj1]
          exact hval b2 hb2v _ (by rw [hbk]; simp)
        · rename_i hb1f
          have hne : 0 < rm.val.length := by
            have : ¬ rm.length = 0 := by intro h; exact hb1f (by rw [hb1]; exact h)
            exact Nat.pos_of_ne_zero this
          step as ⟨i, hi⟩
          step as ⟨i1, hi1⟩
          have hi1v : i1.val = (rm.val[0]).val := by rw [hi1, hi]; scalar_tac
          have hi1l : i1.val < keep.val.length := by rw [hi1v, hkl]; exact hrmb _ (List.getElem_mem _)
          step as ⟨b2, hb2⟩
          have hb2v : b2 = K.firstK keep.val (K.rimAt rim (R j.val)) j.val := by
            rw [← hrmv]
            cases hc : rm.val with
            | nil => rw [hc] at hne; simp at hne
            | cons a rest =>
              simp only [List.map_cons, K.firstK, K.kAt]
              rw [hb2, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by rw [hkl]; have := hrmb a (by rw [hc]; simp); omega)]
              simp only [Option.getD_some]
              congr 1; rw [hi1v]; simp [hc]
          step as ⟨x0, _, hbk⟩
          obtain ⟨x1, bk⟩ := x0
          simp only at hbk ⊢
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [hj1]
          exact hval b2 hb2v _ (by rw [hbk]; simp)
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
        rw [hfl, hj1]
        apply List.map_congr_left
        intro u _
        by_cases e : u = j.val
        · rw [e]; unfold fillI; rw [← hbv]; simp at hbf; simp [hbf]
        · rw [hoff u e]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nt.val := by scalar_tac
      rw [this] at hfl
      exact hfl
  · exact ⟨ht, hu, hf⟩

/-- The write after positions `0 .. t-1`: a thin position takes its fill. -/
def writeI (th : Nat → Bool) (fill keep : List Bool) (t u : Nat) : Bool :=
  if decide (u < t) && th u then fill.getD u false else keep.getD u false

theorem loop8I_spec (nt : Usize) (th : Nat → Bool) (is_thin : alloc.vec.Vec Bool)
    (hit : is_thin.val = (List.range nt.val).map th) (fill : alloc.vec.Vec Bool) (hfl : fill.val.length = nt.val)
    (keep0 : List Bool) (hk0 : keep0.length = nt.val)
    (keep : Slice Bool) (t : Usize) (ht : t.val ≤ nt.val)
    (hk : keep.val = (List.range nt.val).map (writeI th fill.val keep0 t.val)) :
    collapse_loop8 keep nt is_thin fill t ⦃ r =>
      r.val = (List.range nt.val).map (writeI th fill.val keep0 nt.val) ⦄ := by
  unfold collapse_loop8
  apply loop.spec_decr_nat (fun (q : Slice Bool × Usize) => nt.val - q.2.val)
    (fun q => q.2.val ≤ nt.val ∧ q.1.val = (List.range nt.val).map (writeI th fill.val keep0 q.2.val))
  · rintro ⟨kp, j⟩ ⟨hj, hkp⟩
    unfold collapse_loop8.body
    simp only at hj hkp ⊢
    split
    · rename_i hlt
      have hjl : j.val < nt.val := by scalar_tac
      have hitl : is_thin.val.length = nt.val := by rw [hit]; simp
      have hkpl : kp.val.length = nt.val := by rw [hkp]; simp
      step as ⟨b, hb⟩
      have hbv : b = th j.val := by rw [hb, K.read_map hit]
      have hoff : ∀ u, u ≠ j.val → writeI th fill.val keep0 (j.val + 1) u = writeI th fill.val keep0 j.val u := by
        intro u hu; unfold writeI
        have : decide (u < j.val + 1) = decide (u < j.val) := by
          apply decide_eq_decide.mpr; omega
        rw [this]
      split
      · rename_i hbt
        step as ⟨b1, hb1⟩
        step as ⟨kp1, hkp1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hkp1, hj1]
        simp only [Slice.set_val_eq]
        rw [hkp]
        apply List.ext_getElem (by simp)
        intro u h1 h2
        simp only [List.length_set, List.length_map, List.length_range] at h1
        rw [List.getElem_set]
        simp only [List.getElem_map, List.getElem_range]
        split
        · rename_i e; subst e
          unfold writeI; rw [← hbv, hbt]
          simp only [decide_true, Bool.and_self, ite_true, show j.val < j.val + 1 by omega]
          rw [hb1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
        · rename_i e; rw [hoff u (Ne.symm e)]
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hkp, hj1]
        apply List.map_congr_left
        intro u _
        by_cases e : u = j.val
        · rw [e]; unfold writeI; rw [← hbv]; simp at hbf; simp [hbf]
        · rw [hoff u e]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nt.val := by scalar_tac
      rw [this] at hkp
      exact hkp
  · exact ⟨ht, hk⟩


/-! ### The tables, from the split -/

/-- The inc split, read as a v1 split: rooms for topes. -/
def vS (s : Split) : forge_bool_core.Split :=
  { facets := alloc.vec.Vec.new U32, plus := alloc.vec.Vec.new U32, minus := alloc.vec.Vec.new U32,
    topes := s.rooms, tope_pos := s.room_pos, first_face := alloc.vec.Vec.new U32,
    piece_of := alloc.vec.Vec.new _ }

theorem rooms_val_lt (l : Ledger) (s : Split) (hs : SplitOK l s) {t : Nat} (ht : t < s.rooms.val.length) :
    (s.rooms.val[t]).val < l.n_rooms.val := rooms_lt l s hs ht

theorem rooms_val_mem (l : Ledger) (s : Split) (hs : SplitOK l s) {t : Nat} (ht : t < s.rooms.val.length) :
    (s.rooms.val[t]).val ∈ seenRooms l.facet_rooms.val (nfacets l) := by
  have hmem : (s.rooms.val[t]).val ∈ s.rooms.val.map (·.val) := List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
  rw [hs.rinv.1, mem_dedup] at hmem; exact hmem

/-- A room's position, read back, is the room. -/
theorem pos_rooms (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {t : Nat}
    (ht : t < s.rooms.val.length) : posR s (s.rooms.val[t]).val = t := by
  have hx := rooms_val_lt l s hs ht
  have hm := rooms_val_mem l s hs ht
  have hp := pos_lt l hw s hs hx hm
  have he := ((hs.rinv.2.2.2 _ hx).2 hm).1
  change (s.rooms.val.getD (posR s (s.rooms.val[t]).val) 0#u32).val = _ at he
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp, Option.getD_some] at he
  have hnd : (s.rooms.val.map (·.val)).Nodup := by rw [hs.rinv.1]; exact nodup_dedup _
  have := (List.Nodup.getElem_inj_iff hnd (i := posR s (s.rooms.val[t]).val) (j := t)
    (hi := by simpa using hp) (hj := by simpa using ht)).mp (by simpa using he)
  exact this

theorem tp_ofI (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) :
    K.TP (vS s) l.n_rooms.val := by
  have hrl := rooms_len_le l s hs
  have hsm := hw.small
  have hrpl := hs.rinv.2.1
  refine ⟨hrpl, by simp [vS]; omega, fun t ht => ?_, fun c hc hne => ?_⟩
  · simp only [vS] at ht ⊢
    rw [at32_eq ht]
    refine ⟨rooms_val_lt l s hs ht, ?_⟩
    have hx := rooms_val_lt l s hs ht
    rw [at32_eq (by omega)]
    have := pos_rooms l hw s hs ht
    unfold posR at this
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some] at this
    exact this
  · simp only [vS] at hne ⊢
    rw [at32_eq (by omega)] at hne ⊢
    have hc' : (s.room_pos.val.getD c NONE).val = (s.room_pos.val[c]'(by omega)).val := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
    have hmem : c ∈ seenRooms l.facet_rooms.val (nfacets l) := by
      by_contra hn
      have := ((hs.rinv.2.2.2 c hc).1).mpr hn
      apply hne; rw [← hc', this, inc_none_val]
    obtain ⟨he, hlt⟩ := (hs.rinv.2.2.2 c hc).2 hmem
    rw [hc'] at he hlt
    refine ⟨hlt, ?_⟩
    rw [at32_eq hlt]
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hlt, Option.getD_some] at he
    exact he

theorem roomsF_pos (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) {x : Nat}
    (hx : x < l.n_rooms.val) (hm : x ∈ seenRooms l.facet_rooms.val (nfacets l)) :
    (roomsF s)[posR s x]'(by simp [roomsF]; exact pos_lt l hw s hs hx hm) = FCell.room x := by
  have hp := pos_lt l hw s hs hx hm
  have he := ((hs.rinv.2.2.2 x hx).2 hm).1
  change (s.rooms.val.getD (posR s x) 0#u32).val = x at he
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp, Option.getD_some] at he
  rw [roomsF_get s hp, he]

theorem toLF_rooms_eq (l : Ledger) (s : Split) (hs : SplitOK l s) : (toLF l).rooms = roomsF s := hs.rooms.symm

theorem sides_ofI (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) :
    KI.Sides (toLF l) (nfacets l) (ppI l s) (qqI l s) where
  len := by simp [toLF]
  pp_lt k hk := by rw [toLF_rooms_eq l s hs]; simpa [roomsF] using ppI_lt l hw s hs hk
  qq_lt k hk := by rw [toLF_rooms_eq l s hs]; simpa [roomsF] using qqI_lt l hw s hs hk
  pp_eq k hk := by
    have hfr := facet_rooms_get l hw hk
    have e := roomsF_pos l hw s hs hfr.1 (plusR_mem l hk)
    simp only [toLF_rooms_eq l s hs, ppI]
    rw [e]
    simp [toLF, facetNum, plusR]
  qq_eq k hk := by
    have hfr := facet_rooms_get l hw hk
    have e := roomsF_pos l hw s hs hfr.2 (minusR_mem l hk)
    simp only [toLF_rooms_eq l s hs, qqI]
    rw [e]
    simp [toLF, facetNum, minusR]


theorem thinAtI (l : Ledger) (s : Split) (hs : SplitOK l s) (v : alloc.vec.Vec RoomId) :
    KI.ThinAt (toLF l) (v.val.map fun (r : RoomId) => FCell.room (r : U32).val) (K.mk (vS s) (idsOf v)) := by
  intro t ht
  have ht' : t < s.rooms.val.length := by rw [toLF_rooms_eq l s hs] at ht; simpa [roomsF] using ht
  have e : (toLF l).rooms[t] = FCell.room (s.rooms.val[t]).val := by
    simp only [toLF_rooms_eq l s hs]; exact roomsF_get s ht'
  rw [e, K.mk]
  simp only [vS]
  rw [at32_eq ht', Bool.eq_iff_iff, List.contains_iff_mem, List.contains_iff_mem]
  simp [idsOf]

theorem hktI (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule) :
    ∀ u (hu : u < (toLF l).rooms.length),
      keepInI ((toLF l).keepTab (ruleL l.nb.val rule)) (toLF l).rooms[u] = KI.kAt (keepVec l s rule) u := by
  intro u hu
  have hu' : u < s.rooms.val.length := by rw [toLF_rooms_eq l s hs] at hu; simpa [roomsF] using hu
  rw [keepInI_tab _ _ (List.getElem_mem hu), KI.kAt, keepVec, List.getD_eq_getElem?_getD, List.getElem?_map,
    List.getElem?_range hu', Option.map_some, Option.getD_some, keep_eq]
  congr 1
  simp only [toLF_rooms_eq l s hs]
  rw [roomsF_get s hu', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hu']; rfl

theorem rimAtI_eq : rimAtI = K.rimAt := rfl

theorem badP_eq (keep : List Bool) (th : Nat → Bool) (R : Nat → Nat) (rimL : Nat → List Nat) (aenc : Nat → Bool) :
    KI.badP keep th R rimL aenc = K.bad th keep rimL R aenc := rfl



/-! ### v1's specs, on the inc split -/

theorem loop0I_spec (s : Split) (m : MetricLedger) (n : Usize) (hTP : K.TP (vS s) n.val)
    (hthin : ∀ c ∈ m.thin.val, (c : U32).val < n.val)
    (is_thin : alloc.vec.Vec Bool) (any : Bool) (k : Usize) (hk : k.val ≤ m.thin.val.length)
    (hit : is_thin.val = (List.range s.rooms.val.length).map (K.mk (vS s) ((idsOf m.thin).take k.val)))
    (hany : any = true ↔ ∃ t, t < s.rooms.val.length ∧ K.mk (vS s) ((idsOf m.thin).take k.val) t = true) :
    collapse_loop0 s.room_pos m n is_thin any k ⦃ r => r.1 = m.enclosed ∧
      r.2.1.val = (List.range s.rooms.val.length).map (K.mk (vS s) (idsOf m.thin)) ∧
      (r.2.2 = true ↔ ∃ t, t < s.rooms.val.length ∧ K.mk (vS s) (idsOf m.thin) t = true) ⦄ := by
  rw [loop0_eq]
  exact K.loop0_spec (vS s) (v1M m) n hTP hthin is_thin any k hk hit hany

theorem loop1I_spec (s : Split) (enc : alloc.vec.Vec RoomId) (n : Usize) (hTP : K.TP (vS s) n.val)
    (henc : ∀ c ∈ enc.val, (c : U32).val < n.val)
    (is_enc : alloc.vec.Vec Bool) (k : Usize) (hk : k.val ≤ enc.val.length)
    (hit : is_enc.val = (List.range s.rooms.val.length).map (K.mk (vS s) ((idsOf enc).take k.val))) :
    collapse_loop1 s.room_pos enc n is_enc k ⦃ r =>
      r.val = (List.range s.rooms.val.length).map (K.mk (vS s) (idsOf enc)) ⦄ := by
  rw [loop1_eq]
  exact K.loop1_spec (vS s) enc n hTP henc is_enc k hk hit

theorem loop6I_spec (s : Split) (keep : Slice Bool) (nt : Usize) (hnt : nt.val = s.rooms.val.length)
    (hkl : keep.val.length = nt.val) (hsm : ∀ t, t < nt.val → at32 s.rooms t < U32.max)
    (th : Nat → Bool) (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range nt.val).map th)
    (R : Nat → Nat) (hR : ∀ y, y < nt.val → R y < nt.val)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (hrl : rim.val.length = nt.val)
    (hrb : ∀ r, r < nt.val → ∀ x ∈ (rim.val.getD r default).val, x.val < nt.val)
    (aenc : Nat → Bool) (all_enc : alloc.vec.Vec Bool) (hae : all_enc.val = (List.range nt.val).map aenc)
    (uf : union_find.Uf) (hu : K.Roots nt.val R uf) :
    collapse_loop6 s.rooms keep nt is_thin NONE uf rim all_enc 0#usize ⦃ r => K.Roots nt.val R r.2 ∧
      ((r.1 = NONE ∧ (List.range nt.val).find? (K.bad th keep.val (K.rimAt rim) R aenc) = none) ∨
       (r.1 ≠ NONE ∧ ∃ t0, (List.range nt.val).find? (K.bad th keep.val (K.rimAt rim) R aenc) = some t0 ∧
         r.1.val = at32 s.rooms t0)) ⦄ := by
  rw [loop6_eq, none_eq]
  exact K.loop6_spec (vS s) keep nt hnt hkl hsm th is_thin hit R hR rim hrl hrb aenc all_enc hae
    forge_bool_core.NONE uf 0#usize (by simp) hu rfl (fun t ht => by simp at ht)

/-! ### `CollapseSpec` -/

theorem collapse_specI (l : Ledger) (hw : LedgerWF2 l) (m : MetricLedger) (hm : MetricWF2 l m)
    (s : Split) (hs : SplitOK l s) (rule : Rule) : CollapseSpec l s rule m := by
  intro keep hkv
  have hsm := hw.small
  have hrl0 := rooms_len_le l s hs
  have hTP := tp_ofI l hw s hs
  have hS := sides_ofI l hw s hs
  have hTl : (toLF l).rooms.length = s.rooms.val.length := by rw [toLF_rooms_eq l s hs]; simp [roomsF]
  have hkl : keep.val.length = s.rooms.val.length := by rw [hkv]; simp [keepVec]
  have hthA := thinAtI l s hs m.thin
  have henA := thinAtI l s hs m.enclosed
  have hkt := hktI l hw s hs rule
  rw [← hkv] at hkt
  set kt := (toLF l).keepTab (ruleL l.nb.val rule) with hkt_def
  set th := K.mk (vS s) (idsOf m.thin) with hth_def
  set en := K.mk (vS s) (idsOf m.enclosed) with hen_def
  set nf := nfacets l with hnf_def
  have hntv : (alloc.vec.Vec.len s.rooms).val = s.rooms.val.length := by simp
  have hvS : (vS s).topes.val.length = s.rooms.val.length := rfl
  unfold collapse
  step as ⟨nr, hnr⟩
  have hnrv : nr.val = l.n_rooms.val := by rw [hnr]; scalar_tac
  have hTPn : K.TP (vS s) nr.val := by rw [hnrv]; exact hTP
  step as ⟨it0, hit0, hit0l⟩
  step with loop0I_spec s m nr hTPn (fun c hc => by rw [hnrv]; exact hm.thin c hc) it0 false 0#usize
    (by simp) (by rw [hit0, hntv]; exact K.mk_nil_map (vS s) _) (by simp [K.mk]) as ⟨v, it1, any, hv, hit1, hany⟩
  subst hv
  split
  · rename_i hany_t
    step with loop1I_spec s m.enclosed nr hTPn (fun c hc => by rw [hnrv]; exact hm.enclosed c hc) it0 0#usize
      (by simp) (by rw [hit0, hntv]; exact K.mk_nil_map (vS s) _) as ⟨ie, hie⟩
    step with new_spec (alloc.vec.Vec.len s.rooms) (by simp; omega) as ⟨uf0, huf0⟩
    have hinv0 : UInv (KI.kE (ppI l s) (qqI l s) th 0) s.rooms.val.length uf0 := by
      unfold UInv; rw [huf0]; simpa [KI.kE] using inv_range s.rooms.val.length
    step with loop2I_spec l hw s hs th it1 hit1 uf0 0#usize (by simp) hinv0 as ⟨uf1, huf1⟩
    set R := ufRoot (vals uf1.parent) with hR_def
    have hLeast : Least (KI.kE (ppI l s) (qqI l s) th nf) s.rooms.val.length R := huf1.2.2
    have hR : ∀ y, y < s.rooms.val.length → R y < s.rooms.val.length := fun y hy => (hLeast y hy).1
    have hRoots1 : K.Roots s.rooms.val.length R uf1 := ⟨huf1.1, uinv_len huf1, fun _ => rfl⟩
    step with alloc.vec.from_elem_spec (core.clone.CloneallocvecVec core.clone.CloneU32) (alloc.vec.Vec.new U32)
      (alloc.vec.Vec.len s.rooms) K.clone_new_ok as ⟨rim0, hrim0, hrim0l⟩
    step as ⟨ae0, hae0, hae0l⟩
    rw [loop3_eq]
    step with K.loop3_spec (alloc.vec.Vec.len s.rooms) th en it1 ie (by rw [hntv]; exact hit1)
      (by rw [hntv]; exact hie) R (by rw [hntv]; exact hR) uf1 (by rw [hntv]; exact hRoots1) ae0 0#usize (by simp)
      (by rw [hae0, hntv]; apply List.ext_getElem (by simp); intro i _ _; simp [K.aeOf]) as ⟨uf2, ae1, hRoots2, hae1⟩
    rw [hntv] at hRoots2 hae1
    have hRim0 : RimInvI (ppI l s) (qqI l s) th R s.rooms.val.length 0 rim0 := by
      refine ⟨by rw [hrim0]; simp, fun r hr => ⟨?_, fun u => ?_⟩⟩
      · rw [hrim0, List.getD_eq_getElem?_getD, List.getElem?_replicate]; simp [hr, hntv]
      · unfold rimAtI; rw [hrim0, List.getD_eq_getElem?_getD, List.getElem?_replicate]; simp [hr, hntv]
    step with loop4I_spec l hw s hs th it1 hit1 R hR uf2 hRoots2 rim0 0#usize (by simp) hRim0
      as ⟨uf3, rim1, hRoots3, hRim1⟩
    have hrim1l := hRim1.1
    have hnfu : 3 * nf ≤ Usize.max := hsm.2.2.2.2.2.2.2.2.2
    rw [loop5_eq]
    step with K.loop5_spec (alloc.vec.Vec.len s.rooms) rim1 nf hnfu
      (fun r hr => by rw [hntv] at hr; exact (hRim1.2 r hr).1) rim1 0#usize (by simp)
      ⟨by rw [hntv]; exact hrim1l, fun r hr => ⟨fun h => by simp at h, fun _ => rfl⟩⟩ as ⟨rim2, hSort⟩
    rw [hntv] at hSort
    obtain ⟨hrim2l, hSort2⟩ := hSort
    have hmem2 : ∀ r, r < s.rooms.val.length → ∀ u, u ∈ K.rimAt rim2 r ↔
        ∃ k, k < nf ∧ rimHitI (ppI l s) (qqI l s) th R k r u := by
      intro r hr u
      rw [((hSort2 r hr).1 hr).2 u, ← rimAtI_eq]
      exact (hRim1.2 r hr).2 u
    have hrb : ∀ r, r < s.rooms.val.length → ∀ x ∈ (rim2.val.getD r default).val, x.val < s.rooms.val.length := by
      intro r hr x hx
      have : x.val ∈ K.rimAt rim2 r := List.mem_map.mpr ⟨x, hx, rfl⟩
      obtain ⟨k, hk, hh⟩ := (hmem2 r hr x.val).mp this
      rcases hh with ⟨_, _, _, e⟩ | ⟨_, _, _, e⟩
      · rw [← e]; exact qqI_lt l hw s hs hk
      · rw [← e]; exact ppI_lt l hw s hs hk
    have hTabs : KI.Tabs (toLF l) nf (ppI l s) (qqI l s) (thinL m) (enclL m) kt keep.val th en R (K.rimAt rim2)
        (K.aeOf th en R s.rooms.val.length) := by
      refine ⟨hS, hthA, henA, hkt, by rw [hTl]; exact hLeast, fun t ht htt => ?_, fun t ht htt u => ?_,
        fun t ht htt => ?_⟩
      · rw [hTl] at ht; exact ((hSort2 _ (hR t ht)).1 (hR t ht)).1
      · rw [hTl] at ht ⊢
        rw [hmem2 _ (hR t ht) u]
        unfold rimHitI KI.rimP
        constructor
        · rintro ⟨k, hk, ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩⟩
          · refine ⟨by rw [← h4]; exact qqI_lt l hw s hs hk, by rw [← h4]; exact h2, k, hk, Or.inl ⟨h1, ?_, h4⟩⟩
            exact (least_iff hLeast ht (ppI_lt l hw s hs hk)).mpr h3.symm
          · refine ⟨by rw [← h4]; exact ppI_lt l hw s hs hk, by rw [← h4]; exact h2, k, hk, Or.inr ⟨h1, ?_, h4⟩⟩
            exact (least_iff hLeast ht (qqI_lt l hw s hs hk)).mpr h3.symm
        · rintro ⟨_, hthu, k, hk, ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩⟩
          · refine ⟨k, hk, Or.inl ⟨h1, by rw [h3]; exact hthu, ?_, h3⟩⟩
            exact ((least_iff hLeast ht (ppI_lt l hw s hs hk)).mp h2).symm
          · refine ⟨k, hk, Or.inr ⟨h1, by rw [h3]; exact hthu, ?_, h3⟩⟩
            exact ((least_iff hLeast ht (qqI_lt l hw s hs hk)).mp h2).symm
      · rw [K.aeOf_iff, hTl]
    have hsmall : ∀ t, t < (alloc.vec.Vec.len s.rooms).val → at32 s.rooms t < U32.max := by
      intro t ht; rw [hntv] at ht; have := (hTP.pos t ht).1; have := hsm.2.1; simp only [vS] at *; omega
    step with loop6I_spec s keep (alloc.vec.Vec.len s.rooms) hntv (by rw [hntv]; exact hkl) hsmall th it1
      (by rw [hntv]; exact hit1) R (by rw [hntv]; exact hR) rim2 (by rw [hntv]; exact hrim2l)
      (by rw [hntv]; exact hrb) (K.aeOf th en R s.rooms.val.length) ae1 (by rw [hntv]; exact hae1)
      uf3 (by rw [hntv]; exact hRoots3)
      as ⟨bd, uf4, hRoots4, hbad⟩
    rw [hntv] at hRoots4 hbad
    have hcp := KI.collapse_pos hTabs
    simp only [hTl] at hcp
    simp only [badP_eq] at hcp
    split
    · rename_i hbn
      rcases hbad with ⟨_, hnone⟩ | ⟨hne, _⟩
      · step with alloc.vec.from_elem_spec core.clone.CloneBool false (alloc.vec.Vec.len s.rooms) (by rfl)
          as ⟨fl0, hfl0, hfl0l⟩
        step with loop7I_spec keep (alloc.vec.Vec.len s.rooms) (by rw [hntv]; exact hkl) th it1
          (by rw [hntv]; exact hit1) R (by rw [hntv]; exact hR) rim2 (by rw [hntv]; exact hrim2l)
          (by rw [hntv]; exact hrb) uf4 (by rw [hntv]; exact hRoots4) fl0 0#usize (by simp)
          (by rw [hfl0, hntv]; apply List.ext_getElem (by simp); intro i _ _; simp [fillI])
          as ⟨fl1, hfl1⟩
        rw [hntv] at hfl1
        step with loop8I_spec (alloc.vec.Vec.len s.rooms) th it1 (by rw [hntv]; exact hit1) fl1
          (by rw [hfl1, hntv]; simp) keep.val (by rw [hntv]; exact hkl) keep 0#usize (by simp)
          (by rw [hntv]; apply List.ext_getElem (by simp [hkl]); intro i h1 h2
              simp [writeI, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h1])
          as ⟨k1, hk1⟩
        rw [hntv] at hk1
        revert hcp
        cases hc : (toLF l).collapse kt (thinL m) (enclL m) with
        | refuse T =>
          intro ⟨t, ht, _⟩; rw [hnone] at ht; cases ht
        | ok kt' =>
          intro ⟨_, hent⟩
          refine ⟨hbn, by rw [hk1]; simp, ?_⟩
          intro x hx hxm
          have hp := pos_lt l hw s hs hx hxm
          have hpr : posR s x < (toLF l).rooms.length := by rw [hTl]; exact hp
          have e := hent (posR s x) hp
          have er : (toLF l).rooms[posR s x]'hpr = FCell.room x := by
            simp only [toLF_rooms_eq l s hs]; exact roomsF_pos l hw s hs hx hxm
          have e2 : keepInI kt' (FCell.room x) =
              if th (posR s x) = true then KI.firstK keep.val (K.rimAt rim2 (R (posR s x))) (posR s x)
              else KI.kAt keep.val (posR s x) := by
            rw [← er]; exact e
          rw [e2, hk1]
          simp only [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range hp, Option.map_some,
            Option.getD_some, writeI, hfl1, fillI, decide_eq_true hp, Bool.true_and]
          simp only [hth_def]
          cases K.mk (vS s) (idsOf m.thin) (posR s x) <;> rfl
      · exact absurd hbn hne
    · rename_i hbn
      rcases hbad with ⟨hb, _⟩ | ⟨_, t0, hsome, hbv⟩
      · exact absurd hb hbn
      · simp only [WP.spec_ok]
        revert hcp
        cases hc : (toLF l).collapse kt (thinL m) (enclL m) with
        | ok kt' =>
          intro ⟨hn, _⟩; rw [hsome] at hn; cases hn
        | refuse T =>
          intro ⟨t, ht, hT⟩
          rw [hsome] at ht; cases ht
          have ht0 : t0 < s.rooms.val.length := List.mem_range.mp (List.mem_of_find?_eq_some hsome)
          refine ⟨hbn, ?_⟩
          rw [hT, toLF_rooms_eq l s hs]
          unfold KI.at_ roomsF
          rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_eq_getElem ht0]
          simp [hbv, at32_eq ht0]
  · rename_i hany_f
    have hthf : ∀ t, t < s.rooms.val.length → th t = false := by
      intro t ht
      cases h : th t
      · rfl
      · exact absurd (hany.mpr ⟨t, ht, h⟩) hany_f
    have hTabs : KI.Tabs (toLF l) nf (ppI l s) (qqI l s) (thinL m) (enclL m) kt keep.val th en id (fun _ => [])
        (fun _ => true) := by
      have hkE : KI.kE (ppI l s) (qqI l s) th nf = [] :=
        KI.kE_nil _ _ th nf (fun k hk => hthf _ (ppI_lt l hw s hs hk))
      refine ⟨hS, hthA, henA, hkt, by rw [hkE]; exact least_nil _, fun t ht htt => ?_,
        fun t ht htt => ?_, fun t ht htt => ?_⟩
      all_goals (rw [hTl] at ht; rw [hthf t ht] at htt; cases htt)
    have hcp := KI.collapse_pos hTabs
    simp only [hTl] at hcp
    revert hcp
    cases hc : (toLF l).collapse kt (thinL m) (enclL m) with
    | refuse T =>
      intro ⟨t, ht, _⟩
      have := List.find?_some ht
      have ht' := List.mem_range.mp (List.mem_of_find?_eq_some ht)
      simp [KI.badP, hthf t ht'] at this
    | ok kt' =>
      intro ⟨_, hent⟩
      simp only [WP.spec_ok]
      refine ⟨by simp, hkl, ?_⟩
      intro x hx hxm
      have hp := pos_lt l hw s hs hx hxm
      have hpr : posR s x < (toLF l).rooms.length := by rw [hTl]; exact hp
      have e := hent (posR s x) hp
      have er : (toLF l).rooms[posR s x]'hpr = FCell.room x := by
        simp only [toLF_rooms_eq l s hs]; exact roomsF_pos l hw s hs hx hxm
      have e2 : keepInI kt' (FCell.room x) = KI.kAt keep.val (posR s x) := by
        rw [← er, e, hthf _ hp]; rfl
      rw [e2]; rfl

/-- **IncCoreSpec from group U alone**: group M's obligation is proven. -/
theorem incCoreSpec_of_unify (hU : ∀ l s, LedgerWF2 l → SplitOK l s → UnifySpec l s) : IncCoreSpec :=
  incCoreSpec_of hU (fun l s r m hw hm hs => collapse_specI l hw m hm s hs r)

end inc

end forge_bool_core
