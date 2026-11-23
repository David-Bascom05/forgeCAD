import Mathlib.Topology.Connected.Basic
import Mathlib.Topology.Order.OrderClosed
import Mathlib.Topology.Algebra.Order.Field
import Mathlib.Topology.Instances.Real.Lemmas
import ForgeExact.DecideOnce.Curved

/-!
# From Mathlib's topology to forge-exact's

forge-exact states its spec with a core-Lean `ForgeExact.Topology` (open sets as predicates, its
own `interior`, `closure`, `NowhereDense` and `Curved.Connected`), so that the executable
reference links no library. Its Topology.lean says a Mathlib `TopologicalSpace` gives a `Topology`
at once; this is that instance, and the dictionary between the two vocabularies:

* `isOpen_iff`, `closure_iff`, `interior_iff`: forge's sets are Mathlib's;
* `nowhereDense_iff`: forge's `NowhereDense` is Mathlib's `IsNowhereDense`-style statement
  (`interior (closure Z) = ∅`);
* **`connected_iff`**: `Curved.Connected` is exactly `IsPreconnected`.

So every curved statement of forge-exact (`Faithful`, `ExactK`, `CurvedTarget`) can be read and
proven over Mathlib's ℝ³ with no change of meaning.
-/

namespace ForgeCurved

open ForgeExact

/-- **A Mathlib topology is a forge-exact `Topology`.** -/
instance (priority := low) toForge (X : Type) [TopologicalSpace X] : ForgeExact.Topology X where
  IsOpen U := IsOpen {x | U x}
  isOpen_univ := by simp
  isOpen_inter U V hU hV := by
    have : {x | Pred.inter U V x} = {x | U x} ∩ {x | V x} := rfl
    rw [this]; exact hU.inter hV
  isOpen_sUnion F hF := by
    have : {x | ∃ U, F U ∧ U x} = ⋃ U ∈ {U | F U}, {x | U x} := by ext; simp
    rw [this]
    exact isOpen_biUnion fun U hU => hF U hU

variable {X : Type} [TopologicalSpace X]

theorem isOpen_iff (U : Pred X) : ForgeExact.Topology.IsOpen U ↔ IsOpen {x | U x} := Iff.rfl

theorem closure_iff (S : Pred X) (x : X) :
    ForgeExact.Topology.closure S x ↔ x ∈ closure {y | S y} := by
  rw [mem_closure_iff]
  constructor
  · intro h o ho hx
    obtain ⟨y, hy, hs⟩ := h (fun z => z ∈ o) (show IsOpen {x | x ∈ o} from ho) hx
    exact ⟨y, hy, hs⟩
  · intro h U hU hx
    obtain ⟨y, hy, hs⟩ := h {z | U z} hU hx
    exact ⟨y, hy, hs⟩

theorem closure_eq (S : Pred X) : {x | ForgeExact.Topology.closure S x} = closure {y | S y} := by
  ext x; exact closure_iff S x

theorem interior_iff (S : Pred X) (x : X) :
    ForgeExact.Topology.interior S x ↔ x ∈ interior {y | S y} := by
  rw [mem_interior]
  constructor
  · rintro ⟨U, hU, hs, hx⟩
    exact ⟨{y | U y}, fun y hy => hs y hy, hU, hx⟩
  · rintro ⟨t, ht, ho, hx⟩
    exact ⟨fun y => y ∈ t, (show IsOpen {x | x ∈ t} from ho), fun y hy => ht hy, hx⟩

theorem interior_eq (S : Pred X) : {x | ForgeExact.Topology.interior S x} = interior {y | S y} := by
  ext x; exact interior_iff S x

theorem nowhereDense_iff (Z : Pred X) :
    ForgeExact.Topology.NowhereDense Z ↔ interior (closure {y | Z y}) = ∅ := by
  unfold ForgeExact.Topology.NowhereDense
  rw [Set.eq_empty_iff_forall_notMem]
  apply forall_congr'; intro x
  rw [interior_iff, ← closure_eq]

/-- **`Curved.Connected` is `IsPreconnected`.** -/
theorem connected_iff (S : Pred X) :
    ForgeExact.DecideOnce.Curved.Connected S ↔ IsPreconnected {x | S x} := by
  unfold ForgeExact.DecideOnce.Curved.Connected IsPreconnected
  constructor
  · intro h u v hu hv hs ⟨x, hxS, hxu⟩ ⟨y, hyS, hyv⟩
    obtain ⟨z, hz, hzu, hzv⟩ := h (fun z => z ∈ u) (fun z => z ∈ v) (show IsOpen {x | x ∈ u} from hu)
      (show IsOpen {x | x ∈ v} from hv) (fun z hz => hs hz) ⟨x, hxS, hxu⟩ ⟨y, hyS, hyv⟩
    exact ⟨z, hz, hzu, hzv⟩
  · intro h U V hU hV hs ⟨x, hxS, hxU⟩ ⟨y, hyS, hyV⟩
    obtain ⟨z, hz, hzU, hzV⟩ := h {z | U z} {z | V z} hU hV (fun z hz => hs z hz)
      ⟨x, hxS, hxU⟩ ⟨y, hyS, hyV⟩
    exact ⟨z, hz, hzU, hzV⟩

end ForgeCurved
