import ForgeExact.DecideOnce.MetricI

/-!
# A8: (b) for the incidence core, with the metric collapse (`coverageMI`)

Coverage.lean and Metric.lean's `coverageK` / `coverageM`, over `IncLedger`. Past its two checks
(split and classify), and for any room keep table, the core ends in one of three ways:
* `empty`, with no result facet;
* a valid body (`ValidI`);
* a ZeroThickness refusal whose stated condition holds: more than two result wings round an edge,
  or two fans round a vert.

With the collapse there is a fourth: a refusal at a thin room whose feature's rim is kept both
ways, or which is not enclosed. This is the owner's rule that nothing that works may fail and
refusals are exact, for the metric path. Naming is the write's (the least of the pieces' wholes),
so `ValidI` has no `named` clause.
-/

set_option linter.unusedSectionVars false

namespace ForgeExact.DecideOnce

universe u

/-- A valid result body of the incidence core. -/
structure ValidI {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α) (kt : KeepTabI α)
    (b : BodyI α) : Prop where
  nonempty : b.faces ≠ []
  cover : ∀ c, c ∈ L.result kt ↔ ∃ F ∈ b.faces, c ∈ F.facets
  disjoint : ∀ F ∈ b.faces, ∀ G ∈ b.faces, ∀ c, c ∈ F.facets → c ∈ G.facets → F = G
  flat : ∀ F ∈ b.faces, ∀ c ∈ F.facets, L.carrier c = F.plane ∧ L.orient kt c = F.orient
  oneSide : ∀ c ∈ L.result kt, keepInI kt (L.plus c) ≠ keepInI kt (L.minus c)
  edgeManifold : ∀ e ∈ L.edges, (L.aroundE kt e).length ≤ 2
  vertexManifold : ∀ v ∈ L.verts, ∀ c ∈ L.aroundV kt v, ∀ d ∈ L.aroundV kt v,
    Conn IncLedger.fanRel (L.aroundV kt v) c d

/-- The stated condition of a ZeroThickness refusal. -/
def ZeroThicknessAtI {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α) (kt : KeepTabI α)
    (x : α) : Prop :=
  (x ∈ L.edges ∧ 2 < (L.aroundE kt x).length) ∨
  (x ∈ L.verts ∧ ∃ c ∈ L.aroundV kt x, ∃ d ∈ L.aroundV kt x, ¬Conn IncLedger.fanRel (L.aroundV kt x) c d)

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] {L : IncLedger α}

theorem unify_validI {kt : KeepTabI α} (hnd : L.facets.Nodup) (hsnd : ∀ v ∈ L.verts, (L.vertSectors v).Nodup)
    (hne : L.result kt ≠ []) (hE : L.edges.find? (L.badEdge kt) = none) (hV : L.verts.find? (L.badVert kt) = none) :
    ValidI L kt (L.unify kt) := by
  have rn := L.result_nodup kt hnd
  have hsym := L.joinRel_symm kt
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · obtain ⟨c, hc⟩ := List.exists_mem_of_ne_nil _ hne
    obtain ⟨hd, hhd, _⟩ := L.in_some_face hnd hc
    rw [unify_eq]
    simp only [ne_eq, List.map_eq_nil_iff]
    intro hnil; rw [hnil] at hhd; exact nomatch hhd
  · intro c
    rw [unify_eq]
    simp only [List.mem_map]
    constructor
    · intro hc
      obtain ⟨hd, hhd, hcf⟩ := L.in_some_face hnd hc
      exact ⟨_, ⟨hd, hhd, rfl⟩, hcf⟩
    · rintro ⟨F, ⟨hd, hhd, rfl⟩, hcf⟩
      exact ((L.mem_faceOf hnd ((L.mem_heads).mp hhd).1).mp hcf).1
  · rw [unify_eq]
    simp only [List.mem_map]
    rintro F ⟨h1, hh1, rfl⟩ G ⟨h2, hh2, rfl⟩ c hc1 hc2
    have m1 := (L.mem_heads).mp hh1
    have m2 := (L.mem_heads).mp hh2
    have c1 := ((L.mem_faceOf hnd m1.1).mp hc1).2
    have c2 := ((L.mem_faceOf hnd m2.1).mp hc2).2
    have e1 := head_eq (L.joinRel kt) hsym rn m1.1 c1
    have e2 := head_eq (L.joinRel kt) hsym rn m2.1 c2
    rw [m1.2] at e1; rw [m2.2] at e2
    have : h1 = h2 := Option.some.inj (e1.trans e2.symm)
    subst this; rfl
  · rw [unify_eq]
    simp only [List.mem_map]
    rintro F ⟨hd, hhd, rfl⟩ c hc
    have hm := (L.mem_heads).mp hhd
    have hconn := ((L.mem_faceOf hnd hm.1).mp hc).2
    have := L.conn_carrier hconn
    exact ⟨this.1.symm, this.2.symm⟩
  · intro c hc; exact (L.mem_result.mp hc).2
  · intro e he
    have := List.find?_eq_none.mp hE e he
    simp only [badEdge, decide_eq_true_eq] at this
    omega
  · intro v hv c hc d hd
    have hb := List.find?_eq_none.mp hV v hv
    have hand : (L.aroundV kt v).Nodup := (hsnd v hv).filter _
    refine Classical.byContradiction fun hn => ?_
    have := (L.badVert_iff kt v hand).mpr ⟨c, hc, d, hd, hn⟩
    rw [this] at hb; exact absurd rfl hb

/-- (b) past classification, for any room keep table. -/
theorem coverageKI (hnd : L.facets.Nodup) (hsnd : ∀ v ∈ L.verts, (L.vertSectors v).Nodup) (kt : KeepTabI α) :
    (L.coreK kt = .empty ∧ L.result kt = []) ∨
    (∃ b, L.coreK kt = .ok b ∧ ValidI L kt b) ∨
    (∃ x, L.coreK kt = .refused (.zeroThickness x) ∧ ZeroThicknessAtI L kt x) := by
  unfold coreK
  cases hE : L.edges.find? (L.badEdge kt) with
  | some e =>
    right; right
    refine ⟨e, rfl, Or.inl ⟨List.mem_of_find?_eq_some hE, ?_⟩⟩
    have := List.find?_some hE
    simpa [badEdge] using this
  | none =>
    cases hV : L.verts.find? (L.badVert kt) with
    | some v =>
      right; right
      have hv := List.mem_of_find?_eq_some hV
      exact ⟨v, rfl, Or.inr ⟨hv, (L.badVert_iff kt v ((hsnd v hv).filter _)).mp (List.find?_some hV)⟩⟩
    | none =>
      simp only
      by_cases hr : L.result kt = []
      · left; simp [hr]
      · right; left
        simp only [hr, ite_false]
        exact ⟨_, rfl, unify_validI hnd hsnd hr hE hV⟩

/-- **(b) with the metric collapse.** For a ledger past its split and classify checks, every op, and
every thin and enclosed table, the core with the collapse ends in one of four ways:
* empty;
* a valid body of the collapsed table;
* a ZeroThickness refusal whose stated condition holds;
* a refusal at a thin room whose feature's rim is kept both ways, or which is not enclosed. -/
theorem coverageMI (hnd : L.facets.Nodup) (hsnd : ∀ v ∈ L.verts, (L.vertSectors v).Nodup)
    (hs : L.splitOk = true) (hc : L.classifyOk = true) (rule : Rule) (thin enc : List α) :
    (∃ T, L.coreMI rule thin enc = .refused (.zeroThickness T) ∧
      T ∈ L.thinTopes thin ∧
      ((∃ U ∈ L.rim thin T, ∃ V ∈ L.rim thin T, keepInI (L.keepTab rule) U ≠ keepInI (L.keepTab rule) V) ∨
       (∃ V ∈ L.comp thin T, enc.contains V = false))) ∨
    ∃ kt', L.collapse (L.keepTab rule) thin enc = .ok kt' ∧
      ((L.coreMI rule thin enc = .empty ∧ L.result kt' = []) ∨
       (∃ b, L.coreMI rule thin enc = .ok b ∧ ValidI L kt' b) ∨
       (∃ x, L.coreMI rule thin enc = .refused (.zeroThickness x) ∧ ZeroThicknessAtI L kt' x)) := by
  have e : L.coreMI rule thin enc = match L.collapse (L.keepTab rule) thin enc with
      | .refuse T => .refused (.zeroThickness T)
      | .ok kt' => L.coreK kt' := by
    unfold coreMI; rw [hs, hc]; rfl
  rw [e]
  cases hcl : L.collapse (L.keepTab rule) thin enc with
  | refuse T => exact Or.inl ⟨T, rfl, collapse_refuse hcl⟩
  | ok kt' => exact Or.inr ⟨kt', rfl, coverageKI hnd hsnd kt'⟩

end IncLedger

end ForgeExact.DecideOnce
