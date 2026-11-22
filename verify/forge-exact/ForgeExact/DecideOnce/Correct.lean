import ForgeExact.DecideOnce.Coverage
import ForgeExact.Flat

/-!
# Correctness: a geometric ledger's result is the exact regularised boolean

The ledger's facts are about the arrangement of the snapped carriers `ps` (one plane per class, the
representative's, §6.1) and the solids `A i ⊆ ℚ³`. A ledger is **exact** (`Exact`) when every
decision it holds is the snapped geometry's:

* `pieceReal`, `pieceFacet`, `oneFace`: each piece is a cell of the arrangement, a facet in its
  face's carrier, and the input faces of one body do not overlap;
* `witness`: each side cell's witness answer is the solid's membership there;
* `boundary`, `complete`: a facet is a piece of a face of body `i` iff body `i`'s membership flips
  across it (the Jordan–Brouwer property of a closed B-rep, with point-in-face decided right);
* `edges`, `verts`: the edge and vertex cells listed are exactly the arrangement's cells of
  dimension 1 and 0 round the pieces;
* `compat`: each solid's membership is constant on every full cell of the arrangement. For
  polyhedral solids given as formulas (`Flat.HForm`) this is proven (`Flat.HForm.den_iff_onSigns`);
  for a B-rep classified by ray parity it is RefFacts' one assumption.

**Theorem (c).** For an exact ledger and any op:

* `correct`: the core's result facets are exactly the arrangement's facets with one side kept by the
  exact boolean (`XBoundary`), and by `reg_kept` the regularised union of the cells the exact boolean
  keeps is the regularised boolean of the solids;
* `correct_ok`: a valid result is that boolean: its faces partition exactly those facets;
* `zeroThickness_iff`: the core refuses `ZeroThickness` iff the exact result is non-manifold
  (`XNonManifold`: an edge cell of the arrangement with three or more exact boundary facets round
  it, or a vertex cell whose exact boundary facets form more than one fan), defined from the
  geometry alone, not from the ledger.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref Topology Ledger

/-! ## The arrangement's cells, by covector -/

/-- The side of a carrier a value gives. -/
def sgnQ (q : Rat) : Sgn := if q > 0 then .pos else if q < 0 then .neg else .zero

/-- The cell with covector `X`: the points on the given side of each plane. -/
def covCell : List Plane → Cov → Pred V3
  | [], [] => fun _ => True
  | p :: ps, s :: X => fun x => sgnQ (p.eval x) = s ∧ covCell ps X x
  | _, _ => fun _ => False

/-- A cell with a point. -/
def Realised (ps : List Plane) (X : Cov) : Prop := ∃ x, covCell ps X x

/-- A point's covector. -/
def covOf (ps : List Plane) (x : V3) : Cov := ps.map fun p => sgnQ (p.eval x)

theorem covCell_covOf : ∀ (ps : List Plane) (x : V3), covCell ps (covOf ps x) x
  | [], _ => trivial
  | _ :: ps, x => ⟨rfl, covCell_covOf ps x⟩

/-- The full covectors (no zero) of length `n`. -/
def fulls : Nat → List Cov
  | 0 => [[]]
  | n + 1 => (fulls n).flatMap fun l => [.pos :: l, .neg :: l]

theorem covOf_full : ∀ (ps : List Plane) (x : V3), ¬Flat.wall ps x → covOf ps x ∈ fulls ps.length
  | [], _, _ => by simp [covOf, fulls]
  | p :: ps, x, h => by
    rw [show covOf (p :: ps) x = sgnQ (p.eval x) :: covOf ps x from rfl]
    simp only [List.length_cons, fulls, List.mem_flatMap]
    refine ⟨covOf ps x, covOf_full ps x fun ⟨q, hq, h0⟩ => h ⟨q, List.mem_cons_of_mem _ hq, h0⟩, ?_⟩
    have : p.eval x ≠ 0 := fun h0 => h ⟨p, List.mem_cons_self .., h0⟩
    by_cases h1 : p.eval x > 0
    · simp [sgnQ, h1]
    · have h2 : p.eval x < 0 := by grind
      simp [sgnQ, h1, h2]

/-- **The full cells are a decomposition** compatible with any solids whose membership is constant
on each (the generalisation of the box family's grid, Coverage/Correct.lean, to any planes). -/
def topeDecomp (ps : List Plane) (hps : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (A : Nat → Pred V3)
    (hc : ∀ T i, (covCell ps T).Sub (A i) ∨ ∀ x, covCell ps T x → ¬A i x) :
    Decomposition.{0, 0, 0} A where
  κ := Cov
  cells := fulls ps.length
  cell := covCell ps
  wall := Flat.wall ps
  wall_nowhereDense := Flat.wall_nowhereDense ps hps
  cover x h := ⟨covOf ps x, covOf_full ps x h, covCell_covOf ps x⟩
  compatible T i := hc T i

/-! ## The exact facts -/

/-- Membership of the solid `A i` in the cell `T`. -/
def memT (ps : List Plane) (A : Nat → Pred V3) (T : Cov) (i : Nat) : Prop :=
  ∃ x, covCell ps T x ∧ A i x

/-- Body `i`'s membership flips across the facet `φ`. -/
def Flip (ps : List Plane) (A : Nat → Pred V3) (φ : Cov) (i : Nat) : Prop :=
  ¬(memT ps A φ.plus i ↔ memT ps A φ.minus i)

/-- The carriers through a cell. -/
def zerosOf (X : Cov) : List Nat := (List.range X.length).filter fun j => X.getD j .pos == .zero

def det3 (a b c : V3) : Rat := a.dot (b.cross c)

/-- A vertex cell: three independent carriers through it. -/
def IsVert (ps : List Plane) (X : Cov) : Prop :=
  ∃ a ∈ zerosOf X, ∃ b ∈ zerosOf X, ∃ c ∈ zerosOf X,
    det3 (ps.getD a default).n (ps.getD b default).n (ps.getD c default).n ≠ 0

/-- An edge cell: two independent carriers through it, not three. -/
def IsEdge (ps : List Plane) (X : Cov) : Prop :=
  ¬IsVert ps X ∧ ∃ a ∈ zerosOf X, ∃ b ∈ zerosOf X,
    ((ps.getD a default).n.cross (ps.getD b default).n).isZero = false

/-- **An exact ledger**: every decision it holds is the snapped geometry's. -/
structure Exact (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) : Prop where
  nonzero : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩
  compat : ∀ T i, (covCell ps T).Sub (A i) ∨ ∀ x, covCell ps T x → ¬A i x
  pieceFacet : ∀ p ∈ L.pieces, p.1.isFacet = true ∧ p.1.zeroIdx = L.clsOf p.2 ∧ L.bodyOf p.2 < L.nb
  oneFace : ∀ p ∈ L.pieces, ∀ q ∈ L.pieces, q.1 = p.1 → L.bodyOf q.2 = L.bodyOf p.2 → q.2 = p.2
  pieceReal : ∀ p ∈ L.pieces, Realised ps p.1
  witness : ∀ T ∈ L.topes, ∀ i, i < L.nb → (L.inside.contains (T, i) = true ↔ memT ps A T i)
  boundary : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (Flip ps A φ i ↔ L.isPieceOf i φ = true)
  complete : ∀ φ, Realised ps φ → φ.isFacet = true → ∀ i, i < L.nb → Flip ps A φ i → φ ∈ L.facets
  edges : ∀ e, e ∈ L.edges ↔ Realised ps e ∧ IsEdge ps e ∧ ∃ φ ∈ L.facets, Cov.le e φ = true
  verts : ∀ v, v ∈ L.verts ↔ Realised ps v ∧ IsVert ps v ∧ ∃ φ ∈ L.facets, Cov.le v φ = true

variable {L : Ledger} {ps : List Plane} {A : Nat → Pred V3}

/-- The snapped solids' membership, as a Bool. -/
noncomputable def mOf (ps : List Plane) (A : Nat → Pred V3) (T : Cov) (i : Nat) : Bool :=
  @decide (memT ps A T i) (Classical.propDecidable _)

theorem mOf_iff {T : Cov} {i : Nat} : mOf ps A T i = true ↔ memT ps A T i := by
  simp [mOf]

/-- **An exact ledger is consistent**: the snapped solids' membership realises it. -/
theorem Exact.consistent (h : Exact L ps A) : Consistent L where
  pieceFacet := h.pieceFacet
  oneFace := h.oneFace
  realised := ⟨mOf ps A, fun T hT i hi => by
      apply Bool.eq_iff_iff.mpr
      rw [mOf_iff]; exact h.witness T hT i hi,
    fun φ hφ i hi => by
      rw [← h.boundary φ hφ i hi]
      unfold Flip
      rw [← mOf_iff (ps := ps) (A := A), ← mOf_iff (ps := ps) (A := A)]
      cases mOf ps A φ.plus i <;> cases mOf ps A φ.minus i <;> simp⟩

/-- The exact boolean's keep, on a cell. -/
noncomputable def xkeep (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (rule : Rule) (T : Cov) :
    Bool :=
  rule fun i => @decide (i < L.nb ∧ memT ps A T i) (Classical.propDecidable _)

/-- An exact boundary facet: a cell of the arrangement in one carrier, one side kept by the exact
boolean. -/
def XBoundary (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (rule : Rule) (φ : Cov) : Prop :=
  Realised ps φ ∧ φ.isFacet = true ∧ xkeep L ps A rule φ.plus ≠ xkeep L ps A rule φ.minus

/-- The core keeps a side cell iff the exact boolean does. -/
theorem keep_eq (h : Exact L ps A) (rule : Rule) {T : Cov} (hT : T ∈ L.topes) :
    L.keep rule T = xkeep L ps A rule T := by
  have hc := h.consistent
  unfold keep xkeep
  congr 1
  funext i
  by_cases hi : i < L.nb
  · have hm := memb_eq (m := mOf ps A) (fun T hT i hi => by
        apply Bool.eq_iff_iff.mpr; rw [mOf_iff]; exact h.witness T hT i hi)
      (fun φ hφ i hi => by
        rw [← h.boundary φ hφ i hi]; unfold Flip
        rw [← mOf_iff (ps := ps) (A := A), ← mOf_iff (ps := ps) (A := A)]
        cases mOf ps A φ.plus i <;> cases mOf ps A φ.minus i <;> simp) hi hT
    rw [hm]
    apply Bool.eq_iff_iff.mpr
    simp [hi, mOf_iff]
  · apply Bool.eq_iff_iff.mpr
    simp [hi]

/-- A cell with a point has the carriers' length. -/
theorem covCell_length : ∀ {ps : List Plane} {X : Cov} {x : V3}, covCell ps X x → X.length = ps.length
  | [], [], _, _ => rfl
  | _ :: _, _ :: _, _, ⟨_, h⟩ => by simp [covCell_length h]
  | [], _ :: _, _, h => h.elim
  | _ :: _, [], _, h => h.elim

/-- **The result is the exact boundary.** A facet is in the core's result iff it is an exact
boundary facet. -/
theorem result_iff (h : Exact L ps A) (rule : Rule) (φ : Cov) :
    φ ∈ L.result rule ↔ XBoundary L ps A rule φ := by
  rw [mem_result]
  constructor
  · rintro ⟨hf, hk⟩
    obtain ⟨f, hp⟩ := mem_facets.mp hf
    refine ⟨h.pieceReal _ hp, (h.pieceFacet _ hp).1, ?_⟩
    rwa [← keep_eq h rule (plus_mem_topes hf), ← keep_eq h rule (minus_mem_topes hf)]
  · rintro ⟨hr, hfa, hk⟩
    -- some body flips across φ, or the two sides would be kept alike
    have : ∃ i, i < L.nb ∧ Flip ps A φ i := by
      apply Classical.byContradiction
      intro hno
      apply hk
      unfold xkeep
      congr 1; funext i
      apply Bool.eq_iff_iff.mpr
      simp only [decide_eq_true_iff]
      by_cases hi : i < L.nb
      · have : ¬Flip ps A φ i := fun hfl => hno ⟨i, hi, hfl⟩
        unfold Flip at this
        have := Classical.not_not.mp this
        exact ⟨fun ⟨a, b⟩ => ⟨a, this.mp b⟩, fun ⟨a, b⟩ => ⟨a, this.mpr b⟩⟩
      · exact ⟨fun ⟨a, _⟩ => absurd a hi, fun ⟨a, _⟩ => absurd a hi⟩
    obtain ⟨i, hi, hfl⟩ := this
    have hf := h.complete φ hr hfa i hi hfl
    refine ⟨hf, ?_⟩
    rwa [keep_eq h rule (plus_mem_topes hf), keep_eq h rule (minus_mem_topes hf)]

/-! ## The regularised boolean -/

/-- The solids as the boolean's operands: bodies past `nb` are empty. -/
def opnds (L : Ledger) (A : Nat → Pred V3) : Nat → Pred V3 := fun i x => i < L.nb ∧ A i x

/-- The op as a pointwise rule on memberships. -/
noncomputable def ruleOp (rule : Rule) : (Nat → Prop) → Prop :=
  fun m => rule (fun i => @decide (m i) (Classical.propDecidable _)) = true

theorem opnds_compat (h : Exact L ps A) (T : Cov) (i : Nat) :
    (covCell ps T).Sub (opnds L A i) ∨ ∀ x, covCell ps T x → ¬opnds L A i x := by
  by_cases hi : i < L.nb
  · rcases h.compat T i with hs | hd
    · exact Or.inl fun x hx => ⟨hi, hs x hx⟩
    · exact Or.inr fun x hx ⟨_, ha⟩ => hd x hx ha
  · exact Or.inr fun x _ ⟨h', _⟩ => hi h'

/-- The arrangement's full cells, as a decomposition compatible with the operands. -/
def decomp (h : Exact L ps A) : Decomposition.{0, 0, 0} (opnds L A) :=
  topeDecomp ps h.nonzero (opnds L A) (opnds_compat h)

/-- The exact keep is the op at every point of the cell. -/
theorem xkeep_iff (h : Exact L ps A) (rule : Rule) (T : Cov) (x : V3) (hx : covCell ps T x) :
    xkeep L ps A rule T = true ↔ ruleOp rule fun i => opnds L A i x := by
  unfold xkeep ruleOp opnds
  have : (fun i => @decide (i < L.nb ∧ memT ps A T i) (Classical.propDecidable _)) =
      (fun i => @decide (i < L.nb ∧ A i x) (Classical.propDecidable _)) := by
    funext i
    apply Bool.eq_iff_iff.mpr
    simp only [decide_eq_true_iff]
    rcases h.compat T i with hs | hd
    · exact ⟨fun ⟨a, _⟩ => ⟨a, hs x hx⟩, fun ⟨a, _⟩ => ⟨a, x, hx, hs x hx⟩⟩
    · exact ⟨fun ⟨_, y, hy, hay⟩ => absurd hay (hd y hy), fun ⟨_, hax⟩ => absurd hax (hd x hx)⟩
  rw [this]

/-- **Theorem (c), correctness.** For an exact ledger: the core's result facets are exactly the
exact boundary facets, the facets between the cells the exact boolean keeps and those it drops;
and the regularised union of those kept cells is the regularised boolean of the snapped solids
(`reg_kept`). -/
theorem correct (h : Exact L ps A) (rule : Rule) :
    (∀ φ, φ ∈ L.result rule ↔ XBoundary L ps A rule φ) ∧
    reg ((decomp h).kept fun T => xkeep L ps A rule T = true) =
      reg (combine (ruleOp rule) (opnds L A)) :=
  ⟨result_iff h rule, reg_kept (decomp h) (ruleOp rule) _ fun T x hx => xkeep_iff h rule T x hx⟩

/-- **A valid result is the exact boolean.** If the core answers a body, the body is valid, its
faces partition exactly the exact boundary facets, and the cells those facets bound regularise to
the regularised boolean. -/
theorem correct_ok (h : Exact L ps A) (rule : Rule) {b : Body} (hb : core L rule = .ok b) :
    Valid L rule b ∧
    (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ XBoundary L ps A rule φ) ∧
    reg ((decomp h).kept fun T => xkeep L ps A rule T = true) =
      reg (combine (ruleOp rule) (opnds L A)) := by
  rcases coverage h.consistent rule with ⟨e, _⟩ | ⟨b', e, hv⟩ | ⟨x, e, _⟩
  · rw [e] at hb; cases hb
  · rw [e] at hb; cases hb
    exact ⟨hv, fun φ => (hv.cover φ).symm.trans (result_iff h rule φ), (correct h rule).2⟩
  · rw [e] at hb; cases hb

/-! ## Non-manifold, from the geometry alone -/

/-- Two exact boundary facets round `V` in one fan: joined by a chain of exact boundary facets
round `V`, each two sharing an edge cell round `V`. -/
inductive XFan (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (rule : Rule) (V : Cov) (a : Cov) :
    Cov → Prop
  | refl : XFan L ps A rule V a a
  | step {b c : Cov} : XFan L ps A rule V a b → XBoundary L ps A rule c → Cov.le V c = true →
      (∃ E, Realised ps E ∧ IsEdge ps E ∧ Cov.le V E = true ∧ Cov.le E b = true ∧ Cov.le E c = true) →
      XFan L ps A rule V a c

/-- **The exact result is non-manifold**: an edge cell with three exact boundary facets round it,
or a vertex cell with two exact boundary facets round it in different fans. -/
def XNonManifold (L : Ledger) (ps : List Plane) (A : Nat → Pred V3) (rule : Rule) : Prop :=
  (∃ E, Realised ps E ∧ IsEdge ps E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
    (XBoundary L ps A rule a ∧ Cov.le E a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le E b = true) ∧
    (XBoundary L ps A rule c ∧ Cov.le E c = true)) ∨
  (∃ V, Realised ps V ∧ IsVert ps V ∧ ∃ a b,
    (XBoundary L ps A rule a ∧ Cov.le V a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le V b = true) ∧
    ¬XFan L ps A rule V a b)

theorem three_iff {α : Type} [DecidableEq α] {l : List α} (hl : l.Nodup) :
    2 < l.length ↔ ∃ a ∈ l, ∃ b ∈ l, ∃ c ∈ l, a ≠ b ∧ a ≠ c ∧ b ≠ c := by
  constructor
  · intro h
    match l, hl, h with
    | a :: b :: c :: _, hl, _ =>
      simp only [List.nodup_cons, List.mem_cons, not_or] at hl
      refine ⟨a, by simp, b, by simp, c, by simp, ?_, ?_, ?_⟩
      · exact hl.1.1
      · exact hl.1.2.1
      · exact hl.2.1.1
  · rintro ⟨a, ha, b, hb, c, hc, hab, hac, hbc⟩
    have hn : [a, b, c].Nodup := by simp [hab, hac, hbc]
    have := hn.length_le_of_subset (by
      intro x hx; simp only [List.mem_cons, List.mem_nil_iff, or_false] at hx
      rcases hx with rfl | rfl | rfl <;> assumption)
    simp at this; omega

theorem badEdge_iff (h : Exact L ps A) (rule : Rule) :
    (∃ e ∈ L.edges, L.badEdge rule e = true) ↔
    ∃ E, Realised ps E ∧ IsEdge ps E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
      (XBoundary L ps A rule a ∧ Cov.le E a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le E b = true) ∧
      (XBoundary L ps A rule c ∧ Cov.le E c = true) := by
  have mem : ∀ e φ, φ ∈ L.around rule e ↔ XBoundary L ps A rule φ ∧ Cov.le e φ = true := by
    intro e φ; rw [mem_around, result_iff h rule]
  constructor
  · rintro ⟨e, he, hb⟩
    simp only [badEdge, decide_eq_true_eq] at hb
    obtain ⟨a, ha, b, hb', c, hc, hab, hac, hbc⟩ := (three_iff (around_nodup L rule e)).mp hb
    obtain ⟨hr, hed, _⟩ := (h.edges e).mp he
    exact ⟨e, hr, hed, a, b, c, hab, hac, hbc, (mem e a).mp ha, (mem e b).mp hb', (mem e c).mp hc⟩
  · rintro ⟨E, hr, hed, a, b, c, hab, hac, hbc, ha, hb, hc⟩
    have haf := (mem_result.mp ((result_iff h rule a).mpr ha.1)).1
    refine ⟨E, (h.edges E).mpr ⟨hr, hed, a, haf, ha.2⟩, ?_⟩
    simp only [badEdge, decide_eq_true_eq]
    exact (three_iff (around_nodup L rule E)).mpr
      ⟨a, (mem E a).mpr ha, b, (mem E b).mpr hb, c, (mem E c).mpr hc, hab, hac, hbc⟩

theorem badVert_iff (L : Ledger) (rule : Rule) (v : Cov) :
    L.badVert rule v = true ↔ ∃ c ∈ L.around rule v, ∃ d ∈ L.around rule v,
      ¬Conn (L.fanRel v) (L.around rule v) c d := by
  have hand := around_nodup L rule v
  have hfs := fanRel_symm L v
  unfold badVert
  split
  · rename_i hnil; simp [hnil]
  · rename_i c0 rest hcons
    rw [hcons] at hand ⊢
    have h0 : c0 ∈ c0 :: rest := List.mem_cons_self ..
    simp only [List.any_eq_true, Bool.not_eq_true']
    constructor
    · rintro ⟨d, hd, hnot⟩
      refine ⟨c0, h0, d, hd, fun hc => ?_⟩
      have := (cls_spec _ hand h0 d).mpr hc
      simp [this] at hnot
    · rintro ⟨c, hc, d, hd, hnot⟩
      apply Classical.byContradiction
      intro hall
      have hall' : ∀ x ∈ c0 :: rest, Conn (L.fanRel v) (c0 :: rest) c0 x := by
        intro x hx
        apply (cls_spec _ hand h0 x).mp
        apply Classical.byContradiction; intro hn
        exact hall ⟨x, hx, by simpa using hn⟩
      exact hnot ((Conn.symm hfs h0 (hall' c hc)).trans (hall' d hd))

/-- Fans by the ledger are fans by the geometry. -/
theorem fan_iff (h : Exact L ps A) (rule : Rule) (v : Cov) {c d : Cov} (_hc : c ∈ L.around rule v) :
    Conn (L.fanRel v) (L.around rule v) c d ↔ XFan L ps A rule v c d := by
  constructor
  · intro hcd
    induction hcd with
    | refl => exact XFan.refl
    | step _ hy hr ih =>
      rw [mem_around, result_iff h rule] at hy
      simp only [fanRel, List.any_eq_true, Bool.and_eq_true] at hr
      obtain ⟨e, he, ⟨hve, heb⟩, hec⟩ := hr
      obtain ⟨her, hed, _⟩ := (h.edges e).mp he
      exact XFan.step ih hy.1 hy.2 ⟨e, her, hed, hve, heb, hec⟩
  · intro hcd
    induction hcd with
    | refl => exact Conn.refl
    | step _ hb hvc hE ih =>
      obtain ⟨E, her, hed, hvE, hEb, hEc⟩ := hE
      have hm : _ ∈ L.around rule v := (mem_around).mpr ⟨(result_iff h rule _).mpr hb, hvc⟩
      have hf := (mem_result.mp ((result_iff h rule _).mpr hb)).1
      refine Conn.step ih hm ?_
      simp only [fanRel, List.any_eq_true, Bool.and_eq_true]
      exact ⟨E, (h.edges E).mpr ⟨her, hed, _, hf, hEc⟩, ⟨hvE, hEb⟩, hEc⟩

theorem badVert_iff_x (h : Exact L ps A) (rule : Rule) :
    (∃ v ∈ L.verts, L.badVert rule v = true) ↔
    ∃ V, Realised ps V ∧ IsVert ps V ∧ ∃ a b,
      (XBoundary L ps A rule a ∧ Cov.le V a = true) ∧ (XBoundary L ps A rule b ∧ Cov.le V b = true) ∧
      ¬XFan L ps A rule V a b := by
  have mem : ∀ e φ, φ ∈ L.around rule e ↔ XBoundary L ps A rule φ ∧ Cov.le e φ = true := by
    intro e φ; rw [mem_around, result_iff h rule]
  constructor
  · rintro ⟨v, hv, hb⟩
    obtain ⟨c, hc, d, hd, hn⟩ := (badVert_iff L rule v).mp hb
    obtain ⟨hr, hvx, _⟩ := (h.verts v).mp hv
    exact ⟨v, hr, hvx, c, d, (mem v c).mp hc, (mem v d).mp hd,
      fun hx => hn ((fan_iff h rule v hc).mpr hx)⟩
  · rintro ⟨V, hr, hvx, a, b, ha, hb, hn⟩
    have haf := (mem_result.mp ((result_iff h rule a).mpr ha.1)).1
    have hma := (mem V a).mpr ha
    refine ⟨V, (h.verts V).mpr ⟨hr, hvx, a, haf, ha.2⟩, (badVert_iff L rule V).mpr
      ⟨a, hma, b, (mem V b).mpr hb, fun hc => hn ((fan_iff h rule V hma).mp hc)⟩⟩

/-- The core refuses `ZeroThickness` exactly at a bad edge or vertex (for a consistent ledger). -/
theorem refused_iff (h : Consistent L) (rule : Rule) :
    (∃ x, core L rule = .refused (.zeroThickness x)) ↔
    (∃ e ∈ L.edges, L.badEdge rule e = true) ∨ (∃ v ∈ L.verts, L.badVert rule v = true) := by
  rw [core_eq h rule]
  cases hE : L.edges.find? (L.badEdge rule) with
  | some e =>
    simp only
    exact ⟨fun _ => Or.inl ⟨e, List.mem_of_find?_eq_some hE, List.find?_some hE⟩,
      fun _ => ⟨e, rfl⟩⟩
  | none =>
    have hn : ¬∃ e ∈ L.edges, L.badEdge rule e = true := fun ⟨e, he, hb⟩ => by
      have := List.find?_eq_none.mp hE e he; simp_all
    cases hV : L.verts.find? (L.badVert rule) with
    | some v =>
      simp only
      exact ⟨fun _ => Or.inr ⟨v, List.mem_of_find?_eq_some hV, List.find?_some hV⟩,
        fun _ => ⟨v, rfl⟩⟩
    | none =>
      have hn' : ¬∃ v ∈ L.verts, L.badVert rule v = true := fun ⟨v, hv, hb⟩ => by
        have := List.find?_eq_none.mp hV v hv; simp_all
      simp only
      constructor
      · rintro ⟨x, hx⟩
        split at hx <;> cases hx
      · rintro (h1 | h1)
        · exact absurd h1 hn
        · exact absurd h1 hn'

/-- **Theorem (c), the refusal.** For an exact ledger, the core refuses `ZeroThickness` iff the
snapped exact result is non-manifold. -/
theorem zeroThickness_iff (h : Exact L ps A) (rule : Rule) :
    (∃ x, core L rule = .refused (.zeroThickness x)) ↔ XNonManifold L ps A rule := by
  rw [refused_iff h.consistent rule, badEdge_iff h rule, badVert_iff_x h rule]
  rfl

end ForgeExact.DecideOnce
