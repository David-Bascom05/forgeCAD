/-!
# Just enough topology, in core Lean

The reference is built without Mathlib: its prebuilt cache is unreachable from this repository's
build machines, and the theorems need only open sets, interior, closure and nowhere-dense sets.
The definitions are the textbook ones, so a Mathlib `TopologicalSpace` gives a `Topology` at once
(`IsOpen` and the three closure laws), and the statements carry over unchanged.

A set is a predicate (`Pred X := X → Prop`), so that the compiled reference and the proofs share
one notion with no library in between.
-/

namespace ForgeExact

universe u

/-- A set of points, as a predicate. -/
abbrev Pred (X : Type u) := X → Prop

namespace Pred

variable {X : Type u}

def Sub (S T : Pred X) : Prop := ∀ x, S x → T x
def inter (S T : Pred X) : Pred X := fun x => S x ∧ T x
def union (S T : Pred X) : Pred X := fun x => S x ∨ T x
def diff (S T : Pred X) : Pred X := fun x => S x ∧ ¬T x
/-- The symmetric difference. -/
def sdiff (S T : Pred X) : Pred X := fun x => (S x ∧ ¬T x) ∨ (T x ∧ ¬S x)

theorem ext {S T : Pred X} (h : ∀ x, S x ↔ T x) : S = T := funext fun x => propext (h x)

end Pred

/-- A topology: a family of open sets closed under finite intersection and arbitrary union. -/
class Topology (X : Type u) where
  IsOpen : Pred X → Prop
  isOpen_univ : IsOpen fun _ => True
  isOpen_inter : ∀ U V, IsOpen U → IsOpen V → IsOpen (Pred.inter U V)
  isOpen_sUnion : ∀ F : Pred X → Prop, (∀ U, F U → IsOpen U) → IsOpen fun x => ∃ U, F U ∧ U x

namespace Topology

variable {X : Type u} [Topology X]

/-- The interior: points with an open neighbourhood inside `S`. -/
def interior (S : Pred X) : Pred X := fun x => ∃ U, IsOpen U ∧ U.Sub S ∧ U x

/-- The closure: points every open neighbourhood of which meets `S`. -/
def closure (S : Pred X) : Pred X := fun x => ∀ U, IsOpen U → U x → ∃ y, U y ∧ S y

/-- Nowhere dense: the closure has empty interior. -/
def NowhereDense (Z : Pred X) : Prop := ∀ x, ¬interior (closure Z) x

theorem interior_sub (S : Pred X) : (interior S).Sub S := fun _ ⟨_, _, hU, hx⟩ => hU _ hx

theorem sub_closure (S : Pred X) : S.Sub (closure S) := fun x hx _ _ hU => ⟨x, hU, hx⟩

theorem isOpen_interior (S : Pred X) : IsOpen (interior S) := by
  have h := isOpen_sUnion (fun U => IsOpen U ∧ U.Sub S) fun U hU => hU.1
  have e : (fun x => ∃ U, (IsOpen U ∧ U.Sub S) ∧ U x) = interior S :=
    Pred.ext fun x => ⟨fun ⟨U, ⟨h1, h2⟩, h3⟩ => ⟨U, h1, h2, h3⟩,
      fun ⟨U, h1, h2, h3⟩ => ⟨U, ⟨h1, h2⟩, h3⟩⟩
  rw [← e]; exact h

theorem interior_mono {S T : Pred X} (h : S.Sub T) : (interior S).Sub (interior T) :=
  fun _ ⟨U, hU, hUS, hx⟩ => ⟨U, hU, fun y hy => h y (hUS y hy), hx⟩

theorem closure_mono {S T : Pred X} (h : S.Sub T) : (closure S).Sub (closure T) :=
  fun _ hx U hU hxU => let ⟨y, hyU, hyS⟩ := hx U hU hxU; ⟨y, hyU, h y hyS⟩

theorem closure_closure (S : Pred X) : (closure (closure S)).Sub (closure S) := by
  intro x hx U hU hxU
  obtain ⟨y, hyU, hy⟩ := hx U hU hxU
  exact hy U hU hyU

/-- An open set inside `S` is inside its interior. -/
theorem sub_interior {U S : Pred X} (hU : IsOpen U) (h : U.Sub S) : U.Sub (interior S) :=
  fun _ hx => ⟨U, hU, h, hx⟩

end Topology

end ForgeExact
