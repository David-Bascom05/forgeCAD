import ForgeExact.DecideOnce.QSample

/-!
# Q1 L2: the samples' completeness, the exact half

The pieces of `qsampler_covers` that are exact algebra, in every model of the reals:

* `pickEvents_cover`: every raw candidate is matched by a separated event;
* `yTurn_root`, `turnsOn_complete`: a zero of a carrier on the plane `x = c` where `∂_z q = 0` has
  its `y` among the turn events;
* `edgesOn_complete`: a common zero of two carriers on the plane has its `y` among the crossing
  events;
* **`lineSamples_complete`**: every zero of an active carrier on the line `(c, c', t)` is a facet
  sample's point, and the sample's covector is the point's;
* `planeSamples_parts`, `samplesAt_parts`: the samples hold every plane's and every line's.

The geometric half (that a facet reaches these lines) is forge-curved's.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef ForgeExact.DecideOnce
  Lean.Grind

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_isZero {p : Poly} (h : Poly.isZero p = true) (y : K) : M.ev p y = 0 := by
  induction p with
  | nil => simp [Model.ev]
  | cons a p ih =>
    simp only [Poly.isZero, List.all_cons, Bool.and_eq_true, beq_iff_eq] at h
    simp only [Model.ev, evalWith_cons] at ih ⊢
    rw [ih h.2, h.1]
    simp only [M.ι.map_zero]; grind

theorem restr_ev (q : Quadric) (c : Rat) (y z : K) :
    Quadric.evK M q (M.q c, y, z) =
      M.ev (restr q c).1 y + M.ev (restr q c).2.1 y * z + M.q q.zz * z * z := by
  simp only [restr, Model.ev, evalWith_cons, evalWith_nil, Quadric.evK, Model.q_add, Model.q_mul]
  simp only [Model.q]; grind

theorem dzK_plane (q : Quadric) (c : Rat) (y z : K) :
    dzK M q (M.q c, y, z) = M.ev (restr q c).2.1 y + (1 + 1) * M.q q.zz * z := by
  simp only [dzK, restr, Model.ev, evalWith_cons, evalWith_nil, Model.q_add, Model.q_mul]
  simp only [Model.q]; grind

/-- **A zero where `∂_z q = 0` is a root of `yTurn`.** -/
theorem yTurn_root (q : Quadric) (c : Rat) {y z : K} (h0 : Quadric.evK M q (M.q c, y, z) = 0)
    (hz : dzK M q (M.q c, y, z) = 0) : M.ev (yTurn q c) y = 0 := by
  rw [restr_ev] at h0
  rw [dzK_plane] at hz
  unfold yTurn
  dsimp only
  by_cases hzz : (q.zz != 0) = true
  · rw [if_pos hzz]
    simp only [Model.ev, eval_psub, eval_pmul, eval_pscale] at h0 hz ⊢
    simp only [Model.q] at h0 hz
    rw [show M.ι.f (4 * q.zz) = (1 + 1) * (1 + 1) * M.ι.f q.zz by
      rw [M.ι.map_mul, show (4 : Rat) = (1 + 1) * (1 + 1) by grind, M.ι.map_mul, M.ι.map_add,
        M.ι.map_one]]
    generalize evalWith M.ι (restr q c).1 y = A at *
    generalize evalWith M.ι (restr q c).2.1 y = B at *
    generalize M.ι.f q.zz = Z at *
    have hB : B = -((1 + 1) * Z * z) := by grind
    subst hB
    grind
  · rw [if_neg hzz]
    simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hzz
    rw [hzz, Model.q_zero] at h0 hz
    by_cases hb : (!Poly.isZero (restr q c).2.1) = true
    · rw [if_pos hb]; grind
    · rw [if_neg hb]
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hb
      rw [ev_isZero M hb] at h0
      grind

theorem turnsOn_complete {qs : List Quadric} {c : Rat} {ts : List Num}
    (h : turnsOn qs c = some ts) {q : Quadric} (hq : q ∈ qs) {y z : K}
    (h0 : Quadric.evK M q (M.q c, y, z) = 0) (hz : dzK M q (M.q c, y, z) = 0) :
    ∃ n ∈ ts, Denotes M n y := by
  unfold turnsOn at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨ls, hls, rfl⟩ := h
  obtain ⟨r, hr, hrl⟩ := mapM_mem hls q hq
  obtain ⟨n, hn, hd⟩ := rootsOf_complete M hr (yTurn_root M q c h0 hz)
  exact ⟨n, List.mem_flatten.mpr ⟨r, hrl, hn⟩, hd⟩

theorem edgesOn_complete {qs : List Quadric} {c : Rat} {es : List ESample}
    (h : edgesOn qs c = some es) {a b : Quadric} (hab : (a, b) ∈ pairs qs) {y z : K}
    (ha : Quadric.evK M a (M.q c, y, z) = 0) (hb : Quadric.evK M b (M.q c, y, z) = 0) :
    ∃ e ∈ es, Denotes M e.y y := by
  unfold edgesOn at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨ls, hls, rfl⟩ := h
  obtain ⟨r, hr, hrl⟩ := mapM_mem hls (a, b) hab
  obtain ⟨e, he, hd, -⟩ := proposeEdgeAny_complete M hr ha hb
  exact ⟨e, List.mem_flatten.mpr ⟨r, hrl, he⟩, hd⟩

variable [DecidableRel (α := K) (· < ·)]

/-- **Every raw candidate is matched by a separated event.** -/
theorem pickEvents_cover {raw E : List Num} (h : pickEvents raw = some E) {n : Num}
    (hn : n ∈ raw) {t : K} (ht : Denotes M n t) : ∃ e ∈ E, Denotes M e t := by
  have hck := pickEvents_spec h
  simp only [checkEvents, Bool.and_eq_true, List.all_eq_true, List.any_eq_true,
    beq_iff_eq] at hck
  obtain ⟨⟨hcov, hiso⟩, _⟩ := hck
  obtain ⟨e, he, hcmp⟩ := hcov n hn
  obtain ⟨u, hu⟩ := denotes_of_isoOkN M (hiso e he)
  have := cmp_spec M 200 ht hu hcmp
  simp only [CmpIs] at this
  subst this
  exact ⟨e, he, hu⟩


/-- **Every zero of an active carrier on a sample line is a facet sample.** -/
theorem lineSamples_complete {qs : List Quadric} {c c' : Rat} {ls : List QS}
    (h : lineSamples qs c c' = some ls) {q : Quadric} (hq : q ∈ activeOn qs c) {t : K}
    (h0 : Quadric.evK M q (M.q c, M.q c', t) = 0) :
    ∃ s ∈ ls, s.cov = covOfK (qs.map fun q => Quadric.evK M q) (M.q c, M.q c', t) := by
  unfold lineSamples at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.map_eq_some_iff, Option.some.injEq] at h
  obtain ⟨roots, ⟨rs, hrs, rfl⟩, Ez, hEz, facets, hf, cells, -, rfl⟩ := h
  have hpt : linePt M ⟨c, c', 0⟩ ⟨0, 0, 1⟩ t = (M.q c, M.q c', t) := by
    simp only [linePt, Model.q_zero, Model.q_one, Prod.mk.injEq]; grind
  obtain ⟨r, hr, hrl⟩ := mapM_mem hrs q hq
  have hroot : M.ev (alongLine q ⟨c, c', 0⟩ ⟨0, 0, 1⟩) t = 0 := by
    rw [alongLine_ev, hpt]; exact h0
  obtain ⟨n, hn, hd⟩ := rootsOf_complete M hr hroot
  obtain ⟨e, he, hde⟩ := pickEvents_cover M hEz (List.mem_flatten.mpr ⟨r, hrl, hn⟩) hd
  obtain ⟨X, hX, hXf⟩ := mapM_mem hf e he
  refine ⟨⟨.line, X⟩, List.mem_map.mpr ⟨X, List.mem_append_left _ hXf, rfl⟩, ?_⟩
  have := covOn_spec M _ _ hde _ qs X hX
  rw [hpt] at this
  exact this

end Model

theorem planeSamples_parts {qs : List Quadric} {c : Rat} {ss : List QS}
    (h : planeSamples qs false c = some ss) :
    ∃ es turns Ey, edgesOn ((activeOn qs c).filter fun q => !isXPlane q) c = some es ∧
      turnsOn (activeOn qs c) c = some turns ∧ pickEvents (turns ++ es.map ESample.y) = some Ey ∧
      ∀ c' ∈ planesOf Ey, ∃ ls, lineSamples qs c c' = some ls ∧ ∀ s ∈ ls, s ∈ ss := by
  unfold planeSamples at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨es, hes, turns, hturns, Ey, hEy, lines, hl, edges, -, rfl⟩ := h
  refine ⟨es, turns, Ey, hes, hturns, hEy, fun c' hc' => ?_⟩
  obtain ⟨ls, hls, hlm⟩ := mapM_mem hl c' hc'
  exact ⟨ls, hls, fun s hs => List.mem_append_left _ (List.mem_flatten.mpr ⟨ls, hlm, hs⟩)⟩

theorem samplesAt_parts {qs : List Quadric} {ss : List QS} (h : samplesAt qs = some ss) :
    ∃ E, eventsOf qs rootFuel = some E ∧
      ∀ c ∈ planesOf E, ∃ ps, planeSamples qs false c = some ps ∧ ∀ s ∈ ps, s ∈ ss := by
  unfold samplesAt at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨E, hE, ps, hps, es, -, vs, -, rfl⟩ := h
  refine ⟨E, hE, fun c hc => ?_⟩
  obtain ⟨p, hp, hpm⟩ := mapM_mem hps c hc
  exact ⟨p, hp, fun s hs =>
    List.mem_append_left _ (List.mem_append_left _ (List.mem_flatten.mpr ⟨p, hpm, hs⟩))⟩

/-- The frame `samplesOf` takes, and its certificates. -/
theorem samplesOf_frame {qs : List Quadric} {ss : List QS} (h : samplesOf qs = some ss) :
    ∃ f ∈ frames, frameOk (qs.map (inFrame f)) = true ∧ singAll (qs.map (inFrame f)) = true ∧
      samplesAt (qs.map (inFrame f)) = some ss := by
  unfold samplesOf at h
  obtain ⟨f, hf, hl⟩ := List.exists_of_findSome?_eq_some h
  split at hl
  · rename_i hc
    simp only [Bool.and_eq_true] at hc
    exact ⟨f, hf, hc.1, hc.2, hl⟩
  · cases hl

end ForgeExact.DecideOnce.Curved
