import ForgeExact.DecideOnce.Core

/-!
# Coverage: every consistent ledger ends, with no retry, in a valid body or a stated refusal

**Theorem (b), `coverage`.** For every *consistent* ledger (any sign assignment that some
membership realises: not only the geometric one) and every op, `core` returns one of

* `empty`, and then the result has no facet;
* `ok b`, and then `b` is `Valid`: its faces partition the result's facets, each face is flat (one
  carrier, one sense), every edge has at most two result facets round it, every vertex one fan, and
  the faces are listed in the order of their first facets (arena order: names stay stable);
* `refused (zeroThickness x)`, and then `x` is an edge with more than two result facets round it, or
  a vertex with two result facets in different fans (`ZeroThicknessAt`).

`SplitCrossed` and `InconsistentClassification`, the two refusals that started the retry ladder
(boolean.rs `plan_retried`), are unreachable (`splitOk_of`, `classifyOk_of`). `core` is a total
function (Lean accepts it with no `partial`), so it terminates; it is one pass, so no rung exists
to fire.

The heart is `memb_eq`, **the flood is right**: for any membership `m` that realises the ledger
(witness answers and boundary facts), the flood's membership of every side cell is `m`'s, by
induction along the chain that joins the cell to its class's witness.

**Consistent** (`Consistent`): each piece is a facet of its face's carrier, in a body that exists;
no facet is inside two faces of one body (the input is a valid B-rep); and some membership `m`
gives the witness answers and flips exactly across the pieces of each body. A geometric ledger is
consistent by Correct.lean (`Exact.consistent`), with `m` the snapped solids' own membership.
-/

namespace ForgeExact.DecideOnce

open Ledger

/-- A consistent ledger. -/
structure Consistent (L : Ledger) : Prop where
  pieceFacet : ∀ p ∈ L.pieces, p.1.isFacet = true ∧ p.1.zeroIdx = L.clsOf p.2 ∧ L.bodyOf p.2 < L.nb
  oneFace : ∀ p ∈ L.pieces, ∀ q ∈ L.pieces, q.1 = p.1 → L.bodyOf q.2 = L.bodyOf p.2 → q.2 = p.2
  realised : ∃ m : Cov → Nat → Bool,
    (∀ T ∈ L.topes, ∀ i, i < L.nb → L.inside.contains (T, i) = m T i) ∧
    ∀ φ ∈ L.facets, ∀ i, i < L.nb → (m φ.plus i ≠ m φ.minus i ↔ L.isPieceOf i φ = true)

/-- A valid result body. -/
structure Valid (L : Ledger) (kt : KeepTab) (b : Body) : Prop where
  nonempty : b.faces ≠ []
  cover : ∀ c, c ∈ L.result kt ↔ ∃ F ∈ b.faces, c ∈ F.facets
  disjoint : ∀ F ∈ b.faces, ∀ G ∈ b.faces, ∀ c, c ∈ F.facets → c ∈ G.facets → F = G
  flat : ∀ F ∈ b.faces, ∀ c ∈ F.facets, c.zeroIdx = F.plane ∧ Ledger.orient kt c = F.orient
  oneSide : ∀ c ∈ L.result kt, keepIn kt c.plus ≠ keepIn kt c.minus
  edgeManifold : ∀ e ∈ L.edges, (L.around kt e).length ≤ 2
  vertexManifold : ∀ v ∈ L.verts, ∀ c ∈ L.around kt v, ∀ d ∈ L.around kt v,
    Conn (L.fanRel v) (L.around kt v) c d
  /-- names: faces in the order of their first facets, each named by that facet's input face -/
  named : b.faces.map OutFace.origin = (L.heads kt).map L.origin ∧
    (L.heads kt).Sublist (L.result kt)

/-- The stated condition of a `ZeroThickness` refusal. -/
def ZeroThicknessAt (L : Ledger) (kt : KeepTab) (x : Cov) : Prop :=
  (x ∈ L.edges ∧ 2 < (L.around kt x).length) ∨
  (x ∈ L.verts ∧ ∃ c ∈ L.around kt x, ∃ d ∈ L.around kt x,
    ¬Conn (L.fanRel x) (L.around kt x) c d)

namespace Ledger

variable {L : Ledger}

/-! ## Lists -/

theorem facets_nodup (L : Ledger) : L.facets.Nodup := nodup_dedup _
theorem topes_nodup (L : Ledger) : L.topes.Nodup := nodup_dedup _

theorem result_nodup (L : Ledger) (kt : KeepTab) : (L.result kt).Nodup :=
  (facets_nodup L).filter _

theorem around_nodup (L : Ledger) (kt : KeepTab) (x : Cov) : (L.around kt x).Nodup :=
  (result_nodup L kt).filter _

theorem mem_facets {φ : Cov} : φ ∈ L.facets ↔ ∃ f, (φ, f) ∈ L.pieces := by
  simp [facets, mem_dedup]

theorem plus_mem_topes {φ : Cov} (h : φ ∈ L.facets) : φ.plus ∈ L.topes := by
  simp only [topes, mem_dedup, List.mem_flatMap]
  exact ⟨φ, h, by simp⟩

theorem minus_mem_topes {φ : Cov} (h : φ ∈ L.facets) : φ.minus ∈ L.topes := by
  simp only [topes, mem_dedup, List.mem_flatMap]
  exact ⟨φ, h, by simp⟩

theorem mem_result {kt : KeepTab} {φ : Cov} :
    φ ∈ L.result kt ↔ φ ∈ L.facets ∧ keepIn kt φ.plus ≠ keepIn kt φ.minus := by
  simp [result]

theorem mem_around {kt : KeepTab} {x φ : Cov} :
    φ ∈ L.around kt x ↔ φ ∈ L.result kt ∧ Cov.le x φ = true := by
  simp [around]

/-- The keep table returns each side cell's keep. -/
theorem keepIn_tab (L : Ledger) (rule : Rule) {T : Cov} (hT : T ∈ L.topes) :
    keepIn (L.keepTab rule) T = L.keep rule T := by
  unfold keepIn keepTab
  rw [find?_pair (L.keep rule) T _ hT]
  rfl

/-! ## The flood is right -/

/-- One step of the flood keeps a realising membership. -/
theorem adj_keeps {m : Cov → Nat → Bool}
    (h2 : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (m φ.plus i ≠ m φ.minus i ↔ L.isPieceOf i φ = true))
    {i : Nat} (hi : i < L.nb) {x y : Cov} (h : L.adj i x y = true) : m x i = m y i := by
  simp only [adj, adjOf, crossings, List.any_map, List.any_filter, List.any_eq_true,
    Bool.and_eq_true, Bool.not_eq_true', Bool.or_eq_true, beq_iff_eq, Function.comp_def] at h
  obtain ⟨φ, hφ, hnp, hxy⟩ := h
  have he : m φ.plus i = m φ.minus i := by
    apply Classical.byContradiction; intro hne
    have := (h2 φ hφ i hi).mp hne; rw [hnp] at this; cases this
  rcases hxy with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
  · exact he
  · exact he.symm

/-- **The flood is right.** For any membership that gives the witness answers and flips exactly
across each body's pieces, the flood's membership of every side cell is that membership. -/
theorem memb_eq {m : Cov → Nat → Bool}
    (h1 : ∀ T ∈ L.topes, ∀ i, i < L.nb → L.inside.contains (T, i) = m T i)
    (h2 : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (m φ.plus i ≠ m φ.minus i ↔ L.isPieceOf i φ = true))
    {i : Nat} (hi : i < L.nb) {T : Cov} (hT : T ∈ L.topes) : L.memb i T = m T i := by
  obtain ⟨R, hR, hRm, hc⟩ := head_some (L.adj i) (topes_nodup L) hT
  unfold memb
  dsimp only
  rw [show adjOf (L.crossings i) = L.adj i from rfl, hR]
  simp only
  rw [h1 R hRm i hi]
  exact (Conn.invariant (P := fun x => m x i = m T i) hc rfl
    fun x y hx _ hxy => (adj_keeps h2 hi hxy).symm.trans hx)

/-! ## The two internal refusals are unreachable -/

theorem splitOk_of (h : Consistent L) : L.splitOk = true := by
  simp only [splitOk, List.all_eq_true, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq,
    Bool.or_eq_true, Bool.not_eq_true']
  intro p hp
  obtain ⟨h1, h2, h3⟩ := h.pieceFacet p hp
  refine ⟨⟨⟨h1, h2⟩, h3⟩, fun q hq => ?_⟩
  by_cases e : q.1 = p.1 ∧ L.bodyOf q.2 = L.bodyOf p.2
  · exact Or.inr (h.oneFace p hp q hq e.1 e.2)
  · left
    simp only [Bool.and_eq_false_iff, beq_eq_false_iff_ne, ne_eq]
    by_cases e1 : q.1 = p.1
    · exact Or.inr fun e2 => e ⟨e1, e2⟩
    · exact Or.inl e1

theorem classifyOk_of (h : Consistent L) : L.classifyOk = true := by
  obtain ⟨m, h1, h2⟩ := h.realised
  simp only [classifyOk, List.all_eq_true, List.mem_range, Bool.or_eq_true, Bool.not_eq_true',
    bne_iff_ne, ne_eq]
  intro φ hφ i hi
  cases hp : L.isPieceOf i φ
  · exact Or.inl rfl
  · right
    rw [memb_eq h1 h2 hi (plus_mem_topes hφ), memb_eq h1 h2 hi (minus_mem_topes hφ)]
    exact (h2 φ hφ i hi).mpr hp

/-- Past the two checks, the core is the touch rule and unify. -/
theorem core_eq (h : Consistent L) (rule : Rule) :
    core L rule = match L.edges.find? (L.badEdge (L.keepTab rule)) with
      | some e => .refused (.zeroThickness e)
      | none => match L.verts.find? (L.badVert (L.keepTab rule)) with
        | some v => .refused (.zeroThickness v)
        | none => if L.result (L.keepTab rule) = [] then .empty else .ok (L.unify (L.keepTab rule)) := by
  unfold core
  rw [splitOk_of h, classifyOk_of h]
  rfl

/-! ## Symmetric relations -/

theorem fanRel_symm (L : Ledger) (v : Cov) : ∀ c d, L.fanRel v c d = L.fanRel v d c := by
  intro c d
  simp only [fanRel]
  congr 1; funext e
  cases Cov.le v e <;> cases Cov.le e c <;> cases Cov.le e d <;> rfl

theorem joinRel_symm (L : Ledger) (kt : KeepTab) : ∀ c d, L.joinRel kt c d = L.joinRel kt d c := by
  intro c d
  simp only [joinRel]
  have e1 : (c.zeroIdx == d.zeroIdx) = (d.zeroIdx == c.zeroIdx) := by
    cases h : c.zeroIdx == d.zeroIdx <;> cases h' : d.zeroIdx == c.zeroIdx <;> simp_all
  have e2 : (Ledger.orient kt c == Ledger.orient kt d) = (Ledger.orient kt d == Ledger.orient kt c) := by
    cases h : Ledger.orient kt c == Ledger.orient kt d <;> cases h' : Ledger.orient kt d == Ledger.orient kt c <;>
      simp_all
  rw [e1, e2]
  have : (fun e => Cov.le e c && Cov.le e d) = (fun e => Cov.le e d && Cov.le e c) := by
    funext e; cases Cov.le e c <;> cases Cov.le e d <;> rfl
  rw [this]

/-! ## The body unify writes -/

theorem mem_heads {kt : KeepTab} {h : Cov} :
    h ∈ L.heads kt ↔ h ∈ L.result kt ∧ head (L.joinRel kt) (L.result kt) h = some h := by
  simp [heads]

theorem mem_faceOf {kt : KeepTab} {h c : Cov} (hh : h ∈ L.result kt) :
    c ∈ L.faceOf kt h ↔ c ∈ L.result kt ∧ Conn (L.joinRel kt) (L.result kt) h c := by
  simp only [faceOf, List.mem_filter, List.contains_eq_mem, decide_eq_true_eq]
  rw [cls_spec _ (result_nodup L kt) hh]

/-- Every result facet is in the face of its class's head. -/
theorem in_some_face {kt : KeepTab} {c : Cov} (hc : c ∈ L.result kt) :
    ∃ h ∈ L.heads kt, c ∈ L.faceOf kt h := by
  obtain ⟨h, hhd, hm, hconn⟩ := head_some (L.joinRel kt) (result_nodup L kt) hc
  have hh : head (L.joinRel kt) (L.result kt) h = some h := by
    rw [← head_eq (L.joinRel kt) (joinRel_symm L kt) (result_nodup L kt) hc hconn]; exact hhd
  refine ⟨h, mem_heads.mpr ⟨hm, hh⟩, (mem_faceOf hm).mpr ⟨hc, ?_⟩⟩
  exact Conn.symm (joinRel_symm L kt) hc hconn

theorem unify_valid {kt : KeepTab} (hne : L.result kt ≠ [])
    (hE : L.edges.find? (L.badEdge kt) = none) (hV : L.verts.find? (L.badVert kt) = none) :
    Valid L kt (L.unify kt) := by
  have hnd := result_nodup L kt
  have hsym := joinRel_symm L kt
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- nonempty
    obtain ⟨c, hc⟩ := List.exists_mem_of_ne_nil _ hne
    obtain ⟨hd, hhd, _⟩ := in_some_face hc
    simp only [unify, ne_eq, List.map_eq_nil_iff]
    intro hnil; rw [hnil] at hhd; exact nomatch hhd
  · -- cover
    intro c
    simp only [unify, List.mem_map]
    constructor
    · intro hc
      obtain ⟨hd, hhd, hcf⟩ := in_some_face hc
      exact ⟨_, ⟨hd, hhd, rfl⟩, hcf⟩
    · rintro ⟨F, ⟨hd, hhd, rfl⟩, hcf⟩
      exact ((mem_faceOf (mem_heads.mp hhd).1).mp hcf).1
  · -- disjoint
    simp only [unify, List.mem_map]
    rintro F ⟨h1, hh1, rfl⟩ G ⟨h2, hh2, rfl⟩ c hc1 hc2
    have m1 := mem_heads.mp hh1
    have m2 := mem_heads.mp hh2
    have c1 := ((mem_faceOf m1.1).mp hc1).2
    have c2 := ((mem_faceOf m2.1).mp hc2).2
    have e1 := head_eq (L.joinRel kt) hsym hnd m1.1 c1
    have e2 := head_eq (L.joinRel kt) hsym hnd m2.1 c2
    rw [m1.2] at e1; rw [m2.2] at e2
    have : h1 = h2 := Option.some.inj (e1.trans e2.symm)
    subst this; rfl
  · -- flat
    simp only [unify, List.mem_map]
    rintro F ⟨hd, hhd, rfl⟩ c hc
    have hm := mem_heads.mp hhd
    have hconn := ((mem_faceOf hm.1).mp hc).2
    exact Conn.invariant (P := fun x => x.zeroIdx = hd.zeroIdx ∧ Ledger.orient kt x = Ledger.orient kt hd)
      hconn ⟨rfl, rfl⟩ fun x y hx _ hxy => by
        simp only [joinRel, Bool.and_eq_true, beq_iff_eq] at hxy
        exact ⟨hxy.1.1.symm.trans hx.1, hxy.1.2.symm.trans hx.2⟩
  · -- one side kept
    intro c hc; exact (mem_result.mp hc).2
  · -- edges
    intro e he
    have := List.find?_eq_none.mp hE e he
    simp only [badEdge, decide_eq_true_eq] at this
    omega
  · -- vertices: one fan
    intro v hv c hc d hd
    have hb := List.find?_eq_none.mp hV v hv
    have hand := around_nodup L kt v
    have hfs := fanRel_symm L v
    unfold badVert at hb
    split at hb
    · rename_i hnil; rw [hnil] at hc; exact nomatch hc
    · rename_i c0 rest hcons
      rw [hcons] at hc hd hand hb ⊢
      have h0 : c0 ∈ c0 :: rest := List.mem_cons_self ..
      have hall : ∀ x ∈ c0 :: rest, Conn (L.fanRel v) (c0 :: rest) c0 x := by
        intro x hx
        simp only [List.any_eq_true, Bool.not_eq_true', not_exists, not_and, Bool.not_eq_false,
          List.contains_eq_mem, decide_eq_true_eq] at hb
        have := hb x hx
        exact (cls_spec _ hand h0 x).mp (by simpa using this)
      exact (Conn.symm hfs h0 (hall c hc)).trans (hall d hd)
  · -- names
    refine ⟨?_, List.filter_sublist⟩
    simp [unify, List.map_map, Function.comp_def]

/-! ## Theorem (b) -/

/-- **Coverage.** For every consistent ledger and every op, the core terminates, with no retry,
in `empty` (no result facet), in a valid body, or in a `ZeroThickness` refusal whose stated
condition holds; never `SplitCrossed` nor `InconsistentClassification`. -/
theorem coverage (h : Consistent L) (rule : Rule) :
    (core L rule = .empty ∧ L.result (L.keepTab rule) = []) ∨
    (∃ b, core L rule = .ok b ∧ Valid L (L.keepTab rule) b) ∨
    (∃ x, core L rule = .refused (.zeroThickness x) ∧ ZeroThicknessAt L (L.keepTab rule) x) := by
  rw [core_eq h rule]
  cases hE : L.edges.find? (L.badEdge (L.keepTab rule)) with
  | some e =>
    right; right
    refine ⟨e, rfl, Or.inl ⟨List.mem_of_find?_eq_some hE, ?_⟩⟩
    have := List.find?_some hE
    simpa [badEdge] using this
  | none =>
    cases hV : L.verts.find? (L.badVert (L.keepTab rule)) with
    | some v =>
      right; right
      refine ⟨v, rfl, Or.inr ⟨List.mem_of_find?_eq_some hV, ?_⟩⟩
      have hb := List.find?_some hV
      have hand := around_nodup L (L.keepTab rule) v
      unfold badVert at hb
      split at hb
      · cases hb
      · rename_i c0 rest hcons
        rw [hcons] at hand hb ⊢
        simp only [List.any_eq_true, Bool.not_eq_true'] at hb
        obtain ⟨d, hd, hnot⟩ := hb
        refine ⟨c0, List.mem_cons_self .., d, hd, fun hc => ?_⟩
        have := (cls_spec _ hand (List.mem_cons_self ..) d).mpr hc
        simp [this] at hnot
    | none =>
      simp only
      by_cases hr : L.result (L.keepTab rule) = []
      · left; simp [hr]
      · right; left
        simp only [hr, ite_false]
        exact ⟨_, rfl, unify_valid hr hE hV⟩

/-- **No retry.** For a consistent ledger the core never refuses `SplitCrossed` or
`InconsistentClassification`, the two refusals the retry ladder caught. -/
theorem no_retry (h : Consistent L) (rule : Rule) :
    core L rule ≠ .refused .splitCrossed ∧ core L rule ≠ .refused .inconsistentClassification := by
  rcases coverage h rule with ⟨e, _⟩ | ⟨b, e, _⟩ | ⟨x, e, _⟩
  all_goals (rw [e]; constructor <;> intro h <;> cases h)

end Ledger

end ForgeExact.DecideOnce
