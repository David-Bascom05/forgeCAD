import ForgeSheetCore.RefineUF
open Aeneas Aeneas.Std Result

/-!
# `owner` refines Region.lean's hole-owner rule

For `k` outer loops, `holds[o]` (outer `o` holds the hole's vertex) and `nest[o * k + p]` (outer `p`
holds outer `o`'s vertex), the shipped `owner` returns the first `o` that holds the vertex and is
held by every other outer loop holding it: `List.find?` of Region.lean's `isOwner` over `0 .. k-1`
(`owner_spec`). No panic or overflow when `holds` has at least `k` entries, `nest` at least `k²`,
and `k ≤ u32::MAX`.
-/

namespace forge_sheet_core

/-- The inner test for outer `o`: every other `p` holding the vertex holds `o`'s. -/
theorem usize_max_succ : Usize.max + 1 = UScalar.size .Usize := by
  have hpos : 0 < 2 ^ UScalarTy.Usize.numBits := by positivity
  rw [Usize.max_def, UScalar.size_def, Usize.numBits_def]
  omega

def innerOk (H N : List Bool) (k o : Nat) (p : Nat) : Bool :=
  p == o || !H.getD p false || N.getD (o * k + p) false

/-- Region.lean's `isOwner`, on the decided bits. -/
def isOwnerB (H N : List Bool) (k o : Nat) : Bool :=
  H.getD o false && (List.range k).all (innerOk H N k o)

theorem all_range_succ (f : Nat → Bool) (p : Nat) :
    (List.range (p + 1)).all f = ((List.range p).all f && f p) := by
  simp [List.range_succ, List.all_append]

theorem find_range_succ (f : Nat → Bool) (o : Nat) :
    (List.range (o + 1)).find? f = ((List.range o).find? f).or (if f o then some o else none) := by
  rw [List.range_succ, List.find?_append]
  cases (List.range o).find? f <;> simp [List.find?_cons, List.find?_nil] <;> split <;> simp_all

theorem inner_spec (k : Usize) (holds nest : Slice Bool) (o : Usize) (ho : o.val < k.val)
    (hH : k.val ≤ holds.val.length) (hN : k.val * k.val ≤ nest.val.length)
    (hkk : k.val * k.val ≤ Usize.max) (acc : Bool) (p : Usize) (hp : p.val ≤ k.val)
    (hacc : acc = (List.range p.val).all (innerOk holds.val nest.val k.val o.val)) :
    owner_loop0_loop0 k holds nest o acc p ⦃ r =>
      r = (List.range k.val).all (innerOk holds.val nest.val k.val o.val) ⦄ := by
  unfold owner_loop0_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => k.val - q.2.val)
    (fun q => q.2.val ≤ k.val ∧ q.1 = (List.range q.2.val).all (innerOk holds.val nest.val k.val o.val))
  · rintro ⟨a, j⟩ ⟨hj, ha⟩
    unfold owner_loop0_loop0.body
    simp only at hj ha ⊢
    split
    · rename_i h
      have hjl : j.val < k.val := by scalar_tac
      have hstep : ∀ (b : Bool), b = (a && innerOk holds.val nest.val k.val o.val j.val) →
          ∀ j1 : Usize, j1.val = j.val + 1 →
          j1.val ≤ k.val ∧ b = (List.range j1.val).all (innerOk holds.val nest.val k.val o.val) ∧
            k.val - j1.val < k.val - j.val := by
        intro b hb j1 hj1
        refine ⟨by omega, ?_, by omega⟩
        rw [hj1, all_range_succ, ← ha, hb]
      split
      · rename_i hne
        have hne' : j.val ≠ o.val := by
          intro e; apply (bne_iff_ne.mp hne); exact UScalar.eq_of_val_eq e
        step with Slice.index_usize_spec holds j (by simp; omega) as ⟨hb, hhb⟩
        split
        · rename_i hb1
          step as ⟨m, hm⟩
          step as ⟨i1, hi1⟩
          have hmv : m.val = o.val * k.val := by
            rw [hm, Usize.wrapping_mul_val_eq]; apply Nat.mod_eq_of_lt
            have : o.val * k.val ≤ k.val * k.val := Nat.mul_le_mul_right _ ho.le
            have hs : Usize.max + 1 = UScalar.size .Usize := usize_max_succ
            omega
          have hi1v : i1.val = o.val * k.val + j.val := by
            rw [hi1, Usize.wrapping_add_val_eq, hmv]; apply Nat.mod_eq_of_lt
            have : o.val * k.val + j.val < k.val * k.val := by
              have : (o.val + 1) * k.val ≤ k.val * k.val := Nat.mul_le_mul_right _ ho
              rw [Nat.add_mul, one_mul] at this; omega
            have hs : Usize.max + 1 = UScalar.size .Usize := usize_max_succ
            omega
          have hlt : o.val * k.val + j.val < k.val * k.val := by
            have : (o.val + 1) * k.val ≤ k.val * k.val := Nat.mul_le_mul_right _ ho
            rw [Nat.add_mul, one_mul] at this; omega
          step with Slice.index_usize_spec nest i1 (by simp; omega) as ⟨nb, hnb⟩
          have e1 : holds.val.getD j.val false = true := by
            rw [List.getD_eq_getElem _ _ (by omega), ← hhb]; exact hb1
          have e2 : nest.val.getD (o.val * k.val + j.val) false = nb := by
            rw [List.getD_eq_getElem _ _ (by omega), hnb]; simp [hi1v]
          have hinner : innerOk holds.val nest.val k.val o.val j.val = nb := by
            simp only [innerOk]; rw [e1, e2]
            have : (j.val == o.val) = false := by simpa using hne'
            rw [this]; simp
          split
          · rename_i hnb1
            step as ⟨j1, hj1⟩
            exact hstep a (by rw [hinner, hnb1]; simp) j1 (by rw [hj1]; exact usize_inc j k hjl)
          · rename_i hnb1
            step as ⟨j1, hj1⟩
            exact hstep false (by rw [hinner]; simp at hnb1; simp [hnb1]) j1
              (by rw [hj1]; exact usize_inc j k hjl)
        · rename_i hb1
          step as ⟨j1, hj1⟩
          have e1 : holds.val.getD j.val false = false := by
            rw [List.getD_eq_getElem _ _ (by omega), ← hhb]; simpa using hb1
          have hinner : innerOk holds.val nest.val k.val o.val j.val = true := by
            simp only [innerOk]; rw [e1]; simp
          exact hstep a (by rw [hinner]; simp) j1 (by rw [hj1]; exact usize_inc j k hjl)
      · rename_i heq
        have heq' : j.val = o.val := by simp at heq; rw [heq]
        step as ⟨j1, hj1⟩
        have hinner : innerOk holds.val nest.val k.val o.val j.val = true := by
          simp [innerOk, heq']
        exact hstep a (by rw [hinner]; simp) j1 (by rw [hj1]; exact usize_inc j k hjl)
    · rename_i h
      have : j.val = k.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [ha, this]
  · exact ⟨hp, hacc⟩

/-- **`owner` refines the rule**: the first outer loop, in order, that `isOwnerB`. -/
theorem owner_spec (k : Usize) (holds nest : Slice Bool) (hH : k.val ≤ holds.val.length)
    (hN : k.val * k.val ≤ nest.val.length) (hkk : k.val * k.val ≤ Usize.max)
    (hk : k.val ≤ U32.max) :
    owner k holds nest ⦃ r => r.map (·.val) =
      (List.range k.val).find? (isOwnerB holds.val nest.val k.val) ⦄ := by
  unfold owner owner_loop0
  apply loop.spec_decr_nat (fun (q : Option U32 × Usize) => k.val - q.2.val)
    (fun q => q.2.val ≤ k.val ∧
      q.1.map (·.val) = (List.range q.2.val).find? (isOwnerB holds.val nest.val k.val) ∧
      (q.1.isSome → q.1.map (·.val) = (List.range k.val).find? (isOwnerB holds.val nest.val k.val)))
  · rintro ⟨f, o⟩ ⟨ho, hf, hfs⟩
    unfold owner_loop0.body
    simp only at ho hf hfs ⊢
    split
    · rename_i h
      have hol : o.val < k.val := by scalar_tac
      -- the find over 0..k agrees with the one over 0..o once it has found something
      have hext' : ∀ m, m ≤ k.val → ∀ r,
          (List.range m).find? (isOwnerB holds.val nest.val k.val) = some r →
          (List.range k.val).find? (isOwnerB holds.val nest.val k.val) = some r := by
        intro m hm r hr
        have e : List.range k.val = List.range m ++ (List.range (k.val - m)).map (m + ·) := by
          rw [← List.range_add]; congr 1; omega
        rw [e, List.find?_append, hr]; rfl
      split
      · rename_i hnone
        have hf0 : f = none := by simpa [core.option.Option.is_none] using hnone
        step with Slice.index_usize_spec holds o (by simp; omega) as ⟨hb, hhb⟩
        have hHo : holds.val.getD o.val false = hb := by
          rw [List.getD_eq_getElem _ _ (by omega), ← hhb]
        split
        · rename_i hb1
          step with inner_spec k holds nest o hol hH hN hkk true 0#usize (by simp) (by simp)
            as ⟨ok1, hok1⟩
          have hP : isOwnerB holds.val nest.val k.val o.val = ok1 := by
            simp only [isOwnerB]; rw [hHo, hb1, hok1]; simp
          have hfind : (List.range (o.val + 1)).find? (isOwnerB holds.val nest.val k.val) =
              if ok1 then some o.val else none := by
            rw [find_range_succ, ← hf, hf0, hP]; simp
          split
          · rename_i hok
            step as ⟨c, hc⟩
            have hcv : c.val = o.val := by
              rw [hc]; simp only [UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt
              have : U32.max + 1 = 2 ^ UScalarTy.U32.numBits := by
                simp [U32.max_eq, UScalarTy.U32_numBits_eq]
              omega
            step as ⟨o1, ho1⟩
            have ho1v : o1.val = o.val + 1 := by rw [ho1]; exact usize_inc o k hol
            have hres : (some c).map (·.val) = (List.range (o.val + 1)).find?
                (isOwnerB holds.val nest.val k.val) := by
              rw [hfind, if_pos hok]; simp [hcv]
            refine ⟨by omega, by rw [ho1v]; exact hres, fun _ => ?_, by omega⟩
            rw [hres, hfind, if_pos hok]
            exact (hext' (o.val + 1) (by omega) o.val (by rw [hfind, if_pos hok])).symm
          · rename_i hok
            step as ⟨o1, ho1⟩
            have ho1v : o1.val = o.val + 1 := by rw [ho1]; exact usize_inc o k hol
            refine ⟨by omega, ?_, fun hs => by rw [hf0] at hs; simp at hs, by omega⟩
            rw [ho1v, hfind, if_neg hok, hf0]; rfl
        · rename_i hb1
          step as ⟨o1, ho1⟩
          have ho1v : o1.val = o.val + 1 := by rw [ho1]; exact usize_inc o k hol
          have hP : isOwnerB holds.val nest.val k.val o.val = false := by
            simp only [isOwnerB]; rw [hHo]; simp at hb1; simp [hb1]
          refine ⟨by omega, ?_, fun hs => by rw [hf0] at hs; simp at hs, by omega⟩
          rw [ho1v, find_range_succ, ← hf, hf0, hP]; simp
      · rename_i hsome
        have hs : f.isSome := by
          cases f <;> simp_all [core.option.Option.is_none]
        simp only [WP.spec_ok]
        exact hfs hs
    · rename_i h
      have : o.val = k.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hf, this]
  · exact ⟨by simp, by simp, fun h => by simp at h⟩

end forge_sheet_core
