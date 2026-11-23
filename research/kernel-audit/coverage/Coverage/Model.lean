/-!
# The kernel's flat-face boolean, modelled on the box-box family

A hand model of `forge_bool::boolean` (kernel/forge-bool/src/boolean.rs, HEAD 888fc195) as it runs
on two axis-aligned boxes: which predicate each site evaluates, in what order, the branch it takes,
and every refusal. Aeneas could not be run (see coverage.md §1), so each step cites the Rust it
models (file:line).

**Sign patterns.** Every predicate the flat path evaluates on two boxes compares two coordinates
on one axis (a vertex against a plane, a point against an edge's ends, a hit against a face's
extent). So a consistent sign assignment is one weak order of the four interval ends per axis:
one of Allen's 13 relations (`Rel.all`). The 13³ relations × 3 ops are all consistent patterns of
the family. Where the code samples a point (the middle of a stretch), its side of the other
coordinates is not fixed by the pattern; `sv` chooses it, and the theorems quantify over it.

**Coordinates.** Ranks are doubled: on each axis an even number 2r is the r-th distinct
coordinate, an odd number the open interval between two. A cell is a triple; its dimension is how
many of its coordinates are odd.
-/

namespace Coverage

inductive Op | union | subtract | intersect
  deriving DecidableEq, Repr, Inhabited

def Op.name : Op → String
  | .union => "union" | .subtract => "subtract" | .intersect => "intersect"

/-- One axis's sign pattern: the ranks of A's ends and B's ends. -/
structure Rel where
  name : String
  a0 : Nat
  a1 : Nat
  b0 : Nat
  b1 : Nat
  deriving Repr, Inhabited, DecidableEq

/-- Allen's relations of A to B, in the order `forge_coverage::Rel::ALL` lists them. -/
def Rel.all : List Rel :=
  [⟨"before", 0, 1, 2, 3⟩, ⟨"meets", 0, 1, 1, 2⟩, ⟨"overlaps", 0, 2, 1, 3⟩,
   ⟨"finishedBy", 0, 2, 1, 2⟩, ⟨"contains", 0, 3, 1, 2⟩, ⟨"starts", 0, 1, 0, 2⟩,
   ⟨"equal", 0, 1, 0, 1⟩, ⟨"startedBy", 0, 2, 0, 1⟩, ⟨"during", 1, 2, 0, 3⟩,
   ⟨"finishes", 1, 2, 0, 2⟩, ⟨"overlappedBy", 1, 3, 0, 2⟩, ⟨"metBy", 1, 2, 0, 1⟩,
   ⟨"after", 2, 3, 0, 1⟩]

/-- A weak order of two intervals' ends with every rank used is one of the 13: the list is complete
(a consistent pattern on one axis is exactly a rank tuple with a0 < a1, b0 < b1, ranks 0..k-1). -/
def Rel.valid (r : Rel) : Bool :=
  r.a0 < r.a1 && r.b0 < r.b1 &&
  (let k := (max r.a1 r.b1) + 1
   (List.range k).all fun v => v == r.a0 || v == r.a1 || v == r.b0 || v == r.b1)

inductive Side | A | B
  deriving DecidableEq, Repr, Inhabited

def Side.other : Side → Side | .A => .B | .B => .A

structure Pat where
  rx : Rel
  ry : Rel
  rz : Rel
  op : Op
  deriving Repr, Inhabited

def Pat.rel (p : Pat) : Nat → Rel | 0 => p.rx | 1 => p.ry | _ => p.rz

def Pat.label (p : Pat) : String := s!"{p.rx.name}/{p.ry.name}/{p.rz.name} {p.op.name}"

/-- Every pattern, in the order `forge_coverage::Pattern::all` runs them. -/
def Pat.all : List Pat :=
  [Op.union, .subtract, .intersect].flatMap fun op =>
    Rel.all.flatMap fun x => Rel.all.flatMap fun y => Rel.all.map fun z => ⟨x, y, z, op⟩

/-! ## Cells, boxes and faces -/

structure V3 where
  x : Nat
  y : Nat
  z : Nat
  deriving DecidableEq, Repr, Inhabited

def V3.get (c : V3) : Nat → Nat | 0 => c.x | 1 => c.y | _ => c.z
def V3.of (f : Nat → Nat) : V3 := ⟨f 0, f 1, f 2⟩
def V3.set (c : V3) (i v : Nat) : V3 := V3.of fun a => if a == i then v else c.get a

def axes : List Nat := [0, 1, 2]
def others (i : Nat) : List Nat := axes.filter (· != i)
def third (i j : Nat) : Nat := 3 - i - j
def odd (n : Nat) : Bool := n % 2 == 1

def lo (p : Pat) : Side → Nat → Nat | .A, i => 2 * (p.rel i).a0 | .B, i => 2 * (p.rel i).b0
def hi (p : Pat) : Side → Nat → Nat | .A, i => 2 * (p.rel i).a1 | .B, i => 2 * (p.rel i).b1
def closedIn (p : Pat) (X : Side) (i u : Nat) : Bool := lo p X i ≤ u && u ≤ hi p X i
def openIn (p : Pat) (X : Side) (i u : Nat) : Bool := lo p X i < u && u < hi p X i
def atBound (p : Pat) (X : Side) (i u : Nat) : Bool := u == lo p X i || u == hi p X i

/-- A face: body, normal axis, and whether it is the far one (outward normal +axis). -/
structure Face where
  body : Side
  axis : Nat
  top : Bool
  deriving DecidableEq, Repr, Inhabited

def faces (X : Side) : List Face := axes.flatMap fun i => [⟨X, i, false⟩, ⟨X, i, true⟩]
def Face.coord (p : Pat) (f : Face) : Nat := if f.top then hi p f.body f.axis else lo p f.body f.axis

inductive Where | inside | on | outside
  deriving DecidableEq, Repr, Inhabited

/-- `Index::point_in_face` on a box face, for a point in its plane (classify.rs). -/
def pif (p : Pat) (f : Face) (c : V3) : Where :=
  if c.get f.axis != f.coord p then .outside
  else if (others f.axis).all fun j => openIn p f.body j (c.get j) then .inside
  else if (others f.axis).all fun j => closedIn p f.body j (c.get j) then .on
  else .outside

/-- `Index::classify` against a whole box. -/
def classify (p : Pat) (Y : Side) (c : V3) : Where :=
  if axes.all fun i => openIn p Y i (c.get i) then .inside
  else if axes.all fun i => closedIn p Y i (c.get i) then .on
  else .outside

/-- Closed face boxes overlap (`Box3::overlaps`, boolean.rs:992). -/
def boxesMeet (p : Pat) (fa fb : Face) : Bool :=
  axes.all fun i =>
    let (l1, h1) := if i == fa.axis then (fa.coord p, fa.coord p) else (lo p fa.body i, hi p fa.body i)
    let (l2, h2) := if i == fb.axis then (fb.coord p, fb.coord p) else (lo p fb.body i, hi p fb.body i)
    l1 ≤ h2 && l2 ≤ h1

/-! ## Retry flags

The thread-local flags (boolean.rs:853–870). Only the sites a flat pair reaches read them here;
`Coverage.Theorems.flags_inert` proves the outcome never depends on them. -/
structure Flags where
  careful : Bool := false   -- CAREFUL, set by `plan` (896) only with a torus or procedural face
  chords : Bool := false    -- CHORDS, `plan_retried` (926)
  noRunOn : Bool := false   -- NO_RUN_ON, `plan_retried` (937)
  fences : Bool := false    -- FENCES, `plan_fenced` (953)
  absorb : Bool := false    -- ABSORB_HAIR_ROOTS, `plan_retried` (925), read in intersect.rs:559
  deriving Repr, Inhabited, DecidableEq

/-- Every flag combination the ladder can set. -/
def Flags.all : List Flags :=
  [false, true].flatMap fun a => [false, true].flatMap fun b => [false, true].flatMap fun c =>
    [false, true].flatMap fun d => [false, true].map fun e => ⟨a, b, c, d, e⟩

/-- A sample point in the stretch (t0, t1), doubled coordinates: its middle. When the middle is a
coordinate of the pattern (even), which side of it the real middle lies on is a further sign the
pattern does not fix: `sv` picks it. -/
def sample (sv : Bool) (t0 t1 : Nat) : Nat :=
  let m := (t0 + t1) / 2
  if odd m then m else if sv then m + 1 else m - 1

def dedup (l : List Nat) : List Nat := l.foldl (fun acc x => if acc.contains x then acc else acc ++ [x]) []
def insertSorted (x : Nat) : List Nat → List Nat
  | [] => [x]
  | y :: ys => if x ≤ y then x :: y :: ys else y :: insertSorted x ys
def sortNat (l : List Nat) : List Nat := l.foldl (fun acc x => insertSorted x acc) []
def consecutive : List Nat → List (Nat × Nat)
  | a :: b :: rest => (a, b) :: consecutive (b :: rest)
  | _ => []

/-! ## Step 1–2: face pairs and `trim` (boolean.rs:986–1060, 2265–3167) -/

/-- The cuts `trim`'s edge loop (2321–2659) makes on the edges of face `f` (of body `f.body`) for
the line L where `f` meets the perpendicular face `g`, as positions along L's axis.
* An edge across L (at x_k = κ): kept only if its box meets L's (2326), i.e. g's coordinate lies on
  the edge; `plane_meet` has no line–line case (3800) and `tolerant` needs an imported edge (2361),
  so `across` gives the neighbour's plane (4084–4100; box neighbours are square, never `smooth`);
  `line_plane` hits once (intersect.rs:109–114); not parallel, so not run-on (2446–2455); the hit is a
  `Crossing` so `crossings_near` returns it (4405–4407); `on_edge` holds (283–300); kept if g's box
  holds it (2590–2593: the box test, or with CAREFUL `point_in_face`, agree on a box face).
* An edge along L: only where L lies on it (2326). `line_plane` against the neighbour's plane is
  `OnSurface` (intersect.rs:102–104), not `Hits`, so the run-on arm is taken whatever NO_RUN_ON says
  (2446–2456); the neighbour is coplanar with g (2465–2467), so no `along` (2470); cuts at both ends
  (2471–2486), with no test against g. -/
def edgeCuts (p : Pat) (fl : Flags) (f g : Face) : List Nat :=
  let (i, j) := (f.axis, g.axis)
  let k := third i j
  let (α, β) := (f.coord p, g.coord p)
  let S := f.body
  let T := g.body
  (others i).flatMap fun j' => [false, true].flatMap fun t' =>
    let nb : Face := ⟨S, j', t'⟩
    if j' == k then
      let κ := nb.coord p
      let overlaps := closedIn p S j β
      -- (careful only swaps the box test for point_in_face, which agree on a box face)
      let accept := if fl.careful then closedIn p T i α && closedIn p T k κ
                    else closedIn p T i α && closedIn p T k κ
      if overlaps && accept then [κ] else []
    else
      let γ := nb.coord p
      if γ == β then [lo p S k, hi p S k] else []

/-- `trim`'s stretch test (`inside`, 2991–3093) at a stretch's sample point: inside both faces.
A sample on either face's boundary gives `On`; for a box no cut is strictly inside a crossed edge
(`crossed`, 2952–2973, needs `c.s` strictly inside the edge's range: every cut on a face's own
edges is at an edge end or on an edge across L), so `past` is empty, `hair` (3004–3007, the only
read of CHORDS a flat pair reaches) is never asked, and the stretch is `On`; the later samples
break out (3022–3066: the edge it lies along holds it) and it is no new edge. -/
def stretchInside (p : Pat) (fl : Flags) (fa fb : Face) (m : Nat) : Bool :=
  let pt := V3.of fun a => if a == fa.axis then fa.coord p else if a == fb.axis then fb.coord p else m
  let _ := fl.chords
  pif p fa pt == .inside && pif p fb pt == .inside

/-- `trim` for a perpendicular pair: the cells of L's pieces kept as new edges (3095–3118). -/
def trimSegs (p : Pat) (fl : Flags) (sv : Bool) (fa fb : Face) : List V3 :=
  let k := third fa.axis fb.axis
  -- cuts of both faces' edges (2321–2659), sorted (2829) and the same point once (2876–2881)
  let ts := sortNat (dedup (edgeCuts p fl fa fb ++ edgeCuts p fl fb fa))
  -- no along (no `along` is pushed for a box pair), no `free` cuts (a line has neither touch
  -- points nor ends, 2741–2760), `doubtful` false (2421, line edges); pairs = 0 for a line (2830)
  (consecutive ts).flatMap fun (t0, t1) =>
    if t1 ≤ t0 then [] else
    if stretchInside p fl fa fb (sample sv t0 t1) then
      ((List.range (t1 - t0)).map (· + t0)).filter odd |>.map fun m =>
        V3.of fun a => if a == fa.axis then fa.coord p else if a == fb.axis then fb.coord p else m
    else []

/-- The face pairs `plan_once` intersects (986–1060): A's faces near B's box, every B face, boxes
meeting; plane–plane (intersect.rs:850–867, `nearly_flush` ssi.rs:81–92 needs a tilt in
(ANGULAR, 1e-4], never on boxes): parallel and level is `Coincident`, parallel apart `Empty`,
otherwise a line. `rolls_off` is false for two planes (3886) and `touches` finds nothing in
`Coincident` or a `Crossing` line (2048–2101), so the touch test (1061–1094) never runs. -/
def coplanarPairs (p : Pat) : List (Face × Face) :=
  (faces .A).flatMap fun fa => (faces .B).filterMap fun fb =>
    if fa.axis == fb.axis && fa.coord p == fb.coord p && boxesMeet p fa fb then some (fa, fb) else none

def perpPairs (p : Pat) : List (Face × Face) :=
  (faces .A).flatMap fun fa => (faces .B).filterMap fun fb =>
    if fa.axis != fb.axis && boxesMeet p fa fb then some (fa, fb) else none

/-- Every new edge cell, with the two faces whose pair made it. -/
def segs (p : Pat) (fl : Flags) (sv : Bool) : List (Face × Face × V3) :=
  (perpPairs p).flatMap fun (fa, fb) => (trimSegs p fl sv fa fb).map fun c => (fa, fb, c)

/-! ## Step 3: coplanar imprint (boolean.rs:1098–1124, 4109–4274) -/

/-- `imprint` of face `s`'s boundary into the coplanar face `d`: the cells of `s`'s edges lying
inside `d`, which become `d`'s foreign pieces. Per edge e of s (at x_q = q, along m): d's edges
across e cut it where it crosses their neighbour's plane and lies on them (4144, 4183–4203; the
`straight` corner rule is skipped for two lines, 4157–4161); d's edges along e's line are
`OnSurface`, which goes to `coincide` and `along` (4145–4153). The stretches between cuts are
imprinted where their middle is inside d (4216–4271). -/
def imprintCells (p : Pat) (sv : Bool) (s d : Face) : List V3 :=
  let i := s.axis
  let S := s.body
  let D := d.body
  (others i).flatMap fun j' => [false, true].flatMap fun t' =>
    let q := (Face.mk S j' t').coord p
    let m := third i j'
    -- d's edges across e lie at x_m = d's m-bounds; each cuts e where it lies on it
    let hits := [lo p D m, hi p D m].filter fun q' =>
      closedIn p D j' q && closedIn p S m q' && lo p S m < q' && q' < hi p S m
    let marks := sortNat (dedup ([lo p S m, hi p S m] ++ hits))
    (consecutive marks).flatMap fun (w0, w1) =>
      let pt := fun (u : Nat) => V3.of fun a => if a == i then s.coord p else if a == j' then q else u
      if pif p d (pt (sample sv w0 w1)) == .inside then
        ((List.range (w1 - w0)).map (· + w0)).filter odd |>.map pt
      else []

/-- Foreign pieces: (face imprinted into, cell). -/
def foreign (p : Pat) (sv : Bool) : List (Face × V3) :=
  (coplanarPairs p).flatMap fun (fa, fb) =>
    (imprintCells p sv fb fa).map (fa, ·) ++ (imprintCells p sv fa fb).map (fb, ·)

/-! ## Shared pieces (boolean.rs:4464–4535, 1652–1693) -/

/-- A 1-cell on an edge of body X: its odd axis inside X's range, the other two at X's bounds. -/
def edgeCell (p : Pat) (X : Side) (c : V3) : Bool :=
  match axes.filter fun a => odd (c.get a) with
  | [m] => openIn p X m (c.get m) && (others m).all fun a => atBound p X a (c.get a)
  | _ => false

/-- Cells of an A edge and a B edge at once. `coincide` cuts each at the other's ends (from the
coplanar imprint of the two faces at either edge, whose neighbour's plane holds the other edge),
`share_cuts` copies the middle cuts, so the pieces have the same ends and B's gives way to A's
(`shared`): no flood across them (5947–5951). -/
def shared (p : Pat) (c : V3) : Bool := edgeCell p .A c && edgeCell p .B c

/-! ## Step 4: split faces into regions (boolean.rs:1345–1650, `split_face` 5058) -/

/-- The open 2-cells of a face. -/
def faceCells (p : Pat) (f : Face) : List V3 :=
  let ot := others f.axis
  let (j, k) := (ot.getD 0 0, ot.getD 1 0)
  (List.range 7).flatMap fun u => (List.range 7).filterMap fun v =>
    if odd u && odd v && openIn p f.body j u && openIn p f.body k v then
      some (V3.of fun a => if a == f.axis then f.coord p else if a == j then u else v)
    else none

/-- The 1-cell between two cells of a face that differ by 2 on one axis, if they do. -/
def between (c d : V3) : Option V3 :=
  match axes.filter fun a => c.get a != d.get a with
  | [a] => if c.get a + 2 == d.get a || d.get a + 2 == c.get a
           then some (c.set a ((c.get a + d.get a) / 2)) else none
  | _ => none

/-- The face's new edges: its pairs' curves and the other body's edges imprinted into it. -/
def newEdge (sg : List (Face × Face × V3)) (fo : List (Face × V3)) (f : Face) (c : V3) : Bool :=
  sg.any (fun (a, b, e) => (a == f || b == f) && e == c) || fo.any fun (g, e) => g == f && e == c

/-- Connected components of `cells` under `link`. -/
partial def components (cells : List V3) (link : V3 → V3 → Bool) : List (List V3) :=
  let rec grow (comp : List V3) (rest : List V3) : List V3 × List V3 :=
    let (add, keep) := rest.partition fun c => comp.any (link · c)
    if add.isEmpty then (comp, rest) else grow (comp ++ add) keep
  match cells with
  | [] => []
  | c :: rest =>
    let (comp, left) := grow [c] rest
    comp :: components left link

/-- A face's regions: its cells joined across every 1-cell that is no new edge. Every new edge of a
box face ends on the boundary or on another new edge, so `split_face` closes its cycles
(SplitOpen, SplitCrossed, HoleOutside and EmptyCycle are not reached; checked against the kernel). -/
def regions (p : Pat) (sg : List (Face × Face × V3)) (fo : List (Face × V3)) (f : Face) : List (List V3) :=
  components (faceCells p f) fun c d =>
    match between c d with
    | some e => !newEdge sg fo f e
    | none => false

/-! ## Step 5: classify (boolean.rs:5861–6071) -/

inductive Cls | inn | out | onSame | onOpp
  deriving DecidableEq, Repr, Inhabited

/-- `keep_of` (6096–6104): kept, and flipped. -/
def keepOf : Op → Side → Cls → Option Bool
  | .union, _, .out => some false
  | .union, .A, .onSame => some false
  | .subtract, .A, .out => some false
  | .subtract, .A, .onOpp => some false
  | .subtract, .B, .inn => some true
  | .intersect, _, .inn => some false
  | .intersect, .A, .onSame => some false
  | _, _, _ => none

/-- A cell next to a 1-cell (one apart on one axis). -/
def adjacent (c e : V3) : Bool :=
  match axes.filter fun a => c.get a != e.get a with
  | [a] => c.get a + 1 == e.get a || e.get a + 1 == c.get a
  | _ => false

structure Spec where
  face : Face
  cells : List V3
  deriving Repr, Inhabited

inductive Lost | inconsistentClassification | noInteriorPoint
  deriving DecidableEq, Repr, Inhabited

inductive Refusal | lost (l : Lost) | untouchedOnOther | zeroThickness
  deriving DecidableEq, Repr, Inhabited

/-- A's faces the boolean looks at (`near_b`, 980–982): boxes meeting B's box. The others pass
through as they are, outside B. -/
def involved (p : Pat) (f : Face) : Bool :=
  f.body == .B || axes.all fun i =>
    let (l1, h1) := if i == f.axis then (f.coord p, f.coord p) else (lo p .A i, hi p .A i)
    l1 ≤ hi p .B i && lo p .B i ≤ h1

def specs (p : Pat) (sg : List (Face × Face × V3)) (fo : List (Face × V3)) : List Spec :=
  ((faces .A).filter (involved p) ++ faces .B).flatMap fun f =>
    (regions p sg fo f).map fun cs => ⟨f, cs⟩

/-- The coplanar class of a spec (5873–5917): its interior point inside a coplanar partner. -/
def coplanarClass (p : Pat) (s : Spec) : Option Cls :=
  let partners := (coplanarPairs p).filterMap fun (fa, fb) =>
    if s.face.body == .A && fa == s.face then some fb
    else if s.face.body == .B && fb == s.face then some fa else none
  match s.cells.head? with
  | none => none
  | some q => partners.findSome? fun g =>
      if pif p g q == .inside then some (if g.top == s.face.top then Cls.onSame else Cls.onOpp)
      else none

/-- The lean at the spec's first new-edge piece (5918–5932, `lean` 6073–6094): the sign of the way
into the region against the other face's outward normal; never 0 on a box (|lean| = 1 > 1e-9). -/
def leanClass (sg : List (Face × Face × V3)) (s : Spec) : Option Cls :=
  sg.findSome? fun (fa, fb, e) =>
    let other := if fa == s.face then some fb else if fb == s.face then some fa else none
    match other with
    | none => none
    | some g => s.cells.findSome? fun c =>
        if adjacent c e then
          let into := c.get g.axis > e.get g.axis
          some (if into != g.top then Cls.inn else Cls.out)
        else none

/-- Whether spec `s` uses piece cell `e` in its loops: on its face's boundary, or a new edge of its
face, next to one of its cells. -/
def uses (p : Pat) (sg : List (Face × Face × V3)) (fo : List (Face × V3)) (s : Spec) (e : V3) : Bool :=
  s.cells.any (adjacent · e) &&
    (e.get s.face.axis == s.face.coord p) &&
    (pif p s.face e == .on || newEdge sg fo s.face e)

/-- Pieces a flood may cross (5933–5958): an old edge's, not imprinted anywhere, not shared. -/
def floods (p : Pat) (fo : List (Face × V3)) (X : Side) (e : V3) : Bool :=
  edgeCell p X e && !shared p e && !fo.any (·.2 == e)

def allCells1 : List V3 :=
  (List.range 7).flatMap fun x => (List.range 7).flatMap fun y => (List.range 7).filterMap fun z =>
    let c : V3 := ⟨x, y, z⟩
    if (axes.filter fun a => odd (c.get a)).length == 1 then some c else none

/-- `classify_specs`: coplanar first, then leans, the flood, and a ray for the rest. A flood
component with two leans is `InconsistentClassification` (6004–6013); a ray that finds the region on
the other body everywhere is `UntouchedOnOther` (6046–6057). -/
def classifySpecs (p : Pat) (sg : List (Face × Face × V3)) (fo : List (Face × V3)) (ss : List Spec) :
    Except Refusal (List Cls) :=
  let n := ss.length
  let idx := List.range n
  let fixed := ss.map (coplanarClass p)
  let leans := ss.map (leanClass sg)
  -- floodable links between two unfixed specs of one body
  let pieces := allCells1
  let link := fun (a b : Nat) =>
    let (sa, sb) := (ss.getD a default, ss.getD b default)
    a != b && sa.face.body == sb.face.body && (fixed.getD a none).isNone && (fixed.getD b none).isNone &&
      pieces.any fun e => floods p fo sa.face.body e && uses p sg fo sa e && uses p sg fo sb e
  -- components of spec indices (as cells of a line, reusing `components`)
  let asCell := fun (a : Nat) => (⟨a, 0, 0⟩ : V3)
  let comps := components ((idx.filter fun a => (fixed.getD a none).isNone).map asCell)
    (fun a b => link a.x b.x)
  let settle : List V3 → Except Refusal (List (Nat × Cls)) := fun comp =>
    let members := comp.map (·.x)
    let seeds := dedupCls (members.filterMap fun a => leans.getD a none)
    match seeds with
    | [c] => .ok (members.map (·, c))
    | [] =>
      -- a ray from the first spec's interior point (6033–6060)
      let s := ss.getD (members.headD 0) default
      let Y := s.face.body.other
      match s.cells.find? (fun q => classify p Y q != .on) with
      | some q => .ok (members.map (·, if classify p Y q == .inside then Cls.inn else Cls.out))
      | none => if s.cells.isEmpty then .error (.lost .noInteriorPoint) else .error .untouchedOnOther
    | _ => .error (.lost .inconsistentClassification)
  match comps.mapM settle with
  | .error e => .error e
  | .ok assigned =>
    let flat := assigned.flatten
    .ok (idx.map fun a => match fixed.getD a none with
      | some c => c
      | none => (flat.find? (·.1 == a)).map (·.2) |>.getD .out)
where
  dedupCls (l : List Cls) : List Cls := l.foldl (fun acc x => if acc.contains x then acc else acc ++ [x]) []

/-! ## Step 6: keep, the four-use test, the result (boolean.rs:1715–1737, 560–590) -/

/-- An oriented boundary 2-cell: the cell and whether its outward normal is +axis. -/
abbrev Bdry := V3 × Bool

/-- The kernel's kept boundary: kept specs (flipped where `keep_of` says), and A's faces it did not
look at (kept by union and subtract, 1739–1742). -/
def keptBoundary (p : Pat) (ss : List Spec) (cls : List Cls) : List Bdry :=
  let fromSpecs := (ss.zip cls).flatMap fun (s, c) =>
    match keepOf p.op s.face.body c with
    | some flip => s.cells.map (·, s.face.top != flip)
    | none => []
  let passed := if p.op == .intersect then [] else
    ((faces .A).filter fun f => !involved p f).flatMap fun f => (faceCells p f).map (·, f.top)
  fromSpecs ++ passed

/-- The exact answer: the op on the 3-cells; the regularised result's oriented boundary. -/
def kept3 (p : Pat) (c : V3) : Bool :=
  let inA := axes.all fun i => openIn p .A i (c.get i)
  let inB := axes.all fun i => openIn p .B i (c.get i)
  axes.all (fun i => c.get i < 7) &&
  match p.op with
  | .union => inA || inB
  | .subtract => inA && !inB
  | .intersect => inA && inB

def exactBoundary (p : Pat) : List Bdry :=
  (List.range 7).flatMap fun x => (List.range 7).flatMap fun y => (List.range 7).flatMap fun z =>
    let c : V3 := ⟨x, y, z⟩
    match axes.filter fun a => !odd (c.get a) with
    | [m] =>
      let below := if c.get m == 0 then false else kept3 p (c.set m (c.get m - 1))
      let above := kept3 p (c.set m (c.get m + 1))
      if below != above then [(c, below)] else []
    | _ => []

/-- Octant d (bits +x, +y, +z) of vertex v. -/
def octant (v : V3) (d : Nat) : Option V3 :=
  let s := fun (a : Nat) (b : Nat) => if (d / b) % 2 == 1 then some (a + 1) else if a == 0 then none else some (a - 1)
  match s v.x 1, s v.y 2, s v.z 4 with
  | some x, some y, some z => some ⟨x, y, z⟩
  | _, _, _ => none

/-- The kept octants about each vertex and the others each face-connected: the boundary is a
manifold there (the same test as `forge_coverage::manifold_octants`). -/
def manifoldAt (k : V3 → Bool) (v : V3) : Bool :=
  let set := (List.range 8).map fun d => match octant v d with | some c => k c | none => false
  let conn := fun (want : Bool) =>
    let members := (List.range 8).filter fun d => set.getD d false == want
    match members with
    | [] => true
    | m :: _ =>
      let rec reach (fuel : Nat) (seen : List Nat) : List Nat :=
        match fuel with
        | 0 => seen
        | fuel + 1 =>
          let more := seen.flatMap (fun a => [a ^^^ 1, a ^^^ 2, a ^^^ 4]) |>.filter
            (fun b => set.getD b false == want && !seen.contains b)
          if more.isEmpty then seen else reach fuel (seen ++ dedup more)
      (reach 8 [m]).length == members.length
  conn true && conn false

def manifold (k : V3 → Bool) : Bool :=
  (List.range 4).all fun x => (List.range 4).all fun y => (List.range 4).all fun z =>
    manifoldAt k ⟨2 * x, 2 * y, 2 * z⟩

/-! ## Outcomes -/

inductive Outcome
  | ok           -- a valid body, the exact one
  | empty        -- nothing left
  | zeroThickness
  | pinched      -- a body `check` accepts whose boundary is the exact one, of a set that is not a manifold
                 -- (`split_old_pinches`, boolean.rs:631–653, gave each fan at the pinch its own vertex)
  | wrong        -- a body whose boundary is not the exact one
  | lost (l : Lost)   -- Internal: the retry ladder runs (`plan_retried`, 910)
  | unsupported
  deriving DecidableEq, Repr, Inhabited

def Outcome.word : Outcome → String
  | .ok => "ok" | .empty => "empty" | .zeroThickness => "zero-thickness" | .pinched => "pinched"
  | .wrong => "wrong" | .lost .inconsistentClassification => "internal InconsistentClassification"
  | .lost .noInteriorPoint => "internal NoInteriorPoint" | .unsupported => "unsupported"

def sameSet (a b : List Bdry) : Bool := a.all (b.contains ·) && b.all (a.contains ·) && a.length == b.length

/-- What `plan_once` and `general` come to on the pattern, with these flags and sample sides. -/
def kernel (p : Pat) (fl : Flags := {}) (sv : Bool := false) : Outcome :=
  let sg := segs p fl sv
  let fo := foreign p sv
  let ss := specs p sg fo
  match classifySpecs p sg fo ss with
  | .error (.lost l) => .lost l
  | .error .untouchedOnOther => .unsupported
  | .error .zeroThickness => .zeroThickness
  | .ok cls =>
    -- an edge four kept faces meet at (1724–1737)
    let keptSpecs := (ss.zip cls).filter fun (s, c) => (keepOf p.op s.face.body c).isSome
    let four := allCells1.any fun e =>
      (keptSpecs.filter fun (s, _) => uses p sg fo s e).length > 2
    if four then .zeroThickness else
    let b := keptBoundary p ss cls
    if b.isEmpty then .empty
    else if !sameSet b (exactBoundary p) then .wrong
    else if manifold (kept3 p) then .ok else .pinched

/-- The exact outcome: empty, a manifold body, or ZeroThickness where it is not a manifold. -/
def exact (p : Pat) : Outcome :=
  if exactBoundary p |>.isEmpty then .empty
  else if manifold (kept3 p) then .ok else .zeroThickness

end Coverage
