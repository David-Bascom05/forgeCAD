/-!
# A bottom-up merge sort: the list facts

Generic over a transitive relation `R`. `seg l a b` is the slice `l[a..b)`; `Blocks R w l` says every
aligned block of width `w` is sorted. These are the invariants of `sort_by_cov`'s three loops.
-/

namespace ForgeBoolCore.Sort

variable {α : Type} (R : α → α → Prop)

def seg (l : List α) (a b : Nat) : List α := (l.drop a).take (b - a)

/-- Every aligned block of width `w` is sorted. -/
def Blocks (w : Nat) (l : List α) : Prop := ∀ b, (seg l (b * w) ((b + 1) * w)).Pairwise R

theorem seg_length (l : List α) {a b : Nat} (hab : a ≤ b) (hb : b ≤ l.length) :
    (seg l a b).length = b - a := by
  simp [seg]; omega

theorem seg_succ (l : List α) {a b : Nat} (hab : a ≤ b) (hb : b < l.length) :
    seg l a (b + 1) = seg l a b ++ [l[b]] := by
  unfold seg
  rw [show b + 1 - a = (b - a) + 1 by omega, List.take_succ]
  congr
  rw [List.getElem?_drop, show a + (b - a) = b by omega, List.getElem?_eq_getElem hb]; rfl

theorem seg_self (l : List α) (a : Nat) : seg l a a = [] := by simp [seg]

theorem seg_append (l : List α) {a b c : Nat} (hab : a ≤ b) (hbc : b ≤ c) :
    seg l a c = seg l a b ++ seg l b c := by
  unfold seg
  rw [show c - a = (b - a) + (c - b) by omega, List.take_add, List.drop_drop,
    show a + (b - a) = b by omega]

theorem take_eq_seg (l : List α) (b : Nat) : l.take b = seg l 0 b := by simp [seg]

/-- Writing at the end of the output region. -/
theorem set_mid (pre out post : List α) (x : α) (k : Nat) (hk : k = pre.length + out.length)
    (hpost : post ≠ []) :
    (pre ++ out ++ post).set k x = pre ++ (out ++ [x]) ++ post.tail := by
  subst hk
  cases post with
  | nil => exact absurd rfl hpost
  | cons y ys =>
    rw [List.append_assoc, List.set_append_right _ _ (by omega), Nat.add_sub_cancel_left,
      List.set_append_right _ _ (by omega), Nat.sub_self]
    simp

theorem drop_succ_eq_tail (l : List α) (k : Nat) : l.drop (k + 1) = (l.drop k).tail := by
  rw [List.tail_drop]

variable {R}

theorem pairwise_snoc {l : List α} {x : α} (h : l.Pairwise R) (hx : ∀ y ∈ l, R y x) :
    (l ++ [x]).Pairwise R := by
  rw [List.pairwise_append]
  exact ⟨h, List.pairwise_singleton _ _, fun a ha b hb => by simp at hb; subst hb; exact hx a ha⟩

/-- A sorted list's later element is after every earlier one. -/
theorem pairwise_get {l : List α} (h : l.Pairwise R) {p q : Nat} (hpq : p < q) (hq : q < l.length) :
    R (l[p]'(by omega)) l[q] := List.pairwise_iff_getElem.mp h p q (by omega) hq hpq

/-! ## One merge -/

/-- The merge of `src[lo..mid)` and `src[mid..hi)` into `dst`, part done. -/
def MInv (R : α → α → Prop) (src D0 dst : List α) (lo mid hi i j k : Nat) : Prop :=
  lo ≤ i ∧ i ≤ mid ∧ mid ≤ j ∧ j ≤ hi ∧ hi ≤ src.length ∧ D0.length = src.length ∧
  k = lo + (i - lo) + (j - mid) ∧
  ∃ out, dst = D0.take lo ++ out ++ D0.drop k ∧ out.Perm (seg src lo i ++ seg src mid j) ∧
    out.Pairwise R ∧
    ∀ x ∈ out, (i < mid → ∀ y, src[i]? = some y → R x y) ∧ (j < hi → ∀ y, src[j]? = some y → R x y)

theorem minv_start (src D0 : List α) {lo mid hi : Nat} (h1 : lo ≤ mid) (h2 : mid ≤ hi)
    (h3 : hi ≤ src.length) (hD : D0.length = src.length) :
    MInv R src D0 D0 lo mid hi lo mid lo :=
  ⟨Nat.le_refl _, h1, Nat.le_refl _, h2, h3, hD, by omega, [], by simp, by simp [seg_self],
    List.Pairwise.nil, fun _ h => by simp at h⟩

theorem seg_get {src : List α} {a b p : Nat} (hp : a ≤ p) (hpb : p < b) (hb : b ≤ src.length) :
    (seg src a b)[p - a]? = src[p]? := by
  simp [seg, List.getElem?_take, show p - a < b - a by omega, show a + (p - a) = p by omega]

/-- Consecutive elements of a sorted run. -/
theorem run_step {src : List α} {a b p : Nat} (hs : (seg src a b).Pairwise R) (hp : a ≤ p)
    (hpb : p + 1 < b) (hb : b ≤ src.length) {x y : α} (hx : src[p]? = some x)
    (hy : src[p + 1]? = some y) : R x y := by
  rw [List.pairwise_iff_getElem] at hs
  have hl := seg_length src (show a ≤ b by omega) hb
  have h1 := seg_get (src := src) (b := b) hp (by omega) hb
  have h2 := seg_get (src := src) (a := a) (b := b) (p := p + 1) (by omega) hpb hb
  rw [hx] at h1; rw [hy] at h2
  have := hs (p - a) (p + 1 - a) (by rw [hl]; omega) (by rw [hl]; omega) (by omega)
  rw [List.getElem?_eq_some_iff] at h1 h2
  obtain ⟨_, e1⟩ := h1; obtain ⟨_, e2⟩ := h2
  rw [e1, e2] at this; exact this

section Merge
variable (htr : ∀ a b c, R a b → R b c → R a c)
include htr

theorem minv_left {src D0 dst : List α} {lo mid hi i j k : Nat} (hm : MInv R src D0 dst lo mid hi i j k)
    (hsl : (seg src lo mid).Pairwise R) (hilt : i < mid) {y : α} (hy : src[i]? = some y)
    (htake : j < hi → ∀ z, src[j]? = some z → R y z) :
    MInv R src D0 (dst.set k y) lo mid hi (i + 1) j (k + 1) := by
  obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk, out, heq, hperm, hsort, hbnd⟩ := hm
  have hil : i < src.length := by omega
  have hyv : src[i] = y := by rw [List.getElem?_eq_getElem hil] at hy; exact Option.some.inj hy
  have hol : out.length = k - lo := by
    have := hperm.length_eq
    rw [List.length_append, seg_length _ hlo (by omega), seg_length _ hmj (by omega)] at this
    omega
  refine ⟨by omega, hilt, hmj, hjh, hhn, hD0, by omega, out ++ [y], ?_, ?_, ?_, ?_⟩
  · rw [heq, set_mid _ _ _ _ _ (by simp; omega) (by simp; omega), ← drop_succ_eq_tail]
  · rw [seg_succ _ hlo hil, hyv]
    refine (hperm.append_right [y]).trans ?_
    simp only [List.append_assoc]
    exact (List.perm_append_comm.append_left _).trans (by simp)
  · exact pairwise_snoc hsort fun z hz => (hbnd z hz).1 hilt y hy
  · intro x hx
    rcases List.mem_append.mp hx with hx | hx
    · refine ⟨fun h1 z hz => htr _ _ _ ((hbnd x hx).1 hilt y hy) (run_step hsl hlo h1 (by omega) hy hz),
        fun h2 => (hbnd x hx).2 h2⟩
    · simp at hx; subst hx
      exact ⟨fun h1 z hz => run_step hsl hlo h1 (by omega) hy hz, htake⟩

theorem minv_right {src D0 dst : List α} {lo mid hi i j k : Nat} (hm : MInv R src D0 dst lo mid hi i j k)
    (hsr : (seg src mid hi).Pairwise R) (hjlt : j < hi) {y : α} (hy : src[j]? = some y)
    (htake : i < mid → ∀ z, src[i]? = some z → R y z) :
    MInv R src D0 (dst.set k y) lo mid hi i (j + 1) (k + 1) := by
  obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk, out, heq, hperm, hsort, hbnd⟩ := hm
  have hjl : j < src.length := by omega
  have hyv : src[j] = y := by rw [List.getElem?_eq_getElem hjl] at hy; exact Option.some.inj hy
  have hol : out.length = k - lo := by
    have := hperm.length_eq
    rw [List.length_append, seg_length _ hlo (by omega), seg_length _ hmj (by omega)] at this
    omega
  refine ⟨hlo, him, by omega, by omega, hhn, hD0, by omega, out ++ [y], ?_, ?_, ?_, ?_⟩
  · rw [heq, set_mid _ _ _ _ _ (by simp; omega) (by simp; omega), ← drop_succ_eq_tail]
  · rw [seg_succ _ hmj hjl, hyv]
    refine (hperm.append_right [y]).trans ?_
    simp
  · exact pairwise_snoc hsort fun z hz => (hbnd z hz).2 hjlt y hy
  · intro x hx
    rcases List.mem_append.mp hx with hx | hx
    · exact ⟨(hbnd x hx).1, fun h2 z hz => htr _ _ _ ((hbnd x hx).2 hjlt y hy) (run_step hsr hmj h2 (by omega) hy hz)⟩
    · simp at hx; subst hx
      exact ⟨htake, fun h2 z hz => run_step hsr hmj h2 (by omega) hy hz⟩

omit htr in
theorem minv_done {src D0 dst : List α} {lo mid hi i j k : Nat} (hm : MInv R src D0 dst lo mid hi i j k)
    (hk : hi ≤ k) :
    ∃ out, dst = D0.take lo ++ out ++ D0.drop hi ∧ out.Perm (seg src lo hi) ∧ out.Pairwise R := by
  obtain ⟨hlo, him, hmj, hjh, hhn, hD0, hk', out, heq, hperm, hsort, _⟩ := hm
  have hi' : i = mid := by omega
  have hj' : j = hi := by omega
  subst hi' hj'
  refine ⟨out, by rw [heq, show k = j by omega], ?_, hsort⟩
  rw [seg_append src hlo hmj]; exact hperm

end Merge

/-! ## Blocks -/

theorem seg_len_le (l : List α) (a b : Nat) : (seg l a b).length ≤ b - a := by
  simp [seg]; omega

theorem pairwise_of_len_le_one {l : List α} (h : l.length ≤ 1) : l.Pairwise R := by
  match l, h with
  | [], _ => exact List.Pairwise.nil
  | [_], _ => exact List.pairwise_singleton _ _

theorem blocks_one (l : List α) : Blocks R 1 l := fun b =>
  pairwise_of_len_le_one (by have := seg_len_le l (b * 1) ((b + 1) * 1); simp at this ⊢; omega)

theorem blocks_full {w : Nat} {l : List α} (hb : Blocks R w l) (hl : l.length ≤ w) : l.Pairwise R := by
  have := hb 0
  simp only [Nat.zero_mul, Nat.zero_add, Nat.one_mul, seg, List.drop_zero, Nat.sub_zero] at this
  rwa [List.take_of_length_le hl] at this

theorem seg_append_left (l m : List α) {a b : Nat} (hb : b ≤ l.length) :
    seg (l ++ m) a b = seg l a b := by
  unfold seg
  by_cases hab : a ≤ b
  · rw [List.drop_append_of_le_length (by omega), List.take_append_of_le_length (by simp; omega)]
  · rw [show b - a = 0 by omega]; simp

theorem seg_append_right (l m : List α) {a : Nat} (ha : l.length ≤ a) (b : Nat) :
    seg (l ++ m) a b = seg m (a - l.length) (b - l.length) := by
  unfold seg
  rw [List.drop_append_of_le_length' ha]
  congr 1; omega
where
  List.drop_append_of_le_length' {l m : List α} {a : Nat} (ha : l.length ≤ a) :
      (l ++ m).drop a = m.drop (a - l.length) := by
    rw [List.drop_append]; simp [List.drop_of_length_le ha]

theorem blocks_append {w : Nat} {out m : List α} (hw : 0 < w) (hb : Blocks R w out) (b0 : Nat)
    (hlen : out.length = b0 * w) (hm : m.Pairwise R) (hml : m.length ≤ w) : Blocks R w (out ++ m) := by
  intro b
  by_cases h1 : b < b0
  · rw [seg_append_left _ _ (by rw [hlen]; exact Nat.mul_le_mul_right w h1)]
    exact hb b
  · rw [seg_append_right _ _ (by rw [hlen]; exact Nat.mul_le_mul_right w (by omega))]
    by_cases h2 : b = b0
    · subst h2
      rw [hlen, Nat.sub_self, Nat.succ_mul, show b * w + w - b * w = w by omega]
      simp only [seg, List.drop_zero, Nat.sub_zero]
      rw [List.take_of_length_le hml]; exact hm
    · have hmul : (b0 + 1) * w ≤ b * w := Nat.mul_le_mul_right w (by omega)
      apply pairwise_of_len_le_one
      have hle : m.length ≤ b * w - out.length := by
        rw [hlen]; have e := Nat.add_mul b0 1 w; rw [Nat.one_mul] at e; omega
      have : m.drop (b * w - out.length) = [] := List.drop_of_length_le hle
      simp [seg, this]

/-- A run the merge reads: an aligned block, cut at the end. -/
theorem run_of_blocks {w : Nat} {l : List α} (hb : Blocks R w l) (c : Nat) :
    (seg l (c * w) (min ((c + 1) * w) l.length)).Pairwise R := by
  have := hb c
  have e : seg l (c * w) (min ((c + 1) * w) l.length) = seg l (c * w) ((c + 1) * w) := by
    unfold seg
    by_cases h : (c + 1) * w ≤ l.length
    · rw [Nat.min_eq_left h]
    · rw [Nat.min_eq_right (by omega)]
      rw [List.take_of_length_le (l := List.drop (c * w) l) (by simp; try omega)]
      symm; exact List.take_of_length_le (by simp; omega)
  rw [e]; exact this

end ForgeBoolCore.Sort
