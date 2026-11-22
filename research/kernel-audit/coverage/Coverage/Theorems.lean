import Coverage.Model

/-!
# Coverage of the box-box family

* `Rel.complete`: the 13 relations are every consistent sign pattern of one axis (`decide`).
* `flags_inert`: the retry ladder's flags change nothing a flat pair reaches.
* `no_retry`: on every consistent pattern, either sample side, the modelled boolean neither
  loses its way nor refuses, so `plan_retried`'s ladder (boolean.rs:910–947), `plan_fenced`'s
  (949–962) and `plan`'s (889–908) are never entered.
* `gaps`: the patterns whose outcome is not a valid body or a correct refusal, listed exhaustively.
* `correct_off_gaps`: everywhere else the outcome is the exact one (a valid body whose boundary is
  the regularised boolean's, Empty, or ZeroThickness exactly where the result is not a manifold).

The family theorems are checked by evaluation (`native_decide`: they trust the Lean compiler, not
only the kernel); `Rel.complete` and the flag lemmas are kernel-checked.
-/

namespace Coverage

/-- Every rank tuple over 0..3 that is a consistent pattern (ends ordered, ranks 0..k-1 all used)
is one of the 13, and each of the 13 is one. -/
def tuples : List (Nat × Nat × Nat × Nat) :=
  (List.range 4).flatMap fun a0 => (List.range 4).flatMap fun a1 => (List.range 4).flatMap fun b0 =>
    (List.range 4).map fun b1 => (a0, a1, b0, b1)

def inAll (t : Nat × Nat × Nat × Nat) : Bool :=
  Rel.all.any fun s => s.a0 == t.1 && s.a1 == t.2.1 && s.b0 == t.2.2.1 && s.b1 == t.2.2.2

def validT (t : Nat × Nat × Nat × Nat) : Bool := (Rel.mk "" t.1 t.2.1 t.2.2.1 t.2.2.2).valid

theorem Rel.complete : (tuples.all fun t => validT t == inAll t) = true := by decide +kernel

theorem Rel.count : (Rel.all.all Rel.valid) = true ∧ Rel.all.length = 13 := by decide

theorem edgeCuts_flags (p : Pat) (fl : Flags) (f g : Face) : edgeCuts p fl f g = edgeCuts p {} f g := by
  unfold edgeCuts; simp only [ite_self]

theorem segs_flags (p : Pat) (fl : Flags) (sv : Bool) : segs p fl sv = segs p {} sv := by
  unfold segs trimSegs stretchInside; simp only [edgeCuts_flags]

/-- The ladder's flags (CAREFUL, CHORDS, NO_RUN_ON, FENCES, ABSORB_HAIR_ROOTS) change nothing. -/
theorem flags_inert (p : Pat) (fl : Flags) (sv : Bool) : kernel p fl sv = kernel p {} sv := by
  unfold kernel; rw [segs_flags]

def retries : Outcome → Bool
  | .lost _ => true
  | .unsupported => true
  | _ => false

/-- No consistent pattern reaches the retry ladder or a refusal other than ZeroThickness. -/
theorem no_retry : (Pat.all.all fun p => [false, true].all fun sv => !retries (kernel p {} sv)) = true := by
  native_decide

/-- The sample side never matters. -/
theorem sample_inert : (Pat.all.all fun p => kernel p {} true == kernel p {} false) = true := by
  native_decide

def gapLabels : List String :=
  ["meets/meets/meets union", "meets/meets/metBy union", "meets/metBy/meets union",
   "meets/metBy/metBy union", "metBy/meets/meets union", "metBy/meets/metBy union",
   "metBy/metBy/meets union", "metBy/metBy/metBy union"]

/-- The gaps, exhaustively: the eight unions of two boxes meeting at one corner, where the kernel
makes a pinched body that `check` accepts instead of refusing with ZeroThickness. -/
theorem gaps :
    (Pat.all.filter fun p => kernel p != exact p).map Pat.label = gapLabels ∧
    (Pat.all.filter fun p => kernel p != exact p).all (fun p => kernel p == .pinched && exact p == .zeroThickness) = true := by
  native_decide

/-- Every other pattern: the kernel's outcome is the exact one. -/
theorem correct_off_gaps :
    (Pat.all.all fun p => gapLabels.contains p.label || kernel p == exact p) = true := by
  native_decide

end Coverage
