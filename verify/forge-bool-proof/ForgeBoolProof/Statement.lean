import ForgeCurved.BodyModel

/-!
# §E: what "today's boolean is correct" means (research/decide-once/old-path-statements.md §E)

The statement of `boolean_correct`, over the body layer of §0
(`ForgeCurved.BodyModel`): a stored body is read through an abstraction function
`abs : Body → BodyModel E` (the translation's, TRANSLATE's; a parameter until it exists), its
solid is `⟦b⟧ = denote δ (abs b)`, the solid of its canonical exact counterpart (the master's
ruling F1 with vertex clusters), and it is valid when `ValidSolid δ (abs b)`.

* `opSet`, `reg`: the exact result `reg (A op B) = closure (interior (A op B))`.
* `Faithful δ b`: each stored region and curve lies within `δ` of its exact meet (F1). For the
  inputs it is a hypothesis supplied by the feature that made the body (`FaithfulInput`); for the
  output it is proven (D8).
* `ManifoldPt S x`: near `x`, `S` is a closed half-space up to a homeomorphism. A refusal
  `zeroThickness p` is true when the exact result has a frontier point within `δ` of `p` that is
  not one.
* `Scope`: what the other refusals mean. `unhandled k` is true when the input is outside the
  declared scope `k` (`InputTooLarge`, `NoClosedForm`, …); `undecided q` when question `q` is one
  the exact test for it cannot decide. Both are declared, per kind, by the parameter; `internal`
  is never true.
* `Correct`: the postcondition. `ok R`: `R` is a valid solid, faithful (F1), its solid is the exact
  result, and its names follow the rule. `error e`: the refusal is true.
* **`BooleanCorrect boolean`**: for valid, faithful, in-size inputs, every result is `Correct`.
-/

namespace ForgeBoolProof

open Set ForgeCurved.BodyModel

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The three booleans (forge-exact's `Op`). -/
abbrev Op := ForgeExact.Op

/-- The exact boolean of two sets. -/
def opSet : Op → Set E → Set E → Set E
  | .union, A, B => A ∪ B
  | .inter, A, B => A ∩ B
  | .diff, A, B => A \ B

/-- Regularisation: the closure of the interior. -/
def reg (S : Set E) : Set E := closure (interior S)

/-- **F1, per body**: each stored face region and edge curve is within `δ` of its exact meet. -/
def Faithful (δ : ℝ) (b : BodyModel E) : Prop :=
  (∀ f ∈ b.faces, WithinH δ f.region (regionMeet δ b f)) ∧
    ∀ i (hi : i < b.edges.length), WithinH δ b.edges[i].curve (edgeMeet δ b i b.edges[i])

/-- `S` is a manifold with boundary at `x`: an open neighbourhood `U` of `x` is homeomorphic to an
open set `V` on which `S` is a closed half-space `ℓ ≤ 0`. (At a frontier point of `S` the chart
must send `x` onto `ℓ = 0`: elsewhere `x` would be interior to `S` or off its closure.) -/
def ManifoldPt (S : Set E) (x : E) : Prop :=
  ∃ (U V : Set E) (h : U ≃ₜ V) (ℓ : E →L[ℝ] ℝ), IsOpen U ∧ x ∈ U ∧ IsOpen V ∧ ℓ ≠ 0 ∧
    ∀ y : U, ((y : E) ∈ S ↔ ℓ (h y : E) ≤ 0)

/-- A refusal, as boolean.rs's `Error`: a zero-thickness result at a point, an input outside a
declared scope, a question undecided after its exact test, or an internal error. -/
inductive Refusal (E K Q : Type) where
  | zeroThickness (at_ : E)
  | unhandled (k : K)
  | undecided (q : Q)
  | internal

/-- What the scoped refusals mean (declared per kind). -/
structure Scope (Body K Q : Type) where
  /-- the input pair is outside the declared scope `k` -/
  outOf : K → Body → Body → Prop
  /-- question `q` about the pair is one its exact test cannot decide -/
  openQ : Q → Body → Body → Prop

variable {Body K Q : Type}

/-- **The refusal is true** of the exact result. -/
def RefusalTrue (δ : ℝ) (abs : Body → BodyModel E) (sc : Scope Body K Q) (A B : Body) (op : Op) :
    Refusal E K Q → Prop
  | .zeroThickness p =>
      ∃ x, dist x p ≤ δ ∧ x ∈ frontier (reg (opSet op (denote δ (abs A)) (denote δ (abs B)))) ∧
        ¬ ManifoldPt (reg (opSet op (denote δ (abs A)) (denote δ (abs B)))) x
  | .unhandled k => sc.outOf k A B
  | .undecided q => sc.openQ q A B
  | .internal => False

/-- **The postcondition of a boolean.** `Names A B op R`: the result's names follow
boolean.rs's naming rule (§D7, stated per entity there). -/
def Correct (δ : ℝ) (abs : Body → BodyModel E) (sc : Scope Body K Q)
    (Names : Body → Body → Op → Body → Prop) (A B : Body) (op : Op) :
    Except (Refusal E K Q) Body → Prop
  | .ok R => ValidSolid δ (abs R) ∧ Faithful δ (abs R) ∧
      denote δ (abs R) = reg (opSet op (denote δ (abs A)) (denote δ (abs B))) ∧ Names A B op R
  | .error e => RefusalTrue δ abs sc A B op e

/-- The inputs a boolean is proven for: valid solids, faithful (F1), and in size (`InputSize`:
every magnitude bound the screens' IEEE proofs assume; a parameter until the translated surface
data exists). -/
def Input (δ : ℝ) (abs : Body → BodyModel E) (Size : Body → Prop) (b : Body) : Prop :=
  ValidSolid δ (abs b) ∧ Faithful δ (abs b) ∧ Size b

/-- **§E, `boolean_correct`, as a proposition about a boolean function.** -/
def BooleanCorrect (δ : ℝ) (abs : Body → BodyModel E) (sc : Scope Body K Q)
    (Names : Body → Body → Op → Body → Prop) (Size : Body → Prop)
    (boolean : Body → Body → Op → Except (Refusal E K Q) Body) : Prop :=
  ∀ A B op, Input δ abs Size A → Input δ abs Size B → Correct δ abs sc Names A B op (boolean A B op)

end ForgeBoolProof
