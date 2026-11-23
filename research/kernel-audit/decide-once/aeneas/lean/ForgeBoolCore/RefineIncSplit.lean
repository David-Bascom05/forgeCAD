import ForgeBoolCore.RefineIncTables

/-!
# inc refinement, group S: `split_ok`

`split_ok l` is `(toLF l).splitOk`. `splitOk_iff` reads the Lean check on the Rust data. The loop
keeps, per (facet, body), the first face seen, and a later piece agrees with it or fails.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce

/-- A piece's own checks, on the Rust data. -/
def PieceOK (l : Ledger) (c f : Nat) : Prop :=
  c < nfacets l ∧ (l.facet_class.val.getD c 0#u32).val = (l.face_class.val.getD f 0#u32).val ∧
    (toLF l).bodyOf f < l.nb.val ∧
    (l.facet_rooms.val.getD c (0#u32, 0#u32)).1.val ≠ (l.facet_rooms.val.getD c (0#u32, 0#u32)).2.val

theorem map_val_getD (L : List U32) (i : Nat) : (L.map fun x => x.val)[i]?.getD 0 = (L[i]?.getD 0#u32).val := by
  rw [List.getElem?_map]; cases L[i]? <;> simp

theorem splitOk_iff (l : Ledger) : (toLF l).splitOk = true ↔
    (∀ p ∈ l.pieces.val, PieceOK l (p.1 : U32).val (p.2 : U32).val) ∧
    (∀ p ∈ l.pieces.val, ∀ q ∈ l.pieces.val, (q.1 : U32).val = (p.1 : U32).val →
      (toLF l).bodyOf (q.2 : U32).val = (toLF l).bodyOf (p.2 : U32).val → (q.2 : U32).val = (p.2 : U32).val) := by
  simp only [IncLedger.splitOk, List.all_eq_true, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true',
    beq_iff_eq, decide_eq_true_eq, bne_iff_ne, ne_eq, List.contains_iff_mem]
  simp only [toLF, List.mem_map, forall_exists_index, and_imp, forall_apply_eq_imp_iff₂, List.mem_range,
    FCell.facet.injEq, FCell.room.injEq, exists_eq_right, facetNum, PieceOK, IncLedger.bodyOf, IncLedger.clsOf,
    List.getD_eq_getElem?_getD]
  simp only [map_val_getD]
  have key : ∀ (a b : Nat) (x y : Nat) (P : Prop),
      ((FCell.facet a == FCell.facet b && x == y) = false ∨ P) ↔ (a = b → x = y → P) := by
    intro a b x y P
    by_cases h1 : a = b <;> by_cases h2 : x = y <;> simp [h1, h2]
  simp only [key]
  constructor
  · intro h
    exact ⟨fun p hp => ⟨(h p hp).1.1.1.1, (h p hp).1.1.1.2, (h p hp).1.1.2, (h p hp).1.2⟩,
      fun p hp q hq e1 e2 => (h p hp).2 q hq e1 e2⟩
  · rintro ⟨h1, h2⟩ p hp
    exact ⟨⟨⟨⟨(h1 p hp).1, (h1 p hp).2.1⟩, (h1 p hp).2.2.1⟩, (h1 p hp).2.2.2⟩, fun q hq e1 e2 => h2 p hp q hq e1 e2⟩

theorem inc_none_val : NONE.val = U32.max := by simp [NONE, U32.max_eq, U32.rMax]

/-- Piece `j`'s facet, face, body and table key. -/
def pcF (l : Ledger) (j : Nat) : Nat := ((l.pieces.val.getD j default).1 : U32).val
def pfF (l : Ledger) (j : Nat) : Nat := ((l.pieces.val.getD j default).2 : U32).val
def pkey (l : Ledger) (j : Nat) : Nat := pcF l j * l.nb.val + (toLF l).bodyOf (pfF l j)

/-- The loop's invariant over the first `k` pieces. -/
def SInv (l : Ledger) (first : List U32) (ok : Bool) (k : Nat) : Prop :=
  k ≤ l.pieces.val.length ∧ first.length = nfacets l * l.nb.val ∧
  (ok = true →
    (∀ j, j < k → PieceOK l (pcF l j) (pfF l j)) ∧
    (∀ j, j < k → (first.getD (pkey l j) NONE).val = pfF l j) ∧
    (∀ a, a < first.length → first.getD a NONE ≠ NONE → ∃ j, j < k ∧ pkey l j = a)) ∧
  (ok = false → (toLF l).splitOk = false)

theorem pieces_mem (l : Ledger) {j : Nat} (hj : j < l.pieces.val.length) :
    l.pieces.val.getD j default ∈ l.pieces.val := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]; exact List.getElem_mem hj

theorem key_inj (l : Ledger) {j j' : Nat} (h1 : PieceOK l (pcF l j) (pfF l j)) (h2 : PieceOK l (pcF l j') (pfF l j'))
    (e : pkey l j = pkey l j') : pcF l j = pcF l j' ∧ (toLF l).bodyOf (pfF l j) = (toLF l).bodyOf (pfF l j') := by
  unfold pkey at e
  have b1 := h1.2.2.1
  have b2 := h2.2.2.1
  have hnb : 0 < l.nb.val := by omega
  have := Nat.add_mul_div_right ((toLF l).bodyOf (pfF l j)) (pcF l j) hnb
  have := Nat.add_mul_div_right ((toLF l).bodyOf (pfF l j')) (pcF l j') hnb
  have hm1 := Nat.add_mul_mod_self_right ((toLF l).bodyOf (pfF l j)) (pcF l j) l.nb.val
  have hm2 := Nat.add_mul_mod_self_right ((toLF l).bodyOf (pfF l j')) (pcF l j') l.nb.val
  rw [Nat.mod_eq_of_lt b1] at hm1
  rw [Nat.mod_eq_of_lt b2] at hm2
  rw [Nat.add_comm (pcF l j * _), Nat.add_comm (pcF l j' * _)] at e
  constructor
  · have d1 : ((toLF l).bodyOf (pfF l j) + pcF l j * l.nb.val) / l.nb.val = pcF l j := by
      rw [Nat.add_mul_div_right _ _ hnb, Nat.div_eq_of_lt b1, Nat.zero_add]
    have d2 : ((toLF l).bodyOf (pfF l j') + pcF l j' * l.nb.val) / l.nb.val = pcF l j' := by
      rw [Nat.add_mul_div_right _ _ hnb, Nat.div_eq_of_lt b2, Nat.zero_add]
    rw [← d1, ← d2, e]
  · rw [← hm1, ← hm2, e]

theorem split_ok_loop_spec (l : Ledger) (hw : LedgerWF2 l) (first : alloc.vec.Vec U32) (ok : Bool) (k : Usize)
    (hinv : SInv l first.val ok k.val) :
    split_ok_loop l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets l.vert_edges l.vert_sectors (alloc.vec.Vec.len l.facet_class)
      (UScalar.cast .Usize l.nb) first ok k ⦃ r => r = (toLF l).splitOk ⦄ := by
  unfold split_ok_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Bool × Usize) => if q.2.1 then l.pieces.val.length + 1 - q.2.2.val else 0)
    (fun q => SInv l q.1.val q.2.1 q.2.2.val)
  · rintro ⟨fs, o, j⟩ hI
    obtain ⟨hj, hlen, hT, hF⟩ := hI
    unfold split_ok_loop.body
    simp only at hj hlen hT hF ⊢
    split
    · rename_i ho
      split
      · rename_i hlt
        have hjl : j.val < l.pieces.val.length := by scalar_tac
        step as ⟨c2, f, hcf⟩
        have hpj : l.pieces.val.getD j.val default = (c2, f) := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; simp [hcf]
        have hc2 : pcF l j.val = c2.val := by unfold pcF; rw [hpj]
        have hf2 : pfF l j.val = (f : U32).val := by unfold pfF; rw [hpj]
        have hfail : ¬PieceOK l c2.val (f : U32).val → (toLF l).splitOk = false := by
          intro hn
          apply Bool.eq_false_iff.mpr
          intro hs
          have := ((splitOk_iff l).mp hs).1 _ (pieces_mem l hjl)
          rw [hpj] at this; exact hn this
        have hlenF : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
        -- the tail when the piece fails its own checks
        have tailF : ∀ (fs' : alloc.vec.Vec U32), fs'.val = fs.val → ¬PieceOK l c2.val (f : U32).val →
            (do
              let k1 ← j + 1#usize
              Result.ok (ControlFlow.cont (fs', false, k1))) ⦃ r =>
              match r with
              | ControlFlow.done y => y = (toLF l).splitOk
              | ControlFlow.cont x' => SInv l x'.1.val x'.2.1 x'.2.2.val ∧
                  (if x'.2.1 = true then l.pieces.val.length + 1 - x'.2.2.val else 0) < l.pieces.val.length + 1 - j.val ⦄ := by
          intro fs' e hn
          step as ⟨k1, hk1⟩
          refine ⟨⟨by scalar_tac, by rw [e]; exact hlen, fun h => absurd h (by simp), fun _ => hfail hn⟩, ?_⟩
          simp; omega
        step with body_of_spec l f as ⟨b, hb⟩
        step as ⟨i3, hi3⟩
        have hi3v : i3.val = c2.val := by rw [hi3]; scalar_tac
        split
        · rename_i hc
          have hcn : c2.val < nfacets l := by simp [nfacets]; scalar_tac
          have hrl : c2.val < l.facet_rooms.val.length := by rw [hw.rooms.1]; exact hcn
          step as ⟨i4, hi4⟩
          step as ⟨i5, hi5⟩
          step with cls_of_spec l f as ⟨i6, hi6⟩
          have hi4v : i4.val = c2.val := by rw [hi4]; scalar_tac
          have hcls : (l.facet_class.val.getD c2.val 0#u32) = i5 := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hcn]
            simp only [Option.getD_some]; rw [hi5]; simp only [hi4v]; rfl
          split
          · rename_i e56
            split
            · rename_i hbn
              step as ⟨i7, hi7⟩
              step as ⟨p8, hp8⟩
              obtain ⟨i8, x8⟩ := p8
              step as ⟨i9, hi9⟩
              step as ⟨p10, hp10⟩
              obtain ⟨x10, i10⟩ := p10
              have hi7v : i7.val = c2.val := by rw [hi7]; scalar_tac
              have hi9v : i9.val = c2.val := by rw [hi9]; scalar_tac
              have hr : l.facet_rooms.val.getD c2.val (0#u32, 0#u32) = (i8, x8) := by
                rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hrl]; simp only [Option.getD_some]
                rw [hp8]; simp only [hi7v]
              have hr2 : (l.facet_rooms.val.getD c2.val (0#u32, 0#u32)).2 = i10 := by
                rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hrl]; simp only [Option.getD_some]
                have := congrArg Prod.snd hp10; simp only at this; rw [this]; simp only [hi9v]
              have hclsEq : (l.facet_class.val.getD c2.val 0#u32).val = (l.face_class.val.getD (f : U32).val 0#u32).val := by
                rw [hcls, e56, hi6]
                simp only [ForgeExact.DecideOnce.IncLedger.clsOf, toLF, List.getD_eq_getElem?_getD, List.getElem?_map]
                cases l.face_class.val[(f : U32).val]? <;> rfl
              have hbody : (toLF l).bodyOf (f : U32).val < l.nb.val := by rw [← hb]; scalar_tac
              have hx8 : x8 = i10 := by rw [← hr2, hr]
              by_cases hne : i8 = i10
              · have hn : ¬PieceOK l c2.val (f : U32).val := fun h => h.2.2.2 (by
                  rw [hr]; simp only; rw [hx8, hne])
                have hbf : (i8 != i10) = false := by simp [hne]
                show (do let ok2 ← Result.ok (i8 != i10); _) ⦃ _ ⦄
                simp only [Std.bind_ok, hbf, Bool.false_eq_true, ite_false]
                step as ⟨k1, hk1⟩
                refine ⟨⟨by scalar_tac, hlen, fun h => absurd h (by simp), fun _ => hfail hn⟩, ?_⟩
                simp; omega
              · have hpok : PieceOK l c2.val (f : U32).val := ⟨hcn, hclsEq, hbody, by
                  rw [hr]; simp only; rw [hx8]; intro e; exact hne (UScalar.eq_of_val_eq e)⟩
                have hbne : (i8 != i10) = true := by simpa using hne
                show (do let ok2 ← Result.ok (i8 != i10); _) ⦃ _ ⦄
                simp only [Std.bind_ok, hbne, ite_true]
                obtain ⟨h1, h2, h3⟩ := hT ho
                have hsm := hw.small
                have hkey : c2.val * l.nb.val + b.val < nfacets l * l.nb.val := by
                  have : c2.val + 1 ≤ nfacets l := hcn
                  have := Nat.mul_le_mul_right l.nb.val this
                  rw [Nat.add_mul, Nat.one_mul] at this; omega
                step as ⟨i4', hi4'⟩
                step as ⟨i5', hi5'⟩
                step as ⟨i6', hi6'⟩
                step as ⟨atv, hat⟩
                have hatv : atv.val = pkey l j.val := by
                  simp only [pkey, hc2, hf2, ← hb]; scalar_tac
                have hatl : atv.val < fs.val.length := by rw [hlen]; scalar_tac
                step as ⟨i7', hi7'⟩
                have hget : fs.val.getD atv.val NONE = i7' := by
                  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hatl]; simp [hi7']
                have hfnn : f ≠ NONE := by
                  intro e
                  have := (hw.pieces _ (pieces_mem l hjl)).2.1
                  rw [hpj] at this; simp only at this
                  have h0 := inc_none_val; rw [← e] at h0; omega
                split
                · rename_i hnone
                  step as ⟨x, hx1, hx2⟩
                  obtain ⟨xv, bk⟩ := x
                  simp only at hx1 hx2
                  subst hx2
                  show (do let (first1, ok3) ← Result.ok (fs.set atv f, true); _) ⦃ _ ⦄
                  simp only [Std.bind_ok]
                  step as ⟨k1, hk1⟩
                  have hset : (fs.set atv f).val = fs.val.set atv.val f := by simp [alloc.vec.Vec.set_val_eq]
                  have hgetS : ∀ a, (fs.set atv f).val.getD a NONE = if a = atv.val then f else fs.val.getD a NONE := by
                    intro a; rw [hset]; exact getD_set_any _ _ _ _ _ hatl
                  have hnone' : fs.val.getD atv.val NONE = NONE := by rw [hget]; simpa using hnone
                  refine ⟨⟨by scalar_tac, by rw [hset, List.length_set]; exact hlen, fun _ => ⟨?_, ?_, ?_⟩,
                    fun h => absurd h (by simp)⟩, by simp; omega⟩
                  · intro j' hj'
                    rcases Nat.lt_or_ge j' j.val with h | h
                    · exact h1 j' h
                    · have : j' = j.val := by scalar_tac
                      rw [this, hc2, hf2]; exact hpok
                  · intro j' hj'
                    rw [hgetS]
                    rcases Nat.lt_or_ge j' j.val with h | h
                    · have hne' : pkey l j' ≠ atv.val := by
                        intro e
                        have := h2 j' h
                        rw [e, hnone'] at this
                        have hfl := (hw.pieces _ (pieces_mem l (show j' < l.pieces.val.length by omega))).2.1
                        have h0 := inc_none_val
                        simp only [pfF] at this; omega
                      simp only [hne', ite_false]; exact h2 j' h
                    · have : j' = j.val := by scalar_tac
                      rw [this, ← hatv]; simp only [ite_true]; rw [hf2]
                  · intro a ha hne'
                    rw [hgetS] at hne'
                    by_cases e : a = atv.val
                    · exact ⟨j.val, by scalar_tac, by rw [hatv] at e; exact e.symm⟩
                    · simp only [e, ite_false] at hne'
                      obtain ⟨j', hj', hk⟩ := h3 a (by rw [hset, List.length_set] at ha; exact ha) hne'
                      exact ⟨j', by omega, hk⟩
                · rename_i hnone
                  step as ⟨k1, hk1⟩
                  have hnn : fs.val.getD atv.val NONE ≠ NONE := by rw [hget]; simpa using hnone
                  by_cases he : i7' = f
                  · have hd : decide (i7' = f) = true := decide_eq_true he
                    rw [hd]
                    refine ⟨⟨by omega, hlen, fun _ => ⟨?_, ?_, fun a ha hn => ?_⟩, fun h => absurd h (by simp)⟩, by simp; omega⟩
                    · intro j' hj'
                      rcases Nat.lt_or_ge j' j.val with h | h
                      · exact h1 j' h
                      · have : j' = j.val := by omega
                        rw [this, hc2, hf2]; exact hpok
                    · intro j' hj'
                      rcases Nat.lt_or_ge j' j.val with h | h
                      · exact h2 j' h
                      · have : j' = j.val := by omega
                        rw [this, ← hatv, hget, he, hf2]
                    · obtain ⟨j', hj', hk⟩ := h3 a ha hn
                      exact ⟨j', by omega, hk⟩
                  · -- the key was seen with another face: two faces of one body share the facet
                    have hsF : (toLF l).splitOk = false := by
                      obtain ⟨j', hj', hk⟩ := h3 atv.val hatl hnn
                      have hfj := h2 j' hj'
                      rw [hk, hget] at hfj
                      have hok' := h1 j' hj'
                      have hok : PieceOK l (pcF l j.val) (pfF l j.val) := by rw [hc2, hf2]; exact hpok
                      obtain ⟨ec, eb⟩ := key_inj l hok' hok (by rw [hk, hatv])
                      apply Bool.eq_false_iff.mpr
                      intro hs
                      have hpair := ((splitOk_iff l).mp hs).2 _ (pieces_mem l (show j' < l.pieces.val.length by omega))
                        _ (pieces_mem l hjl)
                      have := hpair (by simpa [pcF] using ec.symm) (by simpa [pfF] using eb.symm)
                      apply he
                      apply UScalar.eq_of_val_eq
                      rw [hfj, ← hf2]; exact this.symm
                    have hd : decide (i7' = f) = false := decide_eq_false he
                    rw [hd]
                    refine ⟨⟨by scalar_tac, hlen, fun h => absurd h (by simp), fun _ => hsF⟩, by simp; omega⟩
            · rename_i hbn
              have hn : ¬PieceOK l c2.val (f : U32).val := fun h => hbn (by
                have := h.2.2.1; rw [← hb] at this; scalar_tac)
              simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
              step as ⟨k1, hk1⟩
              refine ⟨⟨by scalar_tac, hlen, fun h => absurd h (by simp), fun _ => hfail hn⟩, ?_⟩
              simp; omega
          · rename_i e56
            have hn : ¬PieceOK l c2.val (f : U32).val := fun h => e56 (by
              apply UScalar.eq_of_val_eq; rw [hi6, ← hcls]; exact h.2.1.trans (by
                simp only [ForgeExact.DecideOnce.IncLedger.clsOf, toLF, List.getD_eq_getElem?_getD, List.getElem?_map]
                cases l.face_class.val[(f : U32).val]? <;> rfl))
            simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
            step as ⟨k1, hk1⟩
            refine ⟨⟨by scalar_tac, hlen, fun h => absurd h (by simp), fun _ => hfail hn⟩, ?_⟩
            simp; omega
        · rename_i hc
          have hn : ¬PieceOK l c2.val (f : U32).val := fun h => hc (by
            have := h.1; simp [nfacets] at this; scalar_tac)
          simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
          step as ⟨k1, hk1⟩
          refine ⟨⟨by scalar_tac, hlen, fun h => absurd h (by simp), fun _ => hfail hn⟩, ?_⟩
          simp; omega
      · rename_i hge
        simp only [WP.spec_ok]
        have hk : j.val = l.pieces.val.length := by scalar_tac
        obtain ⟨h1, h2, _⟩ := hT ho
        rw [ho]
        symm
        apply (splitOk_iff l).mpr
        constructor
        · intro p hp
          obtain ⟨n, hn, rfl⟩ := List.getElem_of_mem hp
          have := h1 n (by omega)
          simpa [pcF, pfF, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hn] using this
        · intro p hp q hq e1 e2
          obtain ⟨n, hn, rfl⟩ := List.getElem_of_mem hp
          obtain ⟨m, hm, rfl⟩ := List.getElem_of_mem hq
          have gp : l.pieces.val.getD n default = l.pieces.val[n] := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hn]; rfl
          have gq : l.pieces.val.getD m default = l.pieces.val[m] := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hm]; rfl
          have hkey : pkey l m = pkey l n := by
            simp only [pkey, pcF, pfF, gp, gq]; rw [e1, e2]
          have a1 := h2 n (by omega)
          have a2 := h2 m (by omega)
          rw [hkey] at a2
          rw [a2] at a1
          simp only [pfF] at a1
          rw [gp, gq] at a1
          exact a1
    · rename_i hno
      simp only [WP.spec_ok]
      have : o = false := by simpa using hno
      rw [this, hF this]
  · exact hinv

/-- **`split_ok` is `splitOk`** of the ledger read into Lean. -/
theorem split_ok_spec (l : Ledger) (hw : LedgerWF2 l) : split_ok l ⦃ r => r = (toLF l).splitOk ⦄ := by
  unfold split_ok
  have hsm := hw.small
  step as ⟨nb, hnb⟩
  have hnbv : nb.val = l.nb.val := by rw [hnb]; scalar_tac
  have hmul : (alloc.vec.Vec.len l.facet_class).val * nb.val ≤ Usize.max := by
    rw [hnbv]; simpa [nfacets] using hsm.2.2.2.2.1
  step as ⟨i, hi⟩
  step with alloc.vec.from_elem_spec core.clone.CloneU32 NONE i (by rfl) as ⟨first, hf1, hf2⟩
  have hr : (decide (alloc.vec.Vec.len l.facet_rooms = alloc.vec.Vec.len l.facet_class)) = true := by
    simp only [decide_eq_true_eq]; apply UScalar.eq_of_val_eq; simp [hw.rooms.1, nfacets]
  rw [hr, hnb]
  refine split_ok_loop_spec l hw first true 0#usize ⟨by simp, ?_, fun _ => ⟨fun j hj => by simp at hj, fun j hj => by simp at hj, fun a ha hne => ?_⟩,
    fun h => absurd h (by simp)⟩
  · rw [hf1, List.length_replicate, hi, hnbv]; simp [nfacets]
  · exfalso; apply hne
    rw [hf1, List.getD_eq_getElem?_getD, List.getElem?_replicate]
    split <;> simp

end inc

end forge_bool_core

