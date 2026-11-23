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

theorem edgesOn_complete2 {qs : List Quadric} {c : Rat} {es : List ESample}
    (h : edgesOn qs c = some es) {a b : Quadric} (hab : (a, b) ∈ pairs qs) {y z : K}
    (ha : Quadric.evK M a (M.q c, y, z) = 0) (hb : Quadric.evK M b (M.q c, y, z) = 0) :
    ∃ e ∈ es, Denotes M e.y y ∧ e.dOk signFuel = true ∧
      planePt M e.c e.N e.D y = (M.q c, y, z) := by
  unfold edgesOn at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨ls, hls, rfl⟩ := h
  obtain ⟨r, hr, hrl⟩ := mapM_mem hls (a, b) hab
  obtain ⟨e, he, hd, hok, -, hpt⟩ := proposeEdgeAny_complete M hr ha hb
  exact ⟨e, List.mem_flatten.mpr ⟨r, hrl, he⟩, hd, hok, hpt⟩

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

theorem planeSamples_edge {qs : List Quadric} {c : Rat} {ss : List QS}
    (h : planeSamples qs false c = some ss) :
    ∃ es, edgesOn ((activeOn qs c).filter fun q => !isXPlane q) c = some es ∧
      ∀ e ∈ es, ∃ s ∈ ss, edgeSample qs .edge e = some s := by
  unfold planeSamples at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨es, hes, turns, -, Ey, -, lines, -, edges, he, rfl⟩ := h
  refine ⟨es, hes, fun e hee => ?_⟩
  obtain ⟨s, hs, hsl⟩ := mapM_mem he e hee
  exact ⟨s, List.mem_append_right _ hsl, by simpa using hs⟩

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

theorem proposeEdge_c {a b : Quadric} {c : Rat} {fuel : Nat} {es : List ESample}
    (h : proposeEdge a b c fuel = some es) : ∀ e ∈ es, e.c = c := by
  unfold proposeEdge at h
  simp only [Option.bind_eq_bind] at h
  split at h
  · cases h
  cases hr : rootsOf (elim a b c).R fuel with
  | none => rw [hr] at h; cases h
  | some ns =>
    rw [hr, Option.bind_some] at h
    split at h
    · cases h; intro e he
      obtain ⟨n, -, rfl⟩ := List.mem_map.mp he; rfl
    · cases h

theorem proposeEdgeLin_c {a b : Quadric} {c : Rat} {fuel : Nat} {es : List ESample}
    (h : proposeEdgeLin a b c fuel = some es) : ∀ e ∈ es, e.c = c := by
  unfold proposeEdgeLin at h
  simp only [Option.bind_eq_bind] at h
  split at h
  · cases h
  cases hr : rootsOf (elimLin a b c).R fuel with
  | none => rw [hr] at h; cases h
  | some ns =>
    rw [hr, Option.bind_some] at h
    split at h
    · cases h; intro e he
      obtain ⟨n, -, rfl⟩ := List.mem_map.mp he; rfl
    · cases h

theorem edgesOn_c {qs : List Quadric} {c : Rat} {es : List ESample} (h : edgesOn qs c = some es) :
    ∀ e ∈ es, e.c = c := by
  unfold edgesOn at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨ls, hls, rfl⟩ := h
  intro e he
  obtain ⟨l, hl, he⟩ := List.mem_flatten.mp he
  obtain ⟨pr, -, hpr⟩ := mapM_mem' hls l hl
  unfold proposeEdgeAny at hpr
  split at hpr
  · split at hpr
    · exact proposeEdgeLin_c hpr e he
    · exact proposeEdgeLin_c hpr e he
  · exact proposeEdge_c hpr e he

section Located

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K) [DecidableRel (α := K) (· < ·)]

theorem lineSamples_located {qs : List Quadric} {c c' : Rat} {ls : List QS}
    (h : lineSamples qs c c' = some ls) : ∀ s ∈ ls, ∃ p : PK K, p.1 = M.q c ∧
      s.cov = covOfK (qs.map fun q => Quadric.evK M q) p := by
  unfold lineSamples at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨roots, -, Ez, hEz, facets, hf, cells, hc, rfl⟩ := h
  have hck := pickEvents_spec hEz
  simp only [checkEvents, Bool.and_eq_true, List.all_eq_true] at hck
  have hpt : ∀ t : K, (linePt M ⟨c, c', 0⟩ ⟨0, 0, 1⟩ t).1 = M.q c := by
    intro t; simp [linePt, Model.q_zero]; grind
  intro s hs
  obtain ⟨X, hX, rfl⟩ := List.mem_map.mp hs
  rcases List.mem_append.mp hX with hX | hX
  · obtain ⟨n, hn, hcov⟩ := mapM_mem' hf X hX
    obtain ⟨t, ht⟩ := denotes_of_isoOkN M (hck.1.2 n hn)
    exact ⟨_, hpt t, covOn_spec M _ _ ht _ qs X hcov⟩
  · obtain ⟨t, -, hcov⟩ := mapM_mem' hc X hX
    exact ⟨_, hpt (M.q t), covOn_spec M _ _ (show Denotes M (.rat t) (M.q t) from rfl) _ qs X hcov⟩

theorem edgeSample_located {qs : List Quadric} {tg : Tag} {e : ESample} {s : QS}
    (h : edgeSample qs tg e = some s) : ∃ p : PK K, p.1 = M.q e.c ∧
      s.cov = covOfK (qs.map fun q => Quadric.evK M q) p := by
  unfold edgeSample at h
  by_cases hc : (isoOkN e.y && e.dOk signFuel) = true
  · rw [if_pos hc] at h
    simp only [Bool.and_eq_true] at hc
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨X, hX, rfl⟩ := h
    obtain ⟨α, hα⟩ := denotes_of_isoOkN M hc.1
    exact ⟨_, rfl, edgeCov_spec M hα hc.2 qs X hX⟩
  · rw [if_neg hc] at h; cases h

/-- **A sample not tagged vertex is on a sample plane.** (No carrier vanishes on a whole plane
`x = c`, so no event plane is sampled.) -/
theorem samplesAt_located {qs : List Quadric} {ss : List QS} (h : samplesAt qs = some ss)
    (hz : ∀ q ∈ qs, ∀ c, zeroOn q c = false) :
    ∃ E, eventsOf qs rootFuel = some E ∧ ∀ s ∈ ss, s.tag ≠ .vert →
      ∃ c ∈ planesOf E, ∃ p : PK K, p.1 = M.q c ∧
        s.cov = covOfK (qs.map fun q => Quadric.evK M q) p := by
  unfold samplesAt at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨E, hE, ps, hps, es, hes, vs, hvs, rfl⟩ := h
  refine ⟨E, hE, fun s hs htag => ?_⟩
  rcases List.mem_append.mp hs with hs | hs
  rcases List.mem_append.mp hs with hs | hs
  · obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨c, hc, hpc⟩ := mapM_mem' hps l hl
    refine ⟨c, hc, ?_⟩
    unfold planeSamples at hpc
    simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
      Option.some.injEq] at hpc
    obtain ⟨es', hes', turns, -, Ey, -, lines, hl', edges, he', rfl⟩ := hpc
    rcases List.mem_append.mp hs with hs | hs
    · obtain ⟨l', hl'', hs⟩ := List.mem_flatten.mp hs
      obtain ⟨c', -, hc'⟩ := mapM_mem' hl' l' hl''
      exact lineSamples_located M hc' s hs
    · obtain ⟨e, he, hse⟩ := mapM_mem' he' s hs
      obtain ⟨p, hp1, hpc'⟩ := edgeSample_located M hse
      have hec : e.c = c := by
        -- edgesOn's samples are on its plane
        exact edgesOn_c hes' e he
      exact ⟨p, by rw [hp1, hec], hpc'⟩
  · exfalso
    obtain ⟨l, hl, -⟩ := List.mem_flatten.mp hs
    obtain ⟨c, hc, -⟩ := mapM_mem' hes l hl
    simp only [List.mem_filter, List.any_eq_true] at hc
    obtain ⟨-, q, hq, hzq⟩ := hc
    rw [hz q hq c] at hzq; cases hzq
  · exfalso
    unfold vertSamples at hvs
    simp only [Option.map_eq_some_iff] at hvs
    obtain ⟨ls, hls, rfl⟩ := hvs
    obtain ⟨l, hl, hs⟩ := List.mem_flatten.mp hs
    obtain ⟨t, -, ht⟩ := mapM_mem' hls l hl
    unfold enumSamples at ht
    simp only [Option.bind_eq_some_iff] at ht
    obtain ⟨ds, -, hds⟩ := ht
    obtain ⟨d, -, hd⟩ := mapM_mem' hds s hs
    simp only [Option.map_eq_some_iff] at hd
    obtain ⟨X, -, rfl⟩ := hd
    exact htag rfl

end Located

/-- The frame `samplesOf` takes, and its certificates. -/
theorem samplesOf_frame {qs : List Quadric} {ss : List QS} (h : samplesOf qs = some ss) :
    ∃ f ∈ frames, frameOk (qs.map (inFrame f)) = true ∧ singAll (qs.map (inFrame f)) = true ∧
      samplesAt (qs.map (inFrame f)) = some ss := by
  obtain ⟨f, hf⟩ := samplesOf_some h
  obtain ⟨h1, h2, h3, h4⟩ := samplesOfF_spec hf
  exact ⟨f, h1, h2, h3, h4⟩

end ForgeExact.DecideOnce.Curved
