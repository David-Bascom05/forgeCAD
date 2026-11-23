import ForgeBoolCore.RefineIncFan

/-!
# Group E: the inc core, assembled

`inc::core_k` is `coreK` and `inc::core` is `coreMI`, given the two obligations still open:
* `UnifySpec` (group U): `unify` is the Lean's `unify`, on an assembled result that passed the touch
  rule;
* `CollapseSpec` (group M): `collapse` is the Lean's `collapse`, by room position.

So `IncCoreSpec` is exactly those two (`incCoreSpec_of`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- **Group U's obligation**: `unify` on an assembled result that passed the touch rule is the
Lean's `unify`. -/
def UnifySpec (l : Ledger) (s : Split) : Prop :=
  ∀ (kv : List Bool) (kt : KeepTabI FCell) (a : Assembled), KeepRel l s kv kt →
    AInv l s kv a.result.val a.rpos.val a.orient.val (nfacets l) →
    (toLF l).edges.find? ((toLF l).badEdge kt) = none →
    (toLF l).verts.find? ((toLF l).badVert kt) = none →
    (toLF l).result kt ≠ [] →
    unify l a ⦃ b => bodyL b = (toLF l).unify kt ⦄

theorem bVL_eq (l : Ledger) (kt : KeepTabI FCell) : bVL l kt = (toLF l).badVert kt ∘ FCell.vert := rfl

/-- **`core_k` is `coreK`**, on a keep vector that agrees with the Lean's keep table. -/
theorem core_k_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (hU : UnifySpec l s)
    (keep : Slice Bool) (kt : KeepTabI FCell) (hlen : keep.val.length = s.rooms.val.length)
    (hk : KeepRel l s keep.val kt) :
    core_k l s keep ⦃ o => outLF o = (toLF l).coreK kt ⦄ := by
  have hsm := hw.small
  unfold core_k
  step with alloc.vec.from_elem_spec core.clone.CloneU32 NONE (alloc.vec.Vec.len l.facet_class) (by rfl)
    as ⟨rp0, hr1, hr2⟩
  have h0 : AInv l s keep.val (alloc.vec.Vec.new U32).val rp0.val (alloc.vec.Vec.new Sgn).val (0#usize).val := by
    refine ⟨by simp, by simp [resL], by rw [hr1]; simp [nfacets], ?_, by simp [resL]⟩
    intro g hg
    rw [hr1, List.getD_eq_getElem?_getD, List.getElem?_replicate]
    have : g < l.facet_class.val.length := by simpa [nfacets] using hg
    simp [this, inc_none_val]
  step with core_k_loop0_spec l hw s hs keep hlen _ _ _ 0#usize h0 as ⟨res, rpos, ori, hA⟩
  set a : Assembled := { result := res, rpos := rpos, orient := ori } with ha
  have hA' : AInv l s keep.val a.result.val a.rpos.val a.orient.val (nfacets l) := hA
  step with rows_spec _ as ⟨ne, hne⟩
  step with rows_spec _ as ⟨nv, hnv⟩
  have hnev : ne.val = nrowsL l.edge_facets := by rw [hne]; simp [nrowsL, alloc.vec.Vec.deref]
  have hnvv : nv.val = nrowsL l.vert_edges := by rw [hnv]; simp [nrowsL, alloc.vec.Vec.deref]
  have hrl : a.rpos.val.length = nfacets l := hA'.rpos_len
  have hE0 : EScan l a Outcome.Empty (0#usize).val Outcome.Empty false := by simp [EScan]
  step with core_k_loop1_spec l hw a hrl ne hnev Outcome.Empty Outcome.Empty false 0#usize (by simp) hE0
    as ⟨bad, found, hE⟩
  have hfind := edges_find_eq l hw s hs keep.val kt hk a hA'
  unfold EScan at hE
  split at hE
  · -- a bad edge: the vert scan returns it
    rename_i e0 he0
    obtain ⟨hfd, x, hx, hbad⟩ := hE
    rw [hfd]
    step with core_k_loop2_found l a nv bad 0#usize as ⟨b2, f2, hb2, hf2⟩
    rw [hf2]
    simp only [ite_true, WP.spec_ok]
    rw [hb2]
    unfold IncLedger.coreK
    rw [hfind, he0]
    simp only [Option.map_some]
    rw [hbad]; simp [outLF, hx]
  · rename_i he0
    obtain ⟨hfd, hbad⟩ := hE
    rw [hfd, hbad]
    have hV0 : VScan l kt Outcome.Empty (0#usize).val Outcome.Empty false := by simp [VScan]
    step with core_k_loop2_spec l hw s hs keep.val kt hk a hA' nv hnvv Outcome.Empty Outcome.Empty false 0#usize
      (by simp) hV0 as ⟨bad1, found1, hV⟩
    have hvfind := find_range_verts l ((toLF l).badVert kt)
    rw [← bVL_eq] at hvfind
    unfold VScan at hV
    split at hV
    · rename_i v0 hv0
      obtain ⟨hfd1, x, hx, hbad1⟩ := hV
      rw [hfd1]
      simp only [ite_true, WP.spec_ok]
      unfold IncLedger.coreK
      rw [hfind, he0, hvfind, hv0]
      simp only [Option.map_some, Option.map_none]
      rw [hbad1]; simp [outLF, hx]
    · rename_i hv0
      obtain ⟨hfd1, hbad1⟩ := hV
      rw [hfd1]
      simp only [Bool.false_eq_true, ite_false]
      have hres := result_eq l hw s hs keep.val kt hk
      have hra : a.result.val.map (·.val) = resL l s keep.val (nfacets l) := hA'.res
      step as ⟨b, hb⟩
      have hEn : (toLF l).edges.find? ((toLF l).badEdge kt) = none := by rw [hfind, he0]; rfl
      have hVn : (toLF l).verts.find? ((toLF l).badVert kt) = none := by rw [hvfind, hv0]; rfl
      split
      · rename_i hb1
        simp only [WP.spec_ok]
        unfold IncLedger.coreK
        rw [hEn, hVn]
        simp only
        have : res.val = [] := by
          rw [hb] at hb1; simpa using hb1
        have hr0 : (toLF l).result kt = [] := by
          rw [hres, ← hra]; simp [ha, this]
        rw [if_pos hr0]; rfl
      · rename_i hb1
        have hlen0 : res.val ≠ [] := by
          intro h0; apply hb1; rw [hb]; simp [h0]
        have hne : (toLF l).result kt ≠ [] := by
          rw [hres, ← hra]; intro h
          apply hlen0; simpa [ha] using h
        step with hU keep.val kt a hk hA' hEn hVn hne as ⟨body, hbody⟩
        unfold IncLedger.coreK
        rw [hEn, hVn]
        simp only
        rw [if_neg hne, outLF, hbody]

/-- **Group M's obligation**: `collapse` on the keep vector is the Lean's `collapse`, by room
position. -/
def CollapseSpec (l : Ledger) (s : Split) (rule : Rule) (m : MetricLedger) : Prop :=
  ∀ keep : Slice Bool, keep.val = keepVec l s rule →
    collapse l s keep m ⦃ q =>
      match (toLF l).collapse ((toLF l).keepTab (ruleL l.nb.val rule)) (thinL m) (enclL m) with
      | .refuse T => q.1 ≠ NONE ∧ T = FCell.room q.1.val
      | .ok kt' => q.1 = NONE ∧ q.2.val.length = s.rooms.val.length ∧ KeepRel l s q.2.val kt' ⦄

/-- **`core` is `coreMI`**, given groups U's and M's obligations. -/
theorem core_spec (l : Ledger) (r : Rule) (m : MetricLedger) (hw : LedgerWF2 l)
    (hU : ∀ s, SplitOK l s → UnifySpec l s) (hC : ∀ s, SplitOK l s → CollapseSpec l s r m) :
    core l r m ⦃ o => outLF o = (toLF l).coreMI (ruleL l.nb.val r) (thinL m) (enclL m) ⦄ := by
  have hsm := hw.small
  unfold core
  step with split_ok_spec l hw as ⟨b, hb⟩
  split
  · rename_i hb1
    have hso : (toLF l).splitOk = true := by rw [← hb]; exact hb1
    step with split_spec l hw hso as ⟨s, hs⟩
    step with membership_spec l hw s hs as ⟨memb, hm⟩
    step as ⟨nb, hnb⟩
    rw [hnb]
    step with core_loop0_spec l hw s hs memb hm true 0#usize (by simp) (by simp [cAll]) as ⟨s1, ok1, hs1, hok1⟩
    rw [hs1]
    have hcl := cAll_eq l hw s hs
    split
    · rename_i hok
      have hco : (toLF l).classifyOk = true := by rw [← hcl, ← hok1]; exact hok
      step with core_loop1_spec l hw s hs r memb hm (alloc.vec.Vec.with_capacity Bool _) 0#usize (by simp)
        (by simp [alloc.vec.Vec.with_capacity]) as ⟨keep1, hkeep1⟩
      simp only [alloc.vec.Vec.deref_mut, lift, Std.bind_ok]
      have hkv : (keep1.slice : Slice Bool).val = keepVec l s r := by show keep1.val = _; rw [hkeep1]; rfl
      step with hC s hs keep1.slice hkv as ⟨bad, s3, hq⟩
      unfold IncLedger.coreMI
      simp only [hso, hco, Bool.not_true, Bool.false_eq_true, ite_false]
      split at hq
      · rename_i T hT
        obtain ⟨hne, hT'⟩ := hq
        have : (bad != NONE) = true := by simpa using hne
        rw [if_pos this, hT]
        simp [outLF, hT']
      · rename_i kt' hkt
        obtain ⟨hbad, hlen, hrel⟩ := hq
        have : (bad != NONE) = false := by simp [hbad]
        rw [if_neg (by simp [this]), hkt]
        simp only
        have e : (alloc.vec.Vec.deref ({ slice := s3 } : alloc.vec.Vec Bool)).val = s3.val := by
          simp [alloc.vec.Vec.deref]; rfl
        exact core_k_spec l hw s hs (hU s hs) _ kt' (by rw [e]; exact hlen) (by rw [e]; exact hrel)
    · rename_i hok
      have hco : (toLF l).classifyOk = false := by rw [← hcl, ← hok1]; simpa using hok
      simp only [WP.spec_ok]
      unfold IncLedger.coreMI
      simp [hso, hco, outLF]
  · rename_i hb1
    have hso : (toLF l).splitOk = false := by rw [← hb]; simpa using hb1
    simp only [WP.spec_ok]
    unfold IncLedger.coreMI
    simp [hso, outLF]

/-- **IncCoreSpec from groups U and M.** -/
theorem incCoreSpec_of
    (hU : ∀ l s, LedgerWF2 l → SplitOK l s → UnifySpec l s)
    (hC : ∀ l s r m, LedgerWF2 l → MetricWF2 l m → SplitOK l s → CollapseSpec l s r m) :
    IncCoreSpec := fun l r m hw hm =>
  core_spec l r m hw (fun s hs => hU l s hw hs) (fun s hs => hC l s r m hw hm hs)

end inc

end forge_bool_core
