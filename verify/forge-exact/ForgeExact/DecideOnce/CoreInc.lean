import ForgeExact.DecideOnce.IncCore
import ForgeExact.DecideOnce.Coverage

/-!
# The plane ledger as an incidence ledger: `core L` is `coreI (inc L)`

face-arrangement.md §2.1. `inc L` reads the plane ledger's covectors as incidence:
* the carrier is `zeroIdx`;
* the rooms are `plus` and `minus`;
* the facets round an edge are those it is a face of (`Cov.le`);
* there is one sector per facet round a vert, bounded by the listed edges between them.

`core_inc` says that for a ledger whose split is sound, `coreI (inc L)` is `core L` with each
face's name dropped (`Outcome.toI`). So every theorem about `core` (Correct.lean, `end_to_end_proof`)
is a theorem about `coreI` on `inc (sampleLedger …)`. That is the reference `Refines` coarsens.
-/

namespace ForgeExact.DecideOnce

universe u

/-! ## Closure under a map and under a relation equal on members -/

section ClsMap

variable {α β : Type u} [DecidableEq α] [DecidableEq β]

theorem contains_map_inj {f : α → β} (hf : Function.Injective f) (s : List α) (b : α) :
    (s.map f).contains (f b) = s.contains b := by
  simp [hf.eq_iff]

theorem closeStep_map {f : α → β} (hf : Function.Injective f) {r : α → α → Bool} {r' : β → β → Bool}
    (hr : ∀ x y, r' (f x) (f y) = r x y) (xs s : List α) :
    closeStep r' (xs.map f) (s.map f) = (closeStep r xs s).map f := by
  unfold closeStep
  rw [List.map_append, List.filter_map]
  congr 2
  apply congrArg (List.filter · xs)
  funext b
  simp only [Function.comp_def, contains_map_inj hf, List.any_map, hr]

theorem closeN_map {f : α → β} (hf : Function.Injective f) {r : α → α → Bool} {r' : β → β → Bool}
    (hr : ∀ x y, r' (f x) (f y) = r x y) (xs : List α) :
    ∀ (n : Nat) (s : List α), closeN r' (xs.map f) n (s.map f) = (closeN r xs n s).map f
  | 0, s => rfl
  | n + 1, s => by
    simp only [closeN, closeStep_map hf hr, List.length_map]
    split
    · rfl
    · exact closeN_map hf hr xs n _

/-- **Classes under an injective map** that carries the relation. -/
theorem cls_map {f : α → β} (hf : Function.Injective f) {r : α → α → Bool} {r' : β → β → Bool}
    (hr : ∀ x y, r' (f x) (f y) = r x y) (xs : List α) (a : α) :
    cls r' (xs.map f) (f a) = (cls r xs a).map f := by
  unfold cls
  rw [List.length_map]
  exact closeN_map hf hr xs _ [a]

end ClsMap

section ClsCongr

variable {α : Type u} [DecidableEq α]

theorem closeStep_congr {r r' : α → α → Bool} {xs s : List α} (h : ∀ x ∈ s, ∀ y ∈ xs, r x y = r' x y) :
    closeStep r xs s = closeStep r' xs s := by
  unfold closeStep
  congr 1
  apply List.filter_congr
  intro b hb
  congr 1
  rw [Bool.eq_iff_iff]
  simp only [List.any_eq_true]
  constructor
  · rintro ⟨a, ha, h'⟩; exact ⟨a, ha, by rw [← h a ha b hb]; exact h'⟩
  · rintro ⟨a, ha, h'⟩; exact ⟨a, ha, by rw [h a ha b hb]; exact h'⟩

theorem closeN_congr {r r' : α → α → Bool} {xs : List α} {a : α} (h : ∀ x ∈ a :: xs, ∀ y ∈ xs, r x y = r' x y) :
    ∀ (n : Nat) (s : List α), (∀ x ∈ s, x ∈ a :: xs) → closeN r xs n s = closeN r' xs n s
  | 0, _, _ => rfl
  | n + 1, s, hs => by
    have e := closeStep_congr (r := r) (r' := r') (xs := xs) (s := s) (fun x hx y hy => h x (hs x hx) y hy)
    simp only [closeN, e]
    split
    · rfl
    · apply closeN_congr h n
      intro x hx
      simp only [closeStep, List.mem_append, List.mem_filter] at hx
      rcases hx with hx | ⟨hx, _⟩
      · exact hs x hx
      · exact List.mem_cons_of_mem _ hx

theorem cls_congr {r r' : α → α → Bool} {xs : List α} {a : α} (h : ∀ x ∈ a :: xs, ∀ y ∈ xs, r x y = r' x y) :
    cls r xs a = cls r' xs a :=
  closeN_congr h _ _ (by simp)

theorem head_congr {r r' : α → α → Bool} {xs : List α} {a : α} (h : ∀ x ∈ a :: xs, ∀ y ∈ xs, r x y = r' x y) :
    head r xs a = head r' xs a := by
  simp only [head, cls_congr h]

theorem map_head_congr {β : Type u} [BEq α] [LawfulBEq α] {r r' : α → α → Bool} {xs : List α} {F F' : α → List α → β}
    (hr : ∀ x ∈ xs, ∀ y ∈ xs, r x y = r' x y) (hF : ∀ e ∈ xs, ∀ c, F e c = F' e c) :
    (xs.filter fun e => head r xs e == some e).map (fun e => F e (cls r xs e)) =
    (xs.filter fun e => head r' xs e == some e).map (fun e => F' e (cls r' xs e)) := by
  have hc : ∀ e ∈ xs, cls r xs e = cls r' xs e := fun e he =>
    cls_congr fun x hx y hy => hr x (by rcases List.mem_cons.mp hx with rfl | hx <;> assumption) y hy
  have hh : ∀ e ∈ xs, head r xs e = head r' xs e := fun e he =>
    head_congr fun x hx y hy => hr x (by rcases List.mem_cons.mp hx with rfl | hx <;> assumption) y hy
  rw [List.filter_congr fun e he => by rw [hh e he]]
  exact List.map_congr_left fun e he => by
    rw [hc e (List.mem_filter.mp he).1, hF e (List.mem_filter.mp he).1]

end ClsCongr

/-! ## The two `BEq`s on covectors agree

Core.lean compares covectors with `List`'s `BEq`. IncCore.lean is generic, so it uses the one from
`DecidableEq`. Both are lawful, so they are one instance. -/

theorem beq_eq_lawful {γ : Type u} [DecidableEq γ] (b : BEq γ) [@LawfulBEq γ b] :
    (instBEqOfDecidableEq : BEq γ) = b := by
  cases b with
  | mk f =>
    have : ∀ x y, f x y = decide (x = y) := fun x y => by
      rw [Bool.eq_iff_iff]; simp only [decide_eq_true_eq]
      exact ⟨fun h => @LawfulBEq.eq_of_beq γ ⟨f⟩ _ x y h, fun h => h ▸ @ReflBEq.rfl γ ⟨f⟩ _ x⟩
    show BEq.mk _ = BEq.mk f
    congr; funext x y; rw [this]

theorem beq_cov : (instBEqOfDecidableEq : BEq Cov) = List.instBEq := beq_eq_lawful _
theorem beq_cov' : (@instBEqOfDecidableEq Cov fun a b => instDecidableEqList a b) = List.instBEq :=
  beq_eq_lawful _
theorem beq_optcov : (instBEqOfDecidableEq : BEq (Option Cov)) = instBEqOption := beq_eq_lawful _

/-! ## The plane ledger, read as incidence -/

namespace Ledger

variable (L : Ledger)

/-- Facet `φ` round vert `v`, as a sector: bounded by the listed edges between them. -/
def sec (v : Cov) (φ : Cov) : Sector Cov := ⟨φ, L.edges.filter fun e => Cov.le v e && Cov.le e φ⟩

/-- **The plane ledger as an incidence ledger.** -/
def inc : IncLedger Cov where
  nb := L.nb
  faceBody := L.faceBody
  faceCls := L.faceCls
  facets := L.facets
  carrier := Cov.zeroIdx
  plus := Cov.plus
  minus := Cov.minus
  pieces := L.pieces
  inside := L.inside
  edges := L.edges
  verts := L.verts
  edgeFacets := fun e => L.facets.filter fun φ => Cov.le e φ
  vertEdges := fun v => L.edges.filter fun e => Cov.le v e
  vertSectors := fun v => (L.facets.filter fun φ => Cov.le v φ).map (L.sec v)

end Ledger

/-- A body with its faces' names dropped. -/
def Body.toI (b : Body) : BodyI Cov where
  faces := b.faces.map fun F => { facets := F.facets, plane := F.plane, orient := F.orient }
  edges := b.edges.map fun E => { cells := E.cells, faces := E.faces }
  verts := b.verts

def Refusal.toI : Refusal → RefusalI Cov
  | .splitCrossed => .splitCrossed
  | .inconsistentClassification => .inconsistentClassification
  | .zeroThickness x => .zeroThickness x

def Outcome.toI : Outcome → OutcomeI Cov
  | .ok b => .ok b.toI
  | .empty => .empty
  | .refused r => .refused r.toI

namespace Ledger

variable {L : Ledger}

theorem inc_rooms : L.inc.rooms = L.topes := rfl

theorem inc_isPieceOf : L.inc.isPieceOf = L.isPieceOf := by
  funext i φ
  simp only [IncLedger.isPieceOf, Ledger.isPieceOf, inc, IncLedger.bodyOf, Ledger.bodyOf]

theorem inc_crossings : L.inc.crossings = L.crossings := by
  funext i
  simp only [IncLedger.crossings, Ledger.crossings, inc_isPieceOf]; rfl

theorem inc_adjOf : (IncLedger.adjOf : List (Cov × Cov) → Cov → Cov → Bool) = Ledger.adjOf := by
  funext cr T U
  simp only [IncLedger.adjOf, Ledger.adjOf]

theorem inc_inside_contains (x : Cov × Nat) :
    @List.contains _ instBEqOfDecidableEq L.inside x = L.inside.contains x := by
  exact Bool.eq_iff_iff.mpr (by simp)

theorem inc_memb : L.inc.memb = L.memb := by
  funext i T
  simp only [IncLedger.memb, Ledger.memb, inc_rooms, inc_crossings, inc_adjOf]
  split <;> simp_all [inc]

theorem inc_classifyOk : L.inc.classifyOk = L.classifyOk := by
  simp only [IncLedger.classifyOk, Ledger.classifyOk, inc_isPieceOf, inc_memb]; rfl

theorem inc_keep : L.inc.keep = L.keep := by
  funext rule T; simp only [IncLedger.keep, Ledger.keep, inc_memb]; rfl

theorem inc_keepTab (rule : Rule) : L.inc.keepTab rule = L.keepTab rule := by
  simp only [IncLedger.keepTab, Ledger.keepTab, inc_keep, inc_rooms]

theorem inc_keepIn (kt : KeepTab) (T : Cov) : keepInI kt T = keepIn kt T := by
  have h : (fun e : Cov × Bool => @BEq.beq _ instBEqOfDecidableEq e.1 T) = fun e => e.1 == T := by
    funext e; exact Bool.eq_iff_iff.mpr (by simp)
  simp only [keepInI, keepIn, h]

theorem inc_result (kt : KeepTab) : L.inc.result kt = L.result kt := by
  simp only [IncLedger.result, Ledger.result, inc_keepIn]; rfl

theorem inc_orient (kt : KeepTab) : L.inc.orient kt = Ledger.orient kt := by
  funext φ; simp only [IncLedger.orient, Ledger.orient, inc_keepIn]; rfl

theorem mem_result_iff {kt : KeepTab} {φ : Cov} :
    φ ∈ L.result kt ↔ φ ∈ L.facets ∧ (keepIn kt φ.plus != keepIn kt φ.minus) = true := by
  simp [Ledger.result, List.mem_filter]

theorem inc_aroundE (kt : KeepTab) (e : Cov) : L.inc.aroundE kt e = L.around kt e := by
  simp only [IncLedger.aroundE, inc_result]
  simp only [inc, Ledger.around, Ledger.result, List.filter_filter]
  apply List.filter_congr
  intro φ hφ
  by_cases h1 : Cov.le e φ = true <;> by_cases h2 : keepIn kt φ.plus = keepIn kt φ.minus <;>
    simp_all

theorem inc_badEdge (kt : KeepTab) : L.inc.badEdge kt = L.badEdge kt := by
  funext e; simp only [IncLedger.badEdge, Ledger.badEdge, inc_aroundE]

theorem sec_inj (v : Cov) : Function.Injective (L.sec v) := by
  intro a b h; simpa [Ledger.sec] using congrArg Sector.facet h

theorem inc_aroundV (kt : KeepTab) (v : Cov) : L.inc.aroundV kt v = (L.around kt v).map (L.sec v) := by
  have h := inc_aroundE (L := L) kt v
  simp only [IncLedger.aroundE, inc] at h
  simp only [IncLedger.aroundV, inc, List.filter_map, Function.comp_def, Ledger.sec]
  rw [← h]

theorem inc_fanRel (v : Cov) (c d : Cov) :
    IncLedger.fanRel (L.sec v c) (L.sec v d) = L.fanRel v c d := by
  simp only [IncLedger.fanRel, Ledger.fanRel, Ledger.sec]
  rw [Bool.eq_iff_iff]
  simp only [List.any_eq_true, List.mem_filter, List.contains_iff_mem, Bool.and_eq_true]
  constructor
  · rintro ⟨e, ⟨he, h1, h2⟩, _, _, h3⟩; exact ⟨e, he, ⟨h1, h2⟩, h3⟩
  · rintro ⟨e, he, ⟨h1, h2⟩, h3⟩; exact ⟨e, ⟨he, h1, h2⟩, he, h1, h3⟩

theorem inc_badVert (kt : KeepTab) : L.inc.badVert kt = L.badVert kt := by
  funext v
  simp only [IncLedger.badVert, Ledger.badVert, inc_aroundV]
  cases hA : L.around kt v with
  | nil => rfl
  | cons c t =>
    simp only [List.map_cons]
    rw [← List.map_cons, cls_map (sec_inj v) (inc_fanRel v) (c :: t) c, List.any_map]
    congr 1; funext d
    simp only [Function.comp_def, contains_map_inj (sec_inj v)]
    rw [Bool.eq_iff_iff]; simp

theorem plus_ne_minus {φ : Cov} (h : φ.isFacet = true) : φ.plus ≠ φ.minus := by
  have hz : Sgn.zero ∈ φ := by
    simp only [Cov.isFacet, beq_iff_eq] at h
    exact List.count_pos_iff.mp (by omega)
  have hl : φ.zeroIdx < φ.length := List.idxOf_lt_length_iff.mpr hz
  intro e
  have := congrArg (fun x : Cov => x[φ.zeroIdx]?) e
  simp [Cov.plus, Cov.minus, List.getElem?_set, hl] at this

theorem inc_splitOk (h : L.splitOk = true) : L.inc.splitOk = true := by
  simp only [Ledger.splitOk, List.all_eq_true, Bool.and_eq_true] at h
  simp only [IncLedger.splitOk, List.all_eq_true, Bool.and_eq_true]
  intro p hp
  obtain ⟨⟨⟨hf, hc⟩, hb⟩, hq⟩ := h p hp
  refine ⟨⟨⟨⟨?_, ?_⟩, hb⟩, ?_⟩, fun q hq' => ?_⟩
  · simp only [inc, List.contains_iff_mem, decide_eq_true_eq]
    exact (mem_dedup _ _).mpr (List.mem_map.mpr ⟨p, hp, rfl⟩)
  · simpa [inc, IncLedger.clsOf, Ledger.clsOf] using hc
  · simpa [inc] using plus_ne_minus hf
  · have := hq q hq'
    simpa [inc, IncLedger.bodyOf, Ledger.bodyOf] using this

theorem inc_carrier : L.inc.carrier = Cov.zeroIdx := rfl
theorem inc_edges : L.inc.edges = L.edges := rfl
theorem inc_verts : L.inc.verts = L.verts := rfl
theorem inc_vertEdges_contains {v e : Cov} (he : e ∈ L.edges) : (L.inc.vertEdges v).contains e = Cov.le v e := by
  simp only [inc]; rw [Bool.eq_iff_iff]; simp [he]

theorem result_sub {kt : KeepTab} {φ : Cov} (h : φ ∈ L.result kt) : φ ∈ L.facets :=
  (List.mem_filter.mp h).1

theorem inc_joinRel (kt : KeepTab) {c d : Cov} (hc : c ∈ L.facets) (hd : d ∈ L.facets) :
    L.inc.joinRel kt c d = L.joinRel kt c d := by
  simp only [IncLedger.joinRel, Ledger.joinRel, inc_orient]
  simp only [inc, beq_cov]
  congr 1
  apply List.any_congr rfl
  intro e
  rw [Bool.eq_iff_iff]
  simp [hc, hd]

theorem inc_heads (kt : KeepTab) : L.inc.heads kt = L.heads kt := by
  simp only [IncLedger.heads, Ledger.heads, inc_result]
  apply List.filter_congr
  intro c hc
  rw [head_congr (r' := L.joinRel kt) fun x hx y hy => inc_joinRel kt
    (by rcases List.mem_cons.mp hx with rfl | hx <;> exact result_sub ‹_›) (result_sub hy)]

theorem inc_faceOf (kt : KeepTab) {h : Cov} (hh : h ∈ L.result kt) : L.inc.faceOf kt h = L.faceOf kt h := by
  simp only [IncLedger.faceOf, Ledger.faceOf, inc_result]
  rw [cls_congr (r' := L.joinRel kt) fun x hx y hy => inc_joinRel kt
    (by rcases List.mem_cons.mp hx with rfl | hx <;> exact result_sub ‹_›) (result_sub hy)]

theorem inc_facesAtE (kt : KeepTab) (key : Cov → Cov) (e : Cov) :
    L.inc.facesAtE kt key e = L.facesAt kt key e := by
  simp only [IncLedger.facesAtE, Ledger.facesAt, inc_aroundE]

theorem inc_facesAtV (kt : KeepTab) (key : Cov → Cov) (v : Cov) :
    L.inc.facesAtV kt key v = L.facesAt kt key v := by
  simp only [IncLedger.facesAtV, Ledger.facesAt, inc_aroundV, List.map_map, Function.comp_def, Ledger.sec]

theorem inc_unify (kt : KeepTab) : L.inc.unify kt = (L.unify kt).toI := by
  have hF : ∀ h ∈ L.heads kt, L.inc.faceOf kt h = L.faceOf kt h :=
    fun h hh => inc_faceOf kt (List.mem_filter.mp hh).1
  have hfk : (L.heads kt).flatMap (fun h => (L.inc.faceOf kt h).map fun c => (c, h)) =
      (L.heads kt).flatMap (fun h => (L.faceOf kt h).map fun c => (c, h)) := by
    simp only [List.flatMap]; congr 1; exact List.map_congr_left (fun h hh => by rw [hF h hh])
  simp only [IncLedger.unify, Ledger.unify, Body.toI, inc_heads, inc_facesAtE, inc_facesAtV]
  rw [hfk]
  simp only [inc_carrier, inc_edges, inc_verts, inc_orient, List.map_map, Function.comp_def]
  have hat : ∀ (key : Cov → Cov), ∀ x ∈ L.edges ++ L.verts, ((Option.map Prod.snd (List.find? (fun e => e.fst == x)
      (List.map (fun y => (y, L.facesAt kt key y)) (L.edges ++ L.verts)))).getD []) = L.facesAt kt key x := by
    intro key x hx; rw [find?_pair _ x _ hx]; rfl
  have hOut : ∀ (key : Cov → Cov), List.filter (fun v => decide (3 ≤ ((Option.map Prod.snd (List.find? (fun e => e.fst == v)
      (List.map (fun y => (y, L.facesAt kt key y)) (L.edges ++ L.verts)))).getD []).length)) L.verts =
      List.filter (fun v => decide (3 ≤ (L.facesAt kt key v).length)) L.verts :=
    fun key => List.filter_congr fun v hv => by rw [hat key v (List.mem_append_right _ hv)]
  have hEcs : ∀ (key : Cov → Cov), List.filter (fun e => ((Option.map Prod.snd (List.find? (fun x => x.fst == e)
      (List.map (fun y => (y, L.facesAt kt key y)) (L.edges ++ L.verts)))).getD []).length == 2) L.edges =
      List.filter (fun e => (L.facesAt kt key e).length == 2) L.edges :=
    fun key => List.filter_congr fun e he => by rw [hat key e (List.mem_append_left _ he)]
  congr 1
  · exact List.map_congr_left fun h hh => by rw [hF h hh]
  · rw [hOut, hEcs]
    generalize (fun c => (Option.map Prod.snd (List.find? (fun e => e.fst == c)
      (List.flatMap (fun h => List.map (fun c => (c, h)) (L.faceOf kt h)) (L.heads kt)))).getD c) = key
    generalize hecs : List.filter (fun e => (L.facesAt kt key e).length == 2) L.edges = ecs
    have hsub : ∀ e ∈ ecs, e ∈ L.edges := fun e he => by rw [← hecs] at he; exact (List.mem_filter.mp he).1
    have hl : ∀ e ∈ ecs, ((Option.map Prod.snd (List.find? (fun x => x.fst == e)
        (List.map (fun x => (x, L.facesAt kt key x)) (L.edges ++ L.verts)))).getD []) = L.facesAt kt key e :=
      fun e he => hat key e (List.mem_append_left _ (hsub e he))
    refine map_head_congr (F := fun e c => OutEdgeI.mk (ecs.filter (fun d => c.contains d))
        (L.facesAt kt key e))
      (F' := fun x c => OutEdgeI.mk (ecs.filter (fun d => c.contains d))
        ((Option.map Prod.snd (List.find? (fun e => e.fst == x)
          (List.map (fun x => (x, L.facesAt kt key x)) (L.edges ++ L.verts)))).getD []))
      (fun x hx y hy => ?_) (fun e he c => by simp only [hl e he])
    rw [hl x hx, hl y hy]
    congr 1
    apply List.any_congr rfl
    intro v
    rw [inc_vertEdges_contains (hsub x hx), inc_vertEdges_contains (hsub y hy)]
  · rw [hOut]

/-- **The old core is the A8 core on the plane ledger**, names dropped, for a sound split. -/
theorem core_inc (hs : L.splitOk = true) (rule : Rule) : coreI L.inc rule = (core L rule).toI := by
  have hk := inc_splitOk hs
  simp only [coreI, core, hk, hs, Bool.not_true, inc_classifyOk, inc_keepTab]
  cases L.classifyOk
  · rfl
  · simp only [IncLedger.coreK, inc_edges, inc_verts, inc_badEdge, inc_badVert, inc_result, inc_unify]
    cases L.edges.find? (L.badEdge (L.keepTab rule)) with
    | some e => rfl
    | none =>
      cases L.verts.find? (L.badVert (L.keepTab rule)) with
      | some v => rfl
      | none => by_cases hr : L.result (L.keepTab rule) = [] <;> simp [hr, Outcome.toI]

end Ledger

end ForgeExact.DecideOnce
