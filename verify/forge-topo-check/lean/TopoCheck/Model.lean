/-!
# The combinatorial part of `forge_topo::check`, as a Lean program

Line references are to `kernel/forge-topo/src/{body,check}.rs` at commit 8250ebb.

The model covers what `check` decides from indices alone:
* `check_edges`' use count and sense test (check.rs:639–669, without the geometric
  `EdgeDegenerate` test);
* `check_loops` (check.rs:671–691);
* `check_shell_euler` (check.rs:595–622);
* `check_fans` (check.rs:626–637) through `Body::leaving`, `Body::fans` and `Body::fans_by`
  (body.rs:931–1016);
* the gate in `check_known` (check.rs:419–444) that runs the last two only when no edge has a use
  count other than 2 and no loop is broken.

The geometric pairing `Body::corners` (body.rs:1018–1081) is a parameter, `corners f v`. It is the
list of (leaving, arriving) coedge pairs `corners` returns for face `f` at vertex `v`.

A body is compact: every entity is alive (`check` compacts first, check.rs:181–183). A body's ids
are list indices.
-/

namespace TopoCheck

/-- body.rs:37–53: `start`/`end` and the coedges using the edge (its curve and range are geometry). -/
structure Edge where
  start : Nat
  stop : Nat
  uses : List Nat
  deriving Inhabited, Repr, DecidableEq

/-- body.rs:62–66 -/
structure Coedge where
  edge : Nat
  fwd : Bool
  lp : Nat
  deriving Inhabited, Repr, DecidableEq

/-- body.rs:72–76: a loop is the run of coedges `first, first+1, …, first+len-1`. -/
structure Loop where
  first : Nat
  len : Nat
  face : Nat
  deriving Inhabited, Repr, DecidableEq

/-- body.rs:223–235, the topology: the vertex count, then edges, coedges, loops, each face's loops
and each shell's faces. -/
structure Body where
  nv : Nat
  edges : Array Edge
  coedges : Array Coedge
  loops : Array Loop
  faces : Array (List Nat)
  shells : Array (List Nat)
  deriving Inhabited, Repr

/-- check.rs:12–89, the variants `check_edges`, `check_loops`, `check_shell_euler` and
`check_fans` emit. -/
inductive Problem where
  | edgeUses (edge uses : Nat)
  | edgeUsesSameWay (edge : Nat)
  | loopEmpty (lp : Nat)
  | loopBroken (lp after : Nat)
  | faceNoLoops (face : Nat)
  | shellEuler (shell : Nat) (value : Int)
  | vertexFans (vertex fans : Nat)
  deriving Repr, DecidableEq

namespace Body
variable (b : Body)

def ed (e : Nat) : Edge := b.edges.getD e default
def co (c : Nat) : Coedge := b.coedges.getD c default
def lo (l : Nat) : Loop := b.loops.getD l default
def faceLoops (f : Nat) : List Nat := b.faces.getD f []

/-- A loop's coedges (body.rs:79–84). -/
def loopCoedges (l : Nat) : List Nat := (List.range (b.lo l).len).map ((b.lo l).first + ·)

/-- body.rs:916–920 -/
def start (c : Nat) : Nat := if (b.co c).fwd then (b.ed (b.co c).edge).start else (b.ed (b.co c).edge).stop
/-- body.rs:921–925 -/
def stop (c : Nat) : Nat := if (b.co c).fwd then (b.ed (b.co c).edge).stop else (b.ed (b.co c).edge).start
/-- body.rs:926–928 -/
def face (c : Nat) : Nat := (b.lo (b.co c).lp).face

/-- body.rs:1084–1087: the first use of the edge that is not `c`. -/
def partner (c : Nat) : Option Nat := (b.ed (b.co c).edge).uses.find? (· != c)

/-- body.rs:991–997: the coedge before `c` in its loop. -/
def prev (c : Nat) : Nat :=
  let l := b.lo (b.co c).lp
  l.first + (c - l.first + l.len - 1) % l.len

/-- body.rs:931–940: the coedges leaving `v`, edge by edge, each edge's uses in order. -/
def leaving (v : Nat) : List Nat :=
  (List.range b.edges.size).flatMap fun e => (b.ed e).uses.filter fun c => b.start c == v

/-- body.rs:983–998: the coedge that closes `c`'s corner: from the corners table where it has
`c`, else the one before it in its loop. -/
def before (closes : Option (List (Nat × List (Nat × Nat)))) (c : Nat) : Nat :=
  match closes with
  | none => b.prev c
  | some cl =>
    match ((cl.filter fun x => x.1 == b.face c).flatMap (·.2)).find? (·.1 == c) with
    | some x => x.2
    | none => b.prev c

/-- body.rs:1005–1012, one fan: walk `c → partner (before c)` until a coedge already seen, or one
with no partner. `fuel` stands for the finiteness of the coedge ids; returns the fan and `seen`. -/
def walk (bef : Nat → Nat) : Nat → Nat → List Nat → List Nat × List Nat
  | 0, _, seen => ([], seen)
  | fuel + 1, c, seen =>
    if seen.contains c then ([], seen)
    else
      match b.partner (bef c) with
      | some n =>
        let r := walk bef fuel n (seen ++ [c])
        (c :: r.1, r.2)
      | none => ([c], seen ++ [c])

/-- body.rs:999–1015 -/
def fansGo (bef : Nat → Nat) (fuel : Nat) : List Nat → List Nat → List (List Nat)
  | [], _ => []
  | c0 :: rest, seen =>
    if seen.contains c0 then fansGo bef fuel rest seen
    else
      let r := b.walk bef fuel c0 seen
      r.1 :: fansGo bef fuel rest r.2

/-- Enough fuel that the walk never runs out: one step per distinct coedge id it can reach. -/
def fuel (frm : List Nat) : Nat := frm.length + (b.edges.toList.map (·.uses.length)).sum + 1

/-- body.rs:972–1016 -/
def fansBy (frm : List Nat) (closes : Option (List (Nat × List (Nat × Nat)))) : List (List Nat) :=
  b.fansGo (b.before closes) (b.fuel frm) frm []

/-- body.rs:950–970: by loop order; where that leaves two fans or more, again with the corners of
each face in them, unless every face's corners are empty. -/
def fans (corners : Nat → Nat → List (Nat × Nat)) (frm : List Nat) : List (List Nat) :=
  let fs := b.fansBy frm none
  if fs.length < 2 then fs
  else
    let v := b.start (frm.headD 0)
    let faces := (fs.flatten.map b.face).eraseDups
    let closes := faces.map fun f => (f, corners f v)
    if closes.all (·.2.isEmpty) then fs else b.fansBy frm (some closes)

/-- check.rs:639–669, the use count and sense. -/
def checkEdges : List Problem :=
  (List.range b.edges.size).flatMap fun e =>
    match (b.ed e).uses with
    | [x, y] => if (b.co x).fwd == (b.co y).fwd then [.edgeUsesSameWay e] else []
    | us => [.edgeUses e us.length]

/-- check.rs:671–691 -/
def checkLoops : List Problem :=
  ((List.range b.faces.size).filterMap fun f =>
      if (b.faceLoops f).isEmpty then some (.faceNoLoops f) else none) ++
  (List.range b.loops.size).flatMap fun l =>
    let lp := b.lo l
    if lp.len == 0 then [.loopEmpty l]
    else (List.range lp.len).filterMap fun i =>
      if b.stop (lp.first + i) != b.start (lp.first + (i + 1) % lp.len)
      then some (.loopBroken l (lp.first + i)) else none

/-- The number of distinct ids (check.rs:612–615, sort then dedup). -/
def distinct (xs : List Nat) : Nat := xs.eraseDups.length

/-- check.rs:597–616, a shell's V − E + 2F − L. -/
def shellValue (fs : List Nat) : Int :=
  let ls := fs.flatMap b.faceLoops
  let cs := ls.flatMap b.loopCoedges
  let es := cs.map fun c => (b.co c).edge
  let vs := es.flatMap fun e => [(b.ed e).start, (b.ed e).stop]
  (distinct vs : Int) - distinct es + 2 * fs.length - ls.length

/-- check.rs:595–623 -/
def checkShellEuler : List Problem :=
  (List.range b.shells.size).filterMap fun s =>
    let value := b.shellValue (b.shells.getD s [])
    if value % 2 != 0 || value > 2 then some (.shellEuler s value) else none

/-- check.rs:626–637 -/
def checkFans (corners : Nat → Nat → List (Nat × Nat)) : List Problem :=
  (List.range b.nv).filterMap fun v =>
    let n := (b.fans corners (b.leaving v)).length
    if n > 1 then some (.vertexFans v n) else none

def blocks : Problem → Bool
  | .edgeUses .. | .loopBroken .. => true
  | _ => false

/-- check.rs:419–444, the combinatorial problems in the order `check` lists them. -/
def check (corners : Nat → Nat → List (Nat × Nat)) : List Problem :=
  let out := b.checkEdges ++ b.checkLoops
  if out.any blocks then out else out ++ b.checkShellEuler ++ b.checkFans corners

end Body
end TopoCheck
