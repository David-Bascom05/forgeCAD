import ForgeExact.DecideOnce.Correct
import ForgeExact.Snap

/-!
# The ledger: each fact decided once, every use a lookup

**The fact kinds** (research/kernel-audit/decisions.md), as questions to the ledger (`Fact`):

| kind | question | answered from |
|---|---|---|
| point on edge | cut point `p` lies on edge cell `e` | `p`'s row ≤ `e`'s row |
| point at vertex | cut point `p` is vertex `v` | rows equal |
| vertex on surface | the side of point `p` of face `f`'s surface | `side (p, surface f)` |
| curve along edge | the curve of faces `f`, `g` runs along edge cell `e` | `e` on both surfaces |
| two cuts one vertex | cut points `p`, `q` are one vertex | rows equal |
| two faces one surface | faces `f`, `g` have one carrier | `surface f = surface g` |
| touch or crossing | against face `f`'s surface, the cells `e₁`, `e₂` either side of a point | their two sides |

There are two primitive tables, each filled once by a named predicate (`Table.decideAll`):

* `surface` : face → its carrier class, by §6.1's snap (`Snap.rep`: the transitive closure of
  tolerance-equality, the least-named carrier of the class);
* `side` : (cell, class) → `neg | zero | pos`, by `sideIn`: the sign of the class's
  representative plane (in the face's own sense) at the cell's exact point. Decided in the
  representative surface, never across the gap (near-coincident.md's rule).

Every fact kind is a function of lookups in these two tables (`Decisions.ask`); a cell's covector,
which is what the core reads, is its row of the side table (`row`). So:

**Theorem (a), consistency by construction.**
* `get?_decideAll`, `decideAll_once`: each key is decided exactly once, and every lookup of it
  returns that decision;
* `ask_build`: every question to a built ledger is answered from the deciders' answers, through
  the formula of its kind, and nothing else;
* the closure laws, which the old kernel's sites broke (decisions.md's contradictions):
  `oneVertex_symm`, `oneVertex_trans`, `oneSurface_symm`, `oneSurface_trans`,
  `side_of_oneSurface` (faces of one surface give one side to every point), `onEdge_of_atVertex`,
  `on_of_onEdge` (a point on an edge lies on every surface the edge lies on), `along_iff`;
* `sideIn_class`: decided in the representative, the on/off answer is one across a class of
  tolerance-equal carriers (from `Snap.onIn_class`), whichever face of the class asks.
-/

namespace ForgeExact.DecideOnce

open ForgeExact.Ref

/-! ## Tables -/

/-- A table of decisions: each key with its answer. -/
structure Table (K V : Type) where
  entries : List (K × V)

namespace Table

variable {K V : Type} [DecidableEq K]

/-- Decide every key once, with the named predicate `d`. -/
def decideAll (keys : List K) (d : K → V) : Table K V := ⟨(dedup keys).map fun k => (k, d k)⟩

/-- Look a key up. -/
def get? (t : Table K V) (k : K) : Option V := (t.entries.find? fun e => e.1 == k).map Prod.snd

/-- **Decided once**: no key appears twice. -/
theorem decideAll_once (keys : List K) (d : K → V) :
    ((decideAll keys d).entries.map Prod.fst).Nodup := by
  simp only [decideAll, List.map_map, Function.comp_def, List.map_id']
  exact nodup_dedup keys

theorem find?_map_key (d : K → V) (k : K) :
    ∀ (ks : List K), k ∈ ks → ((ks.map fun k => (k, d k)).find? fun e => e.1 == k) = some (k, d k)
  | [], h => nomatch h
  | x :: xs, h => by
    simp only [List.map_cons, List.find?_cons]
    by_cases e : x = k
    · subst e; simp
    · have : (x == k) = false := by simp [e]
      simp only [this]
      exact find?_map_key d k xs ((List.mem_cons.mp h).resolve_left (Ne.symm e))

/-- **Every lookup returns the one decided answer.** -/
theorem get?_decideAll {keys : List K} {k : K} (d : K → V) (hk : k ∈ keys) :
    (decideAll keys d).get? k = some (d k) := by
  unfold get? decideAll
  rw [find?_map_key d k _ ((mem_dedup keys k).mpr hk)]
  rfl

end Table

/-! ## The facts -/

/-- How a curve meets a surface at a point: across it, back off it on the side it came from, or
along it. -/
inductive Contact
  | crossing
  | touch
  | along
  deriving DecidableEq, Repr

/-- The contact a pair of sides gives (the cells just before and just after the point). -/
def contactOf : Sgn → Sgn → Contact
  | .zero, _ => .along
  | _, .zero => .along
  | .pos, .neg => .crossing
  | .neg, .pos => .crossing
  | _, _ => .touch

/-- The seven fact kinds, as questions. Cells, cut points and faces are named by their arena
index. -/
inductive Fact
  | pointOnEdge (p e : Nat)
  | pointAtVertex (p v : Nat)
  | vertexOnSurface (p f : Nat)
  | curveAlongEdge (f g e : Nat)
  | twoCutsOneVertex (p q : Nat)
  | twoFacesOneSurface (f g : Nat)
  | touchOrCrossing (e₁ e₂ f : Nat)

inductive Answer
  | bool (b : Bool)
  | side (s : Sgn)
  | contact (c : Contact)
  deriving DecidableEq

/-- The two primitive tables. -/
structure Decisions where
  /-- the number of carrier classes -/
  nc : Nat
  /-- face → carrier class -/
  surface : Table Nat Nat
  /-- (cell, class) → side -/
  side : Table (Nat × Nat) Sgn

namespace Decisions

variable (D : Decisions)

def surf (f : Nat) : Nat := (D.surface.get? f).getD 0
def sideOf (c j : Nat) : Sgn := (D.side.get? (c, j)).getD .zero

/-- A cell's covector: its row of the side table. This is all the core reads of a cell. -/
def row (c : Nat) : Cov := (List.range D.nc).map fun j => D.sideOf c j

/-- **Every fact is a lookup.** -/
def ask : Fact → Answer
  | .pointOnEdge p e => .bool (Cov.le (D.row p) (D.row e))
  | .pointAtVertex p v => .bool (D.row p == D.row v)
  | .vertexOnSurface p f => .side (D.sideOf p (D.surf f))
  | .curveAlongEdge f g e => .bool (D.sideOf e (D.surf f) == .zero && D.sideOf e (D.surf g) == .zero)
  | .twoCutsOneVertex p q => .bool (D.row p == D.row q)
  | .twoFacesOneSurface f g => .bool (D.surf f == D.surf g)
  | .touchOrCrossing e₁ e₂ f => .contact (contactOf (D.sideOf e₁ (D.surf f)) (D.sideOf e₂ (D.surf f)))

/-- The ledger built once: each face's class and each cell's side of each class, decided by the
named predicates `surfaceOf` and `sideOf`. -/
def build (nc : Nat) (faces cells : List Nat) (surfaceOf : Nat → Nat) (sideOf : Nat × Nat → Sgn) :
    Decisions where
  nc := nc
  surface := Table.decideAll faces surfaceOf
  side := Table.decideAll (cells.flatMap fun c => (List.range nc).map fun j => (c, j)) sideOf

/-- The answer each kind's formula gives from the deciders themselves. -/
def spec (surfaceOf : Nat → Nat) (sideOf : Nat × Nat → Sgn) (nc : Nat) : Fact → Answer
  | .pointOnEdge p e => .bool (Cov.le ((List.range nc).map fun j => sideOf (p, j))
      ((List.range nc).map fun j => sideOf (e, j)))
  | .pointAtVertex p v => .bool (((List.range nc).map fun j => sideOf (p, j)) ==
      ((List.range nc).map fun j => sideOf (v, j)))
  | .vertexOnSurface p f => .side (sideOf (p, surfaceOf f))
  | .curveAlongEdge f g e => .bool (sideOf (e, surfaceOf f) == .zero && sideOf (e, surfaceOf g) == .zero)
  | .twoCutsOneVertex p q => .bool (((List.range nc).map fun j => sideOf (p, j)) ==
      ((List.range nc).map fun j => sideOf (q, j)))
  | .twoFacesOneSurface f g => .bool (surfaceOf f == surfaceOf g)
  | .touchOrCrossing e₁ e₂ f => .contact (contactOf (sideOf (e₁, surfaceOf f)) (sideOf (e₂, surfaceOf f)))

/-- The faces and cells a fact names. -/
def _root_.ForgeExact.DecideOnce.Fact.faces : Fact → List Nat
  | .vertexOnSurface _ f => [f]
  | .curveAlongEdge f g _ => [f, g]
  | .twoFacesOneSurface f g => [f, g]
  | .touchOrCrossing _ _ f => [f]
  | _ => []

def _root_.ForgeExact.DecideOnce.Fact.cells : Fact → List Nat
  | .pointOnEdge p e => [p, e]
  | .pointAtVertex p v => [p, v]
  | .vertexOnSurface p _ => [p]
  | .curveAlongEdge _ _ e => [e]
  | .twoCutsOneVertex p q => [p, q]
  | .twoFacesOneSurface _ _ => []
  | .touchOrCrossing e₁ e₂ _ => [e₁, e₂]

theorem surf_build {nc : Nat} {faces cells : List Nat} {surfaceOf : Nat → Nat}
    {sideOf : Nat × Nat → Sgn} {f : Nat} (hf : f ∈ faces) :
    (build nc faces cells surfaceOf sideOf).surf f = surfaceOf f := by
  simp [surf, build, Table.get?_decideAll _ hf]

theorem sideOf_build {nc : Nat} {faces cells : List Nat} {surfaceOf : Nat → Nat}
    {sideOf : Nat × Nat → Sgn} {c j : Nat} (hc : c ∈ cells) (hj : j < nc) :
    (build nc faces cells surfaceOf sideOf).sideOf c j = sideOf (c, j) := by
  have hk : (c, j) ∈ cells.flatMap fun c => (List.range nc).map fun j => (c, j) := by
    simp only [List.mem_flatMap, List.mem_map, List.mem_range]
    exact ⟨c, hc, j, hj, rfl⟩
  simp [Decisions.sideOf, build, Table.get?_decideAll _ hk]

theorem row_build {nc : Nat} {faces cells : List Nat} {surfaceOf : Nat → Nat}
    {sideOf : Nat × Nat → Sgn} {c : Nat} (hc : c ∈ cells) :
    (build nc faces cells surfaceOf sideOf).row c = (List.range nc).map fun j => sideOf (c, j) := by
  unfold row
  apply List.map_congr_left
  intro j hj
  exact sideOf_build hc (List.mem_range.mp hj)

/-- **Theorem (a).** Every question to a built ledger, about faces and cells it decided, is
answered from the named deciders' one answer for each key, by its kind's formula: no fact is
measured again. -/
theorem ask_build (nc : Nat) (faces cells : List Nat) (surfaceOf : Nat → Nat)
    (sideOf : Nat × Nat → Sgn) (hsurf : ∀ f ∈ faces, surfaceOf f < nc) (q : Fact)
    (hf : ∀ f ∈ q.faces, f ∈ faces) (hc : ∀ c ∈ q.cells, c ∈ cells) :
    (build nc faces cells surfaceOf sideOf).ask q = spec surfaceOf sideOf nc q := by
  cases q with
  | pointOnEdge p e =>
    simp only [ask, spec, row_build (hc p (by simp [Fact.cells])), row_build (hc e (by simp [Fact.cells]))]
  | pointAtVertex p v =>
    simp only [ask, spec, row_build (hc p (by simp [Fact.cells])), row_build (hc v (by simp [Fact.cells]))]
  | vertexOnSurface p f =>
    have hf' := hf f (by simp [Fact.faces])
    simp only [ask, spec, surf_build hf', sideOf_build (hc p (by simp [Fact.cells])) (hsurf f hf')]
  | curveAlongEdge f g e =>
    have hf' := hf f (by simp [Fact.faces])
    have hg' := hf g (by simp [Fact.faces])
    have he := hc e (by simp [Fact.cells])
    simp only [ask, spec, surf_build hf', surf_build hg', sideOf_build he (hsurf f hf'),
      sideOf_build he (hsurf g hg')]
  | twoCutsOneVertex p q =>
    simp only [ask, spec, row_build (hc p (by simp [Fact.cells])), row_build (hc q (by simp [Fact.cells]))]
  | twoFacesOneSurface f g =>
    simp only [ask, spec, surf_build (hf f (by simp [Fact.faces])), surf_build (hf g (by simp [Fact.faces]))]
  | touchOrCrossing e₁ e₂ f =>
    have hf' := hf f (by simp [Fact.faces])
    simp only [ask, spec, surf_build hf', sideOf_build (hc e₁ (by simp [Fact.cells])) (hsurf f hf'),
      sideOf_build (hc e₂ (by simp [Fact.cells])) (hsurf f hf')]

/-! ## Closure laws: the contradictions decisions.md measured cannot happen -/

theorem oneVertex_symm (p q : Nat) :
    D.ask (.twoCutsOneVertex p q) = D.ask (.twoCutsOneVertex q p) := by
  simp only [ask]
  cases h : D.row p == D.row q <;> cases h' : D.row q == D.row p <;> simp_all

theorem oneVertex_trans {p q r : Nat} (h1 : D.ask (.twoCutsOneVertex p q) = .bool true)
    (h2 : D.ask (.twoCutsOneVertex q r) = .bool true) : D.ask (.twoCutsOneVertex p r) = .bool true := by
  simp only [ask, Answer.bool.injEq, beq_iff_eq] at *
  rw [h1, h2]

theorem oneSurface_symm (f g : Nat) :
    D.ask (.twoFacesOneSurface f g) = D.ask (.twoFacesOneSurface g f) := by
  simp only [ask]
  cases h : D.surf f == D.surf g <;> cases h' : D.surf g == D.surf f <;> simp_all

theorem oneSurface_trans {f g k : Nat} (h1 : D.ask (.twoFacesOneSurface f g) = .bool true)
    (h2 : D.ask (.twoFacesOneSurface g k) = .bool true) : D.ask (.twoFacesOneSurface f k) = .bool true := by
  simp only [ask, Answer.bool.injEq, beq_iff_eq] at *
  rw [h1, h2]

/-- **Decided in the representative**: faces decided to be one surface give every point one side
(up to the faces' own sense, which `sideIn` carries). -/
theorem side_of_oneSurface {f g : Nat} (h : D.ask (.twoFacesOneSurface f g) = .bool true) (p : Nat) :
    D.ask (.vertexOnSurface p f) = D.ask (.vertexOnSurface p g) := by
  simp only [ask, Answer.bool.injEq, beq_iff_eq] at *
  rw [h]

/-- The face order is transitive. -/
theorem le_trans' : ∀ {X Y Z : Cov}, Cov.le X Y = true → Cov.le Y Z = true → Cov.le X Z = true
  | [], [], [], _, _ => rfl
  | x :: X, y :: Y, z :: Z, h1, h2 => by
    simp only [Cov.le, List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true,
      beq_iff_eq, Bool.or_eq_true, Nat.add_right_cancel_iff] at h1 h2 ⊢
    have ih := @le_trans' X Y Z (by simp [Cov.le, h1.1, h1.2.2]) (by simp [Cov.le, h2.1, h2.2.2])
    simp only [Cov.le, Bool.and_eq_true, beq_iff_eq] at ih
    refine ⟨h1.1.trans h2.1, ?_, ih.2⟩
    rcases h1.2.1 with h | h
    · exact Or.inl h
    · rcases h2.2.1 with h' | h'
      · exact Or.inl (h.trans h')
      · exact Or.inr (h.trans h')
  | [], _ :: _, _, h, _ => by simp [Cov.le] at h
  | _ :: _, [], _, h, _ => by simp [Cov.le] at h
  | [], [], _ :: _, _, h => by simp [Cov.le] at h
  | _ :: _, _ :: _, [], _, h => by simp [Cov.le] at h

/-- A point at a vertex on an edge is on the edge. -/
theorem onEdge_of_atVertex {p v e : Nat} (h1 : D.ask (.pointAtVertex p v) = .bool true)
    (h2 : D.ask (.pointOnEdge v e) = .bool true) : D.ask (.pointOnEdge p e) = .bool true := by
  simp only [ask, Answer.bool.injEq, beq_iff_eq] at *
  rw [h1]; exact h2

theorem row_get {c j : Nat} (hj : j < D.nc) : (D.row c)[j]? = some (D.sideOf c j) := by
  simp [row, hj]

theorem le_zero : ∀ {X Y : Cov} {j : Nat}, Cov.le X Y = true → Y[j]? = some .zero → X[j]? = some .zero
  | [], [], _, _, h => by simp at h
  | x :: X, y :: Y, 0, h1, h2 => by
    simp only [Cov.le, List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true,
      beq_iff_eq, Bool.or_eq_true] at h1
    simp only [List.getElem?_cons_zero, Option.some.injEq] at h2 ⊢
    rcases h1.2.1 with h | h
    · exact h
    · exact h.trans h2
  | x :: X, y :: Y, j + 1, h1, h2 => by
    simp only [List.getElem?_cons_succ] at h2 ⊢
    simp only [Cov.le, List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true,
      beq_iff_eq, Nat.add_right_cancel_iff] at h1
    exact le_zero (by simp [Cov.le, h1.1, h1.2.2]) h2
  | [], _ :: _, _, h, _ => by simp [Cov.le] at h
  | _ :: _, [], _, h, _ => by simp [Cov.le] at h

/-- **A point on an edge lies on every surface the edge lies along** (README §2.1's ledger closure:
`coinc(F, G) ∧ e ⊂ F ⇒ side(ends(e), G) = 0`). -/
theorem on_of_onEdge {p e f g : Nat} (hs : D.surf f < D.nc) (hs' : D.surf g < D.nc)
    (h1 : D.ask (.pointOnEdge p e) = .bool true) (h2 : D.ask (.curveAlongEdge f g e) = .bool true) :
    D.ask (.vertexOnSurface p f) = .side .zero ∧ D.ask (.vertexOnSurface p g) = .side .zero := by
  simp only [ask, Answer.bool.injEq, Bool.and_eq_true, beq_iff_eq, Answer.side.injEq] at *
  refine ⟨?_, ?_⟩
  · have := le_zero h1 (by rw [row_get D hs, h2.1])
    rw [row_get D hs] at this; exact Option.some.inj this
  · have := le_zero h1 (by rw [row_get D hs', h2.2])
    rw [row_get D hs'] at this; exact Option.some.inj this

/-- Touch or crossing is one answer, and it is `along` exactly when a side is on the surface. -/
theorem along_iff (e₁ e₂ f : Nat) :
    D.ask (.touchOrCrossing e₁ e₂ f) = .contact .along ↔
      D.sideOf e₁ (D.surf f) = .zero ∨ D.sideOf e₂ (D.surf f) = .zero := by
  simp only [ask, Answer.contact.injEq]
  cases D.sideOf e₁ (D.surf f) <;> cases D.sideOf e₂ (D.surf f) <;> simp [contactOf]

end Decisions

/-! ## The side decider: in the representative surface (§6.1) -/

/-- The side of `x` of carrier `a`'s surface, decided in the representative (`Snap.snapped`: the
representative's plane in `a`'s own sense). -/
def sideIn (l : Rat) (cs : List Snap.Carrier) (a : Snap.Carrier) (x : V3) : Sgn :=
  sgnQ ((Snap.snapped l cs a).eval x)

theorem sgnQ_zero_iff (q : Rat) : sgnQ q = .zero ↔ q = 0 := by
  unfold sgnQ
  by_cases h1 : q > 0
  · simp only [h1, ite_true, reduceCtorEq, false_iff]; grind
  · by_cases h2 : q < 0
    · simp only [h1, h2, ite_true, ite_false, reduceCtorEq, false_iff]; grind
    · simp only [h1, h2, ite_false, true_iff]; grind

/-- **One answer across a class.** Two tolerance-equal (chained) carriers say the same of whether a
point is on their surface: the decision is the representative's, whichever face asks. -/
theorem sideIn_class (l : Rat) {cs : List Snap.Carrier} (hcs : cs.Nodup)
    (hnames : (cs.map Snap.Carrier.name).Nodup) {a b : Snap.Carrier} (ha : a ∈ cs)
    (hab : Snap.Conn l cs a b) (x : V3) :
    (sideIn l cs a x = .zero ↔ sideIn l cs b x = .zero) := by
  have h := Snap.onIn_class l hcs hnames ha hab x
  unfold Snap.onIn at h
  unfold sideIn
  rw [sgnQ_zero_iff, sgnQ_zero_iff]
  constructor
  · intro h0
    have : ((Snap.snapped l cs a).eval x == 0) = true := by simp [h0]
    rw [h] at this; simpa using this
  · intro h0
    have : ((Snap.snapped l cs b).eval x == 0) = true := by simp [h0]
    rw [← h] at this; simpa using this

/-! ## The core's ledger is read from the tables -/

/-- The core's ledger, made of lookups: every covector it holds is a row of the side table, every
piece and witness an entry of a table decided once. -/
def toCore (D : Decisions) (nb : Nat) (faceBody : List Nat) (faces : List Nat)
    (inFace : Table (Nat × Nat) Bool) (inside : Table (Nat × Nat) Bool)
    (edgeCells vertCells : List Nat) : Ledger where
  nb := nb
  faceBody := faceBody
  faceCls := faces.map D.surf
  pieces := (inFace.entries.filter (·.2)).map fun e => (D.row e.1.1, e.1.2)
  inside := (inside.entries.filter (·.2)).map fun e => (D.row e.1.1, e.1.2)
  edges := edgeCells.map D.row
  verts := vertCells.map D.row

end ForgeExact.DecideOnce
