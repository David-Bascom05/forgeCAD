import ForgeBoolCore.Specs
open Aeneas Aeneas.Std Result

/-!
# Group K: `collapse` (stage 4b) refines the Lean `collapse`

The Rust works on tope positions `0 .. nt-1`: the thin and enclosed tables by position, a
union–find over the facets with both sides thin, each class's rim (its non-thin neighbours'
positions, sorted, without repeats), then the refusal scan and the fill.

Part 1 is pure Lean: over any ledger whose facets' sides sit at positions `pp k`, `qq k` of its
topes, the Lean `collapse` is a function of those position tables (`K.collapse_eq`). Part 2 proves
each Rust loop computes its table. Part 3 assembles `CollapseSpec`.
-/

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-! ## Part 1: the Lean collapse, by positions -/

namespace K

/-- The thin edges among the first `K` facets, as position pairs (the unions the Rust makes). -/
def kE (pp qq : Nat → Nat) (th : Nat → Bool) : Nat → List (Nat × Nat)
  | 0 => []
  | k + 1 => kE pp qq th k ++ (if th (pp k) && th (qq k) then [(pp k, qq k)] else [])

theorem mem_kE (pp qq : Nat → Nat) (th : Nat → Bool) :
    ∀ (K : Nat) (e : Nat × Nat), e ∈ kE pp qq th K ↔
      ∃ k, k < K ∧ th (pp k) = true ∧ th (qq k) = true ∧ e = (pp k, qq k)
  | 0, e => by simp [kE]
  | K + 1, e => by
    rw [kE, List.mem_append, mem_kE pp qq th K e]
    constructor
    · rintro (⟨k, hk, h1, h2, h3⟩ | h)
      · exact ⟨k, by omega, h1, h2, h3⟩
      · split at h
        · rename_i hc
          simp only [Bool.and_eq_true] at hc
          simp only [List.mem_singleton] at h
          exact ⟨K, by omega, hc.1, hc.2, h⟩
        · simp at h
    · rintro ⟨k, hk, h1, h2, h3⟩
      by_cases e1 : k = K
      · subst e1; right; simp [h1, h2, h3]
      · left; exact ⟨k, by omega, h1, h2, h3⟩

theorem kE_nil (pp qq : Nat → Nat) (th : Nat → Bool) (K : Nat)
    (h : ∀ k, k < K → th (pp k) = false) : kE pp qq th K = [] := by
  apply List.eq_nil_iff_forall_not_mem.mpr
  intro e he
  obtain ⟨k, hk, h1, _⟩ := (mem_kE pp qq th K e).mp he
  rw [h k hk] at h1; cases h1

/-- The facets' sides sit at positions `pp k`, `qq k` of the topes. -/
structure Sides (L : ForgeExact.DecideOnce.Ledger) (nf : Nat) (pp qq : Nat → Nat) : Prop where
  len : L.facets.length = nf
  pp_lt : ∀ k, k < nf → pp k < L.topes.length
  qq_lt : ∀ k, k < nf → qq k < L.topes.length
  pp_eq : ∀ k (hk : k < nf), L.topes[pp k]'(pp_lt k hk) = (L.facets[k]'(by omega)).plus
  qq_eq : ∀ k (hk : k < nf), L.topes[qq k]'(qq_lt k hk) = (L.facets[k]'(by omega)).minus

variable {L : ForgeExact.DecideOnce.Ledger} {nf : Nat} {pp qq : Nat → Nat}

theorem topes_inj {a b : Nat} (ha : a < L.topes.length) (hb : b < L.topes.length)
    (h : L.topes[a] = L.topes[b]) : a = b :=
  (List.Nodup.getElem_inj_iff (Ledger.topes_nodup L)).mp h

/-- Adjacent across a facet, by positions. -/
theorem tAdj_iff (hS : Sides L nf pp qq) {a b : Nat} (ha : a < L.topes.length) (hb : b < L.topes.length) :
    L.tAdj L.topes[a] L.topes[b] = true ↔
      ∃ k, k < nf ∧ ((pp k = a ∧ qq k = b) ∨ (pp k = b ∧ qq k = a)) := by
  unfold Ledger.tAdj
  rw [List.any_eq_true]
  constructor
  · rintro ⟨φ, hφ, hh⟩
    obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hφ
    have hk' : k < nf := by rw [← hS.len]; exact hk
    have e1 := hS.pp_eq k hk'
    have e2 := hS.qq_eq k hk'
    simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at hh
    refine ⟨k, hk', ?_⟩
    rcases hh with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · left
      exact ⟨topes_inj (hS.pp_lt k hk') ha (by rw [e1, h1]), topes_inj (hS.qq_lt k hk') hb (by rw [e2, h2])⟩
    · right
      exact ⟨topes_inj (hS.pp_lt k hk') hb (by rw [e1, h1]), topes_inj (hS.qq_lt k hk') ha (by rw [e2, h2])⟩
  · rintro ⟨k, hk, h⟩
    have hk' : k < L.facets.length := by rw [hS.len]; exact hk
    refine ⟨L.facets[k], List.getElem_mem _, ?_⟩
    have e1 := hS.pp_eq k hk
    have e2 := hS.qq_eq k hk
    simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq]
    rcases h with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · left; subst h1 h2; exact ⟨e1.symm, e2.symm⟩
    · right; subst h1 h2; exact ⟨e1.symm, e2.symm⟩

/-- The thin positions are the Lean's thin topes. -/
def ThinAt (L : ForgeExact.DecideOnce.Ledger) (thin : List Cov) (th : Nat → Bool) : Prop :=
  ∀ t (ht : t < L.topes.length), th t = thin.contains L.topes[t]

theorem mem_thinTopes_iff {thin : List Cov} {th : Nat → Bool} (hth : ThinAt L thin th) {V : Cov} :
    V ∈ L.thinTopes thin ↔ ∃ u, ∃ (hu : u < L.topes.length), V = L.topes[u] ∧ th u = true := by
  unfold Ledger.thinTopes
  rw [List.mem_filter]
  constructor
  · rintro ⟨hV, hc⟩
    obtain ⟨u, hu, rfl⟩ := List.getElem_of_mem hV
    exact ⟨u, hu, rfl, by rw [hth u hu]; exact hc⟩
  · rintro ⟨u, hu, rfl, h⟩
    exact ⟨List.getElem_mem _, by rw [← hth u hu]; exact h⟩

theorem eqv_conn (hS : Sides L nf pp qq) {thin : List Cov} {th : Nat → Bool} (hth : ThinAt L thin th)
    {y z : Nat} (h : Eqv (kE pp qq th nf) y z) :
    y = z ∨ ∃ (hy : y < L.topes.length) (hz : z < L.topes.length), th y = true ∧ th z = true ∧
      Conn L.tAdj (L.thinTopes thin) L.topes[y] L.topes[z] := by
  induction h with
  | refl => exact Or.inl rfl
  | edge he =>
    rename_i e
    obtain ⟨k, hk, h1, h2, rfl⟩ := (mem_kE pp qq th nf e).mp he
    right
    refine ⟨hS.pp_lt k hk, hS.qq_lt k hk, h1, h2, Conn.step Conn.refl ?_ ?_⟩
    · exact (mem_thinTopes_iff hth).mpr ⟨_, hS.qq_lt k hk, rfl, h2⟩
    · exact (tAdj_iff hS (hS.pp_lt k hk) (hS.qq_lt k hk)).mpr ⟨k, hk, Or.inl ⟨rfl, rfl⟩⟩
  | symm _ ih =>
    rcases ih with h | ⟨hy, hz, h1, h2, hc⟩
    · exact Or.inl h.symm
    · right
      exact ⟨hz, hy, h2, h1, Conn.symm (Ledger.tAdj_symm L) ((mem_thinTopes_iff hth).mpr ⟨_, hy, rfl, h1⟩) hc⟩
  | trans _ _ ih1 ih2 =>
    rcases ih1 with h1 | ⟨ha, hb, ta, tb, c1⟩
    · subst h1; exact ih2
    · rcases ih2 with h2 | ⟨hb', hc, tb', tc, c2⟩
      · subst h2; exact Or.inr ⟨ha, hb, ta, tb, c1⟩
      · exact Or.inr ⟨ha, hc, ta, tc, c1.trans c2⟩

/-- **The component, by positions.** -/
theorem mem_comp_iff (hS : Sides L nf pp qq) {thin : List Cov} {th : Nat → Bool} (hth : ThinAt L thin th)
    {t : Nat} (ht : t < L.topes.length) (htt : th t = true) (V : Cov) :
    V ∈ L.comp thin L.topes[t] ↔
      ∃ u, ∃ (hu : u < L.topes.length), V = L.topes[u] ∧ th u = true ∧ Eqv (kE pp qq th nf) t u := by
  have hT : L.topes[t] ∈ L.thinTopes thin := (mem_thinTopes_iff hth).mpr ⟨t, ht, rfl, htt⟩
  unfold Ledger.comp
  rw [cls_spec _ (Ledger.thinTopes_nodup L thin) hT]
  constructor
  · intro hc
    refine Conn.invariant (P := fun V => ∃ u, ∃ (hu : u < L.topes.length), V = L.topes[u] ∧ th u = true ∧
      Eqv (kE pp qq th nf) t u) hc ⟨t, ht, rfl, htt, .refl t⟩ ?_
    rintro x y ⟨u, hu, rfl, hthu, he⟩ hy hxy
    obtain ⟨w, hw, rfl, hthw⟩ := (mem_thinTopes_iff hth).mp hy
    refine ⟨w, hw, rfl, hthw, he.trans ?_⟩
    obtain ⟨k, hk, h⟩ := (tAdj_iff hS hu hw).mp hxy
    rcases h with ⟨e1, e2⟩ | ⟨e1, e2⟩
    · have := Eqv.edge (E := kE pp qq th nf) (e := (pp k, qq k))
        ((mem_kE pp qq th nf _).mpr ⟨k, hk, by rw [e1]; exact hthu, by rw [e2]; exact hthw, rfl⟩)
      rw [e1, e2] at this; exact this
    · have := Eqv.edge (E := kE pp qq th nf) (e := (pp k, qq k))
        ((mem_kE pp qq th nf _).mpr ⟨k, hk, by rw [e1]; exact hthw, by rw [e2]; exact hthu, rfl⟩)
      rw [e1, e2] at this; exact this.symm
  · rintro ⟨u, hu, rfl, _, he⟩
    rcases eqv_conn hS hth he with h | ⟨_, _, _, _, hc⟩
    · subst h; exact Conn.refl
    · exact hc

/-! ### Lists by position -/

def at_ (xs : List Cov) (u : Nat) : Cov := xs.getD u []

theorem at_eq {xs : List Cov} {u : Nat} (h : u < xs.length) : at_ xs u = xs[u] := by
  simp [at_, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

theorem map_range (xs : List Cov) : (List.range xs.length).map (at_ xs) = xs := by
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_range]
  exact at_eq h2

theorem map_eq_range {β : Type} (xs : List Cov) (F : Cov → β) :
    xs.map F = (List.range xs.length).map (fun u => F (at_ xs u)) := by
  conv => lhs; rw [← map_range xs]
  rw [List.map_map]; rfl

theorem filter_range (xs : List Cov) (f : Cov → Bool) :
    xs.filter f = ((List.range xs.length).filter (fun u => f (at_ xs u))).map (at_ xs) := by
  conv => lhs; rw [← map_range xs]
  rw [List.filter_map]; rfl

theorem pairwise_lt_eq : ∀ {l1 l2 : List Nat}, l1.Pairwise (· < ·) → l2.Pairwise (· < ·) →
    (∀ x, x ∈ l1 ↔ x ∈ l2) → l1 = l2
  | [], [], _, _, _ => rfl
  | [], b :: _, _, _, h => by have := (h b).mpr (List.mem_cons_self ..); simp at this
  | a :: _, [], _, _, h => by have := (h a).mp (List.mem_cons_self ..); simp at this
  | a :: l1, b :: l2, h1, h2, h => by
    rw [List.pairwise_cons] at h1 h2
    have hab : a = b := by
      have ha := (h a).mp (List.mem_cons_self ..)
      have hb := (h b).mpr (List.mem_cons_self ..)
      rcases List.mem_cons.mp ha with e | ha
      · exact e
      rcases List.mem_cons.mp hb with e | hb
      · exact e.symm
      have := h2.1 a ha; have := h1.1 b hb; omega
    subst hab
    congr 1
    apply pairwise_lt_eq h1.2 h2.2
    intro x
    constructor
    · intro hx
      have := (h x).mp (List.mem_cons_of_mem _ hx)
      rcases List.mem_cons.mp this with e | e
      · have := h1.1 x hx; omega
      · exact e
    · intro hx
      have := (h x).mpr (List.mem_cons_of_mem _ hx)
      rcases List.mem_cons.mp this with e | e
      · have := h2.1 x hx; omega
      · exact e

theorem filter_range_pairwise (n : Nat) (p : Nat → Bool) : ((List.range n).filter p).Pairwise (· < ·) :=
  (List.pairwise_lt_range).filter _

/-- A strictly ascending list with the members of `(range n).filter p` is it. -/
theorem eq_filter_range {l : List Nat} (hl : l.Pairwise (· < ·)) {n : Nat} {p : Nat → Bool}
    (h : ∀ u, u ∈ l ↔ u < n ∧ p u = true) : l = (List.range n).filter p :=
  pairwise_lt_eq hl (filter_range_pairwise n p) (fun u => by
    rw [h u, List.mem_filter, List.mem_range])

/-! ### Keep tables by position -/

theorem keepIn_zip {xs : List Cov} (hnd : xs.Nodup) {keep : List Bool} (hl : keep.length = xs.length)
    {u : Nat} (hu : u < xs.length) : keepIn (xs.zip keep) xs[u] = keep[u]'(by omega) := by
  have : xs.zip keep = xs.map fun x => (x, keep.getD (xs.idxOf x) false) := by
    apply List.ext_getElem (by simp [hl])
    intro i h1 h2
    simp only [List.getElem_zip, List.getElem_map]
    congr 1
    rw [List.Nodup.idxOf_getElem hnd, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem]; rfl
  rw [this, Ledger.keepIn_map _ (List.getElem_mem _), List.Nodup.idxOf_getElem hnd, List.getD_eq_getElem?_getD,
    List.getElem?_eq_getElem]; rfl

theorem zip_map_range (xs : List Cov) (g : Nat → Bool) :
    xs.zip ((List.range xs.length).map g) = (List.range xs.length).map fun i => (at_ xs i, g i) := by
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp only [List.getElem_zip, List.getElem_map, List.getElem_range]
  rw [at_eq (by simpa using h2)]

/-! ### The collapse, by positions -/

/-- `u` is on the rim of `t`'s component: not thin, across a facet from a thin cell of it. -/
def rimP (pp qq : Nat → Nat) (th : Nat → Bool) (nf t u : Nat) : Prop :=
  th u = false ∧ ∃ k, k < nf ∧
    ((th (pp k) = true ∧ Eqv (kE pp qq th nf) t (pp k) ∧ qq k = u) ∨
     (th (qq k) = true ∧ Eqv (kE pp qq th nf) t (qq k) ∧ pp k = u))

def kAt (keep : List Bool) (u : Nat) : Bool := keep.getD u false

/-- The keep a thin cell would take: its rim's first cell's, or its own. -/
def firstK (keep : List Bool) (rm : List Nat) (t : Nat) : Bool :=
  match rm with
  | [] => kAt keep t
  | u :: _ => kAt keep u

def uniR (keep : List Bool) (rm : List Nat) (t : Nat) : Bool := rm.all fun u => kAt keep u == firstK keep rm t

/-- The Rust's tables, by position, as the Lean reads them. -/
structure Tabs (L : ForgeExact.DecideOnce.Ledger) (nf : Nat) (pp qq : Nat → Nat) (thin enc : List Cov)
    (keep : List Bool) (th en : Nat → Bool) (R : Nat → Nat) (rimL : Nat → List Nat) (aenc : Nat → Bool) : Prop where
  sides : Sides L nf pp qq
  hth : ThinAt L thin th
  hen : ThinAt L enc en
  klen : keep.length = L.topes.length
  least : Least (kE pp qq th nf) L.topes.length R
  rim_sorted : ∀ t, t < L.topes.length → th t = true → (rimL (R t)).Pairwise (· < ·)
  rim_mem : ∀ t, t < L.topes.length → th t = true →
    ∀ u, u ∈ rimL (R t) ↔ u < L.topes.length ∧ rimP pp qq th nf t u
  haenc : ∀ t, t < L.topes.length → th t = true →
    (aenc (R t) = true ↔ ∀ u, u < L.topes.length → th u = true → R u = R t → en u = true)

variable {thin enc : List Cov} {keep : List Bool} {th en : Nat → Bool} {R : Nat → Nat}
  {rimL : Nat → List Nat} {aenc : Nat → Bool}

theorem rim_eq (h : Tabs L nf pp qq thin enc keep th en R rimL aenc) {t : Nat} (ht : t < L.topes.length)
    (htt : th t = true) : L.rim thin L.topes[t] = (rimL (R t)).map (at_ L.topes) := by
  unfold Ledger.rim
  rw [filter_range]
  congr 1
  symm
  apply eq_filter_range (h.rim_sorted t ht htt)
  intro u
  rw [h.rim_mem t ht htt u]
  apply and_congr_right
  intro hu
  rw [at_eq hu, Bool.and_eq_true, Bool.not_eq_true', ← h.hth u hu, List.any_eq_true]
  unfold rimP
  apply and_congr_right; intro _
  constructor
  · rintro ⟨k, hk, ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩⟩
    · refine ⟨L.topes[pp k]'(h.sides.pp_lt k hk), (mem_comp_iff h.sides h.hth ht htt _).mpr
        ⟨pp k, h.sides.pp_lt k hk, rfl, h1, h2⟩, ?_⟩
      exact (tAdj_iff h.sides _ hu).mpr ⟨k, hk, Or.inl ⟨rfl, h3⟩⟩
    · refine ⟨L.topes[qq k]'(h.sides.qq_lt k hk), (mem_comp_iff h.sides h.hth ht htt _).mpr
        ⟨qq k, h.sides.qq_lt k hk, rfl, h1, h2⟩, ?_⟩
      exact (tAdj_iff h.sides _ hu).mpr ⟨k, hk, Or.inr ⟨h3, rfl⟩⟩
  · rintro ⟨V, hV, hadj⟩
    obtain ⟨v, hv, rfl, hthv, he⟩ := (mem_comp_iff h.sides h.hth ht htt _).mp hV
    obtain ⟨k, hk, ⟨e1, e2⟩ | ⟨e1, e2⟩⟩ := (tAdj_iff h.sides hv hu).mp hadj
    · exact ⟨k, hk, Or.inl ⟨by rw [e1]; exact hthv, by rw [e1]; exact he, e2⟩⟩
    · exact ⟨k, hk, Or.inr ⟨by rw [e2]; exact hthv, by rw [e2]; exact he, e1⟩⟩

theorem rim_lt (h : Tabs L nf pp qq thin enc keep th en R rimL aenc) {t : Nat} (ht : t < L.topes.length)
    (htt : th t = true) {u : Nat} (hu : u ∈ rimL (R t)) : u < L.topes.length :=
  ((h.rim_mem t ht htt u).mp hu).1

theorem keepIn_at (h : Tabs L nf pp qq thin enc keep th en R rimL aenc) {u : Nat} (hu : u < L.topes.length) :
    keepIn (L.topes.zip keep) (at_ L.topes u) = kAt keep u := by
  rw [at_eq hu, keepIn_zip (Ledger.topes_nodup L) h.klen hu, kAt, List.getD_eq_getElem?_getD,
    List.getElem?_eq_getElem]; rfl

theorem fill_eq (h : Tabs L nf pp qq thin enc keep th en R rimL aenc) {t : Nat} (ht : t < L.topes.length)
    (htt : th t = true) : L.fill (L.topes.zip keep) thin L.topes[t] = firstK keep (rimL (R t)) t := by
  unfold Ledger.fill
  rw [rim_eq h ht htt]
  have hlt := fun u => rim_lt h ht htt (u := u)
  cases hr : rimL (R t) with
  | nil =>
    simp only [List.map_nil, firstK]
    rw [← at_eq ht, keepIn_at h ht]
  | cons u rest =>
    simp only [List.map_cons, firstK]
    exact keepIn_at h (hlt u (by rw [hr]; exact List.mem_cons_self ..))

theorem coll_eq (h : Tabs L nf pp qq thin enc keep th en R rimL aenc) {t : Nat} (ht : t < L.topes.length)
    (htt : th t = true) :
    L.collapsible (L.topes.zip keep) thin enc L.topes[t] = (uniR keep (rimL (R t)) t && aenc (R t)) := by
  unfold Ledger.collapsible
  congr 1
  · unfold Ledger.uniform uniR
    rw [rim_eq h ht htt, List.all_map]
    have hlt := fun u => rim_lt h ht htt (u := u)
    have hhd : keepIn (L.topes.zip keep) (((rimL (R t)).map (at_ L.topes)).headD L.topes[t]) =
        firstK keep (rimL (R t)) t := by
      cases hr : rimL (R t) with
      | nil => simp only [List.map_nil, List.headD_nil, firstK]; rw [← at_eq ht, keepIn_at h ht]
      | cons u rest =>
        simp only [List.map_cons, List.headD_cons, firstK]
        exact keepIn_at h (hlt u (by rw [hr]; exact List.mem_cons_self ..))
    rw [hhd]
    apply Bool.eq_iff_iff.mpr
    simp only [List.all_eq_true, Function.comp]
    constructor
    · intro ha u hu; rw [← keepIn_at h (hlt u hu)]; exact ha u hu
    · intro ha u hu; rw [keepIn_at h (hlt u hu)]; exact ha u hu
  · apply Bool.eq_iff_iff.mpr
    rw [List.all_eq_true, h.haenc t ht htt]
    constructor
    · intro ha u hu hthu hR
      have := ha _ ((mem_comp_iff h.sides h.hth ht htt _).mpr
        ⟨u, hu, rfl, hthu, (least_iff h.least ht hu).mpr hR.symm⟩)
      rw [h.hen u hu]; exact this
    · intro ha V hV
      obtain ⟨u, hu, rfl, hthu, he⟩ := (mem_comp_iff h.sides h.hth ht htt _).mp hV
      rw [← h.hen u hu]
      exact ha u hu hthu ((least_iff h.least ht hu).mp he).symm

/-- **The Lean collapse, by positions**: the first thin position whose component is not
collapsible is refused; otherwise each thin position takes `firstK`. -/
theorem collapse_eq (h : Tabs L nf pp qq thin enc keep th en R rimL aenc) :
    L.collapse (L.topes.zip keep) thin enc =
      match (List.range L.topes.length).find? (fun t => th t && !(uniR keep (rimL (R t)) t && aenc (R t))) with
      | some t => .refuse (at_ L.topes t)
      | none => .ok (L.topes.zip ((List.range L.topes.length).map fun t =>
          if th t then firstK keep (rimL (R t)) t else kAt keep t)) := by
  have hfind : (L.thinTopes thin).find? (fun T => !L.collapsible (L.topes.zip keep) thin enc T) =
      ((List.range L.topes.length).find?
        (fun t => th t && !(uniR keep (rimL (R t)) t && aenc (R t)))).map (at_ L.topes) := by
    unfold Ledger.thinTopes
    rw [filter_range, List.find?_map, List.find?_filter]
    congr 1
    apply find?_congr
    intro u hu
    have hu' := List.mem_range.mp hu
    simp only [Function.comp, at_eq hu', ← h.hth u hu']
    cases hthu : th u
    · simp
    · simp [coll_eq h hu' hthu]
  unfold Ledger.collapse
  rw [hfind]
  cases hf : (List.range L.topes.length).find? (fun t => th t && !(uniR keep (rimL (R t)) t && aenc (R t))) with
  | some t => rfl
  | none =>
    simp only [Option.map_none]
    congr 1
    rw [zip_map_range, map_eq_range]
    apply List.map_congr_left
    intro u hu
    have hu' := List.mem_range.mp hu
    simp only [at_eq hu', ← h.hth u hu']
    cases hthu : th u
    · simp only [Bool.false_eq_true, ite_false]
      rw [← at_eq hu', keepIn_at h hu']
    · simp only [ite_true]
      rw [fill_eq h hu' hthu]

end K


/-! ## Part 2: the Rust loops -/

namespace K

/-- A position is marked by a list of cell ids: its tope is listed. -/
def mk (s : Split) (ids : List Nat) (t : Nat) : Bool := ids.contains (at32 s.topes t)

/-- The tope-position table, as the loops read it. -/
structure TP (s : Split) (N : Nat) : Prop where
  len : s.tope_pos.val.length = N
  small : s.topes.val.length < U32.max
  pos : ∀ t, t < s.topes.val.length → at32 s.topes t < N ∧ at32 s.tope_pos (at32 s.topes t) = t
  some_ : ∀ c, c < N → at32 s.tope_pos c ≠ U32.max →
    at32 s.tope_pos c < s.topes.val.length ∧ at32 s.topes (at32 s.tope_pos c) = c

theorem mk_snoc (s : Split) (ids : List Nat) (c t : Nat) :
    mk s (ids ++ [c]) t = (mk s ids t || decide (c = at32 s.topes t)) := by
  unfold mk; rw [List.contains_append]; simp [eq_comm]

theorem map_range_congr {n : Nat} {f g : Nat → Bool} (h : (List.range n).map f = (List.range n).map g)
    {t : Nat} (ht : t < n) : f t = g t := by
  have := congrArg (·[t]?) h; simpa [ht] using this

theorem take_succ_ids (v : alloc.vec.Vec CellId) (k : Nat) (hk : k < v.val.length) :
    (idsOf v).take (k + 1) = (idsOf v).take k ++ [(v.val[k] : U32).val] := by
  rw [List.take_add_one, List.getElem?_eq_getElem (by simp [idsOf]; exact hk)]
  simp [idsOf]

/-- Marking one cell: the vector of marks, past one more id. -/
theorem mark_step (s : Split) {N : Nat} (hTP : TP s N) (ids : List Nat) (c : Nat) (hc : c < N) :
    (at32 s.tope_pos c ≠ U32.max →
      ((List.range s.topes.val.length).map (mk s ids)).set (at32 s.tope_pos c) true =
        (List.range s.topes.val.length).map (mk s (ids ++ [c]))) ∧
    (at32 s.tope_pos c = U32.max →
      (List.range s.topes.val.length).map (mk s ids) = (List.range s.topes.val.length).map (mk s (ids ++ [c]))) := by
  constructor
  · intro hne
    obtain ⟨hlt, heq⟩ := hTP.some_ c hc hne
    apply List.ext_getElem (by simp)
    intro i h1 h2
    simp only [List.length_set, List.length_map, List.length_range] at h1
    rw [List.getElem_set]
    simp only [List.getElem_map, List.getElem_range, mk_snoc]
    split
    · rename_i e; subst e; simp [heq]
    · rename_i e
      have : decide (c = at32 s.topes i) = false := by
        simp only [decide_eq_false_iff_not]
        intro e2
        have := (hTP.pos i h1).2
        rw [← e2] at this
        exact e this
      rw [this, Bool.or_false]
  · intro he
    apply List.map_congr_left
    intro i hi
    have hi' := List.mem_range.mp hi
    rw [mk_snoc]
    have : decide (c = at32 s.topes i) = false := by
      simp only [decide_eq_false_iff_not]
      intro e2
      have := (hTP.pos i hi').2
      rw [← e2, he] at this
      have := hTP.small; omega
    rw [this, Bool.or_false]

theorem loop0_spec (s : Split) (mt : MetricLedger) (n : Usize) (hTP : TP s n.val)
    (hthin : ∀ c ∈ mt.thin.val, (c : U32).val < n.val)
    (is_thin : alloc.vec.Vec Bool) (any : Bool) (k : Usize) (hk : k.val ≤ mt.thin.val.length)
    (hit : is_thin.val = (List.range s.topes.val.length).map (mk s ((idsOf mt.thin).take k.val)))
    (hany : any = true ↔ ∃ t, t < s.topes.val.length ∧ mk s ((idsOf mt.thin).take k.val) t = true) :
    collapse_loop0 s.tope_pos mt n is_thin any k ⦃ r => r.1 = mt.enclosed ∧
      r.2.1.val = (List.range s.topes.val.length).map (mk s (idsOf mt.thin)) ∧
      (r.2.2 = true ↔ ∃ t, t < s.topes.val.length ∧ mk s (idsOf mt.thin) t = true) ⦄ := by
  unfold collapse_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Bool × Usize) => mt.thin.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ mt.thin.val.length ∧
      q.1.val = (List.range s.topes.val.length).map (mk s ((idsOf mt.thin).take q.2.2.val)) ∧
      (q.2.1 = true ↔ ∃ t, t < s.topes.val.length ∧ mk s ((idsOf mt.thin).take q.2.2.val) t = true))
  · rintro ⟨w, a, j⟩ ⟨hj, hw, ha⟩
    unfold collapse_loop0.body
    simp only at hj hw ha ⊢
    split
    · rename_i hlt
      have hjl : j.val < mt.thin.val.length := by scalar_tac
      step as ⟨ci, hci⟩
      step as ⟨c, hc⟩
      have hcv : c.val = (mt.thin.val[j.val] : U32).val := by rw [hc, hci]; scalar_tac
      have hcn : c.val < n.val := by rw [hcv]; exact hthin _ (List.getElem_mem _)
      have htk := take_succ_ids mt.thin j.val hjl
      rw [← hcv] at htk
      have hms := mark_step s hTP ((idsOf mt.thin).take j.val) c.val hcn
      split
      · have hTl := hTP.len
        step as ⟨i1, hi1⟩
        have hi1v : i1.val = at32 s.tope_pos c.val := by rw [hi1, at32_getD (by omega)]
        split
        · rename_i hne
          have hne' : at32 s.tope_pos c.val ≠ U32.max := by
            rw [← hi1v]; intro e; exact (by simpa using hne : i1 ≠ NONE) (UScalar.eq_of_val_eq (by rw [e, none_val]))
          have hpl := (hTP.some_ c.val hcn hne').1
          step as ⟨i2, hi2⟩
          have hi2v : i2.val = at32 s.tope_pos c.val := by rw [hi2, ← hi1v]; scalar_tac
          have hi2l : i2.val < w.val.length := by rw [hw, hi2v]; simpa using hpl
          step as ⟨w0, hw0a, hback⟩
          obtain ⟨x0, back⟩ := w0
          simp only at hback ⊢
          subst hback
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · rw [alloc.vec.Vec.set_val_eq, hw, hj1, hi2v, htk]
            exact hms.1 hne'
          · simp only [true_iff]
            refine ⟨at32 s.tope_pos c.val, hpl, ?_⟩
            rw [hj1, htk, mk_snoc, (hTP.some_ c.val hcn hne').2]; simp
        · rename_i he
          have he' : at32 s.tope_pos c.val = U32.max := by
            rw [← hi1v]; have : i1 = NONE := UScalar.eq_of_val_eq (by simpa using he); rw [this, none_val]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · rw [hw, hj1, htk]; exact hms.2 he'
          · rw [ha, hj1, htk]
            have hf := fun t (ht : t < s.topes.val.length) => map_range_congr (hms.2 he') ht
            constructor
            · rintro ⟨t, ht, h'⟩; exact ⟨t, ht, by rw [← hf t ht]; exact h'⟩
            · rintro ⟨t, ht, h'⟩; exact ⟨t, ht, by rw [hf t ht]; exact h'⟩
      · rename_i hge; exfalso; scalar_tac
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = mt.thin.val.length := by scalar_tac
      have htk : (idsOf mt.thin).take j.val = idsOf mt.thin := by
        rw [this]; exact List.take_of_length_le (by simp [idsOf])
      rw [htk] at hw ha
      exact ⟨trivial, hw, ha⟩
  · exact ⟨hk, hit, hany⟩

theorem loop1_spec (s : Split) (enc : alloc.vec.Vec CellId) (n : Usize) (hTP : TP s n.val)
    (henc : ∀ c ∈ enc.val, (c : U32).val < n.val)
    (is_enc : alloc.vec.Vec Bool) (k : Usize) (hk : k.val ≤ enc.val.length)
    (hit : is_enc.val = (List.range s.topes.val.length).map (mk s ((idsOf enc).take k.val))) :
    collapse_loop1 s.tope_pos enc n is_enc k ⦃ r =>
      r.val = (List.range s.topes.val.length).map (mk s (idsOf enc)) ⦄ := by
  unfold collapse_loop1
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => enc.val.length - q.2.val)
    (fun q => q.2.val ≤ enc.val.length ∧
      q.1.val = (List.range s.topes.val.length).map (mk s ((idsOf enc).take q.2.val)))
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold collapse_loop1.body
    simp only at hj hw ⊢
    split
    · rename_i hlt
      have hjl : j.val < enc.val.length := by scalar_tac
      step as ⟨ci, hci⟩
      step as ⟨c, hc⟩
      have hcv : c.val = (enc.val[j.val] : U32).val := by rw [hc, hci]; scalar_tac
      have hcn : c.val < n.val := by rw [hcv]; exact henc _ (List.getElem_mem _)
      have htk := take_succ_ids enc j.val hjl
      rw [← hcv] at htk
      have hms := mark_step s hTP ((idsOf enc).take j.val) c.val hcn
      split
      · have hTl := hTP.len
        step as ⟨i1, hi1⟩
        have hi1v : i1.val = at32 s.tope_pos c.val := by rw [hi1, at32_getD (by omega)]
        split
        · rename_i hne
          have hne' : at32 s.tope_pos c.val ≠ U32.max := by
            rw [← hi1v]; intro e; exact (by simpa using hne : i1 ≠ NONE) (UScalar.eq_of_val_eq (by rw [e, none_val]))
          have hpl := (hTP.some_ c.val hcn hne').1
          step as ⟨i2, hi2⟩
          have hi2v : i2.val = at32 s.tope_pos c.val := by rw [hi2, ← hi1v]; scalar_tac
          have hi2l : i2.val < w.val.length := by rw [hw, hi2v]; simpa using hpl
          step as ⟨w0, hw0a, hback⟩
          obtain ⟨x0, back⟩ := w0
          simp only at hback ⊢
          subst hback
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [alloc.vec.Vec.set_val_eq, hw, hj1, hi2v, htk]
          exact hms.1 hne'
        · rename_i he
          have he' : at32 s.tope_pos c.val = U32.max := by
            rw [← hi1v]; have : i1 = NONE := UScalar.eq_of_val_eq (by simpa using he); rw [this, none_val]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hw, hj1, htk]; exact hms.2 he'
      · rename_i hge; exfalso; scalar_tac
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = enc.val.length := by scalar_tac
      have htk : (idsOf enc).take j.val = idsOf enc := by
        rw [this]; exact List.take_of_length_le (by simp [idsOf])
      rw [htk] at hw
      exact hw
  · exact ⟨hk, hit⟩

/-! ### The unions -/

/-- The split's sides by position, as the loops read them. -/
structure SW (s : Split) (N : Nat) : Prop where
  plus_len : s.plus.val.length = s.facets.val.length
  minus_len : s.minus.val.length = s.facets.val.length
  tp_len : s.tope_pos.val.length = N
  plus_in : ∀ k, k < s.facets.val.length → at32 s.plus k < N
  minus_in : ∀ k, k < s.facets.val.length → at32 s.minus k < N
  pp_lt : ∀ k, k < s.facets.val.length → tpP s k < s.topes.val.length
  qq_lt : ∀ k, k < s.facets.val.length → tpM s k < s.topes.val.length
  nt_small : s.topes.val.length ≤ U32.max
  nf_small : 3 * s.facets.val.length ≤ Usize.max

theorem kE_in (s : Split) {N : Nat} (hw : SW s N) (th : Nat → Bool) (K : Nat) (hK : K ≤ s.facets.val.length) :
    ∀ e ∈ kE (tpP s) (tpM s) th K, e.1 < s.topes.val.length ∧ e.2 < s.topes.val.length := by
  intro e he
  obtain ⟨k, hk, _, _, rfl⟩ := (mem_kE _ _ th K e).mp he
  exact ⟨hw.pp_lt k (by omega), hw.qq_lt k (by omega)⟩

/-- Reading facet `k`'s sides' positions. -/
theorem side_reads (s : Split) {N : Nat} (hw : SW s N) {k : Nat} (hk : k < s.facets.val.length) :
    at32 s.tope_pos (s.plus.val[k]'(by rw [hw.plus_len]; exact hk)).val = tpP s k ∧
    at32 s.tope_pos (s.minus.val[k]'(by rw [hw.minus_len]; exact hk)).val = tpM s k := by
  have h1 : k < s.plus.val.length := by rw [hw.plus_len]; exact hk
  have h2 : k < s.minus.val.length := by rw [hw.minus_len]; exact hk
  constructor
  · unfold tpP; rw [at32_getD (v := s.plus) (k := k) h1]
  · unfold tpM; rw [at32_getD (v := s.minus) (k := k) h2]

theorem loop2_spec (s : Split) {N : Nat} (hw : SW s N) (th : Nat → Bool)
    (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range s.topes.val.length).map th)
    (uf : union_find.Uf) (k : Usize) (hk : k.val ≤ s.facets.val.length)
    (hu : UInv (kE (tpP s) (tpM s) th k.val) s.topes.val.length uf) :
    collapse_loop2 s.facets s.plus s.minus s.tope_pos is_thin uf k ⦃ r =>
      UInv (kE (tpP s) (tpM s) th s.facets.val.length) s.topes.val.length r ⦄ := by
  unfold collapse_loop2
  apply loop.spec_decr_nat (fun (q : union_find.Uf × Usize) => s.facets.val.length - q.2.val)
    (fun q => q.2.val ≤ s.facets.val.length ∧ UInv (kE (tpP s) (tpM s) th q.2.val) s.topes.val.length q.1)
  · rintro ⟨u, j⟩ ⟨hj, hu⟩
    unfold collapse_loop2.body
    simp only at hj hu ⊢
    split
    · rename_i hlt
      have hjl : j.val < s.facets.val.length := by scalar_tac
      have hPl := hw.plus_len; have hMl := hw.minus_len; have hTl := hw.tp_len
      have hp1 := hw.plus_in j.val hjl; have hm1 := hw.minus_in j.val hjl
      have hp2 := hw.pp_lt j.val hjl; have hm2 := hw.qq_lt j.val hjl
      have hsr := side_reads s hw hjl
      rw [at32_getD (by omega)] at hp1 hm1
      step as ⟨i1, hi1⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = (s.plus.val[j.val]).val := by rw [hi2, hi1]; scalar_tac
      step as ⟨i3, hi3⟩
      step as ⟨p, hp⟩
      have hpv : p.val = tpP s j.val := by
        rw [← hsr.1, ← hi2v, at32_getD (by omega), ← hi3, hp]; scalar_tac
      step as ⟨i4, hi4⟩
      step as ⟨i5, hi5⟩
      have hi5v : i5.val = (s.minus.val[j.val]).val := by rw [hi5, hi4]; scalar_tac
      step as ⟨i6, hi6⟩
      step as ⟨q, hq⟩
      have hqv : q.val = tpM s j.val := by
        rw [← hsr.2, ← hi5v, at32_getD (by omega), ← hi6, hq]; scalar_tac
      have hitl : is_thin.val.length = s.topes.val.length := by rw [hit]; simp
      have hthr : ∀ t, t < s.topes.val.length → is_thin.val[t]! = th t := by
        intro t ht; rw [hit]; simp [ht]
      have hkE : kE (tpP s) (tpM s) th (j.val + 1) = kE (tpP s) (tpM s) th j.val ++
          (if th (tpP s j.val) && th (tpM s j.val) then [(tpP s j.val, tpM s j.val)] else []) := rfl
      step as ⟨b, hb⟩
      have hbv : b = th (tpP s j.val) := by
        rw [hb, ← hthr _ hp2, ← hpv, getElem!_pos is_thin.val p.val (by omega)]
      split
      · rename_i hbt
        step as ⟨b1, hb1⟩
        have hb1v : b1 = th (tpM s j.val) := by
          rw [hb1, ← hthr _ hm2, ← hqv, getElem!_pos is_thin.val q.val (by omega)]
        split
        · rename_i hb1t
          step with union_spec u hu (kE_in s hw th j.val (by omega)) hw.nt_small p q (by omega) (by omega)
            as ⟨u1, hu1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hkE, ← hbv, ← hb1v, hbt, hb1t, ← hpv, ← hqv]; exact hu1
        · rename_i hb1f
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hkE, ← hb1v]; simp [hb1f]; exact hu
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hj1, hkE, ← hbv]; simp [hbf]; exact hu
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = s.facets.val.length := by scalar_tac
      rw [← this]; exact hu
  · exact ⟨hk, hu⟩

/-! ### Which classes are enclosed -/

/-- Class `r` is enclosed so far: every thin position before `t` in it is enclosed. -/
def aeOf (th en : Nat → Bool) (R : Nat → Nat) (t r : Nat) : Bool :=
  (List.range t).all fun t' => !(th t' && R t' == r) || en t'

theorem aeOf_succ (th en : Nat → Bool) (R : Nat → Nat) (t r : Nat) :
    aeOf th en R (t + 1) r = (aeOf th en R t r && (!(th t && R t == r) || en t)) := by
  unfold aeOf; rw [List.range_succ, List.all_append]; simp

theorem aeOf_iff (th en : Nat → Bool) (R : Nat → Nat) (t r : Nat) :
    aeOf th en R t r = true ↔ ∀ u, u < t → th u = true → R u = r → en u = true := by
  unfold aeOf
  simp only [List.all_eq_true, List.mem_range, Bool.or_eq_true, Bool.not_eq_true', Bool.and_eq_false_iff,
    beq_eq_false_iff_ne]
  constructor
  · intro h u hu h1 h2
    rcases h u hu with (h' | h') | h'
    · rw [h1] at h'; cases h'
    · exact absurd h2 h'
    · exact h'
  · intro h u hu
    cases h1 : th u
    · exact Or.inl (Or.inl rfl)
    · by_cases h2 : R u = r
      · exact Or.inr (h u hu h1 h2)
      · exact Or.inl (Or.inr h2)

/-- The roots of a union–find, kept. -/
def Roots (n : Nat) (R : Nat → Nat) (u : union_find.Uf) : Prop :=
  Good (vals u.parent) ∧ u.parent.val.length = n ∧ ∀ y, ufRoot (vals u.parent) y = R y

theorem roots_find {n : Nat} {R : Nat → Nat} {u : union_find.Uf} (h : Roots n R u) (x : Usize) (hx : x.val < n) :
    union_find.Uf.find u x ⦃ r => r.1.val = R x.val ∧ Roots n R r.2 ⦄ := by
  obtain ⟨hg, hl, hr⟩ := h
  step with find_spec u x (by omega) hg as ⟨r, h1, h2, h3, h4⟩
  exact ⟨by rw [h1, hr], h3, by omega, fun y => by rw [h4, hr]⟩

theorem read_map {α : Type} {v : List α} {n : Nat} {f : Nat → α} (h : v = (List.range n).map f) {j : Nat}
    (hj : j < v.length) : v[j] = f j := by
  subst h; simp

theorem loop3_spec (nt : Usize) (th en : Nat → Bool) (is_thin is_enc : alloc.vec.Vec Bool)
    (hit : is_thin.val = (List.range nt.val).map th) (hie : is_enc.val = (List.range nt.val).map en)
    (R : Nat → Nat) (hR : ∀ y, y < nt.val → R y < nt.val)
    (uf : union_find.Uf) (hu : Roots nt.val R uf) (all_enc : alloc.vec.Vec Bool) (t : Usize) (ht : t.val ≤ nt.val)
    (hae : all_enc.val = (List.range nt.val).map (aeOf th en R t.val)) :
    collapse_loop3 nt is_thin is_enc uf all_enc t ⦃ r =>
      Roots nt.val R r.1 ∧ r.2.val = (List.range nt.val).map (aeOf th en R nt.val) ⦄ := by
  unfold collapse_loop3
  apply loop.spec_decr_nat (fun (q : union_find.Uf × alloc.vec.Vec Bool × Usize) => nt.val - q.2.2.val)
    (fun q => q.2.2.val ≤ nt.val ∧ Roots nt.val R q.1 ∧
      q.2.1.val = (List.range nt.val).map (aeOf th en R q.2.2.val))
  · rintro ⟨u, a, j⟩ ⟨hj, hu, ha⟩
    unfold collapse_loop3.body
    simp only at hj hu ha ⊢
    split
    · rename_i hlt
      have hjl : j.val < nt.val := by scalar_tac
      have hitl : is_thin.val.length = nt.val := by rw [hit]; simp
      have hiel : is_enc.val.length = nt.val := by rw [hie]; simp
      have hal : a.val.length = nt.val := by rw [ha]; simp
      step as ⟨b, hb⟩
      have hbv : b = th j.val := by rw [hb, read_map hit]
      have hsame : (List.range nt.val).map (aeOf th en R (j.val + 1)) = (List.range nt.val).map (aeOf th en R j.val)
          ∨ (th j.val = true ∧ en j.val = false) := by
        by_cases hc : th j.val = true ∧ en j.val = false
        · exact Or.inr hc
        · left; apply List.map_congr_left; intro r _
          rw [aeOf_succ]
          cases h1 : th j.val <;> cases h2 : en j.val <;> simp_all
      split
      · rename_i hbt
        step as ⟨b1, hb1⟩
        have hb1v : b1 = en j.val := by rw [hb1, read_map hie]
        split
        · rename_i hb1t
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
          rw [hj1, ha]
          rcases hsame with h | h
          · exact h.symm
          · rw [← hb1v, hb1t] at h; cases h.2
        · rename_i hb1f
          step with roots_find hu j hjl as ⟨ru, hr, hu2⟩
          obtain ⟨r, u2⟩ := ru
          simp only at hr hu2 ⊢
          have hrl : r.val < a.val.length := by rw [hal, hr]; exact hR _ hjl
          step as ⟨w0, hw0a, hback⟩
          obtain ⟨x0, back⟩ := w0
          simp only at hback ⊢
          subst hback
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [alloc.vec.Vec.set_val_eq, ha, hj1]
          apply List.ext_getElem (by simp)
          intro i h1 h2
          simp only [List.length_set, List.length_map, List.length_range] at h1
          rw [List.getElem_set]
          simp only [List.getElem_map, List.getElem_range, aeOf_succ]
          have hb1f' : en j.val = false := by rw [← hb1v]; simpa using hb1f
          rw [← hbv, hbt, hb1f']
          split
          · rename_i e; subst e; simp [hr]
          · rename_i e
            have : (R j.val == i) = false := by simp only [beq_eq_false_iff_ne]; rw [← hr]; exact fun h' => e h'
            simp [this]
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
        rw [hj1, ha]
        rcases hsame with h | h
        · exact h.symm
        · rw [hbv, h.1] at hbf; exact absurd rfl hbf
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nt.val := by scalar_tac
      rw [this] at ha
      exact ⟨hu, ha⟩
  · exact ⟨ht, hu, hae⟩

/-! ### The rims -/

/-- Facet `k` puts `u` on class `r`'s rim: one side thin in class `r`, the other `u`, not thin. -/
def rimHit (s : Split) (th : Nat → Bool) (R : Nat → Nat) (k r u : Nat) : Prop :=
  (th (tpP s k) = true ∧ th (tpM s k) = false ∧ R (tpP s k) = r ∧ tpM s k = u) ∨
  (th (tpM s k) = true ∧ th (tpP s k) = false ∧ R (tpM s k) = r ∧ tpP s k = u)

def rimAt (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (r : Nat) : List Nat :=
  (rim.val.getD r default).val.map (·.val)

/-- The rims after facets `0 .. k-1`. -/
def RimInv (s : Split) (th : Nat → Bool) (R : Nat → Nat) (nt k : Nat) (rim : alloc.vec.Vec (alloc.vec.Vec U32)) : Prop :=
  rim.val.length = nt ∧ ∀ r, r < nt → (rim.val.getD r default).val.length ≤ k ∧
    ∀ u, u ∈ rimAt rim r ↔ ∃ k', k' < k ∧ rimHit s th R k' r u

theorem rimAt_push (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (r : Nat) (hr : r < rim.val.length)
    (v : alloc.vec.Vec U32) (hv : v = rim.val[r]) (x : alloc.vec.Vec U32) (hx : x.val = v.val ++ [y]) (r' : Nat) :
    (((rim.val.set r x).getD r' default).val.map (·.val)) =
      if r' = r then rimAt rim r ++ [y.val] else rimAt rim r' := by
  unfold rimAt
  rw [List.getD_eq_getElem?_getD, List.getElem?_set]
  split
  · rename_i e; subst e
    simp [hr, hx, hv, List.getD_eq_getElem?_getD]
  · rename_i e
    rw [ite_eq_right_iff.mpr (fun h => absurd h.symm e)]; simp [List.getD_eq_getElem?_getD]

/-- One push to class `r0`'s rim keeps the invariant at `k + 1`, when facet `k` hits exactly `(r0, y)`. -/
theorem rimInv_push {s : Split} {th : Nat → Bool} {R : Nat → Nat} {nt k : Nat}
    {rim : alloc.vec.Vec (alloc.vec.Vec U32)} (h : RimInv s th R nt k rim) {r0 : Nat} (hr0 : r0 < nt)
    {y : U32} (hhit : ∀ r u, rimHit s th R k r u ↔ (r = r0 ∧ u = y.val))
    (v : alloc.vec.Vec U32) (hv : v = rim.val[r0]'(by rw [h.1]; exact hr0)) (x : alloc.vec.Vec U32)
    (hx : x.val = v.val ++ [y]) (rim' : alloc.vec.Vec (alloc.vec.Vec U32)) (hrim' : rim'.val = rim.val.set r0 x) :
    RimInv s th R nt (k + 1) rim' := by
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
  · have e := rimAt_push rim r0 (by omega) v hv x hx r
    unfold rimAt; rw [hrim', e]
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

theorem rimInv_none {s : Split} {th : Nat → Bool} {R : Nat → Nat} {nt k : Nat}
    {rim : alloc.vec.Vec (alloc.vec.Vec U32)} (h : RimInv s th R nt k rim)
    (hhit : ∀ r u, ¬ rimHit s th R k r u) : RimInv s th R nt (k + 1) rim := by
  obtain ⟨hl, hrr⟩ := h
  refine ⟨hl, fun r hr => ⟨by have := (hrr r hr).1; omega, fun u => ?_⟩⟩
  rw [(hrr r hr).2 u]
  constructor
  · rintro ⟨k', hk', hh⟩; exact ⟨k', by omega, hh⟩
  · rintro ⟨k', hk', hh⟩
    by_cases ek : k' = k
    · subst ek; exact absurd hh (hhit r u)
    · exact ⟨k', by omega, hh⟩

theorem loop4_spec (s : Split) {N : Nat} (hw : SW s N) (th : Nat → Bool)
    (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range s.topes.val.length).map th)
    (R : Nat → Nat) (hR : ∀ y, y < s.topes.val.length → R y < s.topes.val.length)
    (uf : union_find.Uf) (hu : Roots s.topes.val.length R uf)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (k : Usize) (hk : k.val ≤ s.facets.val.length)
    (hrim : RimInv s th R s.topes.val.length k.val rim) :
    collapse_loop4 s.facets s.plus s.minus s.tope_pos is_thin uf rim k ⦃ r =>
      Roots s.topes.val.length R r.1 ∧ RimInv s th R s.topes.val.length s.facets.val.length r.2 ⦄ := by
  unfold collapse_loop4
  apply loop.spec_decr_nat
    (fun (q : union_find.Uf × alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => s.facets.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ s.facets.val.length ∧ Roots s.topes.val.length R q.1 ∧
      RimInv s th R s.topes.val.length q.2.2.val q.2.1)
  · rintro ⟨u, rm, j⟩ ⟨hj, hu, hrm⟩
    unfold collapse_loop4.body
    simp only at hj hu hrm ⊢
    split
    · rename_i hlt
      have hjl : j.val < s.facets.val.length := by scalar_tac
      have hPl := hw.plus_len; have hMl := hw.minus_len; have hTl := hw.tp_len
      have hp1 := hw.plus_in j.val hjl; have hm1 := hw.minus_in j.val hjl
      have hp2 := hw.pp_lt j.val hjl; have hm2 := hw.qq_lt j.val hjl
      have hsr := side_reads s hw hjl
      have hsmall := hw.nt_small
      have hnf := hw.nf_small
      rw [at32_getD (by omega)] at hp1 hm1
      step as ⟨i1, hi1⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = (s.plus.val[j.val]).val := by rw [hi2, hi1]; scalar_tac
      step as ⟨i3, hi3⟩
      step as ⟨p, hp⟩
      have hpv : p.val = tpP s j.val := by
        rw [← hsr.1, ← hi2v, at32_getD (by omega), ← hi3, hp]; scalar_tac
      step as ⟨i4, hi4⟩
      step as ⟨i5, hi5⟩
      have hi5v : i5.val = (s.minus.val[j.val]).val := by rw [hi5, hi4]; scalar_tac
      step as ⟨i6, hi6⟩
      step as ⟨q, hq⟩
      have hqv : q.val = tpM s j.val := by
        rw [← hsr.2, ← hi5v, at32_getD (by omega), ← hi6, hq]; scalar_tac
      have hitl : is_thin.val.length = s.topes.val.length := by rw [hit]; simp
      step as ⟨b, hb⟩
      have hbv : b = th (tpP s j.val) := by rw [hb, read_map hit, hpv]
      have hrl := hrm.1
      -- the hits of facet `j`
      have hitP : th (tpP s j.val) = true → th (tpM s j.val) = false →
          ∀ r u, rimHit s th R j.val r u ↔ (r = R p.val ∧ u = q.val) := by
        intro h1 h2 r u; unfold rimHit; rw [hpv, hqv]
        constructor
        · rintro (⟨_, _, e1, e2⟩ | ⟨h3, _⟩)
          · exact ⟨e1.symm, e2.symm⟩
          · rw [h2] at h3; cases h3
        · rintro ⟨e1, e2⟩; exact Or.inl ⟨h1, h2, e1.symm, e2.symm⟩
      have hitM : th (tpM s j.val) = true → th (tpP s j.val) = false →
          ∀ r u, rimHit s th R j.val r u ↔ (r = R q.val ∧ u = p.val) := by
        intro h1 h2 r u; unfold rimHit; rw [hpv, hqv]
        constructor
        · rintro (⟨h3, _⟩ | ⟨_, _, e1, e2⟩)
          · rw [h2] at h3; cases h3
          · exact ⟨e1.symm, e2.symm⟩
        · rintro ⟨e1, e2⟩; exact Or.inr ⟨h1, h2, e1.symm, e2.symm⟩
      have hitN : th (tpP s j.val) = th (tpM s j.val) → ∀ r u, ¬ rimHit s th R j.val r u := by
        intro he r u hh; unfold rimHit at hh
        rcases hh with ⟨h1, h2, _⟩ | ⟨h1, h2, _⟩
        · rw [he, h2] at h1; cases h1
        · rw [← he, h2] at h1; cases h1
      have hpush : ∀ (rm0 : alloc.vec.Vec (alloc.vec.Vec U32)) (r0 : Nat), RimInv s th R s.topes.val.length j.val rm0 →
          r0 < s.topes.val.length → (rm0.val.getD r0 default).val.length < Usize.max := by
        intro rm0 r0 h hr0
        have := (h.2 r0 hr0).1; omega
      split
      · rename_i hbt
        step as ⟨b1, hb1⟩
        have hb1v : b1 = th (tpM s j.val) := by rw [hb1, read_map hit, hqv]
        split
        · rename_i hb1t
          step as ⟨b2, hb2⟩
          have hb2v : b2 = th (tpM s j.val) := by rw [hb2, read_map hit, hqv]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
          rw [hj1]
          exact rimInv_none hrm (hitN (by rw [← hbv, ← hb1v, hbt, hb1t]))
        · rename_i hb1f
          have hthP : th (tpP s j.val) = true := by rw [← hbv]; exact hbt
          have hthM : th (tpM s j.val) = false := by rw [← hb1v]; simpa using hb1f
          step with roots_find hu p (by omega) as ⟨ru, hr, hu2⟩
          obtain ⟨r, u2⟩ := ru
          simp only at hr hu2 ⊢
          have hrlt : r.val < s.topes.val.length := by rw [hr]; exact hR _ (by omega)
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
          have hinv1 := rimInv_push hrm hrlt (y := i7)
            (fun r' u' => by rw [hitP hthP hthM r' u', hr, hi7v]) v4 hw0a v5 hv5 (rm.set r v5)
            (alloc.vec.Vec.set_val_eq ..)
          step as ⟨b2, hb2⟩
          have hb2v : b2 = th (tpM s j.val) := by rw [hb2, read_map hit, hqv]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [hj1]
          exact hinv1
      · rename_i hbf
        have hthP : th (tpP s j.val) = false := by rw [← hbv]; simpa using hbf
        step as ⟨b2, hb2⟩
        have hb2v : b2 = th (tpM s j.val) := by rw [hb2, read_map hit, hqv]
        split
        · rename_i hb2t
          have hthM : th (tpM s j.val) = true := by rw [← hb2v]; exact hb2t
          try (rw [if_neg hbf])
          step with roots_find hu q (by omega) as ⟨ru, hr, hu2⟩
          obtain ⟨r, u2⟩ := ru
          simp only at hr hu2 ⊢
          have hrlt : r.val < s.topes.val.length := by rw [hr]; exact hR _ (by omega)
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
          exact rimInv_push hrm hrlt (y := i7)
            (fun r' u' => by rw [hitM hthM hthP r' u', hr, hi7v]) v4 hw0a v5 hv5 _ (alloc.vec.Vec.set_val_eq ..)
        · rename_i hb2f
          have hthM : th (tpM s j.val) = false := by rw [← hb2v]; simpa using hb2f
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
          rw [hj1]
          exact rimInv_none hrm (hitN (by rw [hthP, hthM]))
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = s.facets.val.length := by scalar_tac
      rw [this] at hrm
      exact ⟨hu, hrm⟩
  · exact ⟨hk, hu, hrim⟩

/-! ### Sorting and deduplicating the rims -/

def DInv (v : List U32) (j : Nat) (d : List U32) : Prop :=
  (d.map (·.val)).Pairwise (· < ·) ∧ (∀ x, x ∈ d.map (·.val) ↔ x ∈ (v.take j).map (·.val)) ∧
    d.length ≤ j ∧ (d.map (·.val)).getLast? = ((v.take j).map (·.val)).getLast?

theorem sorted_le {v : List U32} (hs : v.Pairwise Ru) {i j : Nat} (hij : i ≤ j) (hj : j < v.length) :
    (v[i]'(by omega)).val ≤ v[j].val := by
  rcases Nat.lt_or_eq_of_le hij with h | h
  · exact List.pairwise_iff_getElem.mp hs i j (by omega) hj h
  · subst h; exact Nat.le_refl _

theorem mem_take_le {v : List U32} (hs : v.Pairwise Ru) {j : Nat} (hj : j < v.length) (hj0 : 0 < j) {y : Nat}
    (hy : y ∈ (v.take j).map (·.val)) : y ≤ (v[j - 1]'(by omega)).val := by
  obtain ⟨z, hz, rfl⟩ := List.mem_map.mp hy
  obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hz
  simp only [List.length_take] at hi
  rw [List.getElem_take]
  exact sorted_le hs (by omega) (by omega)

theorem getLast_take {v : List Nat} {j : Nat} (hj : j ≤ v.length) (hj0 : 0 < j) :
    (v.take j).getLast? = some (v[j - 1]'(by omega)) := by
  rw [List.getLast?_eq_getElem?]
  simp only [List.length_take, Nat.min_eq_left hj]
  rw [List.getElem?_take]; simp [show j - 1 < j by omega, List.getElem?_eq_getElem (show j - 1 < v.length by omega)]

theorem dedup_loop_spec (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (t : Usize) (ht : t.val < rim.val.length)
    (hs : (rim.val[t.val]).val.Pairwise Ru) (d : alloc.vec.Vec U32) (j : Usize)
    (hj : j.val ≤ (rim.val[t.val]).val.length) (hd : DInv (rim.val[t.val]).val j.val d.val) :
    collapse_loop5_loop0 rim t d j ⦃ r => (r.val.map (·.val)).Pairwise (· < ·) ∧
      ∀ x, x ∈ r.val.map (·.val) ↔ x ∈ (rim.val[t.val]).val.map (·.val) ⦄ := by
  unfold collapse_loop5_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize) => (rim.val[t.val]).val.length - q.2.val)
    (fun q => q.2.val ≤ (rim.val[t.val]).val.length ∧ DInv (rim.val[t.val]).val q.2.val q.1.val)
  · rintro ⟨w, i⟩ ⟨hi, hw⟩
    unfold collapse_loop5_loop0.body
    simp only at hi hw ⊢
    obtain ⟨hw1, hw2, hw3, hw4⟩ := hw
    step as ⟨v, hv⟩
    have hvv : v = rim.val[t.val] := hv
    subst hvv
    split
    · rename_i hlt
      have hil : i.val < (rim.val[t.val]).val.length := by scalar_tac
      have htake : ((rim.val[t.val]).val.take (i.val + 1)).map (·.val) =
          ((rim.val[t.val]).val.take i.val).map (·.val) ++ [((rim.val[t.val]).val[i.val]).val] := by
        rw [List.take_add_one, List.getElem?_eq_getElem hil]
        simp only [Option.toList_some, List.map_append, List.map_cons, List.map_nil]
      have hwl : w.val.length < Usize.max := by have := hw3; scalar_tac
      step as ⟨b, hb⟩
      split
      · rename_i hbt
        have hw0 : w.val = [] := by
          have : w.length = 0 := by rw [← hb]; exact hbt
          exact List.eq_nil_of_length_eq_zero (by simpa using this)
        have hi0 : i.val = 0 := by
          by_contra hne
          have := getLast_take (v := (rim.val[t.val]).val.map (·.val)) (j := i.val) (by simp; omega) (by omega)
          rw [← List.map_take, ← hw4, hw0] at this; simp at this
        step as ⟨x, hx⟩
        step as ⟨d1, hd1⟩
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, hd1, hw0, hx]
        have ht0 : (rim.val[t.val]).val.take i.val = [] := by rw [hi0]; rfl
        refine ⟨by simp, ?_, by simp, ?_⟩
        · intro y; rw [htake, ht0]; simp
        · rw [htake, ht0]; simp
      · rename_i hbf
        have hwne : 0 < w.val.length := by
          have : ¬ w.length = 0 := by intro h; exact hbf (by rw [hb]; exact h)
          exact Nat.pos_of_ne_zero this
        have hi0 : 0 < i.val := by omega
        have hlast := getLast_take (v := (rim.val[t.val]).val.map (·.val)) (j := i.val) (by simp; omega) hi0
        rw [← List.map_take, ← hw4, List.getLast?_eq_getElem?] at hlast
        simp only [List.length_map, List.getElem?_map, List.getElem?_eq_getElem (show w.val.length - 1 < w.val.length by omega),
          Option.map_some, Option.some.injEq, List.getElem_map] at hlast
        step as ⟨i2, hi2⟩
        step as ⟨i3, hi3⟩
        step as ⟨i4, hi4⟩
        have hi3v : i3.val = ((rim.val[t.val]).val[i.val - 1]).val := by
          rw [hi3]; simp only [hi2]; rw [← hlast]; congr 2
        split
        · rename_i hne
          have hne' : i3.val ≠ i4.val := by intro e; exact (by simpa using hne : i3 ≠ i4) (UScalar.eq_of_val_eq e)
          step as ⟨x, hx⟩
          step as ⟨d1, hd1⟩
          step as ⟨i1, hi1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hi1, hd1, hx]
          have hxle : ∀ y ∈ w.val.map (·.val), y < ((rim.val[t.val]).val[i.val]).val := by
            intro y hy
            have h1 := mem_take_le hs hil hi0 ((hw2 y).mp hy)
            have h2 := sorted_le hs (i := i.val - 1) (j := i.val) (by omega) hil
            have h3 : y ≠ ((rim.val[t.val]).val[i.val]).val := by
              intro e; rw [e] at h1; rw [hi3v, hi4] at hne'
              exact hne' (by omega)
            omega
          refine ⟨?_, ?_, by simp; omega, ?_⟩
          · rw [List.map_append, List.pairwise_append]
            exact ⟨hw1, by simp, fun a ha b hb' => by simp at hb'; rw [hb']; exact hxle a ha⟩
          · intro y; rw [htake, List.map_append, List.mem_append, List.mem_append, hw2]; simp
          · rw [htake, List.map_append]; simp
        · rename_i heq
          have heq' : i3.val = i4.val := by simpa using heq
          step as ⟨i1, hi1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hi1]
          have hlx : ((rim.val[t.val]).val[i.val]).val = (w.val[w.val.length - 1]).val := by
            rw [← hi4, ← heq', hi3v, ← hlast]
          refine ⟨hw1, ?_, by omega, ?_⟩
          · intro y; rw [htake, List.mem_append, ← hw2]
            constructor
            · intro h; exact Or.inl h
            · rintro (h | h)
              · exact h
              · simp at h; rw [h, hlx]; exact List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
          · rw [htake, List.getLast?_append, List.getLast?_eq_getElem?]
            simp [hlx, List.getElem?_eq_getElem (show w.val.length - 1 < w.val.length by omega)]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = (rim.val[t.val]).val.length := by scalar_tac
      rw [this, List.take_of_length_le (Nat.le_refl _)] at hw2
      exact ⟨hw1, hw2⟩
  · exact ⟨hj, hd⟩

/-- The rims as loop 5 leaves them: ascending, each with its old members. -/
def SortInv (rim0 rim : alloc.vec.Vec (alloc.vec.Vec U32)) (nt t : Nat) : Prop :=
  rim.val.length = nt ∧ ∀ r, r < nt →
    (r < t → (rimAt rim r).Pairwise (· < ·) ∧ ∀ x, x ∈ rimAt rim r ↔ x ∈ rimAt rim0 r) ∧
    (t ≤ r → rim.val.getD r default = rim0.val.getD r default)

theorem rimAt_eq (rim : alloc.vec.Vec (alloc.vec.Vec U32)) {r : Nat} (hr : r < rim.val.length) :
    rimAt rim r = (rim.val[r]).val.map (·.val) := by
  simp [rimAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hr]

theorem pairwise_short {l : List Nat} (h : l.length ≤ 1) : l.Pairwise (· < ·) := by
  rcases l with _ | ⟨a, _ | ⟨b, l⟩⟩
  · exact List.Pairwise.nil
  · exact List.pairwise_singleton _ _
  · simp at h

theorem loop5_spec (nt : Usize) (rim0 : alloc.vec.Vec (alloc.vec.Vec U32)) (M : Nat) (hM : 3 * M ≤ Usize.max)
    (hlen : ∀ r, r < nt.val → (rim0.val.getD r default).val.length ≤ M)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (t : Usize) (ht : t.val ≤ nt.val) (hinv : SortInv rim0 rim nt.val t.val) :
    collapse_loop5 nt rim t ⦃ r => SortInv rim0 r nt.val nt.val ⦄ := by
  unfold collapse_loop5
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => nt.val - q.2.val)
    (fun q => q.2.val ≤ nt.val ∧ SortInv rim0 q.1 nt.val q.2.val)
  · rintro ⟨rm, j⟩ ⟨hj, hl, hrr⟩
    unfold collapse_loop5.body
    simp only at hj hl hrr ⊢
    split
    · rename_i hlt
      have hjl : j.val < nt.val := by scalar_tac
      have hold : rm.val[j.val] = rim0.val.getD j.val default := by
        have := (hrr j.val hjl).2 (Nat.le_refl _)
        rwa [List.getD_eq_getElem?_getD (l := rm.val), List.getElem?_eq_getElem (show j.val < rm.val.length by omega),
          Option.getD_some] at this
      have hl0 := hlen j.val hjl
      have hmem0 : ∀ x, x ∈ (rm.val[j.val]).val.map (·.val) ↔ x ∈ rimAt rim0 j.val := by
        intro x; rw [hold]; unfold rimAt; rfl
      step as ⟨v, hv⟩
      split
      · rename_i hgt
        step as ⟨v1, back, hw0a, hback⟩
        subst hback
        have hv1l : 3 * v1.val.length ≤ Usize.max := by
          rw [hw0a, hold]; omega
        step with sort_u32_spec v1 hv1l as ⟨v2, hperm, hsorted⟩
        have hrl1 : j.val < (rm.set j v2).val.length := by simp [alloc.vec.Vec.set_val_eq]; omega
        have hget : ((rm.set j v2).val[j.val]'hrl1) = v2 := by simp [alloc.vec.Vec.set_val_eq]
        step with dedup_loop_spec (rm.set j v2) j hrl1 (by rw [hget]; exact hsorted) (alloc.vec.Vec.new U32) 0#usize
          (by simp) (by simp [DInv]) as ⟨d, hd1, hd2⟩
        step as ⟨x1, back1, hw1a, hback1⟩
        subst hback1
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        refine ⟨by simp [alloc.vec.Vec.set_val_eq, hl], fun r hr => ⟨fun hrj => ?_, fun hrj => ?_⟩⟩
        · by_cases e : r = j.val
          · subst e
            rw [rimAt_eq _ (by simp [alloc.vec.Vec.set_val_eq]; omega)]
            simp only [alloc.vec.Vec.set_val_eq, List.getElem_set_self]
            refine ⟨hd1, fun x => ?_⟩
            rw [hd2, hget, ← hmem0, ← hw0a]
            simp only [List.mem_map]
            exact ⟨fun ⟨a, ha, e⟩ => ⟨a, hperm.mem_iff.mp ha, e⟩, fun ⟨a, ha, e⟩ => ⟨a, hperm.mem_iff.mpr ha, e⟩⟩
          · have : r < j.val := by rw [hj1] at hrj; omega
            have h1 := (hrr r hr).1 this
            unfold rimAt at h1 ⊢
            simp only [alloc.vec.Vec.set_val_eq, List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm e)] at h1 ⊢
            exact h1
        · have e : r ≠ j.val := by rw [hj1] at hrj; omega
          have h1 := (hrr r hr).2 (by rw [hj1] at hrj; omega)
          simp only [alloc.vec.Vec.set_val_eq, List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm e)] at h1 ⊢
          exact h1
      · rename_i hle
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        refine ⟨hl, fun r hr => ⟨fun hrj => ?_, fun hrj => (hrr r hr).2 (by rw [hj1] at hrj; omega)⟩⟩
        have hvl0 : v.val.length ≤ 1 := by scalar_tac
        by_cases e : r = j.val
        · subst e
          rw [rimAt_eq _ (by omega)]
          have hvl : (rm.val[j.val]).val.length ≤ 1 := by rw [← hv]; exact hvl0
          exact ⟨pairwise_short (by simpa using hvl), hmem0⟩
        · exact (hrr r hr).1 (by rw [hj1] at hrj; omega)
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nt.val := by scalar_tac
      rw [this] at hrr
      exact ⟨hl, hrr⟩
  · exact ⟨ht, hinv⟩

/-! ### The refusal scan -/

theorem all_take_false {α : Type} {l : List α} {p : α → Bool} {j : Nat} (h : (l.take j).all p = false) :
    l.all p = false := by
  conv => lhs; rw [← List.take_append_drop j l]
  rw [List.all_append, h, Bool.false_and]

theorem uniform_loop_spec (keep : Slice Bool) (rm : alloc.vec.Vec U32) (first : Bool)
    (hrm : ∀ x ∈ rm.val, x.val < keep.val.length) (uniform : Bool) (j : Usize) (hj : j.val ≤ rm.val.length)
    (hu : uniform = (rm.val.take j.val).all fun x => kAt keep.val x.val == first) :
    collapse_loop6_loop0 keep rm first uniform j ⦃ r => r = rm.val.all fun x => kAt keep.val x.val == first ⦄ := by
  unfold collapse_loop6_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => rm.val.length - q.2.val)
    (fun q => q.2.val ≤ rm.val.length ∧ q.1 = (rm.val.take q.2.val).all fun x => kAt keep.val x.val == first)
  · rintro ⟨u, i⟩ ⟨hi, hu⟩
    unfold collapse_loop6_loop0.body
    simp only at hi hu ⊢
    split
    · rename_i hut
      split
      · rename_i hlt
        have hil : i.val < rm.val.length := by scalar_tac
        step as ⟨x, hx⟩
        step as ⟨c, hc⟩
        have hcv : c.val = (rm.val[i.val]).val := by rw [hc, hx]; scalar_tac
        have hcl : c.val < keep.val.length := by rw [hcv]; exact hrm _ (List.getElem_mem _)
        step as ⟨b, hb⟩
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, List.take_add_one, List.getElem?_eq_getElem hil, Option.toList_some, List.all_append, ← hu, hut]
        simp only [List.all_cons, List.all_nil, Bool.and_true, Bool.true_and, kAt, List.getD_eq_getElem?_getD]
        rw [← hcv, List.getElem?_eq_getElem hcl, Option.getD_some, ← hb]
        cases b <;> cases first <;> rfl
      · rename_i hge
        simp only [WP.spec_ok]
        have : i.val = rm.val.length := by scalar_tac
        rw [hu, this, List.take_of_length_le (Nat.le_refl _)]
    · rename_i huf
      simp only [WP.spec_ok]
      have huf' : u = false := by simpa using huf
      rw [huf'] at hu ⊢
      exact (all_take_false hu.symm).symm
  · exact ⟨hj, hu⟩

/-- Position `t` is refused: thin, and its class's rim kept both ways or the class not enclosed. -/
def bad (th : Nat → Bool) (keep : List Bool) (rimL : Nat → List Nat) (R : Nat → Nat) (aenc : Nat → Bool) (t : Nat) : Bool :=
  th t && !(uniR keep (rimL (R t)) t && aenc (R t))

theorem loop6_spec (s : Split) (keep : Slice Bool) (nt : Usize) (hnt : nt.val = s.topes.val.length)
    (hkl : keep.val.length = nt.val) (hsm : ∀ t, t < nt.val → at32 s.topes t < U32.max)
    (th : Nat → Bool) (is_thin : alloc.vec.Vec Bool) (hit : is_thin.val = (List.range nt.val).map th)
    (R : Nat → Nat) (hR : ∀ y, y < nt.val → R y < nt.val)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (hrl : rim.val.length = nt.val)
    (hrb : ∀ r, r < nt.val → ∀ x ∈ (rim.val.getD r default).val, x.val < nt.val)
    (aenc : Nat → Bool) (all_enc : alloc.vec.Vec Bool) (hae : all_enc.val = (List.range nt.val).map aenc)
    (bd : U32) (uf : union_find.Uf) (t : Usize) (ht : t.val ≤ nt.val) (hu : Roots nt.val R uf)
    (hb0 : bd = NONE) (hb1 : ∀ t', t' < t.val → bad th keep.val (rimAt rim) R aenc t' = false) :
    collapse_loop6 s.topes keep nt is_thin bd uf rim all_enc t ⦃ r => Roots nt.val R r.2 ∧
      ((r.1 = NONE ∧ (List.range nt.val).find? (bad th keep.val (rimAt rim) R aenc) = none) ∨
       (r.1 ≠ NONE ∧ ∃ t0, (List.range nt.val).find? (bad th keep.val (rimAt rim) R aenc) = some t0 ∧
         r.1.val = at32 s.topes t0)) ⦄ := by
  unfold collapse_loop6
  apply loop.spec_decr_nat (fun (q : U32 × union_find.Uf × Usize) => nt.val + 1 - q.2.2.val - (if q.1 = NONE then 0 else 1))
    (fun q => q.2.2.val ≤ nt.val ∧ Roots nt.val R q.2.1 ∧
      (q.1 = NONE → ∀ t', t' < q.2.2.val → bad th keep.val (rimAt rim) R aenc t' = false) ∧
      (q.1 ≠ NONE → ∃ t0, t0 < nt.val ∧ bad th keep.val (rimAt rim) R aenc t0 = true ∧
        (∀ t', t' < t0 → bad th keep.val (rimAt rim) R aenc t' = false) ∧ q.1.val = at32 s.topes t0))
  · rintro ⟨b0, u, j⟩ ⟨hj, hu, hn, hs⟩
    unfold collapse_loop6.body
    simp only at hj hu hn hs ⊢
    split
    · rename_i hbn
      have hn' := hn hbn
      split
      · rename_i hlt
        have hjl : j.val < nt.val := by scalar_tac
        have hitl : is_thin.val.length = nt.val := by rw [hit]; simp
        step as ⟨b, hb⟩
        have hbv : b = th j.val := by rw [hb, read_map hit]
        split
        · rename_i hbt
          step with roots_find hu j hjl as ⟨r, u2, hr, hu2⟩
          have hrlt : r.val < nt.val := by rw [hr]; exact hR _ hjl
          step as ⟨rm, hrm⟩
          have hrmv : rm.val.map (·.val) = rimAt rim (R j.val) := by
            rw [← hr, rimAt_eq _ (by omega), hrm]
          have hrmb : ∀ x ∈ rm.val, x.val < keep.val.length := by
            intro x hx; rw [hkl]
            have := hrb r.val hrlt x
            simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (show r.val < rim.val.length by omega),
              Option.getD_some] at this
            rw [hrm] at hx; exact this hx
          step as ⟨b1, hb1⟩
          -- the first keep of the rim
          have hfirst : ∀ (fst : Bool), (rm.val = [] → fst = kAt keep.val j.val) →
              (∀ h : 0 < rm.val.length, fst = kAt keep.val (rm.val[0]).val) →
              fst = firstK keep.val (rimAt rim (R j.val)) j.val := by
            intro fst h1 h2
            rw [← hrmv]
            cases hc : rm.val with
            | nil => simp only [List.map_nil, firstK]; exact h1 hc
            | cons a rest =>
              simp only [List.map_cons, firstK]
              have := h2 (by rw [hc]; simp)
              simp only [hc, List.getElem_cons_zero] at this; exact this
          have hfin : ∀ (fst : Bool), fst = firstK keep.val (rimAt rim (R j.val)) j.val →
              collapse_loop6_loop0 keep rm fst true 0#usize ⦃ un => un = uniR keep.val (rimAt rim (R j.val)) j.val ⦄ := by
            intro fst hf
            apply WP.spec_mono (uniform_loop_spec keep rm fst hrmb true 0#usize (by simp) (by simp))
            intro un hun
            rw [hun, uniR, ← hf, ← hrmv, List.all_map]; rfl
          have hfst : (if b1 then Slice.index_usize keep j else do
                let i ← alloc.vec.Vec.index_usize rm 0#usize
                let i1 ← lift (UScalar.cast .Usize i)
                Slice.index_usize keep i1) ⦃ fst => fst = firstK keep.val (rimAt rim (R j.val)) j.val ⦄ := by
            split
            · rename_i hb1t
              have hemp : rm.val = [] := by
                have : rm.length = 0 := by rw [← hb1]; exact hb1t
                exact List.eq_nil_of_length_eq_zero (by simpa using this)
              step as ⟨fst, hfst⟩
              exact hfirst fst (fun _ => by
                rw [hfst, kAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl)
                (fun h => by rw [hemp] at h; simp at h)
            · rename_i hb1f
              have hne : 0 < rm.val.length := by
                have : ¬ rm.length = 0 := by intro h; exact hb1f (by rw [hb1]; exact h)
                exact Nat.pos_of_ne_zero this
              step as ⟨i, hi⟩
              step as ⟨i1, hi1⟩
              have hi1v : i1.val = (rm.val[0]).val := by rw [hi1, hi]; scalar_tac
              have hi1l : i1.val < keep.val.length := by rw [hi1v]; exact hrmb _ (List.getElem_mem _)
              step as ⟨fst, hfst⟩
              exact hfirst fst (fun h => by rw [h] at hne; simp at hne)
                (fun h => by rw [hfst, kAt, ← hi1v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi1l]; rfl)
          step with hfst as ⟨fst, hf⟩
          step with hfin fst hf as ⟨un, hun⟩
          have htl : j.val < s.topes.val.length := by omega
          have hbadj : bad th keep.val (rimAt rim) R aenc j.val = !(un && aenc (R j.val)) := by
            unfold bad; rw [← hbv, hbt, hun]; rfl
          have hael : all_enc.val.length = nt.val := by rw [hae]; simp
          split
          · rename_i hut
            step as ⟨b2, hb2⟩
            have hb2v : b2 = aenc (R j.val) := by rw [hb2, read_map hae, hr]
            split
            · rename_i hb2t
              have hbj : bad th keep.val (rimAt rim) R aenc j.val = false := by
                rw [hbadj, hut, ← hb2v, hb2t]; rfl
              step as ⟨j1, hj1⟩
              refine ⟨by scalar_tac, hu2, fun _ t' ht' => ?_, fun e => absurd hbn e, ?_⟩
              · by_cases e : t' = j.val
                · rw [e]; exact hbj
                · exact hn' t' (by omega)
              · (try simp only [hbn, ite_true]); scalar_tac
            · rename_i hb2f
              have hbj : bad th keep.val (rimAt rim) R aenc j.val = true := by
                rw [hbadj, hut, ← hb2v]; simpa using hb2f
              step as ⟨x, hx⟩
              have hxv : x.val = at32 s.topes j.val := by rw [hx, at32_getD htl]
              have hxn : x ≠ NONE := by
                intro e; have := hsm j.val hjl; rw [← hxv, e, none_val] at this; omega
              step as ⟨j1, hj1⟩
              refine ⟨by scalar_tac, hu2, fun e => absurd e hxn, fun _ => ⟨j.val, hjl, hbj, hn', hxv⟩, ?_⟩
              (try simp only [hbn, hxn, ite_true, ite_false]); scalar_tac
          · rename_i huf
            have hbj : bad th keep.val (rimAt rim) R aenc j.val = true := by
              rw [hbadj]; simp at huf; simp [huf]
            step as ⟨x, hx⟩
            have hxv : x.val = at32 s.topes j.val := by rw [hx, at32_getD htl]
            have hxn : x ≠ NONE := by
              intro e; have := hsm j.val hjl; rw [← hxv, e, none_val] at this; omega
            step as ⟨j1, hj1⟩
            refine ⟨by scalar_tac, hu2, fun e => absurd e hxn, fun _ => ⟨j.val, hjl, hbj, hn', hxv⟩, ?_⟩
            (try simp only [hbn, hxn, ite_true, ite_false]); scalar_tac
        · rename_i hbf
          step as ⟨j1, hj1⟩
          have hbj : bad th keep.val (rimAt rim) R aenc j.val = false := by
            unfold bad; rw [← hbv]; simp at hbf; simp [hbf]
          refine ⟨by scalar_tac, hu, fun _ t' ht' => ?_, fun e => absurd hbn e, ?_⟩
          · by_cases e : t' = j.val
            · rw [e]; exact hbj
            · exact hn' t' (by omega)
          · (try simp only [hbn, ite_true]); scalar_tac
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nt.val := by scalar_tac
        refine ⟨hu, Or.inl ⟨hbn, List.find?_eq_none.mpr fun x hx => ?_⟩⟩
        have := hn' x (by rw [this]; exact List.mem_range.mp hx)
        simp [this]
    · rename_i hbn
      simp only [WP.spec_ok]
      obtain ⟨t0, ht0, h1, h2, h3⟩ := hs hbn
      refine ⟨hu, Or.inr ⟨hbn, t0, ?_, h3⟩⟩
      rw [List.find?_eq_some_iff_getElem]
      refine ⟨by simpa using h1, t0, by simpa using ht0, by simp, fun i hi => ?_⟩
      simp only [List.getElem_range]
      rw [h2 i hi]; simp
  · exact ⟨ht, hu, fun _ => hb1, fun e => absurd hb0 e⟩

/-! ### The fill -/

def fillAt (th : Nat → Bool) (old : List Bool) (rimL : Nat → List Nat) (R : Nat → Nat) (t u : Nat) : Bool :=
  if decide (u < t) && th u then firstK old (rimL (R u)) u else kAt old u

theorem loop7_spec (nt : Usize) (th : Nat → Bool) (is_thin : alloc.vec.Vec Bool)
    (hit : is_thin.val = (List.range nt.val).map th)
    (R : Nat → Nat) (hR : ∀ y, y < nt.val → R y < nt.val)
    (rim : alloc.vec.Vec (alloc.vec.Vec U32)) (hrl : rim.val.length = nt.val)
    (hrb : ∀ r, r < nt.val → ∀ x ∈ (rim.val.getD r default).val, x.val < nt.val)
    (old : alloc.vec.Vec Bool) (hol : old.val.length = nt.val)
    (keep : Slice Bool) (uf : union_find.Uf) (t : Usize) (ht : t.val ≤ nt.val) (hu : Roots nt.val R uf)
    (hk : keep.val = (List.range nt.val).map (fillAt th old.val (rimAt rim) R t.val)) :
    collapse_loop7 keep nt is_thin uf rim old t ⦃ r =>
      r.val = (List.range nt.val).map (fillAt th old.val (rimAt rim) R nt.val) ⦄ := by
  unfold collapse_loop7
  apply loop.spec_decr_nat (fun (q : Slice Bool × union_find.Uf × Usize) => nt.val - q.2.2.val)
    (fun q => q.2.2.val ≤ nt.val ∧ Roots nt.val R q.2.1 ∧
      q.1.val = (List.range nt.val).map (fillAt th old.val (rimAt rim) R q.2.2.val))
  · rintro ⟨kp, u, j⟩ ⟨hj, hu, hkp⟩
    unfold collapse_loop7.body
    simp only at hj hu hkp ⊢
    split
    · rename_i hlt
      have hjl : j.val < nt.val := by scalar_tac
      have hitl : is_thin.val.length = nt.val := by rw [hit]; simp
      have hkpl : kp.val.length = nt.val := by rw [hkp]; simp
      step as ⟨b, hb⟩
      have hbv : b = th j.val := by rw [hb, read_map hit]
      -- the fill past `j`, off position `j`, is the fill before it
      have hoff : ∀ u, u ≠ j.val → fillAt th old.val (rimAt rim) R (j.val + 1) u =
          fillAt th old.val (rimAt rim) R j.val u := by
        intro u hu; unfold fillAt
        have : decide (u < j.val + 1) = decide (u < j.val) := by
          apply decide_eq_decide.mpr; omega
        rw [this]
      have hat : fillAt th old.val (rimAt rim) R j.val j.val = kAt old.val j.val := by
        unfold fillAt; simp
      split
      · rename_i hbt
        step with roots_find hu j hjl as ⟨ru, hr, hu2⟩
        obtain ⟨r, u2⟩ := ru
        try simp only at hr hu2 ⊢
        have hrlt : r.val < nt.val := by rw [hr]; exact hR _ hjl
        step as ⟨rm, hrm⟩
        have hrmv : rm.val.map (·.val) = rimAt rim (R j.val) := by
          rw [← hr, rimAt_eq _ (by omega), hrm]
        have hnew : fillAt th old.val (rimAt rim) R (j.val + 1) j.val = firstK old.val (rimAt rim (R j.val)) j.val := by
          unfold fillAt; rw [← hbv, hbt]; simp
        have hrmb : ∀ x ∈ rm.val, x.val < nt.val := by
          intro x hx
          have := hrb r.val hrlt x
          simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (show r.val < rim.val.length by omega),
            Option.getD_some] at this
          rw [hrm] at hx; exact this hx
        step as ⟨b1, hb1⟩
        split
        · rename_i hb1t
          have hemp : rm.val = [] := by
            have : rm.length = 0 := by rw [← hb1]; exact hb1t
            exact List.eq_nil_of_length_eq_zero (by simpa using this)
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
          rw [hkp, hj1]
          apply List.map_congr_left
          intro u hu'
          by_cases e : u = j.val
          · rw [e, hnew, hat, ← hrmv, hemp]; rfl
          · rw [hoff u e]
        · rename_i hb1f
          have hne : 0 < rm.val.length := by
            have : ¬ rm.length = 0 := by intro h; exact hb1f (by rw [hb1]; exact h)
            exact Nat.pos_of_ne_zero this
          step as ⟨i, hi⟩
          step as ⟨i1, hi1⟩
          have hi1v : i1.val = (rm.val[0]).val := by rw [hi1, hi]; scalar_tac
          have hi1l : i1.val < old.val.length := by rw [hi1v, hol]; exact hrmb _ (List.getElem_mem _)
          step as ⟨b2, hb2⟩
          step as ⟨kp1, hkp1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, hu2, ?_, by scalar_tac⟩
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
            rw [hnew, ← hrmv]
            cases hc : rm.val with
            | nil => rw [hc] at hne; simp at hne
            | cons a rest =>
              simp only [List.map_cons, firstK, kAt]
              rw [hb2, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by rw [hol]; have := hrmb a (by rw [hc]; simp); omega)]
              simp only [Option.getD_some]
              congr 1; rw [hi1v]; simp [hc]
          · rename_i e
            rw [hoff u (Ne.symm e)]
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, hu, ?_, by scalar_tac⟩
        rw [hkp, hj1]
        apply List.map_congr_left
        intro u _
        by_cases e : u = j.val
        · rw [e, hat]; unfold fillAt; rw [← hbv]; simp at hbf; simp [hbf]
        · rw [hoff u e]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nt.val := by scalar_tac
      rw [this] at hkp
      exact hkp
  · exact ⟨ht, hu, hk⟩

end K

/-! ## Part 3: the tables from `AfterSplit` -/

namespace K

variable {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext}

theorem clone_new_ok : alloc.vec.CloneVec.clone core.clone.CloneU32 (alloc.vec.Vec.new U32) =
    ok (alloc.vec.Vec.new U32) := by
  apply ok_of_spec
  apply WP.spec_mono (clone_u32_spec (alloc.vec.Vec.new U32))
  intro r hr
  exact (alloc.vec.Vec.eq_iff _ _).mpr hr

theorem mk_nil_map (s : Split) (n : Nat) : List.replicate n false = (List.range n).map (mk s []) := by
  apply List.ext_getElem (by simp); intro i h1 h2; simp [mk]

theorem hpc_of (h : AfterSplit l e0 s ext) : ∀ q ∈ pcs l, q.1 < l.cells.val.length :=
  fun q hq => (h.wf.pieces q hq).1

theorem topes_len (h : AfterSplit l e0 s ext) : (toL l).topes.length = s.topes.val.length := by
  rw [← topes_toL l e0 s ext h.ext0 h.split (hpc_of h)]; simp

theorem topes_at (h : AfterSplit l e0 s ext) {t : Nat} (ht : t < s.topes.val.length) :
    (toL l).topes[t]'(by rw [topes_len h]; exact ht) = covE l ext (at32 s.topes t) := by
  have hTL := topes_toL l e0 s ext h.ext0 h.split (hpc_of h)
  simp only [← hTL, List.getElem_map]
  rw [at32_getD ht]

theorem facets_len (h : AfterSplit l e0 s ext) : (toL l).facets.length = s.facets.val.length := by
  rw [← facets_toL l e0 h.ext0 (hpc_of h) s.facets.val h.split.facets]; simp

theorem sides_of (h : AfterSplit l e0 s ext) : Sides (toL l) s.facets.val.length (tpP s) (tpM s) where
  len := facets_len h
  pp_lt k hk := (tp_sides l e0 s ext h.ext0 h.split (hpc_of h) hk).1
  qq_lt k hk := (tp_sides l e0 s ext h.ext0 h.split (hpc_of h) hk).2.1
  pp_eq k hk := by
    obtain ⟨_, _, h1, _⟩ := tp_sides l e0 s ext h.ext0 h.split (hpc_of h) hk
    rw [h1]
    have := facets_toL l e0 h.ext0 (hpc_of h) s.facets.val h.split.facets
    simp only [← this, List.getElem_map]
  qq_eq k hk := by
    obtain ⟨_, _, _, h2⟩ := tp_sides l e0 s ext h.ext0 h.split (hpc_of h) hk
    rw [h2]
    have := facets_toL l e0 h.ext0 (hpc_of h) s.facets.val h.split.facets
    simp only [← this, List.getElem_map]

theorem nodup_lt_length {xs : List Nat} (hnd : xs.Nodup) {n : Nat} (h : ∀ x ∈ xs, x < n) : xs.length ≤ n := by
  have := List.Nodup.length_le_of_subset hnd (fun x hx => List.mem_range.mpr (h x hx) : xs ⊆ List.range n)
  simpa using this

theorem tp_of (h : AfterSplit l e0 s ext) : TP s (allCells l ext).length := by
  have hT := h.split.topes
  have hok := h.split.ext
  have hN := hok.small
  have hvals : s.topes.val.map (·.val) = dedup (stream s.plus.val s.minus.val s.facets.val.length) := hT.2.1
  have hnt : s.topes.val.length ≤ (allCells l ext).length := by
    have := nodup_lt_length (nodup_dedup (stream s.plus.val s.minus.val s.facets.val.length))
      (n := (allCells l ext).length) (fun x hx => hT.2.2.1 x ((mem_dedup _ _).mp hx))
    rw [← hvals] at this; simpa using this
  have hat : ∀ t (ht : t < s.topes.val.length), at32 s.topes t =
      (dedup (stream s.plus.val s.minus.val s.facets.val.length))[t]'(by rw [← hvals]; simpa using ht) := by
    intro t ht; rw [at32_getD ht]; simp only [← hvals, List.getElem_map]
  have htp : ∀ c, c < (allCells l ext).length → at32 s.tope_pos c =
      if c ∈ stream s.plus.val s.minus.val s.facets.val.length then
        (dedup (stream s.plus.val s.minus.val s.facets.val.length)).idxOf c else U32.max := by
    intro c hc
    rw [← hT.2.2.2 c hc, at32_getD (by rw [hT.1]; exact hc), List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by rw [hT.1]; exact hc)]; rfl
  refine ⟨hT.1, by omega, fun t ht => ?_, fun c hc hne => ?_⟩
  · have hmem : at32 s.topes t ∈ dedup (stream s.plus.val s.minus.val s.facets.val.length) := by
      rw [hat t ht]; exact List.getElem_mem _
    have hS := (mem_dedup _ _).mp hmem
    refine ⟨hT.2.2.1 _ hS, ?_⟩
    rw [htp _ (hT.2.2.1 _ hS), if_pos hS, hat t ht, List.Nodup.idxOf_getElem (nodup_dedup _)]
  · rw [htp c hc] at hne ⊢
    have hS : c ∈ stream s.plus.val s.minus.val s.facets.val.length := by
      by_contra hn; rw [if_neg hn] at hne; exact hne rfl
    rw [if_pos hS]
    have hlt := List.idxOf_lt_length_iff.mpr ((mem_dedup _ _).mpr hS)
    have hl2 : (dedup (stream s.plus.val s.minus.val s.facets.val.length)).length = s.topes.val.length := by
      rw [← hvals]; simp
    refine ⟨by omega, ?_⟩
    rw [hat _ (by omega), List.getElem_idxOf]

theorem nf_le (h : AfterSplit l e0 s ext) :
    s.facets.val.length ≤ l.cells.val.length ∧ s.facets.val.length ≤ l.pieces.val.length := by
  have hF := h.split.facets
  have hnd : (s.facets.val.map (·.val)).Nodup := by rw [hF]; exact nodup_dedup _
  have h1 := nodup_lt_length hnd (n := l.cells.val.length) (fun x hx => by
    rw [hF] at hx
    obtain ⟨q, hq, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hx)
    exact hpc_of h q hq)
  have h2 := dedup_length_le ((pcs l).map Prod.fst)
  rw [← hF] at h2
  simp only [List.length_map] at h1 h2
  exact ⟨h1, by simpa [pcs] using h2⟩

theorem sw_of (h : AfterSplit l e0 s ext) : SW s (allCells l ext).length := by
  obtain ⟨_, _, _, _, hPl, hMl, hPM⟩ := h.split.sides
  have hT := h.split.topes
  have hnf := nf_le h
  have hroom := h.wf.room
  have hU := u32_le_usize
  have hnt := (tp_of h).small
  refine ⟨hPl, hMl, hT.1, fun k hk => ?_, fun k hk => ?_, fun k hk => ?_, fun k hk => ?_, by omega, by omega⟩
  · have := (hPM k hk).1; rw [at32_getD (by omega), ← getElem!_pos s.plus.val k (by omega)]; exact this
  · have := (hPM k hk).2.2.1; rw [at32_getD (by omega), ← getElem!_pos s.minus.val k (by omega)]; exact this
  · have := (tp_sides l e0 s ext h.ext0 h.split (hpc_of h) hk).1; rw [topes_len h] at this; exact this
  · have := (tp_sides l e0 s ext h.ext0 h.split (hpc_of h) hk).2.1; rw [topes_len h] at this; exact this

/-- A list of ledger cells marks the positions of its topes. -/
theorem thinAt_of (h : AfterSplit l e0 s ext) (v : alloc.vec.Vec CellId)
    (hv : ∀ c ∈ v.val, (c : U32).val < l.cells.val.length) :
    ThinAt (toL l) (v.val.map fun c => cellL l (c : U32).val) (mk s (idsOf v)) := by
  intro t ht
  have ht' : t < s.topes.val.length := by rw [← topes_len h]; exact ht
  rw [topes_at h ht']
  have hok := h.split.ext
  have hlen := allCells_length l ext
  have hTt := ((tp_of h).pos t ht').1
  unfold mk idsOf
  apply Bool.eq_iff_iff.mpr
  simp only [List.contains_iff_mem, List.mem_map]
  constructor
  · rintro ⟨c, hc, e⟩
    refine ⟨c, hc, ?_⟩
    rw [← e, cellL_covAt l ext (hv c hc)]; rfl
  · rintro ⟨c, hc, e⟩
    refine ⟨c, hc, ?_⟩
    have hc' := hv c hc
    rw [cellL_covAt l ext hc'] at e
    exact covE_inj l ext hok (by omega) hTt e

end K

/-! ## `CollapseSpec` -/

open K in
theorem collapse_spec_proof : CollapseSpec := by
  intro l e0 s ext keep m hA hM hkl
  have hok := hA.split.ext
  have hN := hok.small
  have hU := u32_le_usize
  have hTP := tp_of hA
  have hSW := sw_of hA
  have hS := sides_of hA
  have hTl := topes_len hA
  have hlen := allCells_length l ext
  have hthinlt : ∀ c ∈ m.thin.val, (c : U32).val < l.cells.val.length := hM.thin
  have henclt : ∀ c ∈ m.enclosed.val, (c : U32).val < l.cells.val.length := hM.enclosed
  have hthA := thinAt_of hA m.thin hthinlt
  have henA := thinAt_of hA m.enclosed henclt
  set th := mk s (idsOf m.thin) with hth_def
  set en := mk s (idsOf m.enclosed) with hen_def
  set nf := s.facets.val.length with hnf_def
  have hnt_small := hTP.small
  have hnfb := nf_le hA
  have hroom := hA.wf.room
  unfold collapse
  step with ncells_spec l ext (by omega) as ⟨n, hn⟩
  have hTPn : TP s n.val := by rw [hn]; exact hTP
  step as ⟨it0, hit0, hit0l⟩
  step with K.loop0_spec s m n hTPn (fun c hc => by rw [hn]; have := hthinlt c hc; omega) it0 false 0#usize
    (by simp) (by rw [hit0]; simpa using mk_nil_map s _) (by simp [mk]) as ⟨v, it1, any, hv, hit1, hany⟩
  subst hv
  split
  · rename_i hany_t
    step with K.loop1_spec s m.enclosed n hTPn (fun c hc => by rw [hn]; have := henclt c hc; omega) it0 0#usize
      (by simp) (by rw [hit0]; simpa using mk_nil_map s _) as ⟨ie, hie⟩
    step with new_spec (alloc.vec.Vec.len s.topes) (by simp; omega) as ⟨uf0, huf0⟩
    have hinv0 : UInv (kE (tpP s) (tpM s) th 0) s.topes.val.length uf0 := by
      unfold UInv; rw [huf0]; simpa [kE] using inv_range s.topes.val.length
    step with K.loop2_spec s hSW th it1 hit1 uf0 0#usize (by simp) hinv0 as ⟨uf1, huf1⟩
    set R := ufRoot (vals uf1.parent) with hR_def
    have hLeast : Least (kE (tpP s) (tpM s) th nf) s.topes.val.length R := huf1.2.2
    have hR : ∀ y, y < s.topes.val.length → R y < s.topes.val.length := fun y hy => (hLeast y hy).1
    have hRoots1 : Roots s.topes.val.length R uf1 := ⟨huf1.1, uinv_len huf1, fun _ => rfl⟩
    step with alloc.vec.from_elem_spec (core.clone.CloneallocvecVec core.clone.CloneU32) (alloc.vec.Vec.new U32)
      (alloc.vec.Vec.len s.topes) clone_new_ok as ⟨rim0, hrim0, hrim0l⟩
    step as ⟨ae0, hae0, hae0l⟩
    have hntv : (alloc.vec.Vec.len s.topes).val = s.topes.val.length := by simp
    step with K.loop3_spec (alloc.vec.Vec.len s.topes) th en it1 ie (by rw [hntv]; exact hit1) (by rw [hntv]; exact hie)
      R (by rw [hntv]; exact hR) uf1 (by rw [hntv]; exact hRoots1) ae0 0#usize (by simp)
      (by rw [hae0, hntv]; apply List.ext_getElem (by simp); intro i _ _; simp [aeOf]) as ⟨uf2, ae1, hRoots2, hae1⟩
    rw [hntv] at hRoots2 hae1
    have hRim0 : RimInv s th R s.topes.val.length 0 rim0 := by
      refine ⟨by rw [hrim0]; simp, fun r hr => ⟨?_, fun u => ?_⟩⟩
      · rw [hrim0, List.getD_eq_getElem?_getD, List.getElem?_replicate]; simp [hr, hntv]
      · unfold rimAt; rw [hrim0, List.getD_eq_getElem?_getD, List.getElem?_replicate]; simp [hr, hntv]
    step with K.loop4_spec s hSW th it1 hit1 R hR uf2 hRoots2 rim0 0#usize (by simp) hRim0 as ⟨uf3, rim1, hRoots3, hRim1⟩
    have hrim1l := hRim1.1
    step with K.loop5_spec (alloc.vec.Vec.len s.topes) rim1 nf (by omega)
      (fun r hr => by rw [hntv] at hr; exact (hRim1.2 r hr).1) rim1 0#usize (by simp)
      ⟨by rw [hntv]; exact hrim1l, fun r hr => ⟨fun h => by simp at h, fun _ => rfl⟩⟩ as ⟨rim2, hSort⟩
    rw [hntv] at hSort
    obtain ⟨hrim2l, hSort2⟩ := hSort
    -- the rims' members, as hits
    have hmem2 : ∀ r, r < s.topes.val.length → ∀ u, u ∈ rimAt rim2 r ↔
        ∃ k, k < nf ∧ rimHit s th R k r u := by
      intro r hr u
      rw [((hSort2 r hr).1 hr).2 u]
      exact (hRim1.2 r hr).2 u
    have hrb : ∀ r, r < s.topes.val.length → ∀ x ∈ (rim2.val.getD r default).val, x.val < s.topes.val.length := by
      intro r hr x hx
      have : x.val ∈ rimAt rim2 r := List.mem_map.mpr ⟨x, hx, rfl⟩
      obtain ⟨k, hk, hh⟩ := (hmem2 r hr x.val).mp this
      rcases hh with ⟨_, _, _, e⟩ | ⟨_, _, _, e⟩
      · rw [← e]; exact hSW.qq_lt k hk
      · rw [← e]; exact hSW.pp_lt k hk
    have hTabs : Tabs (toL l) nf (tpP s) (tpM s) (thinL l m) (enclosedL l m) keep.val th en R (rimAt rim2)
        (aeOf th en R s.topes.val.length) := by
      refine ⟨hS, hthA, henA, by rw [hkl, hTl], by rw [hTl]; exact hLeast, fun t ht htt => ?_,
        fun t ht htt u => ?_, fun t ht htt => ?_⟩
      · rw [hTl] at ht; exact ((hSort2 _ (hR t ht)).1 (hR t ht)).1
      · rw [hTl] at ht ⊢
        rw [hmem2 _ (hR t ht) u]
        unfold rimHit rimP
        constructor
        · rintro ⟨k, hk, ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩⟩
          · refine ⟨by rw [← h4]; exact hSW.qq_lt k hk, by rw [← h4]; exact h2, k, hk, Or.inl ⟨h1, ?_, h4⟩⟩
            exact (least_iff hLeast ht (hSW.pp_lt k hk)).mpr h3.symm
          · refine ⟨by rw [← h4]; exact hSW.pp_lt k hk, by rw [← h4]; exact h2, k, hk, Or.inr ⟨h1, ?_, h4⟩⟩
            exact (least_iff hLeast ht (hSW.qq_lt k hk)).mpr h3.symm
        · rintro ⟨_, hthu, k, hk, ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩⟩
          · refine ⟨k, hk, Or.inl ⟨h1, by rw [h3]; exact hthu, ?_, h3⟩⟩
            exact ((least_iff hLeast ht (hSW.pp_lt k hk)).mp h2).symm
          · refine ⟨k, hk, Or.inr ⟨h1, by rw [h3]; exact hthu, ?_, h3⟩⟩
            exact ((least_iff hLeast ht (hSW.qq_lt k hk)).mp h2).symm
      · rw [aeOf_iff, hTl]
    have hsm : ∀ t, t < (alloc.vec.Vec.len s.topes).val → at32 s.topes t < U32.max := by
      intro t ht; rw [hntv] at ht; have := (hTP.pos t ht).1; omega
    step with K.loop6_spec s keep (alloc.vec.Vec.len s.topes) hntv (by rw [hntv]; exact hkl) hsm th it1
      (by rw [hntv]; exact hit1) R (by rw [hntv]; exact hR) rim2 (by rw [hntv]; exact hrim2l)
      (by rw [hntv]; exact hrb) (aeOf th en R s.topes.val.length) ae1 (by rw [hntv]; exact hae1) NONE uf3 0#usize
      (by simp) (by rw [hntv]; exact hRoots3) rfl (fun t ht => by simp at ht) as ⟨bd, uf4, hRoots4, hbad⟩
    rw [hntv] at hRoots4 hbad
    have hceq := collapse_eq hTabs
    rw [hTl] at hceq
    unfold ktOf
    rw [hceq]
    split
    · rename_i hbn
      rcases hbad with ⟨_, hnone⟩ | ⟨hne, _⟩
      · step as ⟨old, hold⟩
        have holdv : old.val = keep.val := by rw [hold]; rfl
        step with K.loop7_spec (alloc.vec.Vec.len s.topes) th it1 (by rw [hntv]; exact hit1) R
          (by rw [hntv]; exact hR) rim2 (by rw [hntv]; exact hrim2l) (by rw [hntv]; exact hrb) old
          (by rw [hntv, holdv, hkl]) keep uf4 0#usize (by simp) (by rw [hntv]; exact hRoots4)
          (by rw [hntv, holdv]; apply List.ext_getElem (by simp [hkl]); intro i h1 h2
              simp [fillAt, kAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h1])
          as ⟨k1, hk1⟩
        rw [hntv] at hk1
        unfold bad at hnone
        rw [hnone]
        simp only
        refine ⟨hbn, by rw [hk1]; simp, ?_⟩
        rw [hk1, holdv]
        congr 1
        apply List.map_congr_left
        intro u hu
        have hu' := List.mem_range.mp hu
        simp only [fillAt, decide_eq_true hu', Bool.true_and]
        rfl
      · exact absurd hbn hne
    · rename_i hbn
      rcases hbad with ⟨hb, _⟩ | ⟨_, t0, hsome, hbv⟩
      · exact absurd hb hbn
      · unfold bad at hsome
        rw [hsome]
        simp only [WP.spec_ok]
        have ht0 : t0 < s.topes.val.length := List.mem_range.mp (List.mem_of_find?_eq_some hsome)
        refine ⟨hbn, by rw [hbv]; have := (hTP.pos t0 ht0).1; omega, ?_⟩
        rw [hbv, at_eq (by rw [hTl]; exact ht0), topes_at hA ht0]
  · rename_i hany_f
    have hthf : ∀ t, t < s.topes.val.length → th t = false := by
      intro t ht
      cases h : th t
      · rfl
      · exact absurd (hany.mpr ⟨t, ht, h⟩) hany_f
    have hTabs : Tabs (toL l) nf (tpP s) (tpM s) (thinL l m) (enclosedL l m) keep.val th en id (fun _ => [])
        (fun _ => true) := by
      have hkE : kE (tpP s) (tpM s) th nf = [] := kE_nil _ _ th nf (fun k hk => hthf _ (hSW.pp_lt k hk))
      refine ⟨hS, hthA, henA, by rw [hkl, hTl], by rw [hkE]; exact least_nil _, fun t ht htt => ?_,
        fun t ht htt => ?_, fun t ht htt => ?_⟩
      all_goals (rw [hTl] at ht; rw [hthf t ht] at htt; cases htt)
    have hceq := collapse_eq hTabs
    rw [hTl] at hceq
    unfold ktOf
    rw [hceq]
    have hfn : (List.range s.topes.val.length).find?
        (fun t => th t && !(uniR keep.val ((fun _ => ([] : List Nat)) (id t)) t && (fun _ => true) (id t))) = none := by
      apply List.find?_eq_none.mpr
      intro x hx; rw [hthf x (List.mem_range.mp hx)]; simp
    rw [hfn]
    simp only [WP.spec_ok]
    refine ⟨trivial, hkl, ?_⟩
    congr 1
    apply List.ext_getElem (by simp [hkl])
    intro i h1 h2
    simp only [List.getElem_map, List.getElem_range]
    rw [hthf i (by rw [← hkl]; exact h1)]
    simp [kAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h1]

end forge_bool_core
