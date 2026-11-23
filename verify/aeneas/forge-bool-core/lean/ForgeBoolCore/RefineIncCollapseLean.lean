import ForgeBoolCore.RefineIncCore
import ForgeBoolCore.RefineCollapse

/-!
# Group M: the Lean collapse over the face ledger, by positions

RefineCollapse.lean's Part 1, ported from the plane ledger's topes to an incidence ledger's rooms.
Over any `IncLedger FCell` whose facets' sides sit at positions `pp k`, `qq k` of its rooms, and any
keep table that reads a keep vector by position, the Lean `collapse` is a function of the position
tables (`collapse_pos`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

namespace KI

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

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
structure Sides (L : IncLedger FCell) (nf : Nat) (pp qq : Nat → Nat) : Prop where
  len : L.facets.length = nf
  pp_lt : ∀ k, k < nf → pp k < L.rooms.length
  qq_lt : ∀ k, k < nf → qq k < L.rooms.length
  pp_eq : ∀ k (hk : k < nf), L.rooms[pp k]'(pp_lt k hk) = L.plus (L.facets[k]'(by omega))
  qq_eq : ∀ k (hk : k < nf), L.rooms[qq k]'(qq_lt k hk) = L.minus (L.facets[k]'(by omega))

variable {L : IncLedger FCell} {nf : Nat} {pp qq : Nat → Nat}

theorem topes_inj {a b : Nat} (ha : a < L.rooms.length) (hb : b < L.rooms.length)
    (h : L.rooms[a] = L.rooms[b]) : a = b :=
  (List.Nodup.getElem_inj_iff (nodup_dedup _)).mp h

/-- Adjacent across a facet, by positions. -/
theorem tAdj_iff (hS : Sides L nf pp qq) {a b : Nat} (ha : a < L.rooms.length) (hb : b < L.rooms.length) :
    L.tAdj L.rooms[a] L.rooms[b] = true ↔
      ∃ k, k < nf ∧ ((pp k = a ∧ qq k = b) ∨ (pp k = b ∧ qq k = a)) := by
  unfold IncLedger.tAdj
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
def ThinAt (L : IncLedger FCell) (thin : List FCell) (th : Nat → Bool) : Prop :=
  ∀ t (ht : t < L.rooms.length), th t = thin.contains L.rooms[t]

theorem mem_thinTopes_iff {thin : List FCell} {th : Nat → Bool} (hth : ThinAt L thin th) {V : FCell} :
    V ∈ L.thinTopes thin ↔ ∃ u, ∃ (hu : u < L.rooms.length), V = L.rooms[u] ∧ th u = true := by
  unfold IncLedger.thinTopes
  rw [List.mem_filter]
  constructor
  · rintro ⟨hV, hc⟩
    obtain ⟨u, hu, rfl⟩ := List.getElem_of_mem hV
    exact ⟨u, hu, rfl, by rw [hth u hu]; exact hc⟩
  · rintro ⟨u, hu, rfl, h⟩
    exact ⟨List.getElem_mem _, by rw [← hth u hu]; exact h⟩

theorem eqv_conn (hS : Sides L nf pp qq) {thin : List FCell} {th : Nat → Bool} (hth : ThinAt L thin th)
    {y z : Nat} (h : Eqv (kE pp qq th nf) y z) :
    y = z ∨ ∃ (hy : y < L.rooms.length) (hz : z < L.rooms.length), th y = true ∧ th z = true ∧
      Conn L.tAdj (L.thinTopes thin) L.rooms[y] L.rooms[z] := by
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
      exact ⟨hz, hy, h2, h1, Conn.symm (IncLedger.tAdj_symm L) ((mem_thinTopes_iff hth).mpr ⟨_, hy, rfl, h1⟩) hc⟩
  | trans _ _ ih1 ih2 =>
    rcases ih1 with h1 | ⟨ha, hb, ta, tb, c1⟩
    · subst h1; exact ih2
    · rcases ih2 with h2 | ⟨hb', hc, tb', tc, c2⟩
      · subst h2; exact Or.inr ⟨ha, hb, ta, tb, c1⟩
      · exact Or.inr ⟨ha, hc, ta, tc, c1.trans c2⟩

/-- **The component, by positions.** -/
theorem mem_comp_iff (hS : Sides L nf pp qq) {thin : List FCell} {th : Nat → Bool} (hth : ThinAt L thin th)
    {t : Nat} (ht : t < L.rooms.length) (htt : th t = true) (V : FCell) :
    V ∈ L.comp thin L.rooms[t] ↔
      ∃ u, ∃ (hu : u < L.rooms.length), V = L.rooms[u] ∧ th u = true ∧ Eqv (kE pp qq th nf) t u := by
  have hT : L.rooms[t] ∈ L.thinTopes thin := (mem_thinTopes_iff hth).mpr ⟨t, ht, rfl, htt⟩
  unfold IncLedger.comp
  rw [cls_spec _ (IncLedger.thinTopes_nodup L thin) hT]
  constructor
  · intro hc
    refine Conn.invariant (P := fun V => ∃ u, ∃ (hu : u < L.rooms.length), V = L.rooms[u] ∧ th u = true ∧
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

def at_ (xs : List FCell) (u : Nat) : FCell := xs.getD u (FCell.room 0)

theorem at_eq {xs : List FCell} {u : Nat} (h : u < xs.length) : at_ xs u = xs[u] := by
  simp [at_, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

theorem map_range (xs : List FCell) : (List.range xs.length).map (at_ xs) = xs := by
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_range]
  exact at_eq h2

theorem map_eq_range {β : Type} (xs : List FCell) (F : FCell → β) :
    xs.map F = (List.range xs.length).map (fun u => F (at_ xs u)) := by
  conv => lhs; rw [← map_range xs]
  rw [List.map_map]; rfl

theorem filter_range (xs : List FCell) (f : FCell → Bool) :
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
structure Tabs (L : IncLedger FCell) (nf : Nat) (pp qq : Nat → Nat) (thin enc : List FCell)
    (kt : KeepTabI FCell) (keep : List Bool) (th en : Nat → Bool) (R : Nat → Nat) (rimL : Nat → List Nat) (aenc : Nat → Bool) : Prop where
  sides : Sides L nf pp qq
  hth : ThinAt L thin th
  hen : ThinAt L enc en
  hkt : ∀ u (hu : u < L.rooms.length), keepInI kt L.rooms[u] = kAt keep u
  least : Least (kE pp qq th nf) L.rooms.length R
  rim_sorted : ∀ t, t < L.rooms.length → th t = true → (rimL (R t)).Pairwise (· < ·)
  rim_mem : ∀ t, t < L.rooms.length → th t = true →
    ∀ u, u ∈ rimL (R t) ↔ u < L.rooms.length ∧ rimP pp qq th nf t u
  haenc : ∀ t, t < L.rooms.length → th t = true →
    (aenc (R t) = true ↔ ∀ u, u < L.rooms.length → th u = true → R u = R t → en u = true)

variable {thin enc : List FCell} {kt : KeepTabI FCell} {keep : List Bool} {th en : Nat → Bool} {R : Nat → Nat}
  {rimL : Nat → List Nat} {aenc : Nat → Bool}

theorem rim_eq (h : Tabs L nf pp qq thin enc kt keep th en R rimL aenc) {t : Nat} (ht : t < L.rooms.length)
    (htt : th t = true) : L.rim thin L.rooms[t] = (rimL (R t)).map (at_ L.rooms) := by
  unfold IncLedger.rim
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
    · refine ⟨L.rooms[pp k]'(h.sides.pp_lt k hk), (mem_comp_iff h.sides h.hth ht htt _).mpr
        ⟨pp k, h.sides.pp_lt k hk, rfl, h1, h2⟩, ?_⟩
      exact (tAdj_iff h.sides _ hu).mpr ⟨k, hk, Or.inl ⟨rfl, h3⟩⟩
    · refine ⟨L.rooms[qq k]'(h.sides.qq_lt k hk), (mem_comp_iff h.sides h.hth ht htt _).mpr
        ⟨qq k, h.sides.qq_lt k hk, rfl, h1, h2⟩, ?_⟩
      exact (tAdj_iff h.sides _ hu).mpr ⟨k, hk, Or.inr ⟨h3, rfl⟩⟩
  · rintro ⟨V, hV, hadj⟩
    obtain ⟨v, hv, rfl, hthv, he⟩ := (mem_comp_iff h.sides h.hth ht htt _).mp hV
    obtain ⟨k, hk, ⟨e1, e2⟩ | ⟨e1, e2⟩⟩ := (tAdj_iff h.sides hv hu).mp hadj
    · exact ⟨k, hk, Or.inl ⟨by rw [e1]; exact hthv, by rw [e1]; exact he, e2⟩⟩
    · exact ⟨k, hk, Or.inr ⟨by rw [e2]; exact hthv, by rw [e2]; exact he, e1⟩⟩

theorem rim_lt (h : Tabs L nf pp qq thin enc kt keep th en R rimL aenc) {t : Nat} (ht : t < L.rooms.length)
    (htt : th t = true) {u : Nat} (hu : u ∈ rimL (R t)) : u < L.rooms.length :=
  ((h.rim_mem t ht htt u).mp hu).1

theorem keepIn_at (h : Tabs L nf pp qq thin enc kt keep th en R rimL aenc) {u : Nat} (hu : u < L.rooms.length) :
    keepInI kt (at_ L.rooms u) = kAt keep u := by
  rw [at_eq hu]; exact h.hkt u hu

theorem fill_eq (h : Tabs L nf pp qq thin enc kt keep th en R rimL aenc) {t : Nat} (ht : t < L.rooms.length)
    (htt : th t = true) : L.fill kt thin L.rooms[t] = firstK keep (rimL (R t)) t := by
  unfold IncLedger.fill
  rw [rim_eq h ht htt]
  have hlt := fun u => rim_lt h ht htt (u := u)
  cases hr : rimL (R t) with
  | nil =>
    simp only [List.map_nil, firstK]
    rw [← at_eq ht, keepIn_at h ht]
  | cons u rest =>
    simp only [List.map_cons, firstK]
    exact keepIn_at h (hlt u (by rw [hr]; exact List.mem_cons_self ..))

theorem coll_eq (h : Tabs L nf pp qq thin enc kt keep th en R rimL aenc) {t : Nat} (ht : t < L.rooms.length)
    (htt : th t = true) :
    L.collapsible kt thin enc L.rooms[t] = (uniR keep (rimL (R t)) t && aenc (R t)) := by
  unfold IncLedger.collapsible
  congr 1
  · unfold IncLedger.uniform uniR
    rw [rim_eq h ht htt, List.all_map]
    have hlt := fun u => rim_lt h ht htt (u := u)
    have hhd : keepInI kt (((rimL (R t)).map (at_ L.rooms)).headD L.rooms[t]) =
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

/-- The refusal test, by position. -/
def badP (keep : List Bool) (th : Nat → Bool) (R : Nat → Nat) (rimL : Nat → List Nat) (aenc : Nat → Bool)
    (t : Nat) : Bool :=
  th t && !(uniR keep (rimL (R t)) t && aenc (R t))

/-- **The Lean collapse, by positions**: refused at the first thin position whose component is not
collapsible; otherwise the new table reads, at each position, `firstK` if thin and the old keep if
not. -/
theorem collapse_pos (h : Tabs L nf pp qq thin enc kt keep th en R rimL aenc) :
    match L.collapse kt thin enc with
    | .refuse T => ∃ t, (List.range L.rooms.length).find? (badP keep th R rimL aenc) = some t ∧
        T = at_ L.rooms t
    | .ok kt' => (List.range L.rooms.length).find? (badP keep th R rimL aenc) = none ∧
        ∀ u (hu : u < L.rooms.length),
          keepInI kt' L.rooms[u] = (if th u then firstK keep (rimL (R u)) u else kAt keep u) := by
  have hfind : (L.thinTopes thin).find? (fun T => !L.collapsible kt thin enc T) =
      ((List.range L.rooms.length).find? (badP keep th R rimL aenc)).map (at_ L.rooms) := by
    unfold IncLedger.thinTopes
    rw [filter_range, List.find?_map, List.find?_filter]
    congr 1
    apply find?_congr
    intro u hu
    have hu' := List.mem_range.mp hu
    simp only [Function.comp, at_eq hu', ← h.hth u hu', badP]
    cases hthu : th u
    · simp
    · simp [coll_eq h hu' hthu]
  unfold IncLedger.collapse
  rw [hfind]
  cases hf : (List.range L.rooms.length).find? (badP keep th R rimL aenc) with
  | some t => exact ⟨t, rfl, rfl⟩
  | none =>
    refine ⟨rfl, ?_⟩
    intro u hu
    rw [IncLedger.keepIn_map _ (List.getElem_mem hu), ← h.hth u hu]
    cases hthu : th u
    · simp only [Bool.false_eq_true, ite_false]
      rw [← at_eq hu, keepIn_at h hu]
    · simp only [ite_true]
      rw [fill_eq h hu hthu]

end KI

end inc

end forge_bool_core
