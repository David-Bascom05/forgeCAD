import ForgeExact.DecideOnce.QSweep

/-!
# Q1 L2: the samples (plan §2, module 5)

On each rational sweep plane `x = c` (`planesOf (eventsOf qs)`, QSweep.lean):

* **y-events** of the plane: every carrier's turning points over `y` (the roots of its
  discriminant in `z`, or of its `z`-coefficient where it is linear in `z`, or of the whole
  restriction where it is free of `z`), and the `y` of every crossing of two carriers
  (`proposeEdgeAny`, whose samples are the **edge samples**);
* rational **lines** `y = c'` between the y-events (`planesOf` again);
* on each line, the **facet samples** are the roots in `z` of every carrier (separated and checked
  by `pickEvents`), and the **cell samples** are rationals between them.

The **vertex samples** are the common points of every triple of carriers (`proposeTriple`, and
Cramer's point for three planes).

Every sample carries its covector, decided exactly (`covOn`, `edgeCov`, `vertCov`) and a tag
(`line`, `edge`, `vert`), which the ledger's dimension rule reads.

* **`samplesOf_realised`**: every sample's covector is the covector of a point of `K³`, in every
  model of the reals. Every cell the ledger lists is therefore realised.

Completeness (`qsampler_covers`: every realised cell in a carrier is some sample's) is the next
step, against the sweep's delineability (LemmaD).
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-- Where a sample came from: a point of a rational line, a crossing of two carriers on a sweep
plane, or a common point of three carriers. -/
inductive Tag | line | edge | vert
  deriving DecidableEq, Repr

/-- A sample: its tag and its exact covector. -/
structure QS where
  tag : Tag
  cov : Cov
  deriving DecidableEq, Repr

/-- The `y` where a carrier's trace on `x = c` turns, or meets `z = ±∞`, or is a line `y = const`. -/
def yTurn (q : Quadric) (c : Rat) : Poly :=
  let r := restr q c
  if q.zz != 0 then psub (pmul r.2.1 r.2.1) (pscale (4 * q.zz) r.1)
  else if !Poly.isZero r.2.1 then r.2.1
  else r.1

def turnsOn (qs : List Quadric) (c : Rat) : Option (List Num) :=
  (qs.mapM fun q => rootsOf (yTurn q c) rootFuel).map List.flatten

def edgesOn (qs : List Quadric) (c : Rat) : Option (List ESample) :=
  ((pairs qs).mapM fun (p : Quadric × Quadric) => proposeEdgeAny p.1 p.2 c rootFuel).map
    List.flatten

def isoOkB (n : Num) : Bool := isoOkN n

/-- The samples of the line `(c, c', t)`. -/
def lineSamples (qs : List Quadric) (c c' : Rat) : Option (List QS) := do
  let P : V3 := ⟨c, c', 0⟩
  let D : V3 := ⟨0, 0, 1⟩
  let roots ← (qs.mapM fun q => rootsOf (alongLine q P D) rootFuel).map List.flatten
  let Ez ← pickEvents roots
  let facets ← Ez.mapM fun n => covOn qs P D n signFuel
  let cells ← (planesOf Ez).mapM fun t => covOn qs P D (.rat t) signFuel
  pure ((facets ++ cells).map fun X => ⟨.line, X⟩)

def edgeSample (qs : List Quadric) (e : ESample) : Option QS :=
  if isoOkN e.y && e.dOk signFuel then (edgeCov qs e signFuel).map fun X => ⟨.edge, X⟩ else none

/-- The samples of the sweep plane `x = c`. -/
def planeSamples (qs : List Quadric) (c : Rat) : Option (List QS) := do
  let es ← edgesOn qs c
  let turns ← turnsOn qs c
  let Ey ← pickEvents (turns ++ es.map ESample.y)
  let lines ← (planesOf Ey).mapM (lineSamples qs c)
  let edges ← es.mapM (edgeSample qs)
  pure (lines.flatten ++ edges)

def numYQ (a b c : Quadric) : Rat :=
  -(a.lx * (b.c * c.lz - b.lz * c.c) - a.c * (b.lx * c.lz - b.lz * c.lx) +
    a.lz * (b.lx * c.c - b.c * c.lx))
def numZQ (a b c : Quadric) : Rat :=
  -(a.lx * (b.ly * c.c - b.c * c.ly) - a.ly * (b.lx * c.c - b.c * c.lx) +
    a.c * (b.lx * c.ly - b.ly * c.lx))

def vertSample (qs : List Quadric) (v : VSample) : Option QS :=
  if isoOkN v.x && v.dOk signFuel then (vertCov qs v signFuel).map fun X => ⟨.vert, X⟩ else none

/-- The vertex samples of a triple. -/
def tripleSamples (qs : List Quadric) (a b c : Quadric) : Option (List QS) :=
  if (isConstQ a && a.c != 0) || (isConstQ b && b.c != 0) || (isConstQ c && c.c != 0) then some []
  else if isPlaneQ a && isPlaneQ b && isPlaneQ c then
    let d := det3Q a b c
    if d != 0 then
      let P : V3 := ⟨numXQ a b c / d, numYQ a b c / d, numZQ a b c / d⟩
      (covOn qs P ⟨0, 0, 0⟩ (.rat 0) signFuel).map fun X => [⟨.vert, X⟩]
    else none
  else (proposeTriple a b c rootFuel).bind fun vs => vs.mapM (vertSample qs)

def vertSamples (qs : List Quadric) : Option (List QS) :=
  ((tri qs).mapM fun (t : Quadric × Quadric × Quadric) => tripleSamples qs t.1 t.2.1 t.2.2).map
    List.flatten

/-- The samples in the frame the carriers are given in. -/
def samplesAt (qs : List Quadric) : Option (List QS) := do
  let E ← eventsOf qs rootFuel
  let ps ← (planesOf E).mapM (planeSamples qs)
  let vs ← vertSamples qs
  pure (ps.flatten ++ vs)

/-- **The samples**: every sweep plane's, and the vertices, in the first sheared frame
`(x, y + λz, z)` (`shears`, QPropose.lean) where every step succeeds. Two crossings on one sweep
plane with one `y` need the shear. The shear keeps `x`, so the sweep is the same one. `none`:
undecided in every frame (fuel, or a degenerate input the proposers do not cover). Never a wrong
covector. -/
def samplesOf (qs : List Quadric) : Option (List QS) :=
  shears.findSome? fun l => samplesAt (qs.map (Quadric.shear l))

/-! ## Every sample is realised -/

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K) [DecidableRel (α := K) (· < ·)]

/-- `X` is the covector of a point. -/
def RealCov (qs : List Quadric) (X : Cov) : Prop :=
  ∃ p : PK K, X = covOfK (qs.map fun q => Quadric.evK M q) p

theorem denotes_of_isoOkN {n : Num} (h : isoOkN n = true) : ∃ t, Denotes M n t := by
  cases n with
  | rat q => exact ⟨M.q q, rfl⟩
  | iso r => exact Iso.denotes_of_ok M r h

theorem mapM_mem' {α β : Type} {f : α → Option β} {l : List α} {ls : List β}
    (h : l.mapM f = some ls) : ∀ b ∈ ls, ∃ a ∈ l, f a = some b := by
  induction l generalizing ls with
  | nil => simp at h; subst h; simp
  | cons x xs ih =>
    simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at h
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    intro b hb
    rcases List.mem_cons.mp hb with rfl | hb
    · exact ⟨x, List.mem_cons_self .., hr⟩
    · obtain ⟨a, ha, hfa⟩ := ih hrs b hb
      exact ⟨a, List.mem_cons_of_mem _ ha, hfa⟩

theorem lineSamples_realised {qs : List Quadric} {c c' : Rat} {ss : List QS}
    (h : lineSamples qs c c' = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold lineSamples at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.map_eq_some_iff, Option.some.injEq] at h
  obtain ⟨roots, -, Ez, hEz, facets, hf, cells, hc, rfl⟩ := h
  have hck := pickEvents_spec hEz
  simp only [checkEvents, Bool.and_eq_true, List.all_eq_true] at hck
  intro s hs
  obtain ⟨X, hX, rfl⟩ := List.mem_map.mp hs
  rcases List.mem_append.mp hX with hX | hX
  · obtain ⟨n, hn, hcov⟩ := mapM_mem' hf X hX
    obtain ⟨t, ht⟩ := denotes_of_isoOkN M (hck.1.2 n hn)
    exact ⟨_, covOn_spec M _ _ ht _ qs X hcov⟩
  · obtain ⟨t, -, hcov⟩ := mapM_mem' hc X hX
    exact ⟨_, covOn_spec M _ _ (show Denotes M (.rat t) (M.q t) from rfl) _ qs X hcov⟩

theorem edgeSample_realised {qs : List Quadric} {e : ESample} {s : QS}
    (h : edgeSample qs e = some s) : RealCov M qs s.cov := by
  unfold edgeSample at h
  by_cases hc : (isoOkN e.y && e.dOk signFuel) = true
  · rw [if_pos hc] at h
    simp only [Bool.and_eq_true] at hc
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨X, hX, rfl⟩ := h
    obtain ⟨α, hα⟩ := denotes_of_isoOkN M hc.1
    exact ⟨_, edgeCov_spec M hα hc.2 qs X hX⟩
  · rw [if_neg hc] at h; cases h

theorem planeSamples_realised {qs : List Quadric} {c : Rat} {ss : List QS}
    (h : planeSamples qs c = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold planeSamples at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨es, -, turns, -, Ey, -, lines, hl, edges, he, rfl⟩ := h
  intro s hs
  rcases List.mem_append.mp hs with hs | hs
  · obtain ⟨l, hl', hs⟩ := List.mem_flatten.mp hs
    obtain ⟨c', -, hc'⟩ := mapM_mem' hl l hl'
    exact lineSamples_realised M hc' s hs
  · obtain ⟨e, -, he'⟩ := mapM_mem' he s hs
    exact edgeSample_realised M he'

theorem vertSample_realised {qs : List Quadric} {v : VSample} {s : QS}
    (h : vertSample qs v = some s) : RealCov M qs s.cov := by
  unfold vertSample at h
  by_cases hc : (isoOkN v.x && v.dOk signFuel) = true
  · rw [if_pos hc] at h
    simp only [Bool.and_eq_true] at hc
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨X, hX, rfl⟩ := h
    obtain ⟨α, hα⟩ := denotes_of_isoOkN M hc.1
    exact ⟨_, vertCov_spec M hα hc.2 qs X hX⟩
  · rw [if_neg hc] at h; cases h

theorem tripleSamples_realised {qs : List Quadric} {a b c : Quadric} {ss : List QS}
    (h : tripleSamples qs a b c = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold tripleSamples at h
  split at h
  · cases h; intro s hs; nomatch hs
  split at h
  · dsimp only at h
    split at h
    · simp only [Option.map_eq_some_iff] at h
      obtain ⟨X, hX, rfl⟩ := h
      intro s hs
      simp only [List.mem_singleton] at hs
      subst hs
      exact ⟨_, covOn_spec M _ _ (show Denotes M (.rat 0) (M.q 0) from rfl) _ qs X hX⟩
    · cases h
  · simp only [Option.bind_eq_some_iff] at h
    obtain ⟨vs, -, hvs⟩ := h
    intro s hs
    obtain ⟨v, -, hv⟩ := mapM_mem' hvs s hs
    exact vertSample_realised M hv

theorem samplesAt_realised {qs : List Quadric} {ss : List QS} (h : samplesAt qs = some ss) :
    ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold samplesAt at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨E, -, ps, hps, vs, hvs, rfl⟩ := h
  intro s hs
  rcases List.mem_append.mp hs with hs | hs
  · obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨c, -, hc⟩ := mapM_mem' hps l hl
    exact planeSamples_realised M hc s hs
  · unfold vertSamples at hvs
    simp only [Option.map_eq_some_iff] at hvs
    obtain ⟨ls, hls, rfl⟩ := hvs
    obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨t, -, ht⟩ := mapM_mem' hls l hl
    exact tripleSamples_realised M ht s hs

/-- A covector realised for the sheared carriers is realised for the carriers. -/
theorem realCov_shear {qs : List Quadric} {l : Rat} {X : Cov}
    (h : RealCov M (qs.map (Quadric.shear l)) X) : RealCov M qs X := by
  obtain ⟨⟨x, y, z⟩, rfl⟩ := h
  refine ⟨(x, y + M.q l * z, z), ?_⟩
  simp only [covOfK, List.map_map]
  apply List.map_congr_left
  intro q _
  simp only [Function.comp, shear_ev]

/-- **Every sample's covector is realised**, in every model of the reals. -/
theorem samplesOf_realised {qs : List Quadric} {ss : List QS} (h : samplesOf qs = some ss) :
    ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold samplesOf at h
  obtain ⟨l, -, hl⟩ := List.exists_of_findSome?_eq_some h
  intro s hs
  exact realCov_shear M (samplesAt_realised M hl s hs)

end Model

-- the unit sphere with y = 0 and z = 0: 27 cells (8 solid, 12 facets, 6 arcs and the vertex
-- covector (0, 0, 0) of (±1, 0, 0)), every one sampled, the vertex by a vertex sample
#guard (samplesOf [sphere1, planeY0, planeZ]).map (fun ss => (ss.map (·.cov)).eraseDups.length) =
  some 27
#guard (samplesOf [sphere1, planeY0, planeZ]).map
  (fun ss => ss.any fun s => s.tag == .vert && s.cov == [.zero, .zero, .zero]) = some true

end ForgeExact.DecideOnce.Curved
