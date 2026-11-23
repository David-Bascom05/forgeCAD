import ForgeBoolCore.Integrate
import ForgeCurved.Manifold
import ForgeExact.DecideOnce.QForm

/-!
# End to end, curved: the shipped Rust core on a curved ledger

The flat theorem (verify/aeneas/forge-bool-core, `end_to_end_of`) is `CoreSpec` (the
Rust `forge_bool_core::core` computes the Lean `coreM`), then `coreM_nil`, then `flat_correct`.
The core reads covectors and nothing else (design §0.1), so the curved theorem is the same
`CoreSpec` followed by **`curved_correct`** in `flat_correct`'s place:

* `curved_correct`: on an exact curved ledger (`ExactK`, any carriers, any points), the core's
  `ok` answer is a valid body whose faces are the exact boundary facets and whose kept cells
  regularise to the regularised boolean; its `ZeroThickness` refusal is `XNonManifoldK`. This is
  `correct_ok_k` and `zeroThickness_iff_k` (forge-exact, Q0) put together;
* `curved_correct_top`: over continuous carriers into ℝ with faithful cells (R1–R5), the refusal
  is also the topological `TopNonManifold` of the regularised result (`zeroThickness_iff_top`).

**The domain** (design D4): quadric carriers (`Quadric`, separators being planes, i.e. quadrics
of degree one) on formula-carrying bodies (`SForm.den`, D2). For formula bodies `ExactK.compat`
holds by `qform_compat` (`formula_compat`), so what the ledger must give is the rest of
`CurvedTarget` (the enumerator's L2 obligation, `curved_exact` on dol-curved's side).

**`EndToEndCurved`** quantifies over every such input and every Rust ledger reading as a curved
target ledger; **`end_to_end_curved : EndToEndCurved`** is proven, with no hypothesis, from the
proven `CoreSpec` (`core_spec_proof`, assembled from the aeneas project's group proofs as its
`end_to_end_proof` is). Size limits are `LedgerWF`'s, as for flat.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

open ForgeExact.DecideOnce.Curved in
/-- `CoreSpec`, proven: the aeneas project's groups (S, K, R, U) assembled as `end_to_end_proof`
assembles them. -/
theorem core_spec_proof : CoreSpec :=
  core_spec_of split_ok_spec_proof
    (classify_on_spec_of collapse_spec_proof
      (core_k_spec_of result_bridge_proof bad_edge_bridge_proof bad_vert_bridge_proof unify_spec_proof))

end forge_bool_core

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Ledger

/-- **`curved_correct`**: theorem (c) on any exact curved ledger, as `flat_correct` gives it on
the sampler's flat ledger. -/
theorem curved_correct {P K : Type} [Zero K] [LT K] [DecidableRel (α := K) (· < ·)]
    [ForgeExact.Topology P] {L : Ledger} {cs : List (Carrier P K)} {A : Nat → Pred P}
    (h : ExactK L cs A) (rule : Rule) :
    (∀ b, core L rule = .ok b →
      Valid L (L.keepTab rule) b ∧
      (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ XBoundaryK L cs A rule φ) ∧
      reg ((decompK h).kept fun T => xkeepK L cs A rule T = true) =
        reg (combine (ruleOp rule) (opndsK L A))) ∧
    ((∃ x, core L rule = .refused (.zeroThickness x)) ↔ XNonManifoldK L cs A rule) :=
  ⟨fun _ hb => correct_ok_k h rule hb, zeroThickness_iff_k h rule⟩

/-- Over ℝ, with faithful cells: the refusal is topological too. -/
theorem curved_correct_top {P : Type} [TopologicalSpace P] {L : Ledger}
    {cs : List (Carrier P ℝ)} {A : Nat → Pred P} (hc : ∀ c ∈ cs, Continuous c)
    (h : CurvedTarget L cs A) (rule : Rule) :
    (∃ x, core L rule = .refused (.zeroThickness x)) ↔ TopNonManifold cs (Rset L A rule) :=
  zeroThickness_iff_top hc h.1 h.2 rule

/-! ## The domain: quadric carriers, formula bodies -/

/-- The points: `(x, y, z)`. -/
abbrev R3 := ℝ × ℝ × ℝ

/-- A quadric carrier `xᵀ M x + b·x + c` with real coefficients (the rational ones of
`QFace.quadric`, and the seams and walls, which are planes: `M = 0`). Not normalised: the
enumerator's snap makes one class per surface. -/
structure Quadric where
  m : Fin 3 → Fin 3 → ℝ
  b : Fin 3 → ℝ
  c : ℝ

/-- A point's coordinates. -/
def coord (p : R3) : Fin 3 → ℝ := ![p.1, p.2.1, p.2.2]

/-- A quadric's value. -/
def Quadric.eval (q : Quadric) : Carrier R3 ℝ := fun p =>
  (∑ i, ∑ j, q.m i j * coord p i * coord p j) + (∑ i, q.b i * coord p i) + q.c

theorem coord_cont (i : Fin 3) : Continuous fun p : R3 => coord p i := by
  fin_cases i <;> simp [coord] <;> fun_prop

theorem Quadric.continuous (q : Quadric) : Continuous q.eval := by
  unfold Quadric.eval
  have := coord_cont
  fun_prop

theorem quadrics_cont (qs : List Quadric) : ∀ c ∈ qs.map Quadric.eval, Continuous c := by
  intro c hc
  obtain ⟨q, _, rfl⟩ := List.mem_map.mp hc
  exact q.continuous

/-- For formula bodies, `compat` is free (`qform_compat`, QForm.lean). -/
theorem formula_compat (qs : List Quadric) (F : Nat → SForm) (T : Cov) (i : Nat) :
    (covCellK (qs.map Quadric.eval) T).Sub ((F i).den (qs.map Quadric.eval)) ∨
      ∀ x, covCellK (qs.map Quadric.eval) T x → ¬(F i).den (qs.map Quadric.eval) x :=
  qform_compat _ F T i

end ForgeCurved

namespace forge_bool_core

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved

/-- **End to end, curved.** For quadric carriers and formula bodies, on every Rust ledger that
reads as a curved target ledger of them (exact, faithful: R1–R5), `forge_bool_core::core`'s answer is
the exact regularised boolean, and it refuses `ZeroThickness` exactly when the result is
non-manifold: as covectors (`XNonManifoldK`) and as sets (`TopNonManifold`). -/
def EndToEndCurved : Prop :=
  ∀ (qs : List Quadric) (F : Nat → SForm) (L : ForgeExact.DecideOnce.Ledger),
    (hT : CurvedTarget L (qs.map Quadric.eval) (fun i => (F i).den (qs.map Quadric.eval))) →
    ∀ (l : forge_bool_core.Ledger) (r : forge_bool_core.Rule), toL l = L → LedgerWF l →
    core l r ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ ⦃ o => ∃ ext,
      (∀ b, outcomeL l ext o = .ok b →
        ForgeExact.DecideOnce.Valid L (L.keepTab (ruleL l.nb.val r)) b ∧
        (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔
          XBoundaryK L (qs.map Quadric.eval) (fun i => (F i).den (qs.map Quadric.eval)) (ruleL l.nb.val r) φ) ∧
        reg ((decompK hT.1).kept fun T =>
            xkeepK L (qs.map Quadric.eval) (fun i => (F i).den (qs.map Quadric.eval)) (ruleL l.nb.val r) T = true) =
          reg (combine (ruleOp (ruleL l.nb.val r)) (opndsK L fun i => (F i).den (qs.map Quadric.eval)))) ∧
      ((∃ x, outcomeL l ext o = .refused (.zeroThickness x)) ↔
        XNonManifoldK L (qs.map Quadric.eval) (fun i => (F i).den (qs.map Quadric.eval)) (ruleL l.nb.val r)) ∧
      ((∃ x, outcomeL l ext o = .refused (.zeroThickness x)) ↔
        TopNonManifold (qs.map Quadric.eval) (Rset L (fun i => (F i).den (qs.map Quadric.eval)) (ruleL l.nb.val r))) ⦄

/-- **`EndToEndCurved`, proven**: `CoreSpec`, `coreM_nil`, then `curved_correct`, as
`end_to_end_of` is for flat. -/
theorem end_to_end_curved : EndToEndCurved := by
  intro qs F L hT l r hL hw
  apply WP.spec_mono (core_spec_proof l r ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ hw
    ⟨fun c hc => by simp at hc, fun c hc => by simp at hc⟩)
  rintro o ⟨ext, _, ho⟩
  have hthin : thinL l ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ = [] := by simp [thinL]
  have henc : enclosedL l ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ = [] := by
    simp [enclosedL]
  rw [hthin, henc, coreM_nil, hL] at ho
  obtain ⟨hok, hzt⟩ := curved_correct hT.1 (ruleL l.nb.val r)
  refine ⟨ext, fun b hb => ?_, ?_, ?_⟩
  · exact hok b (ho ▸ hb)
  · rw [ho]; exact hzt
  · rw [ho]; exact curved_correct_top (quadrics_cont qs) hT _

end forge_bool_core
