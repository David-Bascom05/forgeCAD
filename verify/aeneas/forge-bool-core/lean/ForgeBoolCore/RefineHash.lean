import ForgeBoolCore.RefineHashSort
import ForgeBoolCore.RefineIndex

/-!
# The hashed index: `hash_cov`, `same`, `hash_table`, `Index::find_h`

`hash_cov` is a fold of `U64` operations over the signs (`hashF`): equal covectors hash alike, which
is all the lookup needs. `hash_table` lists `(hash, id)` for every cell, sorted by `le2`
(`HashOK`). `find_h` finds the first entry of hash `h` by binary search, then scans that run,
comparing covectors with `same`. It returns the id with that covector, or `NONE` if there is none
(`find_h_spec`, the same `FindOK` as `Index::find`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

def h64 (s : Sgn) : U64 := UScalar.ofNatCore (rk s) (by cases s <;> simp [rk])

def hashStep (h : U64) (s : Sgn) : U64 := core.num.U64.wrapping_mul (h ^^^ h64 s) 1099511628211#u64

/-- The hash of a covector. -/
def hashF (c : List Sgn) : U64 := c.foldl hashStep 14695981039346656037#u64

theorem hash_cov_loop_spec (c : Slice Sgn) (h : U64) (i : Usize) (hi : i.val ≤ c.val.length)
    (hh : h = (c.val.take i.val).foldl hashStep 14695981039346656037#u64) :
    index.hash_cov_loop c h i ⦃ r => r = hashF c.val ⦄ := by
  unfold index.hash_cov_loop
  apply loop.spec_decr_nat (fun (q : U64 × Usize) => c.val.length - q.2.val)
    (fun q => q.2.val ≤ c.val.length ∧ q.1 = (c.val.take q.2.val).foldl hashStep 14695981039346656037#u64)
  · rintro ⟨h0, j⟩ ⟨hj, hh0⟩
    unfold index.hash_cov_loop.body
    simp only at hj hh0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < c.val.length := by scalar_tac
      step as ⟨s, hs⟩
      step with rank_spec s as ⟨r, hr⟩
      step as ⟨r64, hr64⟩
      step as ⟨x, hx⟩
      step as ⟨h1, hh1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      have e64 : r64 = h64 s := by
        apply UScalar.eq_of_val_eq
        rw [hr64]; simp only [h64, UScalar.ofNatCore_val_eq]; scalar_tac
      rw [hj1, List.take_succ_eq_append_getElem hjl, List.foldl_append, ← hh0]
      simp only [List.foldl_cons, List.foldl_nil, hashStep]
      have ex : x = h0 ^^^ r64 := UScalar.eq_of_val_eq hx
      rw [hh1, ex, e64, hs]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = c.val.length := by scalar_tac
      rw [hh0, this, List.take_length]; rfl
  · exact ⟨hi, hh⟩

theorem hash_cov_spec (c : Slice Sgn) : index.hash_cov c ⦃ r => r = hashF c.val ⦄ :=
  hash_cov_loop_spec c _ 0#usize (by simp) (by simp)

theorem same_loop_spec (a b : Slice Sgn) (sm : Bool) (i : Usize) (hi : i.val ≤ a.val.length)
    (h1 : sm = true → a.val.length = b.val.length ∧ a.val.take i.val = b.val.take i.val)
    (h2 : sm = false → a.val ≠ b.val) :
    index.same_loop a b sm i ⦃ r => r = decide (a.val = b.val) ⦄ := by
  unfold index.same_loop
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => a.val.length + 1 - q.2.val - (if q.1 then 0 else 1))
    (fun q => q.2.val ≤ a.val.length ∧
      (q.1 = true → a.val.length = b.val.length ∧ a.val.take q.2.val = b.val.take q.2.val) ∧
      (q.1 = false → a.val ≠ b.val))
  · rintro ⟨s0, j⟩ ⟨hj, hs1, hs2⟩
    unfold index.same_loop.body
    simp only at hj hs1 hs2 ⊢
    split
    · rename_i hst
      obtain ⟨hl, ht⟩ := hs1 hst
      split
      · rename_i hlt
        have hja : j.val < a.val.length := by scalar_tac
        have hjb : j.val < b.val.length := by omega
        step as ⟨x, hx⟩
        step as ⟨y, hy⟩
        rw [sgn_eq]; simp only [Std.bind_ok]
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, fun he => ⟨hl, ?_⟩, fun he => ?_, ?_⟩
        · have : x = y := by simpa using he
          rw [hj1, List.take_succ_eq_append_getElem hja, List.take_succ_eq_append_getElem hjb, ht, ← hx, ← hy, this]
        · have : x ≠ y := by simpa using he
          intro hab; apply this; rw [hx, hy]; simp [hab]
        · simp; split <;> simp_all <;> omega
      · rename_i hge
        simp only [WP.spec_ok]
        have hja : a.val.length ≤ j.val := by scalar_tac
        have : a.val = b.val := by
          rw [← List.take_of_length_le hja, ← List.take_of_length_le (show b.val.length ≤ j.val by omega)]; exact ht
        simp [hst, this]
    · rename_i hsf
      simp only [WP.spec_ok]
      have : s0 = false := by simpa using hsf
      simp [this, hs2 this]
  · exact ⟨hi, h1, h2⟩

theorem same_spec (a b : Slice Sgn) : index.same a b ⦃ r => r = decide (a.val = b.val) ⦄ := by
  unfold index.same
  apply same_loop_spec a b _ 0#usize (by simp)
  · intro h; simp at h; exact ⟨by scalar_tac, by simp⟩
  · intro h hab; simp at h; apply h; rw [hab]

/-! ## The hash table -/

def hsAt (hs : alloc.vec.Vec (U64 × U32)) (p : Nat) : U64 × U32 := hs.val.getD p (0#u64, 0#u32)

theorem hsAt_eq {hs : alloc.vec.Vec (U64 × U32)} {p : Nat} (h : p < hs.val.length) :
    hsAt hs p = hs.val[p] := by
  simp [hsAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

/-- Every entry is `(hash of cell id, id)` for an id in range, every id has an entry, and the
entries are sorted by hash. -/
structure HashOK (hs : alloc.vec.Vec (U64 × U32)) (cells : Slice (alloc.vec.Vec Sgn)) : Prop where
  ids_lt : ∀ p, p < hs.val.length → (hsAt hs p).2.val < cells.val.length
  hash : ∀ p, p < hs.val.length → (hsAt hs p).1 = hashF (cellAt cells (hsAt hs p).2.val)
  all : ∀ id, id < cells.val.length → ∃ p, p < hs.val.length ∧ (hsAt hs p).2.val = id
  sorted : ∀ p q, p ≤ q → q < hs.val.length → (hsAt hs p).1.val ≤ (hsAt hs q).1.val
  small : cells.val.length < U32.max

/-- The unsorted table: entry `p` is `(hash of cell p, p)`. -/
def RawOK (hs : List (U64 × U32)) (cells : Slice (alloc.vec.Vec Sgn)) (n : Nat) : Prop :=
  hs.length = n ∧ ∀ p (hp : p < hs.length), hs[p].2.val = p ∧ hs[p].1 = hashF (cellAt cells p)

theorem hash_table_loop_spec (cells : Slice (alloc.vec.Vec Sgn)) (hsmall : cells.val.length < U32.max)
    (hs : alloc.vec.Vec (U64 × U32)) (i : Usize) (hi : i.val ≤ cells.val.length)
    (hraw : RawOK hs.val cells i.val) :
    index.hash_table_loop cells hs i ⦃ r => RawOK r.val cells cells.val.length ⦄ := by
  unfold index.hash_table_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec (U64 × U32) × Usize) => cells.val.length - q.2.val)
    (fun q => q.2.val ≤ cells.val.length ∧ RawOK q.1.val cells q.2.val)
  · rintro ⟨h0, j⟩ ⟨hj, hr0⟩
    unfold index.hash_table_loop.body
    simp only at hj hr0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < cells.val.length := by scalar_tac
      step as ⟨v, hv⟩
      step with hash_cov_spec (alloc.vec.Vec.deref v) as ⟨hh, hhh⟩
      step as ⟨i3, hi3⟩
      step as ⟨h1, hh1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ⟨?_, fun p hp => ?_⟩, by scalar_tac⟩
      · rw [hh1]; simp [hr0.1]; scalar_tac
      · have hl : h0.val.length = j.val := hr0.1
        simp only [hh1] at hp ⊢
        by_cases hpj : p < h0.val.length
        · rw [List.getElem_append_left hpj]; exact hr0.2 p hpj
        · have : p = h0.val.length := by simp at hp; omega
          subst this
          simp only [List.getElem_concat_length]
          refine ⟨by rw [hi3, hl]; scalar_tac, ?_⟩
          rw [hhh, hl, cellAt_eq hjl, hv]; simp [alloc.vec.Vec.deref]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = cells.val.length := by scalar_tac
      rw [← this]; exact hr0
  · exact ⟨hi, hraw⟩

theorem hash_table_spec (cells : Slice (alloc.vec.Vec Sgn)) (hsmall : cells.val.length < U32.max)
    (hmax : 2 * cells.val.length ≤ Usize.max) :
    index.hash_table cells ⦃ hs => HashOK hs cells ⦄ := by
  unfold index.hash_table
  step with hash_table_loop_spec cells hsmall (alloc.vec.Vec.with_capacity (U64 × U32) (Slice.len cells)) 0#usize (by simp)
    (by simp [RawOK, alloc.vec.Vec.with_capacity]) as ⟨raw, hraw⟩
  step with h_sort_hashed_spec raw (by rw [hraw.1]; exact hmax) as ⟨r, hrp, hrs⟩
  have hrl : r.val.length = cells.val.length := by rw [hrp.length_eq, hraw.1]
  -- each entry of `r` is an entry of `raw`
  have hmem : ∀ p (hp : p < r.val.length), r.val[p].2.val < cells.val.length ∧
      r.val[p].1 = hashF (cellAt cells r.val[p].2.val) := by
    intro p hp
    have hm : r.val[p] ∈ raw.val := hrp.mem_iff.mp (List.getElem_mem hp)
    obtain ⟨q, hq, hqe⟩ := List.getElem_of_mem hm
    obtain ⟨h1, h2⟩ := hraw.2 q hq
    rw [← hqe, h1, h2]; exact ⟨by rw [← hraw.1]; exact hq, rfl⟩
  refine ⟨fun p hp => ?_, fun p hp => ?_, fun id hid => ?_, fun p q hpq hq => ?_, hsmall⟩
  · rw [hsAt_eq hp]; exact (hmem p hp).1
  · rw [hsAt_eq hp]; exact (hmem p hp).2
  · have hid' : id < raw.val.length := by rw [hraw.1]; exact hid
    have hm : raw.val[id] ∈ r.val := hrp.mem_iff.mpr (List.getElem_mem hid')
    obtain ⟨p, hp, hpe⟩ := List.getElem_of_mem hm
    exact ⟨p, hp, by rw [hsAt_eq hp, hpe]; exact (hraw.2 id hid').1⟩
  · rw [hsAt_eq (by omega), hsAt_eq hq]
    rcases Nat.lt_or_eq_of_le hpq with h | h
    · have := List.pairwise_iff_getElem.mp hrs p q (by omega) hq h
      rcases this with e | e
      · exact Nat.le_of_lt e
      · exact Nat.le_of_eq e.1
    · subst h; exact Nat.le_refl _

/-! ## The lookup -/

theorem find_h_loop0_spec (hs : alloc.vec.Vec (U64 × U32)) (h : U64) (lo hi : Usize)
    (hsort : ∀ p q, p ≤ q → q < hs.val.length → (hsAt hs p).1.val ≤ (hsAt hs q).1.val)
    (hl : lo.val ≤ hi.val ∧ hi.val ≤ hs.val.length)
    (hb : ∀ q, q < lo.val → (hsAt hs q).1.val < h.val)
    (ha : ∀ q, hi.val ≤ q → q < hs.val.length → h.val ≤ (hsAt hs q).1.val) :
    index.Index.find_h_loop0 hs h lo hi ⦃ r => r.val ≤ hs.val.length ∧
      (∀ q, q < r.val → (hsAt hs q).1.val < h.val) ∧
      (∀ q, r.val ≤ q → q < hs.val.length → h.val ≤ (hsAt hs q).1.val) ⦄ := by
  unfold index.Index.find_h_loop0
  apply loop.spec_decr_nat (fun (q : Usize × Usize) => q.2.val - q.1.val)
    (fun q => q.1.val ≤ q.2.val ∧ q.2.val ≤ hs.val.length ∧
      (∀ p, p < q.1.val → (hsAt hs p).1.val < h.val) ∧
      (∀ p, q.2.val ≤ p → p < hs.val.length → h.val ≤ (hsAt hs p).1.val))
  · rintro ⟨l, u⟩ ⟨h1, h2, h3, h4⟩
    unfold index.Index.find_h_loop0.body
    simp only at h1 h2 h3 h4 ⊢
    split
    · rename_i hlt
      step as ⟨d, hd⟩
      step as ⟨d2, hd2⟩
      step as ⟨mid, hmid⟩
      have hml : mid.val < hs.val.length := by scalar_tac
      have hmlo : l.val ≤ mid.val ∧ mid.val < u.val := by scalar_tac
      step as ⟨e, e2, he⟩
      have hev : (hsAt hs mid.val).1 = e := by rw [hsAt_eq hml, ← he]
      split
      · rename_i hc
        have hc' : e.val < h.val := by scalar_tac
        step as ⟨l1, hl1⟩
        refine ⟨by scalar_tac, h2, fun p hp => ?_, h4, by scalar_tac⟩
        have := hsort p mid.val (by scalar_tac) hml
        rw [hev] at this; omega
      · rename_i hc
        have hc' : h.val ≤ e.val := by scalar_tac
        simp only [WP.spec_ok]
        refine ⟨by scalar_tac, by scalar_tac, h3, fun p hp hpl => ?_, by scalar_tac⟩
        have := hsort mid.val p hp hpl
        rw [hev] at this; omega
    · rename_i hge
      simp only [WP.spec_ok]
      have : u.val ≤ l.val := by scalar_tac
      exact ⟨by omega, h3, fun p hp hpl => h4 p (by omega) hpl⟩
  · exact ⟨hl.1, hl.2, hb, ha⟩

/-- Where the scan stops with nothing found, no cell has the key: its entry would have hash `h`,
so lie at or past `lo`, and (the hashes being sorted) before the stop. -/
theorem scan_none {hs : alloc.vec.Vec (U64 × U32)} {cells : Slice (alloc.vec.Vec Sgn)} {key : List Sgn}
    (hok : HashOK hs cells) {lo p : Nat} (hlp : lo ≤ p)
    (hb : ∀ q, q < lo → (hsAt hs q).1.val < (hashF key).val)
    (ha : ∀ q, lo ≤ q → q < hs.val.length → (hashF key).val ≤ (hsAt hs q).1.val)
    (hscan : ∀ q, lo ≤ q → q < p → cellAt cells (hsAt hs q).2.val ≠ key)
    (hstop : p < hs.val.length → (hsAt hs p).1 ≠ hashF key) :
    ∀ id, id < cells.val.length → cellAt cells id ≠ key := by
  intro id hid heq
  obtain ⟨q, hq, hqe⟩ := hok.all id hid
  have hhq : (hsAt hs q).1 = hashF key := by rw [hok.hash q hq, hqe, heq]
  by_cases hql : q < lo
  · have := hb q hql; rw [hhq] at this; omega
  by_cases hqp : q < p
  · exact hscan q (by omega) hqp (by rw [hqe]; exact heq)
  · have hpl : p < hs.val.length := by omega
    apply hstop hpl
    apply UScalar.eq_of_val_eq
    have h1 := hok.sorted p q (by omega) hq
    have h2 := ha p hlp hpl
    rw [hhq] at h1
    omega

theorem find_h_loop1_spec (hs : alloc.vec.Vec (U64 × U32)) (cells : Slice (alloc.vec.Vec Sgn))
    (key : Slice Sgn) (hok : HashOK hs cells) (lo : Nat)
    (hb : ∀ q, q < lo → (hsAt hs q).1.val < (hashF key.val).val)
    (ha : ∀ q, lo ≤ q → q < hs.val.length → (hashF key.val).val ≤ (hsAt hs q).1.val)
    (at0 : U32) (p : Usize) (hp : lo ≤ p.val ∧ p.val ≤ hs.val.length)
    (hnone : at0 = NONE → ∀ q, lo ≤ q → q < p.val → cellAt cells (hsAt hs q).2.val ≠ key.val)
    (hsome : at0 ≠ NONE → at0.val < cells.val.length ∧ cellAt cells at0.val = key.val) :
    index.Index.find_h_loop1 hs cells key (hashF key.val) at0 p ⦃ r => FindOK cells key.val r ⦄ := by
  unfold index.Index.find_h_loop1
  apply loop.spec_decr_nat
    (fun (q : U32 × Usize) => if q.1 = NONE then hs.val.length + 1 - q.2.val else 0)
    (fun q => lo ≤ q.2.val ∧ q.2.val ≤ hs.val.length ∧
      (q.1 = NONE → ∀ r, lo ≤ r → r < q.2.val → cellAt cells (hsAt hs r).2.val ≠ key.val) ∧
      (q.1 ≠ NONE → q.1.val < cells.val.length ∧ cellAt cells q.1.val = key.val))
  · rintro ⟨a, j⟩ ⟨h1, h2, h3, h4⟩
    unfold index.Index.find_h_loop1.body
    simp only at h1 h2 h3 h4 ⊢
    split
    · rename_i hna
      have hscan := h3 hna
      split
      · rename_i hlt
        have hjl : j.val < hs.val.length := by scalar_tac
        step as ⟨e1, e2, he⟩
        have hev : hsAt hs j.val = (e1, e2) := by rw [hsAt_eq hjl, ← he]
        split
        · rename_i heh
          have hidl : e2.val < cells.val.length := by have := hok.ids_lt j.val hjl; rw [hev] at this; exact this
          step as ⟨i3, hi3⟩
          have hi3v : i3.val = e2.val := by rw [hi3]; scalar_tac
          step as ⟨v1, hv1⟩
          step with same_spec (alloc.vec.Vec.deref v1) key as ⟨b, hbv⟩
          have hcell : (alloc.vec.Vec.deref v1).val = cellAt cells e2.val := by
            rw [cellAt_eq hidl, hv1]; simp [alloc.vec.Vec.deref, hi3v]
          rw [hcell] at hbv
          split
          · rename_i hbt
            have hkey : cellAt cells e2.val = key.val := by simpa [hbv] using hbt
            have hn : e2 ≠ NONE := by
              intro e; have := hok.small; rw [e, none_val] at hidl; omega
            step as ⟨j1, hj1⟩
            simp only [hn, ite_false]
            refine ⟨by scalar_tac, by scalar_tac, fun e => e.elim, fun _ => ⟨hidl, hkey⟩, by omega⟩
          · rename_i hbf
            have hkey : cellAt cells e2.val ≠ key.val := by simpa [hbv] using hbf
            step as ⟨j1, hj1⟩
            simp only [hna, ite_true]
            refine ⟨by scalar_tac, by scalar_tac, fun _ r hr1 hr2 => ?_, fun e => absurd rfl e, by scalar_tac⟩
            by_cases hrj : r = j.val
            · rw [hrj, hev]; exact hkey
            · exact hscan r hr1 (by scalar_tac)
        · rename_i heh
          simp only [WP.spec_ok]
          refine ⟨fun _ => scan_none hok h1 hb ha hscan fun _ e => heh ?_, fun e => absurd hna e⟩
          rw [hev] at e; simpa using e
      · rename_i hge
        simp only [WP.spec_ok]
        refine ⟨fun _ => scan_none hok h1 hb ha hscan fun hl => absurd hl (by scalar_tac), fun e => absurd hna e⟩
    · rename_i hna
      simp only [WP.spec_ok]
      exact ⟨fun e => absurd e hna, fun _ => h4 hna⟩
  · exact ⟨hp.1, hp.2, hnone, hsome⟩

/-- **`Index::find_h` is lookup by covector**, over a hash table that is sorted and holds every id. -/
theorem find_h_spec (ix : index.Index) (cells : Slice (alloc.vec.Vec Sgn)) (key : Slice Sgn)
    (hok : HashOK ix.hs cells) :
    index.Index.find_h ix cells key ⦃ r => FindOK cells key.val r ⦄ := by
  unfold index.Index.find_h
  step with hash_cov_spec key as ⟨h, hh⟩
  step with find_h_loop0_spec ix.hs h 0#usize (alloc.vec.Vec.len ix.hs) hok.sorted (by simp)
    (fun q hq => by simp at hq) (fun q hq hql => by simp at hq; omega) as ⟨lo, hlo1, hlo2, hlo3⟩
  subst hh
  exact find_h_loop1_spec ix.hs cells key hok lo.val hlo2 hlo3 NONE lo ⟨le_refl _, hlo1⟩
    (fun _ q h1 h2 => absurd h2 (by omega)) (fun e => absurd rfl e)

end forge_bool_core
