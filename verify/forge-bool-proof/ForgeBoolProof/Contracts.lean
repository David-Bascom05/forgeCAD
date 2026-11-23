import ForgeBoolProof.Statement
import ForgeCurved.F6

/-!
# §D: the chain of contracts of boolean.rs `general` (old-path-statements.md §D)

`general` is read as a pipeline of nine steps, each ending `Ok` or a refusal:

| step | boolean.rs | hands on |
|---|---|---|
| D1 `index` | `Index::new`, `face_box` | candidate face pairs |
| D2 `meet` | `meet_region`, ssi | curves per pair |
| D3 `touch` | `touches`, `flows_on`, `crack_tip`, `local_class` | contacts |
| D4 `trim` | `trim`, `rank_pairs` | segments per pair |
| D5 `imprint` | `imprint`, `place_cuts`, `split_face`, … | the regions: facets of the arrangement |
| D6 `classify` | `classify_specs`, `Index::classify` (§1a) | a class per region |
| D7 `keep` | `keep_of`, `lean` | the kept regions |
| D8 `apply` | `apply`, `edge_split`, `sole_survivors` | the result `R` |
| D9 `pinch` | `pinched_*`, `undecided_touch`, `refuse_strays` | `R`, or a refusal |

A `Pipeline` holds the nine steps over the translation's own data types, and a `View` reads each
step's output as exact sets (the abstraction functions of each stage, as `abs` is for bodies).
That the translated `general` is `Pipeline.run` of the translated steps, with these views, is the
shape lemma (ledger row E.shape, TRANSLATE).

Each contract `Dk` is a Hoare triple over the exact counterparts `Aex`, `Bex` of the inputs: from
the postconditions of the steps before it, an `ok` output meets its postcondition and an `error`
is a true refusal. The postconditions:
* `PairsOK` (D1): every pair of faces that meet is a candidate;
* `CurvesOK` (D2): a candidate pair's meet lies on its curves, which lie on both carriers;
* `TouchesOK` (D3): every non-transversal meeting point is a listed contact;
* `SegsOK` (D4): a pair's segments are exactly the meet of the two face regions;
* `ArrOK` (D5): the regions are the facets of an arrangement `G` with F5's `FacetData`, connected
  rooms, and face set `Faces Aex ∪ Faces Bex`; each region lies on a face of its body with that
  face's sides;
* `ClassesOK` (D6): each region's class is the exact class against the other solid;
* `KeepOK` (D7, the code): the kept flags are `keepRule` of the classes;
* `KeptIsBd` (D7, the mathematics, `KeepRuleBd`): those flags pick, once each, the facets whose two
  rooms the op keeps differently;
* D8: `R`'s exact counterpart is well formed, its edges isolated, it bounds `Res G keepRoom` (the
  closure of the kept rooms), and `R` is faithful and correctly named;
* D9: an `ok` is the same `R`.
-/

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeCurved.FacetSides
  ForgeCurved.F6

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-! ## The exact reading of each stage -/

/-- `true` names the first operand. -/
def pick {α : Type _} (s : Bool) (a b : α) : α := if s then a else b

/-- The exact arrangement a run of `imprint` denotes. Regions are the code's per-body face
pieces, by id; `gf f` is the facet of `G` region `f` covers, `src f` its body (`true`: A), and
`sense f` the sense of the face it lies on. -/
structure Arr (E : Type) where
  G : FaceGeo E
  FacesG : Set E
  cf : Nat → E → ℝ
  regions : List Nat
  gf : Nat → Nat
  src : Nat → Bool
  sense : Nat → Bool

/-- A region's class against the other solid (boolean.rs `Class`). -/
inductive Class where
  | inside
  | outside
  | onSame
  | onOpposite
  deriving DecidableEq

/-- The faces of `f` and `g` are transversal at `x`: both carriers strictly differentiable with
independent derivatives. -/
def Transversal (f g : FaceModel E) (x : E) : Prop :=
  ∃ La Lb : E →L[ℝ] ℝ, HasStrictFDerivAt f.carrier La x ∧ HasStrictFDerivAt g.carrier Lb x ∧
    LinearIndependent ℝ ![La, Lb]

/-- D1: every pair of faces that meet is a candidate. -/
def PairsOK (Aex Bex : BodyModel E) (S : Set (Nat × Nat)) : Prop :=
  ∀ (i j : Nat) fa fb, Aex.faces[i]? = some fa → Bex.faces[j]? = some fb →
    (fa.region ∩ fb.region).Nonempty → (i, j) ∈ S

/-- D2: a candidate pair's meet lies on its curves, and the curves on both carriers. -/
def CurvesOK (Aex Bex : BodyModel E) (S : Set (Nat × Nat)) (C : Nat × Nat → Set E) : Prop :=
  ∀ (i j : Nat) fa fb, Aex.faces[i]? = some fa → Bex.faces[j]? = some fb → (i, j) ∈ S →
    fa.region ∩ fb.region ⊆ C (i, j) ∧ C (i, j) ⊆ {x | fa.carrier x = 0 ∧ fb.carrier x = 0}

/-- D3: every meeting point where the faces are not transversal is a listed contact. -/
def TouchesOK (Aex Bex : BodyModel E) (T : Set E) : Prop :=
  ∀ (i j : Nat) fa fb, Aex.faces[i]? = some fa → Bex.faces[j]? = some fb →
    ∀ x ∈ fa.region ∩ fb.region, ¬ Transversal fa fb x → x ∈ T

/-- D4: a pair's segments are exactly the meet of the two face regions. -/
def SegsOK (Aex Bex : BodyModel E) (Sg : Nat × Nat → Set E) : Prop :=
  ∀ (i j : Nat) fa fb, Aex.faces[i]? = some fa → Bex.faces[j]? = some fb → Sg (i, j) = fa.region ∩ fb.region

/-- D5: the regions are the facets of an arrangement of the two bodies' faces. -/
structure ArrOK (Aex Bex : BodyModel E) (a : Arr E) : Prop where
  data : FacetData a.G a.FacesG a.cf
  conn : ∀ r, IsPreconnected (a.G.room r)
  faces : a.FacesG = Faces Aex ∪ Faces Bex
  /-- each nonempty facet lies on the faces of one body (or both) -/
  facet_side : ∀ g, (a.G.facet g).Nonempty → a.G.facet g ⊆ Faces Aex ∨ a.G.facet g ⊆ Faces Bex
  /-- every nonempty facet on a body's faces is a region of that body -/
  cover : ∀ g s, (a.G.facet g).Nonempty → a.G.facet g ⊆ Faces (pick s Aex Bex) →
    ∃ f ∈ a.regions, a.gf f = g ∧ a.src f = s
  /-- one region per facet and body -/
  inj : ∀ f ∈ a.regions, ∀ f' ∈ a.regions, a.gf f = a.gf f' → a.src f = a.src f' → f = f'
  /-- a facet on a body's faces is on that solid's frontier. Off the vertex-cluster zones this
  is `Bounds.faces_sub`; inside a zone (micro-faces of near-concurrent carriers) it is part of the
  contract: open, with the cluster ruling (ledger row D5.zone) -/
  on_frontier : ∀ g s, (a.G.facet g).Nonempty → a.G.facet g ⊆ Faces (pick s Aex Bex) →
    a.G.facet g ⊆ frontier (denoteExact (pick s Aex Bex))
  /-- each region lies on a face of its body, with that face's inside side -/
  on_face : ∀ f ∈ a.regions, ∃ F ∈ (pick (a.src f) Aex Bex).faces, a.G.facet (a.gf f) ⊆ F.region ∧
    ∀ y ∈ a.G.facet (a.gf f), ∀ᶠ z in 𝓝 y,
      (inSide (a.sense f) (a.cf (a.gf f) z) ↔ inSide F.sense (F.carrier z)) ∧
      (outSide (a.sense f) (a.cf (a.gf f) z) ↔ outSide F.sense (F.carrier z))

/-- The exact class of region `f` against the other solid. -/
def ClassOK (a : Arr E) (SA SB : Set E) (f : Nat) : Class → Prop
  | .inside => a.G.facet (a.gf f) ⊆ interior (pick (a.src f) SB SA)
  | .outside => Disjoint (a.G.facet (a.gf f)) (pick (a.src f) SB SA)
  | .onSame => a.G.facet (a.gf f) ⊆ frontier (pick (a.src f) SB SA) ∧
      a.G.room (inRoom a.G (a.sense f) (a.gf f)) ⊆ pick (a.src f) SB SA
  | .onOpposite => a.G.facet (a.gf f) ⊆ frontier (pick (a.src f) SB SA) ∧
      a.G.room (outRoom a.G (a.sense f) (a.gf f)) ⊆ pick (a.src f) SB SA

/-- D6: every region's class is its exact class. -/
def ClassesOK (a : Arr E) (SA SB : Set E) (c : Nat → Class) : Prop :=
  ∀ f ∈ a.regions, ClassOK a SA SB f (c f)

/-- The keep rule, per op, body and class. A coplanar piece is kept once, as A's region. -/
def keepRule : Op → Bool → Class → Bool
  | .union, true, c => c == .outside || c == .onSame
  | .union, false, c => c == .outside
  | .inter, true, c => c == .inside || c == .onSame
  | .inter, false, c => c == .inside
  | .diff, true, c => c == .outside || c == .onOpposite
  | .diff, false, c => c == .inside

/-- D7, the code: the kept flags are the rule applied to the classes. -/
def KeepOK (a : Arr E) (op : Op) (c : Nat → Class) (k : Nat → Bool) : Prop :=
  ∀ f ∈ a.regions, k f = keepRule op (a.src f) (c f)

open Classical in
/-- A room is kept when the op keeps its points (the membership is constant on a room). -/
noncomputable def keepRoom (G : FaceGeo E) (SA SB : Set E) (op : Op) (r : Nat) : Bool :=
  decide (∃ x ∈ G.room r, op.eval (x ∈ SA) (x ∈ SB))

/-- The kept regions are the boundary facets of the kept rooms, each once. -/
def KeptIsBd (a : Arr E) (k : Nat → Bool) (keepR : Nat → Bool) : Prop :=
  (∀ g, (a.G.facet g).Nonempty → (Bd a.G keepR g ↔ ∃ f ∈ a.regions, a.gf f = g ∧ k f = true)) ∧
    ∀ f ∈ a.regions, ∀ f' ∈ a.regions, k f = true → k f' = true → a.gf f = a.gf f' → f = f'

/-- **D7, the mathematics (the boundary lemma, per facet)**: on an arrangement of two valid exact
solids' faces, the keep rule on exact classes picks each boundary facet of the kept rooms once,
and nothing else. -/
def KeepRuleBd : Prop :=
  ∀ (Aex Bex : BodyModel E) (a : Arr E) (op : Op) (c : Nat → Class),
    ValidExact Aex → ValidExact Bex → ArrOK Aex Bex a →
    ClassesOK a (denoteExact Aex) (denoteExact Bex) c →
    KeptIsBd a (fun f => keepRule op (a.src f) (c f)) (keepRoom a.G (denoteExact Aex) (denoteExact Bex) op)

/-! ## The pipeline and its views -/

/-- `general`'s nine steps over the translation's data types. -/
structure Pipeline (Body Err : Type) where
  Pairs : Type
  Curves : Type
  Touches : Type
  Segs : Type
  Facets : Type
  Classes : Type
  Kept : Type
  index : Body → Body → Except Err Pairs
  meet : Body → Body → Pairs → Except Err Curves
  touch : Body → Body → Curves → Except Err Touches
  trim : Body → Body → Curves → Touches → Except Err Segs
  imprint : Body → Body → Segs → Except Err Facets
  classify : Body → Body → Facets → Except Err Classes
  keep : Op → Facets → Classes → Except Err Kept
  apply : Body → Body → Op → Facets → Kept → Except Err Body
  pinch : Body → Body → Op → Body → Except Err Body

/-- **`general`, as the pipeline runs it.** -/
def Pipeline.run {Body Err : Type} (P : Pipeline Body Err) (A B : Body) (op : Op) :
    Except Err Body :=
  match P.index A B with
  | .error e => .error e
  | .ok pr => match P.meet A B pr with
    | .error e => .error e
    | .ok cv => match P.touch A B cv with
      | .error e => .error e
      | .ok tc => match P.trim A B cv tc with
        | .error e => .error e
        | .ok sg => match P.imprint A B sg with
          | .error e => .error e
          | .ok fs => match P.classify A B fs with
            | .error e => .error e
            | .ok cs => match P.keep op fs cs with
              | .error e => .error e
              | .ok kp => match P.apply A B op fs kp with
                | .error e => .error e
                | .ok R => P.pinch A B op R

/-- Each stage's output read as exact sets. -/
structure View {Body Err : Type} (P : Pipeline Body Err) (E : Type) where
  pairs : P.Pairs → Set (Nat × Nat)
  curves : P.Curves → Nat × Nat → Set E
  touches : P.Touches → Set E
  segs : P.Segs → Nat × Nat → Set E
  arr : P.Facets → Arr E
  cls : P.Classes → Nat → Class
  kept : P.Kept → Nat → Bool

/-! ## The contracts D1–D9 -/

section Contracts

variable {Body K Q : Type} (δ : ℝ) (abs : Body → BodyModel E) (sc : Scope Body K Q)
  (Names : Body → Body → Op → Body → Prop) (Size : Body → Prop)
  (P : Pipeline Body (Refusal E K Q)) (V : View P E)

/-- The context every step may assume: valid, faithful, in-size inputs, and their exact
counterparts. -/
def Ctx (A B : Body) (Aex Bex : BodyModel E) : Prop :=
  Input δ abs Size A ∧ Input δ abs Size B ∧ exactOf δ (abs A) = some Aex ∧
    exactOf δ (abs B) = some Bex ∧ ValidExact Aex ∧ ValidExact Bex

/-- A step's result: `ok` meets `post`, `error` is a true refusal. -/
def Step {X : Type} (A B : Body) (op : Op) (post : X → Prop) : Except (Refusal E K Q) X → Prop
  | .ok x => post x
  | .error e => RefusalTrue δ abs sc A B op e

def D1 : Prop := ∀ A B Aex Bex op, Ctx δ abs Size A B Aex Bex →
  Step δ abs sc A B op (fun pr => PairsOK Aex Bex (V.pairs pr)) (P.index A B)

def D2 : Prop := ∀ A B Aex Bex op pr, Ctx δ abs Size A B Aex Bex → PairsOK Aex Bex (V.pairs pr) →
  Step δ abs sc A B op (fun cv => CurvesOK Aex Bex (V.pairs pr) (V.curves cv)) (P.meet A B pr)

def D3 : Prop := ∀ A B Aex Bex op S cv, Ctx δ abs Size A B Aex Bex → PairsOK Aex Bex S →
  CurvesOK Aex Bex S (V.curves cv) →
  Step δ abs sc A B op (fun tc => TouchesOK Aex Bex (V.touches tc)) (P.touch A B cv)

def D4 : Prop := ∀ A B Aex Bex op S cv tc, Ctx δ abs Size A B Aex Bex → PairsOK Aex Bex S →
  CurvesOK Aex Bex S (V.curves cv) → TouchesOK Aex Bex (V.touches tc) →
  Step δ abs sc A B op (fun sg => SegsOK Aex Bex (V.segs sg)) (P.trim A B cv tc)

def D5 : Prop := ∀ A B Aex Bex op sg, Ctx δ abs Size A B Aex Bex → SegsOK Aex Bex (V.segs sg) →
  Step δ abs sc A B op (fun fs => ArrOK Aex Bex (V.arr fs)) (P.imprint A B sg)

def D6 : Prop := ∀ A B Aex Bex op fs, Ctx δ abs Size A B Aex Bex → ArrOK Aex Bex (V.arr fs) →
  Step δ abs sc A B op (fun cs => ClassesOK (V.arr fs) (denoteExact Aex) (denoteExact Bex) (V.cls cs))
    (P.classify A B fs)

def D7 : Prop := ∀ A B Aex Bex op fs cs, Ctx δ abs Size A B Aex Bex → ArrOK Aex Bex (V.arr fs) →
  ClassesOK (V.arr fs) (denoteExact Aex) (denoteExact Bex) (V.cls cs) →
  Step δ abs sc A B op (fun kp => KeepOK (V.arr fs) op (V.cls cs) (V.kept kp)) (P.keep op fs cs)

/-- D8's postcondition: the result's exact counterpart bounds the closure of the kept rooms. -/
def ApplyPost (A B : Body) (op : Op) (a : Arr E) (SA SB : Set E) (R : Body) : Prop :=
  ∃ Rs, exactOf δ (abs R) = some Rs ∧
    (∀ i (h : i < (abs R).edges.length), EdgeIsolated δ (abs R) i (abs R).edges[i]) ∧
    WellFormed Rs ∧ Bounds Rs (Res a.G (keepRoom a.G SA SB op)) ∧ Faithful δ (abs R) ∧
    Names A B op R

def D8 : Prop := ∀ A B Aex Bex op fs kp, Ctx δ abs Size A B Aex Bex → ArrOK Aex Bex (V.arr fs) →
  KeptIsBd (V.arr fs) (V.kept kp) (keepRoom (V.arr fs).G (denoteExact Aex) (denoteExact Bex) op) →
  Step δ abs sc A B op (ApplyPost δ abs Names A B op (V.arr fs) (denoteExact Aex) (denoteExact Bex))
    (P.apply A B op fs kp)

def D9 : Prop := ∀ A B Aex Bex op R, Ctx δ abs Size A B Aex Bex →
  Step δ abs sc A B op (fun R' => R' = R) (P.pinch A B op R)

end Contracts

end ForgeBoolProof
