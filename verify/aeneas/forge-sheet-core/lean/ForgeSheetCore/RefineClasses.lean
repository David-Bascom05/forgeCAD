import ForgeSheetCore.RefineUF
open Aeneas Aeneas.Std Result

/-!
# `classes` refines the vertex classes (verify/sheet Classes.lean, Region.lean §3)

The translation of `forge_sheet_core::classes`, the shipped Rust, is proven to return, for points
`0 .. n-1` and decided pairs `E`, the class of each point numbered by its least member: two points
get one number iff the pairs connect them (`classes_spec`), and the numbers count the classes'
least members in order. It cannot panic or overflow when `n ≤ u32::MAX` and every index is below
`n` (the stated limits).
-/

namespace forge_sheet_core

open ForgeSheetCore.UF

/-- The pairs as the model's list. -/
def pairsOf (pairs : Slice (U32 × U32)) : List (Nat × Nat) := pairs.val.map fun p => (p.1.val, p.2.val)

theorem u32_to_usize (a : U32) (r : Usize) (h : UScalar.cast .Usize a = r) : r.val = a.val := by
  rw [← h]; exact U32.cast_Usize_val_eq a

theorem lt_usize_max (j n : Usize) (h : j.val < n.val) : j.val < Usize.max := by scalar_tac

theorem len_of_vals {v : alloc.vec.Vec U32} {l : List Nat} (h : vals v = l) :
    v.val.length = l.length := by rw [← vals_length, h]

theorem vals_push (v : alloc.vec.Vec U32) (x : U32) (w : alloc.vec.Vec U32)
    (h : w.val = v.val ++ [x]) : vals w = vals v ++ [x.val] := by simp [vals, h]

theorem loop0_spec (pairs : Slice (U32 × U32)) (n : Nat) (hn : n ≤ U32.max)
    (hE : ∀ e ∈ pairsOf pairs, e.1 < n ∧ e.2 < n) (uf : union_find.Uf) (k : Usize)
    (hk : k.val ≤ pairs.val.length) (hi : UInv ((pairsOf pairs).take k.val) n uf) :
    classes_loop0 pairs uf k ⦃ u => UInv (pairsOf pairs) n u ⦄ := by
  unfold classes_loop0
  apply loop.spec_decr_nat (fun (q : union_find.Uf × Usize) => pairs.val.length - q.2.val)
    (fun q => q.2.val ≤ pairs.val.length ∧ UInv ((pairsOf pairs).take q.2.val) n q.1)
  · rintro ⟨u, j⟩ ⟨hj, hu⟩
    unfold classes_loop0.body
    simp only at hj hu ⊢
    split
    · rename_i h
      have hjl : j.val < pairs.val.length := by scalar_tac
      step with Slice.index_usize_spec pairs j (by simpa using hjl) as ⟨pa, pb, hp⟩
      have hmem : (pa.val, pb.val) ∈ pairsOf pairs := by
        simp only [pairsOf, List.mem_map]
        exact ⟨(pa, pb), by rw [hp]; exact List.getElem_mem hjl, rfl⟩
      have hab := hE _ hmem
      simp only at hab
      step as ⟨a, ha⟩
      step as ⟨b, hb⟩
      have hav : a.val = pa.val := by rw [ha]; exact U32.cast_Usize_val_eq _
      have hbv : b.val = pb.val := by rw [hb]; exact U32.cast_Usize_val_eq _
      have htake : ∀ e ∈ (pairsOf pairs).take j.val, e.1 < n ∧ e.2 < n :=
        fun e he => hE e (List.mem_of_mem_take he)
      step with union_spec u hu htake hn a b (by omega) (by omega) as ⟨u1, hu1⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by
        rw [hj1]; exact usize_inc j (Slice.len pairs) (by simpa using hjl)
      refine ⟨by omega, ?_, by omega⟩
      rw [hj1v]
      have e : (pairsOf pairs).take (j.val + 1) = (pairsOf pairs).take j.val ++ [(a.val, b.val)] := by
        rw [List.take_succ]
        congr 1
        simp only [pairsOf, List.getElem?_map]
        rw [List.getElem?_eq_getElem hjl, ← hp, hav, hbv]
        rfl
      rw [e]; exact hu1
    · rename_i h
      have : j.val = pairs.val.length := by scalar_tac
      simp only [WP.spec_ok]
      have ht : (pairsOf pairs).take j.val = pairsOf pairs := by
        rw [this]; simp [pairsOf]
      rw [← ht]; exact hu
  · exact ⟨hk, hi⟩

/-- How many least members (roots) lie below `m`. -/
def cnt (R : Nat → Nat) (m : Nat) : Nat := ((List.range m).filter fun k => R k = k).length

theorem cnt_succ (R : Nat → Nat) (m : Nat) :
    cnt R (m + 1) = cnt R m + if R m = m then 1 else 0 := by
  simp only [cnt, List.range_succ, List.filter_append, List.length_append]
  split <;> simp_all

theorem cnt_le (R : Nat → Nat) (m : Nat) : cnt R m ≤ m := by
  have := List.length_filter_le (fun k => R k = k) (List.range m)
  simpa [cnt] using this

theorem cnt_mono (R : Nat → Nat) {a b : Nat} (h : a ≤ b) : cnt R a ≤ cnt R b := by
  induction b with
  | zero => have : a = 0 := by omega
            subst this; exact le_rfl
  | succ b ih =>
    rcases Nat.eq_or_lt_of_le h with e | e
    · rw [e]
    · rw [cnt_succ]; have := ih (by omega); omega

/-- **Roots are numbered injectively**: below a root `b`, a root `a < b` has a smaller count. -/
theorem cnt_lt (R : Nat → Nat) {a b : Nat} (ha : R a = a) (h : a < b) : cnt R a < cnt R b := by
  have h1 : cnt R (a + 1) = cnt R a + 1 := by rw [cnt_succ, if_pos ha]
  have := cnt_mono R (show a + 1 ≤ b by omega)
  omega

/-- The two vectors the second loop builds, as lists. -/
def numberList (R : Nat → Nat) (i : Nat) : List Nat :=
  (List.range i).map fun j => if R j = j then cnt R j else 0

def classList (R : Nat → Nat) (i : Nat) : List Nat := (List.range i).map fun j => cnt R (R j)

theorem loop1_spec (n : Usize) (hn : n.val ≤ U32.max) (p0 : List Nat) (hg0 : Good p0)
    (hl0 : p0.length = n.val) (uf : union_find.Uf) (number cl : alloc.vec.Vec U32) (next : U32)
    (i : Usize) (hi : i.val ≤ n.val) (hg : Good (vals uf.parent))
    (hl : uf.parent.val.length = n.val)
    (hr : ∀ y, ufRoot (vals uf.parent) y = ufRoot p0 y)
    (hnum : vals number = numberList (ufRoot p0) i.val)
    (hcl : vals cl = classList (ufRoot p0) i.val) (hnext : next.val = cnt (ufRoot p0) i.val) :
    classes_loop1 n uf number cl next i ⦃ c => vals c = classList (ufRoot p0) n.val ⦄ := by
  unfold classes_loop1
  apply loop.spec_decr_nat
    (fun (q : union_find.Uf × alloc.vec.Vec U32 × alloc.vec.Vec U32 × U32 × Usize) => n.val - q.2.2.2.2.val)
    (fun q => q.2.2.2.2.val ≤ n.val ∧ Good (vals q.1.parent) ∧ q.1.parent.val.length = n.val ∧
      (∀ y, ufRoot (vals q.1.parent) y = ufRoot p0 y) ∧
      vals q.2.1 = numberList (ufRoot p0) q.2.2.2.2.val ∧
      vals q.2.2.1 = classList (ufRoot p0) q.2.2.2.2.val ∧
      q.2.2.2.1.val = cnt (ufRoot p0) q.2.2.2.2.val)
  · rintro ⟨u, nu, c, nx, j⟩ ⟨hj, hgu, hlu, hru, hnu, hc, hnx⟩
    unfold classes_loop1.body
    simp only at hj hgu hlu hru hnu hc hnx ⊢
    split
    · rename_i h
      have hjl : j.val < n.val := by scalar_tac
      step with find_spec u j (by omega) hgu as ⟨r, u1, hrv, hl1, hg1, hr1⟩
      have hrr : r.val = ufRoot p0 j.val := by rw [hrv, hru]
      have hrle : ufRoot p0 j.val ≤ j.val := root_le hg0 j.val
      have hrr2 : ufRoot p0 (ufRoot p0 j.val) = ufRoot p0 j.val := root_root hg0 j.val
      have hj1v : ∀ j1 : Usize, j1 = j.wrapping_add 1#usize → j1.val = j.val + 1 :=
        fun j1 e => by rw [e]; exact usize_inc j n hjl
      split
      · rename_i heq
        have hroot : ufRoot p0 j.val = j.val := by rw [← hrr]; exact congrArg UScalar.val heq
        step with alloc.vec.Vec.push_spec nu nx (by rw [len_of_vals hnu]; simp only [numberList, List.length_map, List.length_range]; exact lt_usize_max j n hjl)
          as ⟨nu2, hnu2⟩
        step with alloc.vec.Vec.push_spec c nx (by rw [len_of_vals hc]; simp only [classList, List.length_map, List.length_range]; exact lt_usize_max j n hjl)
          as ⟨c2, hc2⟩
        step as ⟨nx2, hnx2⟩
        have hnx2v : nx2.val = nx.val + 1 := by
          rw [hnx2, U32.wrapping_add_val_eq]; apply Nat.mod_eq_of_lt
          have hc := cnt_le (ufRoot p0) j.val
          have h1 : (1#u32 : U32).val = 1 := by simp
          have hs : UScalar.size .U32 = 2 ^ 32 := by simp [U32.size, U32.numBits]
          have hm : U32.max = 2 ^ 32 - 1 := by simp [U32.max_eq]
          rw [h1, hs]
          omega
        step as ⟨j1, hj1⟩
        have hj1' := hj1v j1 hj1
        refine ⟨by omega, hg1, by omega, fun y => by rw [hr1, hru], ?_, ?_, ?_, by omega⟩
        · rw [hj1', vals_push _ _ _ hnu2, hnu]
          simp [numberList, List.range_succ, hroot, hnx]
        · rw [hj1', vals_push _ _ _ hc2, hc]
          simp [classList, List.range_succ, hroot, hnx]
        · rw [hj1', hnx2v, cnt_succ, if_pos hroot, hnx]
      · rename_i hne
        have hroot : ufRoot p0 j.val ≠ j.val := by
          intro e; apply hne; apply UScalar.eq_of_val_eq; rw [hrr, e]
        have hlt : ufRoot p0 j.val < j.val := by omega
        step with alloc.vec.Vec.push_spec nu 0#u32 (by rw [len_of_vals hnu]; simp only [numberList, List.length_map, List.length_range]; exact lt_usize_max j n hjl)
          as ⟨nu2, hnu2⟩
        have hvn2 : vals nu2 = numberList (ufRoot p0) j.val ++ [0] := by
          rw [vals_push _ _ _ hnu2, hnu]; simp
        have hlen2 : nu2.val.length = j.val + 1 := by
          rw [len_of_vals hvn2]; simp [numberList]
        have hrl : r.val < nu2.val.length := by omega
        step with alloc.vec.Vec.index_usize_spec nu2 r (by simpa using hrl) as ⟨cc, hcc⟩
        have hccv : cc.val = cnt (ufRoot p0) (ufRoot p0 j.val) := by
          have e1 : cc.val = (vals nu2)[r.val]'(by rw [vals_length]; exact hrl) := by
            rw [hcc]; simp [vals]
          rw [e1]; simp only [hvn2]
          rw [List.getElem_append_left (by simp [numberList]; omega)]
          simp [numberList, hrr, hrr2]
        step with alloc.vec.Vec.push_spec c cc (by rw [len_of_vals hc]; simp only [classList, List.length_map, List.length_range]; exact lt_usize_max j n hjl)
          as ⟨c2, hc2⟩
        step as ⟨j1, hj1⟩
        have hj1' := hj1v j1 hj1
        refine ⟨by omega, hg1, by omega, fun y => by rw [hr1, hru], ?_, ?_, ?_, by omega⟩
        · rw [hj1', hvn2]; simp [numberList, List.range_succ, hroot]
        · rw [hj1', vals_push _ _ _ hc2, hc, hccv]
          simp [classList, List.range_succ]
        · rw [hj1', cnt_succ, if_neg hroot, hnx]; simp
    · rename_i h
      have : j.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [hc, this]
  · exact ⟨hi, hg, hl, hr, hnum, hcl, hnext⟩

/-- With `R` naming each class by its least member, `cnt R (R ·)` is one number per class, and
different classes get different numbers. -/
theorem classList_iff {E : List (Nat × Nat)} {n : Nat} {R : Nat → Nat} (hl : Least E n R)
    (hrr : ∀ y, y < n → R (R y) = R y) {y z : Nat} (hy : y < n) (hz : z < n) :
    cnt R (R y) = cnt R (R z) ↔ Eqv E y z := by
  rw [least_iff hl hy hz]
  constructor
  · intro h
    by_contra hne
    rcases Nat.lt_or_gt_of_ne hne with h' | h'
    · have := cnt_lt R (hrr y hy) h'; omega
    · have := cnt_lt R (hrr z hz) h'; omega
  · intro h; rw [h]

/-- **`classes` refines the vertex classes**: for `n ≤ u32::MAX` points and decided pairs whose
indices are below `n`, the shipped `classes` returns without panic or overflow a vector of `n`
numbers; two points get one number iff the pairs connect them (`Eqv`, the equivalence they
generate: Classes.lean's `Conn` over the decided relation), and each number counts the classes
whose least member is below the point's class's least member (classes numbered in order of least
members, region.rs's numbering). -/
theorem classes_spec (n : Usize) (hn : n.val ≤ U32.max) (pairs : Slice (U32 × U32))
    (hE : ∀ e ∈ pairsOf pairs, e.1 < n.val ∧ e.2 < n.val) :
    classes n pairs ⦃ c => ∃ R : Nat → Nat, Least (pairsOf pairs) n.val R ∧
      vals c = (List.range n.val).map (fun j => cnt R (R j)) ∧
      ∀ y z, y < n.val → z < n.val →
        ((vals c)[y]? = (vals c)[z]? ↔ Eqv (pairsOf pairs) y z) ⦄ := by
  unfold classes
  step with new_spec n (by omega) as ⟨u0, hu0⟩
  have hi0 : UInv ((pairsOf pairs).take (0#usize : Usize).val) n.val u0 := by
    simp only [UInv, List.take_zero]
    rw [hu0]; exact inv_range n.val
  step with loop0_spec pairs n.val hn hE u0 0#usize (by simp) hi0 as ⟨u1, hu1⟩
  obtain ⟨hg1, hl1, hle1⟩ := hu1
  rw [vals_length] at hl1
  step with loop1_spec n hn (vals u1.parent) hg1 (by rw [vals_length]; exact hl1) u1
    (alloc.vec.Vec.with_capacity U32 n) (alloc.vec.Vec.with_capacity U32 n) 0#u32 0#usize
    (by simp) hg1 hl1 (fun _ => rfl) (by simp [vals, numberList, alloc.vec.Vec.with_capacity])
    (by simp [vals, classList, alloc.vec.Vec.with_capacity]) (by simp [cnt]) as ⟨c, hc⟩
  refine ⟨ufRoot (vals u1.parent), hle1, hc, fun y z hy hz => ?_⟩
  rw [hc]
  simp only [classList, List.getElem?_map, List.getElem?_range hy, List.getElem?_range hz,
    Option.map_some, Option.some.injEq]
  exact classList_iff hle1 (fun w _ => root_root hg1 w) hy hz

end forge_sheet_core
