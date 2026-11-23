import ForgeBoolCore.RefineFan
import ForgeExact.DecideOnce.Metric
import ForgeExact.DecideOnce.Sampler

/-!
# The remaining refinement obligations, as statements

Each remaining translated function's spec is a `def …Spec : Prop`, so this file builds without
`sorry` and the end-to-end theorem can be assembled from the Props before they are proven. The
groups that prove them are in `SPLIT.md`. **Edit this file only through the coordinator**: if a
proof needs a stronger hypothesis, send the change; do not weaken a statement locally.

Already proven (in `Refine*.lean`, standard axioms only), and usable as lemmas:

* the index, `cov`/`find`/`intern`, `split` (`split_spec`, `SplitOK`), `membership`
  (`membership_spec`), the classify check (`ckAll_eq`), the keep table (`keep_loop_spec`), the result
  table (`keep_res_spec`), `zeros`, `sort_u32`;
* `around` (`around_spec`), `between`/`edges_between` (`edges_between_spec`), `bad_vert`
  (`bad_vert_spec`), and `core_k`'s loops (`is_edge_spec`, `edge_scan_spec`, `vert_scan_spec`).

## The encoding

A Rust ledger is read as the Lean one by `toL` (each `CellId c` as `covOf cells[c]`, ids as their
values). A cell id of the core's own `Ext` (a side not in the ledger) is read by `covE l ext`.
Bodies and outcomes are read cell by cell (`bodyL`, `outcomeL`). A Rust `Rule` is the Lean rule
`ruleL nb r`. A keep vector indexed by tope position is the keep table `ktOf l keep`.

## The frontier (local ledger, spec §3a)

The core is unchanged: the region is the bridge's choice of which cells to list. `Valid` and
`Consistent` quantify only over listed pieces, topes, facets, edges and verts, so (b′) is (b) as
stated. The frontier cells are not listed, so no Prop here mentions them. (c′) is `Exact` over the
local arrangement, which `flat_correct` already covers once the local ledger is a `sampleLedger` of
the near carriers plus the six walls; F1 and F2 are geometry obligations outside this file (see
`SPLIT.md`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

open ForgeExact.DecideOnce (Cov KeepTab)

/-! ## Reading Rust values as Lean ones -/

-- `ruleL nb r` (a Rust rule as the Lean one) is in `RefineKeep.lean`.

/-- A keep vector (by tope position) as a keep table. -/
def ktOf (l : Ledger) (keep : List Bool) : KeepTab := (toL l).topes.zip keep

def faceL (l : Ledger) (ext : Ext) (f : OutFace) : ForgeExact.DecideOnce.OutFace where
  facets := f.facets.val.map fun c => covE l ext (c : U32).val
  plane := f.plane.val
  orient := toS f.orient
  origin := (f.origin : U32).val

def edgeL (l : Ledger) (ext : Ext) (e : OutEdge) : ForgeExact.DecideOnce.OutEdge where
  cells := e.cells.val.map fun c => covE l ext (c : U32).val
  faces := e.faces.val.map fun c => covE l ext (c : U32).val

def bodyL (l : Ledger) (ext : Ext) (b : Body) : ForgeExact.DecideOnce.Body where
  faces := b.faces.val.map (faceL l ext)
  edges := b.edges.val.map (edgeL l ext)
  verts := b.verts.val.map fun c => covE l ext (c : U32).val

def outcomeL (l : Ledger) (ext : Ext) : Outcome → ForgeExact.DecideOnce.Outcome
  | .Ok b => .ok (bodyL l ext b)
  | .Empty => .empty
  | .Refused .SplitCrossed => .refused .splitCrossed
  | .Refused .InconsistentClassification => .refused .inconsistentClassification
  | .Refused (.ZeroThickness c) => .refused (.zeroThickness (covE l ext (c : U32).val))

/-- The metric ledger's cells as covectors. -/
def thinL (l : Ledger) (m : MetricLedger) : List Cov := m.thin.val.map fun c => cellL l (c : U32).val
def enclosedL (l : Ledger) (m : MetricLedger) : List Cov := m.enclosed.val.map fun c => cellL l (c : U32).val

/-! ## Hypotheses on the inputs (the stated size limits) -/

/-- A well-formed Rust ledger: distinct cells, ids in range, and room for every id below `NONE`. -/
structure LedgerWF (l : Ledger) : Prop where
  /-- at least the target body -/
  nb_pos : 0 < l.nb.val
  cells_nodup : (l.cells.val.map (·.val)).Nodup
  pieces : ∀ q ∈ pcs l, q.1 < l.cells.val.length ∧ q.2 < U32.max
  inside : InsideWF l
  edges : ∀ e ∈ l.edges.val, (e : U32).val < l.cells.val.length
  /-- the edge cells are distinct (`unify`'s edge positions need it) -/
  edges_nodup : (idsOf l.edges).Nodup
  verts : ∀ v ∈ l.verts.val, (v : U32).val < l.cells.val.length
  /-- cells, plus the at most two new sides per piece, stay below `u32::MAX` (= `NONE`) -/
  room : l.cells.val.length + 2 * l.pieces.val.length + 1 < U32.max
  lists : l.edges.val.length < U32.max ∧ l.verts.val.length < U32.max
  /-- the merge sorts' scratch (`3·n`) fits `usize` -/
  sort_room : 3 * (l.cells.val.length + 2 * l.pieces.val.length + 1) ≤ Usize.max

/-- Every facet's two sides are cells of the ledger (the bridge's precondition): then the core
adds no cell of its own, and every id it returns is a ledger cell. -/
def SidesListed (l : Ledger) : Prop :=
  ∀ φ ∈ (toL l).facets, φ.plus ∈ l.cells.val.map (fun c => c.val.map toS) ∧
    φ.minus ∈ l.cells.val.map (fun c => c.val.map toS)

structure MetricWF (l : Ledger) (m : MetricLedger) : Prop where
  thin : ∀ c ∈ m.thin.val, (c : U32).val < l.cells.val.length
  enclosed : ∀ c ∈ m.enclosed.val, (c : U32).val < l.cells.val.length

/-- The assembled result as `core_k`'s first loop leaves it (`keep_res_spec`). -/
def AsmOf (l : Ledger) (ext : Ext) (s : Split) (keep : List Bool) (a : Assembled) : Prop :=
  TInv a.rpos.val a.result.val (resIds s keep s.facets.val.length) (allCells l ext).length ∧
  a.orient.val = (resIdx s keep s.facets.val.length).map (orientOf s keep)

/-- `is_edge` marks the edge cells (`is_edge_spec`). -/
def MarksEdges (l : Ledger) (ext : Ext) (isE : List Bool) : Prop :=
  isE.length = (allCells l ext).length ∧
  ∀ c, c < isE.length → isE.getD c false = decide (c ∈ idsOf l.edges)

/-- The state after `split` on a well-formed ledger, which every later stage is given. -/
structure AfterSplit (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) : Prop where
  wf : LedgerWF l
  /-- `split_ok` passed (the core splits only then): every piece cell is a facet of its face's class -/
  ok : (toL l).splitOk = true
  ext0 : ExtOK l e0
  extra0 : e0.extra.val = []
  split : SplitOK l e0 s ext

/-! ## Group S: `split_ok` -/

/-- `split_ok` is the Lean `splitOk` (no two facets of one carrier crossing). Needs the `le3`
translation (no tuple `PartialOrd` axiom). -/
def SplitOkSpec : Prop :=
  ∀ (l : Ledger) (ext : Ext), LedgerWF l → ExtOK l ext → ext.extra.val = [] →
    split_ok l ext ⦃ b => b = (toL l).splitOk ⦄

/-! ## Group K: `collapse` (stage 4b) -/

/-- `collapse` is the Lean `collapse` on the keep table: a refusal at the same tope, or the same
collapsed keep table. -/
def CollapseSpec : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : Slice Bool) (m : MetricLedger),
    AfterSplit l e0 s ext → MetricWF l m → keep.val.length = s.topes.val.length →
    collapse l ext s keep m ⦃ r =>
      match (toL l).collapse (ktOf l keep.val) (thinL l m) (enclosedL l m) with
      | .refuse T => r.1 ≠ NONE ∧ r.1.val < (allCells l ext).length ∧ covE l ext r.1.val = T
      | .ok kt' => r.1 = NONE ∧ r.2.val.length = s.topes.val.length ∧ ktOf l r.2.val = kt' ⦄

/-! ## Group U: `unify` (with `faces_at`, `same_set`, `has`) -/

/-- `unify`, past the touch rule, writes the Lean `unify`'s body. -/
def UnifySpec : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : Slice Bool) (a : Assembled)
    (isE : Slice Bool) (ae av : Slice (alloc.vec.Vec U32)),
    AfterSplit l e0 s ext → keep.val.length = s.topes.val.length → AsmOf l ext s keep.val a →
    a.result.val ≠ [] →
    MarksEdges l ext isE.val →
    ae.val.map (fun v => v.val.map (·.val)) = (idsOf l.edges).map (fun e => aroundL l ext a (covAt l ext e)) →
    av.val.map (fun v => v.val.map (·.val)) = (idsOf l.verts).map (fun v => aroundL l ext a (covAt l ext v)) →
    (toL l).edges.find? ((toL l).badEdge (ktOf l keep.val)) = none →
    (toL l).verts.find? ((toL l).badVert (ktOf l keep.val)) = none →
    unify l ext s a isE ae av ⦃ b => bodyL l ext b = (toL l).unify (ktOf l keep.val) ⦄

/-! ## Group R: from result positions to the Lean's covectors (pure Lean, no `step`) -/

/-- The result table is the Lean's result, in order and oriented, and `around` may read it. -/
def ResultBridge : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : List Bool) (a : Assembled),
    AfterSplit l e0 s ext → keep.length = s.topes.val.length → AsmOf l ext s keep a →
    a.result.val.map (fun r => covE l ext r.val) = (toL l).result (ktOf l keep) ∧
    a.orient.val.map toS = ((toL l).result (ktOf l keep)).map (ForgeExact.DecideOnce.Ledger.orient (ktOf l keep)) ∧
    ∃ W, ResOK l ext a W

/-- `around` (positions) read as covectors is the Lean `around`. -/
def AroundBridge : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : List Bool) (a : Assembled),
    AfterSplit l e0 s ext → keep.length = s.topes.val.length → AsmOf l ext s keep a →
    ∀ x, x < (allCells l ext).length →
      (aroundL l ext a (covAt l ext x)).map (fun p => covE l ext (resAt a p)) =
        (toL l).around (ktOf l keep) (covE l ext x)

/-- The edge test on ids is the Lean `badEdge`. -/
def BadEdgeBridge : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : List Bool) (a : Assembled),
    AfterSplit l e0 s ext → keep.length = s.topes.val.length → AsmOf l ext s keep a →
    ∀ e ∈ idsOf l.edges, badE l ext a e = (toL l).badEdge (ktOf l keep) (covE l ext e)

/-- The fan test on ids (`bad_vert`, through `cls` on positions) is the Lean `badVert`. -/
def BadVertBridge : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : List Bool) (a : Assembled) (isE : List Bool),
    AfterSplit l e0 s ext → keep.length = s.topes.val.length → AsmOf l ext s keep a →
    MarksEdges l ext isE →
    ∀ v ∈ idsOf l.verts, badV l ext a isE v = (toL l).badVert (ktOf l keep) (covE l ext v)

/-! ## Group E: the glue and the end-to-end theorem (coordinator) -/

def NewExtSpec : Prop :=
  ∀ (l : Ledger), LedgerWF l → new_ext l ⦃ e => ExtOK l e ∧ e.extra.val = [] ⦄

/-- Stages 5–8 on a keep vector. -/
def CoreKSpec : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : Slice Bool),
    AfterSplit l e0 s ext → keep.val.length = s.topes.val.length →
    core_k l ext s keep ⦃ o => outcomeL l ext o = (toL l).coreK (ktOf l keep.val) ⦄

/-- Classification on: the consistency check, the keep table, the collapse, stages 5–8. -/
def ClassifyOnSpec : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (r : Rule) (m : MetricLedger),
    AfterSplit l e0 s ext → MetricWF l m →
    classify_on l ext s r m ⦃ o => outcomeL l ext o =
      if !(toL l).classifyOk then .refused .inconsistentClassification
      else match (toL l).collapse ((toL l).keepTab (ruleL l.nb.val r)) (thinL l m) (enclosedL l m) with
        | .refuse T => .refused (.zeroThickness T)
        | .ok kt' => (toL l).coreK kt' ⦄

/-- **The core refines the Lean core with the metric ledger.** The outcome is read through the
core's own `Ext` (its cells are the ledger's, then any side the ledger lacked). -/
def CoreSpec : Prop :=
  ∀ (l : Ledger) (r : Rule) (m : MetricLedger), LedgerWF l → MetricWF l m →
    core l r m ⦃ o => ∃ ext, ExtOK l ext ∧
      outcomeL l ext o = (toL l).coreM (ruleL l.nb.val r) (thinL l m) (enclosedL l m) ⦄

/-- With every side listed, `split` adds no cell, so every id the core returns is a ledger cell
(optional: the bridge's reading of ids; `EndToEnd` reads through `covE` and does not need it). -/
def NoExtraSpec : Prop :=
  ∀ (l : Ledger) (e0 : Ext), LedgerWF l → ExtOK l e0 → e0.extra.val = [] → SidesListed l →
    split l e0 ⦃ r => r.2.extra.val = [] ⦄

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Sample ForgeExact.DecideOnce.Sampler ForgeExact.Ref in
/-- **End to end.** On a Rust ledger that reads as the sampler's ledger of a flat input, with no
thin cells, `forge_bool_core::core`'s answer is the exact regularised boolean: an `Ok` body is valid,
its faces are exactly the boundary facets, and its kept cells' union is `reg` of the boolean; a
`ZeroThickness` refusal is exactly non-manifoldness. -/
def EndToEnd : Prop :=
  ∀ (ps : List Plane) (ss : List Solid) (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss)
    (l : forge_bool_core.Ledger) (r : forge_bool_core.Rule),
    toL l = sampleLedger ps ss (sampler ps) → LedgerWF l →
    core l r ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ ⦃ o => ∃ ext,
      (∀ b, outcomeL l ext o = .ok b →
        ForgeExact.DecideOnce.Valid (sampleLedger ps ss (sampler ps))
          ((sampleLedger ps ss (sampler ps)).keepTab (ruleL l.nb.val r)) b ∧
        (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔
          XBoundary (sampleLedger ps ss (sampler ps)) ps (brepA ss) (ruleL l.nb.val r) φ) ∧
        reg ((decomp (flat_exact hnz hi)).kept fun T =>
            xkeep (sampleLedger ps ss (sampler ps)) ps (brepA ss) (ruleL l.nb.val r) T = true) =
          reg (combine (ruleOp (ruleL l.nb.val r)) (opnds (sampleLedger ps ss (sampler ps)) (brepA ss)))) ∧
      ((∃ x, outcomeL l ext o = .refused (.zeroThickness x)) ↔
        XNonManifold (sampleLedger ps ss (sampler ps)) ps (brepA ss) (ruleL l.nb.val r)) ⦄

/-- A Lean ledger small enough for `u32` ids: a target body, every id and covector width below
`u32::MAX`, and the lists small enough that the cells (pieces, their sides, inside, edges, verts)
plus two new sides per piece, and the sorts' `3·n` scratch, stay below `u32::MAX`. -/
structure LFits (L : ForgeExact.DecideOnce.Ledger) : Prop where
  nb : 0 < L.nb ∧ L.nb < U32.max
  faces : L.faceBody.length < U32.max ∧ L.faceCls.length < U32.max ∧
    (∀ b ∈ L.faceBody, b < U32.max) ∧ (∀ c ∈ L.faceCls, c < U32.max)
  ids : (∀ p ∈ L.pieces, p.2 < U32.max) ∧ (∀ q ∈ L.inside, q.2 < U32.max)
  edges_nodup : L.edges.Nodup
  width : ∀ X ∈ L.pieces.map Prod.fst ++ L.inside.map Prod.fst ++ L.edges ++ L.verts, X.length < U32.max
  size : 32 * (L.pieces.length + L.inside.length + L.edges.length + L.verts.length + 1) < U32.max

/-- A Lean ledger within the limits, every facet's sides among its cells, has a Rust ledger reading
as it. (Its cells: the distinct covectors of pieces, sides, inside, edges and verts.) -/
def EncodeSpec : Prop :=
  ∀ (L : ForgeExact.DecideOnce.Ledger), LFits L → ∃ l : Ledger, toL l = L ∧ LedgerWF l ∧ SidesListed l

end forge_bool_core
