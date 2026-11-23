import ForgeExact.DecideOnce.QSample
import ForgeExact.DecideOnce.QForm

/-!
# Q1 L2: the curved ledger (plan §2, module 6)

`curvedLedger` builds a `Ledger` from the samples' covectors. The bodies are sign formulas
(`SForm`, QForm.lean) over the carriers.

* **faces**: one per body and carrier, `f = i·nc + j` (body `i`, carrier `j`);
* **pieces**: a sampled facet `φ` is a piece of body `i` iff `i`'s formula differs on `φ.plus` and
  `φ.minus`;
* **inside**: a side cell `T` is inside body `i` iff `i`'s formula holds on `T`;
* **edges**: the covectors on two or more carriers that a sample off a vertex hits;
* **verts**: the covectors only vertex samples hit.
Both edges and verts lie round a facet.

`qledger` builds it and checks two things: `splitOk`, and `sidesOk`, which says both sides of every
facet are sampled. Otherwise it answers `none`.

Proven here, for every model of the reals, with `A i` the formula's set:
* `qledger_pieceFacet`, `qledger_oneFace`: the split is sound;
* `qledger_pieceReal`: every piece is realised;
* `qledger_witness`: `inside` is exactly membership, on every side cell;
* `qledger_boundary`: the pieces of body `i` are exactly the facets it flips across.

These are five of `ExactK`'s fields. The rest are `wallND` (the carriers), `compat`
(`qform_compat`), and `complete`, `edges` and `verts`. Those three need the samples' completeness
(`qsampler_covers`) and the dimension rule, against the sweep's delineability.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef ForgeExact.DecideOnce
  Lean.Grind

/-- Body `i`'s formula. -/
def formOf (forms : List SForm) (i : Nat) : SForm := forms.getD i .ff

/-- The ledger of the samples `ss` over `nc` carriers, for the bodies `forms`. -/
def curvedLedger (nc : Nat) (forms : List SForm) (ss : List QS) : Ledger :=
  let nb := forms.length
  let covs := dedup (ss.map (·.cov))
  let pieces := (covs.filter Cov.isFacet).flatMap fun φ =>
    ((List.range nb).filter fun i =>
      (formOf forms i).onCov φ.plus != (formOf forms i).onCov φ.minus).map
      fun i => (φ, i * nc + φ.zeroIdx)
  let L0 : Ledger :=
    ⟨nb, (List.range (nb * nc)).map (· / nc), (List.range (nb * nc)).map (· % nc), pieces, [], [], []⟩
  let inside := L0.topes.flatMap fun T =>
    ((List.range nb).filter fun i => (formOf forms i).onCov T).map fun i => (T, i)
  let edges := (dedup ((ss.filter fun s => s.tag != .vert && decide (2 ≤ s.cov.count .zero)).map
    (·.cov))).filter fun e => L0.facets.any (Cov.le e)
  let verts := (dedup ((ss.filter fun s => s.tag == .vert).map (·.cov))).filter fun v =>
    !(ss.any fun s => s.tag != .vert && s.cov == v) && L0.facets.any (Cov.le v)
  { L0 with inside := inside, edges := edges, verts := verts }

/-- Both sides of every facet are sampled. -/
def sidesOk (L : Ledger) (ss : List QS) : Bool :=
  L.facets.all fun φ => decide (φ.plus ∈ ss.map (·.cov)) && decide (φ.minus ∈ ss.map (·.cov))

/-- **The curved ledger**, checked. `none`: a sample step undecided, a side unsampled, or the
split unsound. Never a wrong ledger. -/
def qledger (qs : List Quadric) (forms : List SForm) : Option Ledger := do
  let ss ← samplesOf qs
  let L := curvedLedger qs.length forms ss
  if L.splitOk && sidesOk L ss then some L else none

/-! ## The fields that follow from realisation -/

theorem qledger_spec' {qs : List Quadric} {forms : List SForm} {L : Ledger}
    (h : qledger qs forms = some L) :
    ∃ ss, L = curvedLedger qs.length forms ss ∧ L.splitOk = true ∧ sidesOk L ss = true := by
  unfold qledger at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at h
  obtain ⟨ss, -, h⟩ := h
  by_cases hc : ((curvedLedger qs.length forms ss).splitOk &&
      sidesOk (curvedLedger qs.length forms ss) ss) = true
  · rw [if_pos hc] at h
    cases h
    simp only [Bool.and_eq_true] at hc
    exact ⟨ss, rfl, hc.1, hc.2⟩
  · rw [if_neg hc] at h; cases h

section Split

variable {qs : List Quadric} {forms : List SForm} {L : Ledger}

theorem qledger_pieceFacet (h : qledger qs forms = some L) :
    ∀ p ∈ L.pieces, p.1.isFacet = true ∧ p.1.zeroIdx = L.clsOf p.2 ∧ L.bodyOf p.2 < L.nb := by
  obtain ⟨ss, rfl, hs, -⟩ := qledger_spec' h
  intro p hp
  simp only [Ledger.splitOk, List.all_eq_true, Bool.and_eq_true, beq_iff_eq,
    decide_eq_true_eq] at hs
  obtain ⟨⟨⟨h1, h2⟩, h3⟩, -⟩ := hs p hp
  exact ⟨h1, h2, h3⟩

theorem qledger_oneFace (h : qledger qs forms = some L) :
    ∀ p ∈ L.pieces, ∀ q ∈ L.pieces, q.1 = p.1 → L.bodyOf q.2 = L.bodyOf p.2 → q.2 = p.2 := by
  obtain ⟨ss, rfl, hs, -⟩ := qledger_spec' h
  intro p hp q hq h1 h2
  simp only [Ledger.splitOk, List.all_eq_true, Bool.and_eq_true] at hs
  have := (hs p hp).2 q hq
  simp only [Bool.or_eq_true, Bool.not_eq_true', Bool.and_eq_false_iff, beq_iff_eq] at this
  rcases this with (h | h) | h
  · exact absurd (beq_iff_eq.mpr h1) (by simp [h])
  · exact absurd (beq_iff_eq.mpr h2) (by simp [h])
  · simpa using h

end Split



section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K) [DecidableRel (α := K) (· < ·)]

/-- The carriers. -/
def carriersOf (qs : List Quadric) : List (Carrier (PK K) K) := qs.map fun q => Quadric.evK M q

/-- The bodies. -/
def bodiesOf (qs : List Quadric) (forms : List SForm) : Nat → Pred (PK K) :=
  fun i => (formOf forms i).den (carriersOf M qs)

theorem qledger_spec {qs : List Quadric} {forms : List SForm} {L : Ledger}
    (h : qledger qs forms = some L) :
    ∃ ss, (∀ s ∈ ss, RealCov M qs s.cov) ∧ L = curvedLedger qs.length forms ss ∧
      L.splitOk = true ∧ sidesOk L ss = true := by
  unfold qledger at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at h
  obtain ⟨ss, hss, h⟩ := h
  by_cases hc : ((curvedLedger qs.length forms ss).splitOk &&
      sidesOk (curvedLedger qs.length forms ss) ss) = true
  · rw [if_pos hc] at h
    cases h
    simp only [Bool.and_eq_true] at hc
    exact ⟨ss, samplesOf_realised M hss, rfl, hc.1, hc.2⟩
  · rw [if_neg hc] at h; cases h

theorem realised_of_mem {qs : List Quadric} {ss : List QS} (hr : ∀ s ∈ ss, RealCov M qs s.cov)
    {X : Cov} (hX : X ∈ ss.map (·.cov)) : RealisedK (carriersOf M qs) X := by
  obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hX
  obtain ⟨p, hp⟩ := hr s hs
  exact ⟨p, hp.symm⟩

theorem curvedLedger_pieces {nc : Nat} {forms : List SForm} {ss : List QS} {p : Cov × Nat}
    (hp : p ∈ (curvedLedger nc forms ss).pieces) :
    ∃ i, i < forms.length ∧ p = (p.1, i * nc + p.1.zeroIdx) ∧ p.1 ∈ ss.map (·.cov) ∧
      p.1.isFacet = true ∧
      (formOf forms i).onCov p.1.plus ≠ (formOf forms i).onCov p.1.minus := by
  simp only [curvedLedger, List.mem_flatMap, List.mem_filter, List.mem_map, List.mem_range,
    bne_iff_ne, ne_eq, decide_eq_true_eq] at hp
  obtain ⟨φ, ⟨hφ, hf⟩, i, ⟨hi, hne⟩, rfl⟩ := hp
  exact ⟨i, hi, rfl, (mem_dedup _ _).mp hφ, hf, hne⟩

theorem curvedLedger_facets {nc : Nat} {forms : List SForm} {ss : List QS} {φ : Cov}
    (hφ : φ ∈ (curvedLedger nc forms ss).facets) : φ ∈ ss.map (·.cov) := by
  unfold Ledger.facets at hφ
  obtain ⟨p, hp, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hφ)
  obtain ⟨_, _, _, h, _⟩ := curvedLedger_pieces hp
  exact h

theorem curvedLedger_topes {nc : Nat} {forms : List SForm} {ss : List QS} {L : Ledger}
    (hL : L = curvedLedger nc forms ss) (hs : sidesOk L ss = true) {T : Cov} (hT : T ∈ L.topes) :
    T ∈ ss.map (·.cov) := by
  unfold Ledger.topes at hT
  obtain ⟨φ, hφ, hT⟩ := List.mem_flatMap.mp ((mem_dedup _ _).mp hT)
  unfold sidesOk at hs
  have := List.all_eq_true.mp hs φ hφ
  simp only [Bool.and_eq_true, decide_eq_true_eq] at this
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hT
  rcases hT with rfl | rfl
  · exact this.1
  · exact this.2

/-- Membership of a realised cell is its formula's value there. -/
theorem qmemTK_iff {qs : List Quadric} {forms : List SForm} {T : Cov}
    (hT : RealisedK (carriersOf M qs) T) (i : Nat) :
    memTK (carriersOf M qs) (bodiesOf M qs forms) T i ↔ (formOf forms i).onCov T = true := by
  obtain ⟨x, hx⟩ := hT
  constructor
  · rintro ⟨y, hy, hA⟩; exact ((formOf forms i).den_cell hy).mp hA
  · intro h; exact ⟨x, hx, ((formOf forms i).den_cell hx).mpr h⟩

theorem cov_length {qs : List Quadric} {X : Cov} (h : RealisedK (carriersOf M qs) X) :
    X.length = qs.length := by
  obtain ⟨x, hx⟩ := h
  unfold covCellK covOfK carriersOf at hx
  rw [← hx]; simp

theorem zeroIdx_lt {X : Cov} (h : X.isFacet = true) : X.zeroIdx < X.length := by
  unfold Cov.isFacet at h
  unfold Cov.zeroIdx
  apply List.idxOf_lt_length_of_mem
  apply List.count_pos_iff.mp
  simp only [beq_iff_eq] at h
  omega

theorem bodyOf_face {nc : Nat} {forms : List SForm} {ss : List QS} {i j : Nat}
    (hi : i < forms.length) (hj : j < nc) :
    (curvedLedger nc forms ss).bodyOf (i * nc + j) = i ∧
      (curvedLedger nc forms ss).clsOf (i * nc + j) = j := by
  have hlt : i * nc + j < forms.length * nc := by
    have : (i + 1) * nc ≤ forms.length * nc := Nat.mul_le_mul_right _ hi
    rw [Nat.succ_mul] at this; omega
  have hnc : 0 < nc := by omega
  simp only [Ledger.bodyOf, Ledger.clsOf, curvedLedger, List.getD_eq_getElem?_getD,
    List.getElem?_map, List.getElem?_range hlt, Option.map_some, Option.getD_some]
  constructor
  · rw [Nat.add_comm, Nat.add_mul_div_right _ _ hnc, Nat.div_eq_of_lt hj, Nat.zero_add]
  · rw [Nat.add_comm, Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt hj]

variable {qs : List Quadric} {forms : List SForm} {L : Ledger}

theorem qledger_pieceReal (h : qledger qs forms = some L) :
    ∀ p ∈ L.pieces, RealisedK (carriersOf M qs) p.1 := by
  obtain ⟨ss, hr, rfl, -, -⟩ := qledger_spec M h
  intro p hp
  obtain ⟨_, _, _, h, _⟩ := curvedLedger_pieces hp
  exact realised_of_mem M hr h

/-- **`inside` is membership**, on every side cell. -/
theorem qledger_witness (h : qledger qs forms = some L) :
    ∀ T ∈ L.topes, ∀ i, i < L.nb →
      (L.inside.contains (T, i) = true ↔ memTK (carriersOf M qs) (bodiesOf M qs forms) T i) := by
  obtain ⟨ss, hr, hL, -, hs⟩ := qledger_spec M h
  intro T hT i hi
  have hreal := realised_of_mem M hr (curvedLedger_topes hL hs hT)
  rw [qmemTK_iff M hreal, List.contains_eq_mem, decide_eq_true_eq]
  have hT' : T ∈ L.topes := hT
  subst hL
  simp only [curvedLedger] at hT' hi ⊢
  simp only [List.mem_flatMap, List.mem_map, List.mem_filter, List.mem_range, Prod.mk.injEq]
  constructor
  · rintro ⟨U, -, j, ⟨-, hj⟩, rfl, rfl⟩; exact hj
  · intro hv; exact ⟨T, hT', i, ⟨hi, hv⟩, rfl, rfl⟩

/-- **The pieces of body `i` are the facets it flips across.** -/
theorem qledger_boundary (h : qledger qs forms = some L) :
    ∀ φ ∈ L.facets, ∀ i, i < L.nb →
      (FlipK (carriersOf M qs) (bodiesOf M qs forms) φ i ↔ L.isPieceOf i φ = true) := by
  obtain ⟨ss, hr, hL, -, hs⟩ := qledger_spec M h
  intro φ hφ i hi
  have hφs : φ ∈ L.facets := hφ
  -- both sides are sampled, so realised
  unfold sidesOk at hs
  have hsd := List.all_eq_true.mp hs φ hφ
  simp only [Bool.and_eq_true, decide_eq_true_eq] at hsd
  have rp := realised_of_mem M hr hsd.1
  have rm := realised_of_mem M hr hsd.2
  unfold FlipK
  rw [qmemTK_iff M rp, qmemTK_iff M rm]
  subst hL
  have hφc := curvedLedger_facets hφs
  have hfac : φ.isFacet = true := by
    unfold Ledger.facets at hφs
    obtain ⟨p, hp, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hφs)
    obtain ⟨_, _, _, _, h, _⟩ := curvedLedger_pieces hp
    exact h
  have hzl : φ.zeroIdx < qs.length := by
    rw [← cov_length M (realised_of_mem M hr hφc)]; exact zeroIdx_lt hfac
  have hnb : i < forms.length := hi
  unfold Ledger.isPieceOf
  rw [List.any_eq_true]
  constructor
  · intro hne
    refine ⟨(φ, i * qs.length + φ.zeroIdx), ?_, ?_⟩
    · simp only [curvedLedger, List.mem_flatMap, List.mem_filter, List.mem_map, List.mem_range,
        bne_iff_ne, ne_eq, decide_eq_true_eq]
      refine ⟨φ, ⟨(mem_dedup _ _).mpr hφc, hfac⟩, i, ⟨hnb, ?_⟩, rfl⟩
      intro heq; exact hne (by rw [heq])
    · simp only [Bool.and_eq_true, beq_iff_eq, true_and]
      exact (bodyOf_face hnb hzl).1
  · rintro ⟨p, hp, hpq⟩
    simp only [Bool.and_eq_true, beq_iff_eq] at hpq
    obtain ⟨j, hj, hpe, -, -, hne⟩ := curvedLedger_pieces hp
    obtain ⟨h1, h2⟩ := hpq
    rw [hpe] at h2
    simp only at h2
    rw [h1] at h2 hne
    rw [(bodyOf_face hj hzl).1] at h2
    subst h2
    intro hiff
    apply hne
    revert hiff
    generalize (formOf forms j).onCov φ.plus = a
    generalize (formOf forms j).onCov φ.minus = b
    cases a <;> cases b <;> decide

/-- The bodies are compatible with every cell (`qform_compat`). -/
theorem qledger_compat (T : Cov) (i : Nat) :
    (covCellK (carriersOf M qs) T).Sub (bodiesOf M qs forms i) ∨
      ∀ x, covCellK (carriersOf M qs) T x → ¬bodiesOf M qs forms i x :=
  qform_compat (carriersOf M qs) (formOf forms) T i

end Model

-- the unit sphere, cut by y = 0 and z = 0, as one body: "inside the sphere"
def showCov (X : Cov) : String :=
  String.ofList (X.map fun s => match s with | .neg => '-' | .zero => '0' | .pos => '+')

-- the unit sphere, cut by y = 0 and z = 0, as one body "inside the sphere": its 4 facets, 4 arcs
-- and the vertex covector of (±1, 0, 0)
def sameSet (a b : List String) : Bool := a.length == b.length && a.all (b.contains ·)

#guard (qledger [sphere1, planeY0, planeZ] [.lit 0 .neg]).map
  (fun L => sameSet (L.facets.map showCov) ["0-+", "0--", "0++", "0+-"] &&
    sameSet (L.edges.map showCov) ["00+", "00-", "0-0", "0+0"] &&
    sameSet (L.verts.map showCov) ["000"]) = some true

/-- A plane `a x + b y + c z + d = 0`. -/
def planeQ (a b c d : Rat) : Quadric := ⟨0, 0, 0, 0, 0, 0, a, b, c, d⟩

/-- The unit box `[0, 1]³`: its planes and its formula. -/
def unitBox : List Quadric :=
  [planeQ 1 0 0 0, planeQ 1 0 0 (-1), planeQ 0 1 0 0, planeQ 0 1 0 (-1), planeQ 0 0 1 0,
   planeQ 0 0 1 (-1)]
def unitBoxF : SForm :=
  .and (.lit 0 .pos) (.and (.lit 1 .neg) (.and (.lit 2 .pos) (.and (.lit 3 .neg)
    (.and (.lit 4 .pos) (.lit 5 .neg)))))

-- the axis-aligned box: 6 faces, 12 edges, 8 vertices
#guard (qledger unitBox [unitBoxF]).map (fun L => (L.facets.length, L.edges.length,
  L.verts.length)) = some (6, 12, 8)

end ForgeExact.DecideOnce.Curved
