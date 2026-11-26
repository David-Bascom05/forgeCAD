import ForgeBoolProof.KeepRule
import Aeneas

/-!
# The bridge to the translation: Aeneas's `Result`, the in-place target (ledger row E.shape)

The shipped `general(target: &mut Body, b: &Body, op) -> R<()>` edits its target in place, and
Aeneas translates it to a monadic function

  `general : Body → Body → Op → Result (core.result.Result Unit Error × Body)`

returning the Rust result and the target as it ends. `Result` is Aeneas's monad: `ok`, a panic
(`fail`), or divergence. Its specs are total: `spec m P` holds only when `m` returns `ok`.
So a proven spec also proves "no panic, no overflow, terminates".

* `readInPlace`: the functional reading of the in-place call. `Ok(())` reads as `ok` of the
  edited target, `Err(e)` as the refusal `readErr e`.
* **`ShapeR general read run`** (E.shape, TRANSLATE): for every input, the translated `general`
  returns `ok`, and its output read through `read` is `run A B op`, the `Pipeline.run` of the
  translated steps. The retry ladder fits the same shape. Its rungs are one `general` with the
  ladder's flags as an explicit argument (FALLBACK), and `general` here is the ladder's outer
  function (or `fun A B op => general_with flags A B op` for one rung).
* **`BooleanCorrectR`**: §E for the translated function itself. Every call returns `ok` (no
  panic, no divergence), and what it returns, read, is `Correct`.
* **`booleanCorrectR_of_shape`**: the shape lemma and §E for `run` give §E for the translation.
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Aeneas Aeneas.Std ForgeCurved.BodyModel

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E] {Body K Q Err : Type}

/-- The in-place call, read functionally: `Ok(())` is the edited target, `Err(e)` its refusal. -/
def readInPlace (readErr : Err → Refusal E K Q) :
    core.result.Result Unit Err × Body → Except (Refusal E K Q) Body
  | (.Ok (), target) => .ok target
  | (.Err e, _) => .error (readErr e)

/-- **E.shape**: the translated function returns `ok`, and its output read is `run`'s. -/
def ShapeR {Out : Type} (general : Body → Body → Op → Result Out)
    (read : Out → Except (Refusal E K Q) Body) (run : Body → Body → Op → Except (Refusal E K Q) Body) :
    Prop :=
  ∀ A B op, WP.spec (general A B op) fun o => read o = run A B op

/-- **§E for the translation**: every call on valid inputs returns `ok`, and its output read is
`Correct`. -/
def BooleanCorrectR {Out : Type} (δ : ℝ) (abs : Body → BodyModel E) (sc : Scope Body K Q)
    (Names : Body → Body → Op → Body → Prop) (Size : Body → Prop)
    (general : Body → Body → Op → Result Out) (read : Out → Except (Refusal E K Q) Body) : Prop :=
  ∀ A B op, Input δ abs Size A → Input δ abs Size B →
    WP.spec (general A B op) fun o => Correct δ abs sc Names A B op (read o)

/-- **The shape lemma and §E for the pipeline give §E for the translation.** -/
theorem booleanCorrectR_of_shape {Out : Type} {δ : ℝ} {abs : Body → BodyModel E}
    {sc : Scope Body K Q} {Names : Body → Body → Op → Body → Prop} {Size : Body → Prop}
    {general : Body → Body → Op → Result Out} {read : Out → Except (Refusal E K Q) Body}
    {run : Body → Body → Op → Except (Refusal E K Q) Body}
    (hs : ShapeR general read run) (hc : BooleanCorrect δ abs sc Names Size run) :
    BooleanCorrectR δ abs sc Names Size general read := by
  intro A B op hA hB
  exact WP.spec_mono (hs A B op) fun o ho => ho ▸ hc A B op hA hB

/-- **End to end, for the translation**: the shape lemma and D1–D9. -/
theorem booleanCorrectR_of_steps {Out : Type} {δ : ℝ} {abs : Body → BodyModel E}
    {sc : Scope Body K Q} {Names : Body → Body → Op → Body → Prop} {Size : Body → Prop}
    {general : Body → Body → Op → Result Out} {read : Out → Except (Refusal E K Q) Body}
    {P : Pipeline Body (Refusal E K Q)} {V : View P E}
    (hs : ShapeR general read P.run)
    (h1 : D1 δ abs sc Size P V) (h2 : D2 δ abs sc Size P V) (h3 : D3 δ abs sc Size P V)
    (h4 : D4 δ abs sc Size P V) (h5 : D5 δ abs sc Size P V) (h6 : D6 δ abs sc Size P V)
    (h7 : D7 δ abs sc Size P V) (h8 : D8 δ abs sc Names Size P V) (h9 : D9 δ abs sc Size P) :
    BooleanCorrectR δ abs sc Names Size general read :=
  booleanCorrectR_of_shape hs (boolean_correct_of_steps h1 h2 h3 h4 h5 h6 h7 h8 h9)

end ForgeBoolProof
