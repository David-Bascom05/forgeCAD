/-!
# A8.unneeded: the flat fallback adds no answer where today's path refuses

verify/forge-bool-proof/ForgeBoolProof/Compose.lean `withFallback` runs today's path and, on a
refusal the fallback is entered on (`retry`), A8 from the original inputs. `fallback_correct`
makes the composition correct given A8's own `BooleanCorrect` (rows A8.*, open). A8.unneeded is the
other way to close E.fb: where today's path refuses and A8 would be entered, A8 has no answer
either, so the fallback gives nothing today's path does not, and the shipped boolean (today's path
alone, A8 unwired) answers exactly where the composition does.

`withFallback` is restated here over any inputs and refusals (core Lean; it is the same match as
Compose.lean's, which reads `Body`, `Op` and `Refusal E K Q` through Mathlib):

* **`A8Unneeded today a8 retry`**: for every input, if today's path refuses with `e` and `retry e`,
  A8 does not answer `ok`.
* **`ok_of_withFallback`**: under it, an `ok` of the composition is today's `ok`.
* **`withFallback_ok_iff`**: and conversely, so the composition and today's path have the same
  `ok`s, the same answers.

Evidence (measured, not proven; FALLBACK, main 4846170a): today's path refuses once over the suite
and fuzz 1–300 (316 parts), sequential and n-ary alike: fuzz55, `ZeroThickness`, a subtracted ball
(curved), where A8 (flat faces only) skips; on every refusal the sequential regen met, A8 was run
from the boolean's original operands (a scratch build with `decide_once`): it answered none.
So on the corpus the premise of `A8Unneeded` holds with nothing to spare and nothing to show.
Whether it holds beyond the corpus is open (the ledger's A8.unneeded). Core Lean only.
-/

namespace ForgeBoolFallback.Unneeded

variable {I ε β : Type}

/-- Compose.lean's `withFallback`, over inputs `I`, refusals `ε` and answers `β`. -/
def withFallback (today a8 : I → Except ε β) (retry : ε → Bool) (x : I) : Except ε β :=
  match today x with
  | .ok r => .ok r
  | .error e => if retry e then a8 x else .error e

/-- **A8.unneeded**: wherever today's path refuses and A8 would be entered, A8 does not answer. -/
def A8Unneeded (today a8 : I → Except ε β) (retry : ε → Bool) : Prop :=
  ∀ x e r, today x = .error e → retry e = true → a8 x ≠ .ok r

/-- Under `A8Unneeded`, an answer of the composition is today's answer. -/
theorem ok_of_withFallback {today a8 : I → Except ε β} {retry : ε → Bool}
    (h : A8Unneeded today a8 retry) {x : I} {r : β} (hr : withFallback today a8 retry x = .ok r) :
    today x = .ok r := by
  unfold withFallback at hr
  cases ht : today x with
  | ok r' => rw [ht] at hr; exact hr
  | error e =>
    rw [ht] at hr
    by_cases hre : retry e = true
    · simp only [hre, ite_true] at hr
      exact absurd hr (h x e r ht hre)
    · simp [hre] at hr

/-- **The fallback is unneeded**: under `A8Unneeded` the composition and today's path answer
alike: the same `ok`s. -/
theorem withFallback_ok_iff {today a8 : I → Except ε β} {retry : ε → Bool}
    (h : A8Unneeded today a8 retry) (x : I) (r : β) :
    withFallback today a8 retry x = .ok r ↔ today x = .ok r := by
  constructor
  · exact ok_of_withFallback h
  · intro ht
    unfold withFallback
    rw [ht]

end ForgeBoolFallback.Unneeded
