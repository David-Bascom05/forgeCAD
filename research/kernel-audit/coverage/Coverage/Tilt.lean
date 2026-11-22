/-!
# The flat path on tilted planes

The box family (`Coverage.Model`) has every plane pair parallel or perpendicular. Turning one face
by an angle θ brings in the angle thresholds the flat path reads, each a comparison of θ (or of a
length θ makes) with a constant:

| site | test | here |
|---|---|---|
| `plane_plane`, `planes_coincident` (forge-geom intersect.rs:846, 853) | sin θ ≤ ANGULAR = 1e-11 | `parallel` |
| `classify_specs`' lean (boolean.rs:5926) | lean ≤ 1e-9: no decision | `leanSilent` |
| trim's run-on (boolean.rs:2450) | cross ≤ 1e-6 and both edge ends within a vertex tolerance | `runOn` |
| `nearly_flush` (ssi.rs:66, 85) | sin θ ≤ NEAR = 1e-4 and every region corner within LINEAR | `flush` |
| the across face's `smooth` (boolean.rs:4077) | sin θ ≤ 1e-3 on a straight edge | `smooth` |
| `clear_step` (boolean.rs:6186–6188) | a probe one step clear inside a region; steps 0.05·2⁻ᵏ ≥ 2·LINEAR | `probeable` |

**Units.** θ is a natural number of 1e-13 rad (sin θ = θ to 1e-13 relative over the thresholds'
range, below 1e-3); lengths are whole millimetres. So ANGULAR is 100, 1e-9 is 10⁴, 1e-6 is 10⁷,
NEAR is 10⁹, 1e-3 is 10¹⁰; a length L·θ is within LINEAR = 1e-5 mm when L·θ ≤ 10⁸; and the last
step `clear_step` tries at a half chord of 5 or more is 0.05·2⁻¹¹ mm, so a strip whose width at
the probe, (L/2)·θ, is under twice that (L·θ < 10¹²/1024 = 976 562 500) has no clear probe.

**The families** (verify/forge-exact/src/tilt.rs) are convex pairs whose tilted face crosses the
other body's face along a line at distance `reach` from the region's far corners. In those with
`flush` sides, B's side faces lie on A's: the strip of B's side between the crossing line and A's
edge is a region of width (reach/2)·θ at its probe.

**What the model says.** The decision vector of a family at θ depends only on where θ falls among
the family's breakpoints (`complete`: proved for every θ, not sampled). The outcome is the exact
one unless the sides are flush, the planes have stopped reading coincident (reach·θ > LINEAR) and
the strip is still too narrow to probe (`gap_iff`). Two sites that decide "one surface" differently
— the pair's `closed_or_flush` and `ssi::coincident` — disagree on θ ∈ (ANGULAR, LINEAR/reach]
(`disagree_iff`), and the model predicts that band harmless (`disagree_harmless`): the sweep agrees
(no family fails there).
-/

namespace Coverage.Tilt

def ANGULAR : Nat := 100
def LEAN : Nat := 10 ^ 4
def RUNON : Nat := 10 ^ 7
def NEAR : Nat := 10 ^ 9
def SMOOTH : Nat := 10 ^ 10
/-- LINEAR in mm·(1e-13 rad). -/
def LIN : Nat := 10 ^ 8
/-- Twice the last `clear_step` step at a half chord ≥ 5 (0.05·2⁻¹⁰ mm), times two (the strip's
width at its probe is half the reach's), in mm·(1e-13 rad). -/
def PROBE : Nat := 976562500

structure Fam where
  name : String
  /-- B's sides lie on A's (a strip of B's side between the crossing line and A's edge). -/
  flushSides : Bool
  /-- The crossing line's distance to the region's far corners, in mm. -/
  reach : Nat
  /-- A straight edge between two faces turned θ apart that a curve crosses (the ridge). -/
  crease : Bool
  /-- Half the length of an edge the crossing line all but runs along, in mm (0: none). -/
  runHalf : Nat
  deriving Repr, Inhabited

/-- The families of tilt.rs (`cap` and `wall_flush`, whose end faces are tilted against A's sides
as well, are left out: their second strip is θ² wide and not modelled). -/
def fams : List Fam :=
  [⟨"lid", false, 3, false, 0⟩, ⟨"plug", false, 3, false, 0⟩, ⟨"cap_over", true, 5, false, 0⟩,
   ⟨"wall", false, 3, false, 0⟩, ⟨"wall_over", true, 5, false, 5⟩, ⟨"twist", false, 0, false, 0⟩,
   ⟨"ridge", false, 0, true, 0⟩]

/-- What each site decides at θ. -/
structure Decisions where
  parallel : Bool
  leanSilent : Bool
  runOn : Bool
  flush : Bool
  smooth : Bool
  probeable : Bool
  deriving DecidableEq, Repr

def decide (f : Fam) (θ : Nat) : Decisions where
  parallel := θ ≤ ANGULAR
  leanSilent := θ ≤ LEAN
  runOn := θ ≤ RUNON && f.runHalf * θ ≤ LIN
  flush := θ ≤ NEAR && f.reach * θ ≤ LIN
  smooth := f.crease && θ ≤ SMOOTH
  probeable := PROBE ≤ f.reach * θ

/-- The pair path's answer (`closed_or_flush`): one surface. -/
def pairCoincident (d : Decisions) : Bool := d.parallel || d.flush
/-- `ssi::coincident`'s answer, read across edges and by trim's run-on. -/
def ssiCoincident (d : Decisions) : Bool := d.parallel

inductive Outcome | exact | gap
  deriving DecidableEq, Repr

/-- The modelled outcome: exact, unless a flush side's strip exists (the planes cross: neither
parallel nor flush) and is too narrow to probe (`clear_step`), so `probe` lands past it and the
strip is classed on the far face. -/
def outcome (f : Fam) (θ : Nat) : Outcome :=
  let d := decide f θ
  if f.flushSides && !pairCoincident d && !d.probeable then .gap else .exact

/-! ## Breakpoints: the decision vector is constant between them -/

def breaks (f : Fam) : List (Nat × Nat) :=
  -- (coefficient k, constant c): the test k·θ ≤ c (or c ≤ k·θ)
  [(1, ANGULAR), (1, LEAN), (1, RUNON), (f.runHalf, LIN), (1, NEAR), (f.reach, LIN), (1, SMOOTH),
   (f.reach, PROBE - 1)]

/-- Every comparison `decide` makes is one of `breaks` (as k·θ ≤ c), so θ and θ' on the same side
of every breakpoint get the same decisions. -/
theorem complete (f : Fam) (θ θ' : Nat)
    (h : (breaks f).all (fun (k, c) => (k * θ ≤ c) == (k * θ' ≤ c)) = true) :
    decide f θ = decide f θ' := by
  simp only [breaks, List.all_cons, List.all_nil, Bool.and_true, Bool.and_eq_true, beq_iff_eq,
    Nat.one_mul, decide_eq_decide] at h
  obtain ⟨h1, h2, h3, h4, h5, h6, h7, h8⟩ := h
  have h8' : (PROBE ≤ f.reach * θ) ↔ (PROBE ≤ f.reach * θ') := by
    simp only [PROBE] at h8 ⊢; omega
  simp only [decide, Decisions.mk.injEq]
  refine ⟨by simp [h1], by simp [h2], by simp [h3, h4], by simp [h5, h6], by simp [h7],
    by simp [h8']⟩

/-- The gaps, exactly: flush sides, the corners past LINEAR, the strip under the probe floor. -/
theorem gap_iff (f : Fam) (θ : Nat) :
    outcome f θ = .gap ↔
      f.flushSides = true ∧ ANGULAR < θ ∧ (NEAR < θ ∨ LIN < f.reach * θ) ∧ f.reach * θ < PROBE := by
  unfold outcome pairCoincident decide
  by_cases hf : f.flushSides <;> simp [hf] <;> omega

/-- On the families here (reach ≤ 5) the gap is the band reach·θ ∈ (LINEAR, 0.05·2⁻¹⁰ mm). -/
theorem gap_band (f : Fam) (θ : Nat) (hr : 0 < f.reach ∧ f.reach ≤ 5) :
    outcome f θ = .gap ↔ f.flushSides = true ∧ LIN < f.reach * θ ∧ f.reach * θ < PROBE := by
  rw [gap_iff]
  constructor
  · rintro ⟨h1, h2, h3, h4⟩
    refine ⟨h1, ?_, h4⟩
    rcases h3 with h3 | h3
    · simp only [NEAR, LIN] at *; have : θ ≤ f.reach * θ := Nat.le_mul_of_pos_left θ hr.1; omega
    · exact h3
  · rintro ⟨h1, h2, h3⟩
    refine ⟨h1, ?_, Or.inr h2, h3⟩
    simp only [ANGULAR, LIN] at *
    have : f.reach * θ ≤ 5 * θ := Nat.mul_le_mul_right θ hr.2
    omega

/-- The two "one surface" sites disagree exactly when θ is past ANGULAR but the planes are flush. -/
theorem disagree_iff (f : Fam) (θ : Nat) :
    pairCoincident (decide f θ) ≠ ssiCoincident (decide f θ) ↔
      ANGULAR < θ ∧ θ ≤ NEAR ∧ f.reach * θ ≤ LIN := by
  unfold pairCoincident ssiCoincident decide
  simp only [ne_eq]
  by_cases h : θ ≤ ANGULAR <;> simp [h] <;> omega

/-- …and the model predicts that disagreement harmless: no gap where the sites disagree. -/
theorem disagree_harmless (f : Fam) (θ : Nat) (h : pairCoincident (decide f θ) ≠ ssiCoincident (decide f θ)) :
    outcome f θ = .exact := by
  rw [disagree_iff] at h
  cases ho : outcome f θ with
  | exact => rfl
  | gap => rw [gap_iff] at ho; omega

/-- Each threshold is crossed inside the swept range [1e-13, 0.5]: the decision vector differs just
below and just above it (for a family that reaches it). -/
theorem thresholds_reached :
    (decide ⟨"", false, 0, false, 0⟩ 90 ≠ decide ⟨"", false, 0, false, 0⟩ 110) ∧
    (decide ⟨"", false, 0, false, 0⟩ (9 * 10 ^ 3) ≠ decide ⟨"", false, 0, false, 0⟩ (11 * 10 ^ 3)) ∧
    (decide ⟨"", false, 0, false, 0⟩ (9 * 10 ^ 6) ≠ decide ⟨"", false, 0, false, 0⟩ (11 * 10 ^ 6)) ∧
    (decide ⟨"", false, 0, false, 0⟩ (9 * 10 ^ 8) ≠ decide ⟨"", false, 0, false, 0⟩ (11 * 10 ^ 8)) ∧
    (decide ⟨"", false, 0, true, 0⟩ (9 * 10 ^ 9) ≠ decide ⟨"", false, 0, true, 0⟩ (11 * 10 ^ 9)) := by
  decide

/-- The modelled gaps on the sweep's grid (θ in 1e-13 rad), family by family. -/
def predicted (θs : List Nat) : List (String × Nat) :=
  fams.flatMap fun f => (θs.filter fun θ => outcome f θ == .gap).map fun θ => (f.name, θ)

end Coverage.Tilt
