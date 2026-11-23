import ForgeExact.DecideOnce.QSweep

/-!
# Q1 L2: every common point of a triple, enumerated

`enumTriple a b c` lists **point descriptors** (`PtD`), one per common point of three carriers:
* a vertex sample (`proposeTriple`: three quadrics, or a plane and two);
* a root on two planes' line (`lineOf`).

* `PtD.at`: the point a descriptor names; `at_unique`: it names at most one;
* `PtD.cov`, `cov_spec`: its covector, decided exactly;
* **`enumTriple_complete`**: every common point is named by a descriptor of the list.

These are the vertex samples, and the ground for the vertex-uniqueness certificate (U). A
covector carried by exactly one descriptor of a triple's complete list is one point.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef ForgeExact.DecideOnce
  Lean.Grind

inductive PtD
  | vs (v : VSample)
  | ln (P D : V3) (n : Num)

/-- The descriptors of a triple's common points (`none`: undecided). -/
def enumTriple (a b c : Quadric) (fuel : Nat) : Option (List PtD) :=
  if (isConstQ a && a.c != 0) || (isConstQ b && b.c != 0) || (isConstQ c && c.c != 0) ||
    parallelApart a b || parallelApart a c || parallelApart b c then some []
  else if lineOk a b (lineOf a b) then
    (rootsOf (alongLine c (lineOf a b).1 (lineOf a b).2.1) fuel).map
      (·.map (PtD.ln (lineOf a b).1 (lineOf a b).2.1))
  else if lineOk a c (lineOf a c) then
    (rootsOf (alongLine b (lineOf a c).1 (lineOf a c).2.1) fuel).map
      (·.map (PtD.ln (lineOf a c).1 (lineOf a c).2.1))
  else if lineOk b c (lineOf b c) then
    (rootsOf (alongLine a (lineOf b c).1 (lineOf b c).2.1) fuel).map
      (·.map (PtD.ln (lineOf b c).1 (lineOf b c).2.1))
  else if isPlaneQ a && isPlaneQ b && isPlaneQ c then none
  else (proposeTriple a b c fuel).map (·.map PtD.vs)

/-- A descriptor's covector over the carriers. -/
def PtD.cov (qs : List Quadric) : PtD → Option Cov
  | .vs v => if isoOkN v.x && v.dOk signFuel then vertCov qs v signFuel else none
  | .ln P D n => if isoOkN n then covOn qs P D n signFuel else none

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem denotes_of_isoOkN' {n : Num} (h : isoOkN n = true) : ∃ t, Denotes M n t := by
  cases n with
  | rat q => exact ⟨M.q q, rfl⟩
  | iso r => exact Iso.denotes_of_ok M r h

/-- The point a descriptor names. -/
def PtD.at : PtD → PK K → Prop
  | .vs v, p => Denotes M v.x p.1 ∧ v.dOk signFuel = true ∧ vertPt M v.Ny v.Dy v.Nz v.Dz p.1 = p
  | .ln P D n, p => ∃ t, Denotes M n t ∧ p = linePt M P D t

theorem PtD.at_unique {d : PtD} {p p' : PK K} (h : d.at M p) (h' : d.at M p') : p = p' := by
  cases d with
  | vs v =>
    obtain ⟨h1, -, h3⟩ := h
    obtain ⟨h1', -, h3'⟩ := h'
    rw [← h3, ← h3', Denotes.unique M h1 h1']
  | ln P D n =>
    obtain ⟨t, ht, rfl⟩ := h
    obtain ⟨t', ht', rfl⟩ := h'
    rw [Denotes.unique M ht ht']

theorem proposeTriple_complete2 {a b c : Quadric} {fuel : Nat} {vs : List VSample}
    (h : proposeTriple a b c fuel = some vs) {x y z : K}
    (ha : Quadric.evK M a (x, y, z) = 0) (hb : Quadric.evK M b (x, y, z) = 0)
    (hc : Quadric.evK M c (x, y, z) = 0) : ∃ v ∈ vs, (PtD.vs v).at M (x, y, z) := by
  unfold proposeTriple at h
  by_cases h1 : (!isPlaneQ a && !isPlaneQ b && !isPlaneQ c) = true
  · rw [if_pos h1] at h
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVert3_complete M h ha hb hc; exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h1] at h
  by_cases h2 : (isPlaneQ c && !isPlaneQ a) = true
  · rw [if_pos h2] at h; simp only [Bool.and_eq_true] at h2
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h2.1) h ha hb hc
    exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h2] at h
  by_cases h3 : (isPlaneQ c && !isPlaneQ b) = true
  · rw [if_pos h3] at h; simp only [Bool.and_eq_true] at h3
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h3.1) h hb ha hc
    exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h3] at h
  by_cases h4 : (isPlaneQ b && !isPlaneQ a) = true
  · rw [if_pos h4] at h; simp only [Bool.and_eq_true] at h4
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h4.1) h ha hc hb
    exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h4] at h
  by_cases h5 : (isPlaneQ b && !isPlaneQ c) = true
  · rw [if_pos h5] at h; simp only [Bool.and_eq_true] at h5
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h5.1) h hc ha hb
    exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h5] at h
  by_cases h6 : (isPlaneQ a && !isPlaneQ b) = true
  · rw [if_pos h6] at h; simp only [Bool.and_eq_true] at h6
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h6.1) h hb hc ha
    exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h6] at h
  by_cases h7 : (isPlaneQ a && !isPlaneQ c) = true
  · rw [if_pos h7] at h; simp only [Bool.and_eq_true] at h7
    obtain ⟨v, hv, hd, hok, -, hp⟩ := proposeVertPlaneAny_complete M (isPlaneQ_iff h7.1) h hc hb ha
    exact ⟨v, hv, hd, hok, hp⟩
  rw [if_neg h7] at h
  cases h

theorem line_enum {a b q : Quadric} {fuel : Nat} {ns : List Num} (hok : lineOk a b (lineOf a b) = true)
    (h : rootsOf (alongLine q (lineOf a b).1 (lineOf a b).2.1) fuel = some ns) {x y z : K}
    (ea : Quadric.evK M a (x, y, z) = 0) (eb : Quadric.evK M b (x, y, z) = 0)
    (eq : Quadric.evK M q (x, y, z) = 0) :
    ∃ d ∈ ns.map (PtD.ln (lineOf a b).1 (lineOf a b).2.1), d.at M (x, y, z) := by
  obtain ⟨s, hs⟩ := line_cover M hok ea eb
  have hr : M.ev (alongLine q (lineOf a b).1 (lineOf a b).2.1) s = 0 := by
    rw [alongLine_ev, ← hs]; exact eq
  obtain ⟨n, hn, hd⟩ := rootsOf_complete M h hr
  exact ⟨_, List.mem_map_of_mem hn, s, hd, hs⟩

/-- **Every common point of a triple is named by a descriptor.** -/
theorem enumTriple_complete {a b c : Quadric} {fuel : Nat} {ds : List PtD}
    (h : enumTriple a b c fuel = some ds) {x y z : K}
    (ha : Quadric.evK M a (x, y, z) = 0) (hb : Quadric.evK M b (x, y, z) = 0)
    (hc : Quadric.evK M c (x, y, z) = 0) : ∃ d ∈ ds, d.at M (x, y, z) := by
  unfold enumTriple at h
  split at h
  · rename_i h1
    exfalso
    simp only [Bool.or_eq_true] at h1
    rcases h1 with ((((h1 | h1) | h1) | h1) | h1) | h1
    · exact const_ne M h1 x y z ha
    · exact const_ne M h1 x y z hb
    · exact const_ne M h1 x y z hc
    · exact parallelApart_ne M h1 ha hb
    · exact parallelApart_ne M h1 ha hc
    · exact parallelApart_ne M h1 hb hc
  split at h
  · rename_i hok
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨ns, hns, rfl⟩ := h
    exact line_enum M hok hns ha hb hc
  split at h
  · rename_i hok
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨ns, hns, rfl⟩ := h
    exact line_enum M hok hns ha hc hb
  split at h
  · rename_i hok
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨ns, hns, rfl⟩ := h
    exact line_enum M hok hns hb hc ha
  split at h
  · cases h
  · simp only [Option.map_eq_some_iff] at h
    obtain ⟨vs, hvs, rfl⟩ := h
    obtain ⟨v, hv, hat⟩ := proposeTriple_complete2 M hvs ha hb hc
    exact ⟨_, List.mem_map_of_mem hv, hat⟩

variable [DecidableRel (α := K) (· < ·)]

/-- **A descriptor's covector is its point's.** -/
theorem PtD.cov_spec {qs : List Quadric} {d : PtD} {X : Cov} (hX : d.cov qs = some X) {p : PK K}
    (hp : d.at M p) : X = covOfK (qs.map fun q => Quadric.evK M q) p := by
  cases d with
  | vs v =>
    simp only [PtD.cov] at hX
    split at hX
    · obtain ⟨hd, hok, hpt⟩ := hp
      have := vertCov_spec M hd hok qs X hX
      rw [hpt] at this; exact this
    · cases hX
  | ln P D n =>
    simp only [PtD.cov] at hX
    split at hX
    · obtain ⟨t, ht, rfl⟩ := hp
      exact covOn_spec M P D ht signFuel qs X hX
    · cases hX

/-- A descriptor with a covector names a point. -/
theorem PtD.cov_at {qs : List Quadric} {d : PtD} {X : Cov} (hX : d.cov qs = some X) :
    ∃ p : PK K, d.at M p := by
  cases d with
  | vs v =>
    simp only [PtD.cov] at hX
    split at hX
    · rename_i hc
      simp only [Bool.and_eq_true] at hc
      obtain ⟨α, hα⟩ := denotes_of_isoOkN' M hc.1
      refine ⟨vertPt M v.Ny v.Dy v.Nz v.Dz α, ?_, hc.2, ?_⟩
      · simpa [vertPt] using hα
      · simp [vertPt]
    · cases hX
  | ln P D n =>
    simp only [PtD.cov] at hX
    split at hX
    · rename_i hc
      obtain ⟨t, ht⟩ := denotes_of_isoOkN' M hc
      exact ⟨_, t, ht, rfl⟩
    · cases hX

end Model

end ForgeExact.DecideOnce.Curved
