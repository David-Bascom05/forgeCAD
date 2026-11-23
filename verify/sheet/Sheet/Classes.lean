import Mathlib.Data.List.Perm.Basic
import Mathlib.Order.Basic
import Mathlib.Data.List.Nodup

/-!
# Classes by one decided relation, generic (verify/forge-exact/ForgeExact/Snap.lean, ported)

Snap.lean proves the one tolerance rule for planes (`Carrier`, `tolEq`). The same proofs hold for
any type with a symmetric decided relation and names in a linear order; here they are once, so
the 2D vertex classes of region.rs (Sheet/Region.lean) and Snap's planes are both instances.
(forge-exact is core Lean on 4.36.0-rc2 and cannot be a lake dependency of a Mathlib project, so
the proofs are ported rather than imported.)

* `cls`: the transitive closure of `rel` within the list, by rounds (what a union–find over the
  decided pairs computes). `cls_spec`: membership is connection by a chain.
* `rep`: the least-named member of the class. `rep_order`: independent of input order.
  `rep_class`: one representative per class. `rep_unique`: fixed by the class and names alone.
-/

namespace Sheet.Classes

set_option linter.unusedSectionVars false

variable {α N : Type} [DecidableEq α] [LinearOrder N]

/-- Connected by a chain of related members of the list. -/
inductive Conn (rel : α → α → Bool) (cs : List α) (a : α) : α → Prop
  | refl : Conn rel cs a a
  | step {b c : α} : Conn rel cs a b → c ∈ cs → rel b c = true → Conn rel cs a c

/-- One round: everything related to the set so far, not yet in it. -/
def closeStep (rel : α → α → Bool) (cs s : List α) : List α :=
  s ++ cs.filter fun b => !s.contains b && s.any fun a => rel a b

def closeN (rel : α → α → Bool) (cs : List α) : Nat → List α → List α
  | 0, s => s
  | n + 1, s => if (closeStep rel cs s).length = s.length then s else closeN rel cs n (closeStep rel cs s)

/-- The class of `a`. -/
def cls (rel : α → α → Bool) (cs : List α) (a : α) : List α :=
  closeN rel cs (cs.length + 1) [a]

variable {rel : α → α → Bool}

def Closed (rel : α → α → Bool) (cs s : List α) : Prop :=
  ∀ x ∈ s, ∀ c ∈ cs, rel x c = true → c ∈ s

theorem closed_of_fix {cs s : List α} (h : (closeStep rel cs s).length = s.length) :
    Closed rel cs s := by
  intro x hx c hc hxc
  by_contra hcs
  have hnil : (cs.filter fun b => !s.contains b && s.any fun a => rel a b) = [] := by
    simp only [closeStep, List.length_append] at h
    exact List.length_eq_zero_iff.mp (by omega)
  have : c ∈ cs.filter fun b => !s.contains b && s.any fun a => rel a b := by
    simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true', List.any_eq_true]
    exact ⟨hc, by simp [hcs], x, hx, hxc⟩
  rw [hnil] at this; exact nomatch this

theorem closeStep_sound {cs : List α} {a : α} {s : List α} (hs : ∀ x ∈ s, Conn rel cs a x) :
    ∀ x ∈ closeStep rel cs s, Conn rel cs a x := by
  intro x hx
  simp only [closeStep, List.mem_append, List.mem_filter, Bool.and_eq_true, List.any_eq_true] at hx
  rcases hx with hx | ⟨hc, _, y, hy, hyx⟩
  · exact hs x hx
  · exact Conn.step (hs y hy) hc hyx

theorem closeStep_nodup {cs s : List α} (hcs : cs.Nodup) (hs : s.Nodup) :
    (closeStep rel cs s).Nodup := by
  unfold closeStep
  rw [List.nodup_append]
  refine ⟨hs, hcs.filter _, fun x hx y hy => ?_⟩
  simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
    decide_eq_false_iff_not] at hy
  rintro rfl; exact hy.2.1 hx

theorem closeStep_sub {cs s : List α} (hs : s ⊆ cs) : closeStep rel cs s ⊆ cs := by
  intro x hx
  simp only [closeStep, List.mem_append, List.mem_filter] at hx
  rcases hx with hx | ⟨hx, _⟩
  · exact hs hx
  · exact hx

theorem closeN_spec {cs : List α} {a : α} (hcs : cs.Nodup) :
    ∀ (n : Nat) (s : List α), s.Nodup → s ⊆ cs → cs.length < s.length + n →
      (∀ x ∈ s, Conn rel cs a x) → a ∈ s →
      Closed rel cs (closeN rel cs n s) ∧ (∀ x ∈ closeN rel cs n s, Conn rel cs a x) ∧
        a ∈ closeN rel cs n s
  | 0, s, hn, hsub, hlen, _, _ => by
    have := List.Nodup.length_le_of_subset hn hsub; omega
  | n + 1, s, hn, hsub, hlen, hconn, ha => by
    simp only [closeN]
    split
    · exact ⟨closed_of_fix (by assumption), hconn, ha⟩
    · rename_i hne
      have hgrow : s.length + 1 ≤ (closeStep rel cs s).length := by
        simp only [closeStep, List.length_append] at hne ⊢; omega
      exact closeN_spec hcs n _ (closeStep_nodup hcs hn) (closeStep_sub hsub) (by omega)
        (closeStep_sound hconn) (List.mem_append_left _ ha)

/-- **The classes are the transitive closure.** -/
theorem cls_spec {cs : List α} (hcs : cs.Nodup) {a : α} (ha : a ∈ cs) (b : α) :
    b ∈ cls rel cs a ↔ Conn rel cs a b := by
  obtain ⟨hcl, hsound, hmem⟩ := closeN_spec (rel := rel) (a := a) hcs (cs.length + 1) [a]
    (List.nodup_cons.mpr ⟨by simp, List.nodup_nil⟩) (fun x hx => by simp at hx; exact hx ▸ ha)
    (by simp; omega) (fun x hx => by simp at hx; exact hx ▸ Conn.refl) (List.mem_singleton_self a)
  refine ⟨hsound b, fun h => ?_⟩
  induction h with
  | refl => exact hmem
  | step _ hc hbc ih => exact hcl _ ih _ hc hbc

theorem conn_mem {cs : List α} {a b : α} (ha : a ∈ cs) (h : Conn rel cs a b) : b ∈ cs := by
  induction h with
  | refl => exact ha
  | step _ hc _ _ => exact hc

theorem conn_trans {cs : List α} {x y z : α} (h1 : Conn rel cs x y) (h2 : Conn rel cs y z) :
    Conn rel cs x z := by
  induction h2 with
  | refl => exact h1
  | step _ hc hbc ih => exact Conn.step ih hc hbc

theorem conn_symm (hsym : ∀ a b, rel a b = rel b a) {cs : List α} {a b : α} (ha : a ∈ cs)
    (h : Conn rel cs a b) : Conn rel cs b a := by
  induction h with
  | refl => exact Conn.refl
  | @step b c hab hc hbc ih =>
    have hcb : Conn rel cs c b :=
      Conn.step Conn.refl (conn_mem ha hab) (by rw [hsym]; exact hbc)
    exact conn_trans hcb ih

/-! ## The representative: the least name -/

/-- The least-named of `a` and `s` (the first such, which names make unique). -/
def leastName (name : α → N) (a : α) (s : List α) : α :=
  s.foldl (fun m b => if name b < name m then b else m) a

theorem leastName_spec (name : α → N) : ∀ (s : List α) (m : α),
    (leastName name m s = m ∨ leastName name m s ∈ s) ∧ name (leastName name m s) ≤ name m ∧
      ∀ b ∈ s, name (leastName name m s) ≤ name b
  | [], m => ⟨Or.inl rfl, le_rfl, fun _ h => nomatch h⟩
  | b :: s, m => by
    have e : leastName name m (b :: s) =
        leastName name (if name b < name m then b else m) s := rfl
    rw [e]
    by_cases hb : name b < name m
    · simp only [hb, ite_true]
      obtain ⟨h1, h2, h3⟩ := leastName_spec name s b
      refine ⟨Or.inr (h1.elim (fun h => by rw [h]; exact List.mem_cons_self ..)
        (List.mem_cons_of_mem _)), le_trans h2 hb.le, ?_⟩
      intro c hc
      rcases List.mem_cons.mp hc with rfl | hc
      · exact h2
      · exact h3 c hc
    · simp only [hb, ite_false]
      obtain ⟨h1, h2, h3⟩ := leastName_spec name s m
      refine ⟨h1.elim Or.inl (fun h => Or.inr (List.mem_cons_of_mem _ h)), h2, ?_⟩
      intro c hc
      rcases List.mem_cons.mp hc with rfl | hc
      · exact le_trans h2 (not_lt.mp hb)
      · exact h3 c hc

/-- The representative: the least-named member of the class. -/
def rep (rel : α → α → Bool) (name : α → N) (cs : List α) (a : α) : α :=
  leastName name a (cls rel cs a)

variable (name : α → N)

theorem rep_mem {cs : List α} (hcs : cs.Nodup) {a : α} (ha : a ∈ cs) :
    Conn rel cs a (rep rel name cs a) := by
  rcases (leastName_spec name (cls rel cs a) a).1 with h | h
  · unfold rep; rw [h]; exact Conn.refl
  · exact (cls_spec hcs ha _).mp h

theorem rep_least {cs : List α} (hcs : cs.Nodup) {a : α} (ha : a ∈ cs) (b : α)
    (hb : Conn rel cs a b) : name (rep rel name cs a) ≤ name b :=
  (leastName_spec name (cls rel cs a) a).2.2 b ((cls_spec hcs ha b).mpr hb)

/-- **Fixed by the class and the names alone.** -/
theorem rep_unique {cs : List α} (hcs : cs.Nodup) (hnames : (cs.map name).Nodup) {a r : α}
    (ha : a ∈ cs) (hr : Conn rel cs a r) (hmin : ∀ b, Conn rel cs a b → name r ≤ name b) :
    rep rel name cs a = r := by
  have h1 := rep_least name hcs ha r hr
  have h2 := hmin _ (rep_mem name hcs ha)
  have hn : name (rep rel name cs a) = name r := le_antisymm h1 h2
  exact List.inj_on_of_nodup_map hnames (conn_mem ha (rep_mem name hcs ha)) (conn_mem ha hr) hn

theorem conn_congr {cs cs' : List α} (h : ∀ x, x ∈ cs ↔ x ∈ cs') {a b : α}
    (hab : Conn rel cs a b) : Conn rel cs' a b := by
  induction hab with
  | refl => exact Conn.refl
  | step _ hc hbc ih => exact Conn.step ih ((h _).mp hc) hbc

/-- **Independent of input order.** -/
theorem rep_order {cs cs' : List α} (hp : cs.Perm cs') (hcs : cs.Nodup)
    (hnames : (cs.map name).Nodup) {a : α} (ha : a ∈ cs) :
    rep rel name cs' a = rep rel name cs a := by
  have hm : ∀ x, x ∈ cs' ↔ x ∈ cs := fun x => hp.mem_iff.symm
  apply rep_unique name (hp.nodup_iff.mp hcs) ((hp.map name).nodup_iff.mp hnames)
    (hp.mem_iff.mp ha) (conn_congr (fun x => (hm x).symm) (rep_mem name hcs ha))
  intro b hb
  exact rep_least name hcs ha b (conn_congr hm hb)

/-- **One representative per class.** -/
theorem rep_class (hsym : ∀ a b, rel a b = rel b a) {cs : List α} (hcs : cs.Nodup)
    (hnames : (cs.map name).Nodup) {a b : α} (ha : a ∈ cs) (hab : Conn rel cs a b) :
    rep rel name cs b = rep rel name cs a := by
  apply rep_unique name hcs hnames (conn_mem ha hab)
    (conn_trans (conn_symm hsym ha hab) (rep_mem name hcs ha))
  intro c hc
  exact rep_least name hcs ha c (conn_trans hab hc)

/-- A relation that a map preserves gives classes the map carries to classes: the rule is
invariant under every symmetry of the decided relation that keeps the names. -/
theorem conn_map {β : Type} {relB : β → β → Bool} (f : α → β)
    (hf : ∀ a b, relB (f a) (f b) = rel a b) {cs : List α} {a b : α} (h : Conn rel cs a b) :
    Conn relB (cs.map f) (f a) (f b) := by
  induction h with
  | refl => exact Conn.refl
  | step _ hc hbc ih => exact Conn.step ih (List.mem_map_of_mem hc) (by rw [hf]; exact hbc)

end Sheet.Classes
