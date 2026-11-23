import ForgeBoolCore.HeadBridgeI

/-!
# All related pairs of positions = the Lean's classes

`inc::fans` unites positions `p < q` of a list whose members are related. For a symmetric relation
on a list without repeats, the classes of those pairs (`Eqv`) are the classes of the relation over
the list (`Conn`). So a union-find naming each class by its least position decides `badVert`'s
test: some member outside the first one's class.
-/

namespace ForgeBoolCore.UF

open ForgeExact.DecideOnce

universe u

/-- The pairs `fans` unites over the first `p` rows, then row `p` up to `q`. -/
def fanE (rel : Nat → Nat → Bool) (n p q : Nat) : List (Nat × Nat) :=
  (List.range p).flatMap (fun p' => ((List.range' (p' + 1) (n - (p' + 1))).filter (rel p')).map (p', ·)) ++
    ((List.range' (p + 1) (q - (p + 1))).filter (rel p)).map (p, ·)

/-- All of them. -/
def fanAll (rel : Nat → Nat → Bool) (n : Nat) : List (Nat × Nat) := fanE rel n n (n + 1)

theorem mem_fanAll {rel : Nat → Nat → Bool} {n x y : Nat} :
    (x, y) ∈ fanAll rel n ↔ x < y ∧ y < n ∧ rel x y = true := by
  unfold fanAll fanE
  simp only [List.mem_append, List.mem_flatMap, List.mem_map, List.mem_filter, List.mem_range'_1,
    List.mem_range, Prod.mk.injEq]
  constructor
  · rintro (⟨p', hp', q', ⟨⟨h1, h2⟩, h3⟩, rfl, rfl⟩ | ⟨q', ⟨⟨h1, h2⟩, h3⟩, rfl, rfl⟩)
    · exact ⟨by omega, by omega, h3⟩
    · omega
  · rintro ⟨h1, h2, h3⟩
    exact Or.inl ⟨x, by omega, y, ⟨⟨by omega, by omega⟩, h3⟩, rfl, rfl⟩

variable {α : Type u} [DecidableEq α]

theorem fan_eqv_conn {r : α → α → Bool} (hr : ∀ a b, r a b = r b a) {L : List α}
    {rel : Nat → Nat → Bool} (hrel : ∀ x y (hx : x < L.length) (hy : y < L.length), rel x y = r L[x] L[y])
    {y z : Nat} (h : Eqv (fanAll rel L.length) y z) (hy : y < L.length) (hz : z < L.length) :
    Conn r L L[y] L[z] := by
  have hin : ∀ e ∈ fanAll rel L.length, e.1 < L.length ∧ e.2 < L.length := by
    rintro ⟨a, b⟩ he; have := mem_fanAll.mp he; exact ⟨by omega, this.2.1⟩
  induction h with
  | refl => exact .refl
  | edge he =>
    rename_i e
    obtain ⟨a, b⟩ := e
    have := mem_fanAll.mp he
    refine Conn.step .refl (List.getElem_mem _) ?_
    rw [← hrel a b hy hz]; exact this.2.2
  | symm h' ih => exact Conn.symm hr (List.getElem_mem _) (ih hz hy)
  | trans h1 _ ih1 ih2 =>
    have hm := (Eqv.range hin h1).elim (fun e => e ▸ hy) (fun e => e.2)
    exact (ih1 hy hm).trans (ih2 hm hz)

theorem fan_conn_eqv {r : α → α → Bool} (hr : ∀ a b, r a b = r b a) {L : List α} (hnd : L.Nodup)
    {rel : Nat → Nat → Bool} (hrel : ∀ x y (hx : x < L.length) (hy : y < L.length), rel x y = r L[x] L[y])
    {y : Nat} (hy : y < L.length) {b : α} (h : Conn r L L[y] b) :
    Eqv (fanAll rel L.length) y (L.idxOf b) := by
  induction h with
  | refl => rw [List.Nodup.idxOf_getElem hnd]; exact .refl _
  | step hab hc hbc ih =>
    rename_i b c
    have hb := Conn.mem (List.getElem_mem hy) hab
    have hi := List.idxOf_lt_length_iff.mpr hb
    have hj := List.idxOf_lt_length_iff.mpr hc
    have gb : L[L.idxOf b] = b := List.getElem_idxOf hi
    have gc : L[L.idxOf c] = c := List.getElem_idxOf hj
    rcases Nat.lt_trichotomy (L.idxOf b) (L.idxOf c) with hlt | heq | hgt
    · have : (L.idxOf b, L.idxOf c) ∈ fanAll rel L.length :=
        mem_fanAll.mpr ⟨hlt, hj, by rw [hrel _ _ hi hj, gb, gc]; exact hbc⟩
      exact ih.trans (Eqv.edge this)
    · rw [← heq]; exact ih
    · have : (L.idxOf c, L.idxOf b) ∈ fanAll rel L.length :=
        mem_fanAll.mpr ⟨hgt, hi, by rw [hrel _ _ hj hi, gb, gc, hr]; exact hbc⟩
      exact ih.trans (Eqv.edge this).symm

/-- **`badVert`'s test is "some root differs from position 0's".** -/
theorem fan_bad {r : α → α → Bool} (hr : ∀ a b, r a b = r b a) {L : List α} (hnd : L.Nodup)
    {rel : Nat → Nat → Bool} (hrel : ∀ x y (hx : x < L.length) (hy : y < L.length), rel x y = r L[x] L[y])
    {R : Nat → Nat} (hl : Least (fanAll rel L.length) L.length R) :
    (match L with
      | [] => false
      | c :: _ => L.any fun d => !(cls r L c).contains d) =
      (List.range L.length).any fun p => R p != R 0 := by
  cases hL : L with
  | nil => simp
  | cons c t =>
    simp only
    rw [← hL]
    have h0 : 0 < L.length := by rw [hL]; simp
    have hc : c = L[0] := by simp [hL]
    rw [hc, Bool.eq_iff_iff, List.any_eq_true, List.any_eq_true]
    constructor
    · rintro ⟨d, hd, hnc⟩
      have hi := List.idxOf_lt_length_iff.mpr hd
      refine ⟨L.idxOf d, List.mem_range.mpr hi, ?_⟩
      simp only [bne_iff_ne, ne_eq]
      intro he
      have hq := (least_iff hl hi h0).mpr he
      have := fan_eqv_conn hr hrel hq.symm h0 hi
      rw [List.getElem_idxOf hi] at this
      simp only [Bool.not_eq_true', Bool.eq_false_iff, ne_eq] at hnc
      apply hnc
      rw [@List.contains_iff_mem _ instBEqOfDecidableEq, cls_spec _ hnd (List.getElem_mem h0)]
      exact this
    · rintro ⟨p, hp, hne⟩
      have hpl := List.mem_range.mp hp
      refine ⟨L[p], List.getElem_mem hpl, ?_⟩
      simp only [Bool.not_eq_true', Bool.eq_false_iff, ne_eq]
      intro hcon
      rw [@List.contains_iff_mem _ instBEqOfDecidableEq, cls_spec _ hnd (List.getElem_mem h0)] at hcon
      have := fan_conn_eqv hr hnd hrel h0 hcon
      rw [List.Nodup.idxOf_getElem hnd] at this
      have := (least_iff hl h0 hpl).mp this
      simp only [bne_iff_ne, ne_eq] at hne
      exact hne this.symm

end ForgeBoolCore.UF
