import ForgeExact.DecideOnce.QSing
import ForgeExact.DecideOnce.QEnum

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

/-- The carrier vanishes on the whole plane `x = c` (a plane `x = c` itself). -/
def zeroOn (q : Quadric) (c : Rat) : Bool :=
  let r := restr q c
  Poly.isZero r.1 && Poly.isZero r.2.1 && Poly.isZero r.2.2

/-- The carriers that do not vanish on the plane: the ones whose roots, turns and crossings there
are finite. -/
def activeOn (qs : List Quadric) (c : Rat) : List Quadric := qs.filter fun q => !zeroOn q c

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
  let roots ← ((activeOn qs c).mapM fun q => rootsOf (alongLine q P D) rootFuel).map List.flatten
  let Ez ← pickEvents roots
  let facets ← Ez.mapM fun n => covOn qs P D n signFuel
  let cells ← (planesOf Ez).mapM fun t => covOn qs P D (.rat t) signFuel
  pure ((facets ++ cells).map fun X => ⟨.line, X⟩)

def edgeSample (qs : List Quadric) (tg : Tag) (e : ESample) : Option QS :=
  if isoOkN e.y && e.dOk signFuel then (edgeCov qs e signFuel).map fun X => ⟨tg, X⟩ else none

/-- The samples of the sweep plane `x = c`. On a plane carrier `x = c` (`onEvent`), two other
carriers' traces cross at a point of three carriers, a vertex, and the crossing is tagged so. -/
def planeSamples (qs : List Quadric) (onEvent : Bool) (c : Rat) : Option (List QS) := do
  let es ← edgesOn ((activeOn qs c).filter fun q => !isXPlane q) c
  let turns ← turnsOn (activeOn qs c) c
  let Ey ← pickEvents (turns ++ es.map ESample.y)
  let lines ← (planesOf Ey).mapM (lineSamples qs c)
  let edges ← es.mapM (edgeSample qs (if onEvent then .vert else .edge))
  pure (lines.flatten ++ edges)

def numYQ (a b c : Quadric) : Rat :=
  -(a.lx * (b.c * c.lz - b.lz * c.c) - a.c * (b.lx * c.lz - b.lz * c.lx) +
    a.lz * (b.lx * c.c - b.c * c.lx))
def numZQ (a b c : Quadric) : Rat :=
  -(a.lx * (b.ly * c.c - b.c * c.ly) - a.ly * (b.lx * c.c - b.c * c.lx) +
    a.c * (b.lx * c.ly - b.ly * c.lx))

def vertSample (qs : List Quadric) (v : VSample) : Option QS :=
  if isoOkN v.x && v.dOk signFuel then (vertCov qs v signFuel).map fun X => ⟨.vert, X⟩ else none

/-- The common points of two planes and `q`, on the planes' line. -/
def lineVerts (qs : List Quadric) (a b q : Quadric) : Option (List QS) := do
  let L := lineOf a b
  if !lineOk a b L then none
  let ns ← rootsOf (alongLine q L.1 L.2.1) rootFuel
  if !ns.all isoOkN then none
  ns.mapM fun n => (covOn qs L.1 L.2.1 n signFuel).map fun X => ⟨.vert, X⟩

def lineTripleS (qs : List Quadric) (a b c : Quadric) : Option (List QS) :=
  if lineOk a b (lineOf a b) then lineVerts qs a b c
  else if lineOk a c (lineOf a c) then lineVerts qs a c b
  else if lineOk b c (lineOf b c) then lineVerts qs b c a
  else none

/-- The vertex samples of a triple. -/
def tripleSamples (qs : List Quadric) (a b c : Quadric) : Option (List QS) :=
  if (isConstQ a && a.c != 0) || (isConstQ b && b.c != 0) || (isConstQ c && c.c != 0) ||
    parallelApart a b || parallelApart a c || parallelApart b c then some []
  else if (lineTripleS qs a b c).isSome then lineTripleS qs a b c
  else if isPlaneQ a && isPlaneQ b && isPlaneQ c then
    let d := det3Q a b c
    if d != 0 then
      let P : V3 := ⟨numXQ a b c / d, numYQ a b c / d, numZQ a b c / d⟩
      (covOn qs P ⟨0, 0, 0⟩ (.rat 0) signFuel).map fun X => [⟨.vert, X⟩]
    else none
  else (proposeTriple a b c rootFuel).bind fun vs => vs.mapM (vertSample qs)

/-- The pairs with their turning-point quadric: their common points are the curves' turns in `x`
and the tangencies. -/
def critTriples (qs : List Quadric) : List (Quadric × Quadric × Quadric) :=
  (pairs qs).map fun p => (p.1, p.2, critQ p.1 p.2)

/-- The vertex samples of a triple: one per common point (`enumTriple`). -/
def enumSamples (qs : List Quadric) (t : Quadric × Quadric × Quadric) : Option (List QS) :=
  (enumTriple t.1 t.2.1 t.2.2 rootFuel).bind fun ds => ds.mapM fun d => (d.cov qs).map (⟨.vert, ·⟩)

/-- **The vertex samples**: every common point of three carriers, and every turn and tangency of
two (`critTriples`). -/
def vertSamples (qs : List Quadric) : Option (List QS) :=
  ((tri qs ++ critTriples qs).mapM (enumSamples qs)).map List.flatten

/-- The rational events: the planes `x = e` a plane carrier `x = e` lies in, sampled too, since
its cells lie in no other sweep plane. -/
def ratEvents (E : List Num) : List Rat := E.filterMap fun n => match n with
  | .rat q => some q
  | .iso _ => none

/-- **The frame certificate**: every carrier meets every line parallel to `z` in finitely many
points: a plane with a `z` term, or `zz ≠ 0`. Then no carrier's trace on a sweep plane has a
vertical line, so the rational lines `y = c'` meet every curve of it. No carrier is a plane
`x = const`, so no cell lies in a level plane: the tilted frames take axis-aligned planes. -/
def frameOk (qs : List Quadric) : Bool :=
  qs.all fun q => (isPlaneQ q && q.lz != 0) || q.zz != 0

/-- The samples in the frame the carriers are given in. -/
def samplesAt (qs : List Quadric) : Option (List QS) := do
  let E ← eventsOf qs rootFuel
  let ps ← (planesOf E).mapM (planeSamples qs false)
  let es ← ((ratEvents E).filter fun e => qs.any fun q => zeroOn q e).mapM (planeSamples qs true)
  let vs ← vertSamples qs
  pure (ps.flatten ++ es.flatten ++ vs)

/-- `q(x + μy + νz, y, z)`: tilts the sweep, so that no axis direction (a box's edges, a
cylinder's rulings) lies in a level plane. -/
def _root_.ForgeExact.Quad.Quadric.shearX (μ ν : Rat) (q : Quadric) : Quadric :=
  ⟨q.xx, q.yy + q.xx * μ * μ + q.xy * μ, q.zz + q.xx * ν * ν + q.xz * ν, q.xy + 2 * q.xx * μ,
   q.xz + 2 * q.xx * ν, q.yz + 2 * q.xx * μ * ν + q.xy * ν + q.xz * μ, q.lx, q.ly + q.lx * μ,
   q.lz + q.lx * ν, q.c⟩

/-- A frame: the sweep tilt `(μ, ν)` and the projection shear `λ`. -/
abbrev Frame := Rat × Rat × Rat

/-- The carrier in the frame: `q(x + μ(y + λz) + νz, y + λz, z)`. -/
def inFrame (f : Frame) (q : Quadric) : Quadric := (q.shearX f.1 f.2.1).shear f.2.2

/-- **The frames tried, in order.** Tilted ones first (no axis direction is level there), then
the untilted ones (where planes `x = c` are rational events). -/
def frames : List Frame :=
  [(1/3, 1/5, 1), (1/3, 1/5, 2), (2/7, 3/11, 1), (2/7, 3/11, 3), (3/13, 5/17, 1), (3/13, 5/17, 2)] ++
    shears.map (fun l => (0, 0, l))

/-- **The samples**: every sweep plane's, and the vertices, in the first frame of `frames` (a
tilted sweep and a projection shear) that passes `frameOk` and `singAll` and where every
step succeeds. The tilt
keeps every rational line and curve direction out of the level planes; the projection shear
separates two crossings of one sweep plane with one `y`. `none`:
undecided in every frame (fuel, or a degenerate input the proposers do not cover). Never a wrong
covector. -/
def samplesOfF (qs : List Quadric) : Option (Frame × List QS) :=
  frames.findSome? fun f =>
    if frameOk (qs.map (inFrame f)) && singAll (qs.map (inFrame f)) then
      (samplesAt (qs.map (inFrame f))).map (f, ·) else none

/-- The samples, without their frame. -/
def samplesOf (qs : List Quadric) : Option (List QS) := (samplesOfF qs).map Prod.snd

theorem samplesOfF_spec {qs : List Quadric} {f : Frame} {ss : List QS}
    (h : samplesOfF qs = some (f, ss)) :
    f ∈ frames ∧ frameOk (qs.map (inFrame f)) = true ∧ singAll (qs.map (inFrame f)) = true ∧
      samplesAt (qs.map (inFrame f)) = some ss := by
  unfold samplesOfF at h
  obtain ⟨f', hf', hl⟩ := List.exists_of_findSome?_eq_some h
  split at hl
  · rename_i hc
    simp only [Bool.and_eq_true] at hc
    simp only [Option.map_eq_some_iff, Prod.mk.injEq] at hl
    obtain ⟨ss', hss', rfl, rfl⟩ := hl
    exact ⟨hf', hc.1, hc.2, hss'⟩
  · cases hl

theorem samplesOf_some {qs : List Quadric} {ss : List QS} (h : samplesOf qs = some ss) :
    ∃ f, samplesOfF qs = some (f, ss) := by
  unfold samplesOf at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨⟨f, ss'⟩, h1, rfl⟩ := h
  exact ⟨f, h1⟩

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

theorem edgeSample_realised {qs : List Quadric} {tg : Tag} {e : ESample} {s : QS}
    (h : edgeSample qs tg e = some s) : RealCov M qs s.cov := by
  unfold edgeSample at h
  by_cases hc : (isoOkN e.y && e.dOk signFuel) = true
  · rw [if_pos hc] at h
    simp only [Bool.and_eq_true] at hc
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨X, hX, rfl⟩ := h
    obtain ⟨α, hα⟩ := denotes_of_isoOkN M hc.1
    exact ⟨_, edgeCov_spec M hα hc.2 qs X hX⟩
  · rw [if_neg hc] at h; cases h

theorem planeSamples_realised {qs : List Quadric} {b : Bool} {c : Rat} {ss : List QS}
    (h : planeSamples qs b c = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
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

theorem lineVerts_realised {qs : List Quadric} {a b q : Quadric} {ss : List QS}
    (h : lineVerts qs a b q = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold lineVerts at h
  simp only [Option.bind_eq_bind] at h
  split at h
  · cases h
  cases hr : rootsOf (alongLine q (lineOf a b).1 (lineOf a b).2.1) rootFuel with
  | none => rw [hr] at h; cases h
  | some ns =>
    rw [hr, Option.bind_some] at h
    split at h
    · cases h
    rename_i hok
    simp only [Bool.not_eq_true', Bool.not_eq_false, List.all_eq_true] at hok
    intro s hs
    obtain ⟨n, hn, hX⟩ := mapM_mem' h s hs
    simp only [Option.map_eq_some_iff] at hX
    obtain ⟨X, hX, rfl⟩ := hX
    obtain ⟨t, ht⟩ := denotes_of_isoOkN M (hok n hn)
    exact ⟨_, covOn_spec M _ _ ht _ qs X hX⟩

theorem lineTripleS_realised {qs : List Quadric} {a b c : Quadric} {ss : List QS}
    (h : lineTripleS qs a b c = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold lineTripleS at h
  split at h
  · exact lineVerts_realised M h
  split at h
  · exact lineVerts_realised M h
  split at h
  · exact lineVerts_realised M h
  · cases h

theorem tripleSamples_realised {qs : List Quadric} {a b c : Quadric} {ss : List QS}
    (h : tripleSamples qs a b c = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold tripleSamples at h
  split at h
  · cases h; intro s hs; nomatch hs
  split at h
  · exact lineTripleS_realised M h
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

theorem enumSamples_realised {qs : List Quadric} {t : Quadric × Quadric × Quadric}
    {ss : List QS} (h : enumSamples qs t = some ss) : ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold enumSamples at h
  simp only [Option.bind_eq_some_iff] at h
  obtain ⟨ds, -, hds⟩ := h
  intro s hs
  obtain ⟨d, -, hd⟩ := mapM_mem' hds s hs
  simp only [Option.map_eq_some_iff] at hd
  obtain ⟨X, hX, rfl⟩ := hd
  obtain ⟨p, hp⟩ := PtD.cov_at M hX
  exact ⟨p, PtD.cov_spec M hX hp⟩

theorem samplesAt_realised {qs : List Quadric} {ss : List QS} (h : samplesAt qs = some ss) :
    ∀ s ∈ ss, RealCov M qs s.cov := by
  unfold samplesAt at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨E, -, ps, hps, es, hes, vs, hvs, rfl⟩ := h
  intro s hs
  rcases List.mem_append.mp hs with hs | hs
  rcases List.mem_append.mp hs with hs | hs
  · obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨c, -, hc⟩ := mapM_mem' hps l hl
    exact planeSamples_realised M hc s hs
  · obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨c, -, hc⟩ := mapM_mem' hes l hl
    exact planeSamples_realised M hc s hs
  · unfold vertSamples at hvs
    simp only [Option.map_eq_some_iff] at hvs
    obtain ⟨ls, hls, rfl⟩ := hvs
    obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨t, -, ht⟩ := mapM_mem' hls l hl
    exact enumSamples_realised M ht s hs

/-- A covector realised for the sheared carriers is realised for the carriers. -/
theorem shearX_ev (μ ν : Rat) (q : Quadric) (x y z : K) :
    Quadric.evK M (q.shearX μ ν) (x, y, z) =
      Quadric.evK M q (x + M.q μ * y + M.q ν * z, y, z) := by
  simp only [Quadric.evK, Quadric.shearX, Model.q, M.ι.map_add, M.ι.map_mul]
  have h2 : M.ι.f 2 = 1 + 1 := by
    rw [show (2 : Rat) = 1 + 1 by grind, M.ι.map_add, M.ι.map_one]
  rw [h2]
  grind

theorem inFrame_ev (f : Frame) (q : Quadric) (x y z : K) :
    Quadric.evK M (inFrame f q) (x, y, z) =
      Quadric.evK M q (x + M.q f.1 * (y + M.q f.2.2 * z) + M.q f.2.1 * z, y + M.q f.2.2 * z, z) := by
  simp only [inFrame, shear_ev, shearX_ev]

theorem realCov_frame {qs : List Quadric} {f : Frame} {X : Cov}
    (h : RealCov M (qs.map (inFrame f)) X) : RealCov M qs X := by
  obtain ⟨⟨x, y, z⟩, rfl⟩ := h
  refine ⟨(x + M.q f.1 * (y + M.q f.2.2 * z) + M.q f.2.1 * z, y + M.q f.2.2 * z, z), ?_⟩
  simp only [covOfK, List.map_map]
  apply List.map_congr_left
  intro q _
  simp only [Function.comp, inFrame_ev]

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
  obtain ⟨f, hf⟩ := samplesOf_some h
  obtain ⟨-, -, -, hl⟩ := samplesOfF_spec hf
  intro s hs
  exact realCov_frame M (samplesAt_realised M hl s hs)

end Model

-- the unit sphere with y = 0 and z = 0: 27 cells (8 solid, 12 facets, 6 arcs and the vertex
-- covector (0, 0, 0) of (±1, 0, 0)), every one sampled, the vertex by a vertex sample
#guard (samplesOf [sphere1, planeY0, planeZ]).map (fun ss => (ss.map (·.cov)).eraseDups.length) =
  some 27
#guard (samplesOf [sphere1, planeY0, planeZ]).map
  (fun ss => ss.any fun s => s.tag == .vert && s.cov == [.zero, .zero, .zero]) = some true

end ForgeExact.DecideOnce.Curved
