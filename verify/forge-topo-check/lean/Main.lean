import TopoCheck

/-!
The model as a program: reads bodies on stdin, writes each one's problems, by `Body.check` with the
corner tables given (`Body::corners` is geometry).

Input, by whitespace-separated tokens, one body after another:
```
body <nv>
E <start> <end> <k> <coedge>*k      one per edge, in order
C <edge> <forward 0|1> <loop>       one per coedge
L <first> <len> <face>              one per loop
F <k> <loop>*k                      one per face
S <k> <face>*k                      one per shell
K <face> <vertex> <k> (<leaving> <arriving>)*k   `Body::corners`, where not empty
end
```
Output, per body: `body`, one line per problem (as `Problem::rule`, then its ids), then `end`.
-/

open TopoCheck

def TopoCheck.Problem.line : Problem → String
  | .edgeUses e n => s!"edge-uses {e} {n}"
  | .edgeUsesSameWay e => s!"edge-same-way {e}"
  | .loopEmpty l => s!"loop-empty {l}"
  | .loopBroken l c => s!"loop-broken {l} {c}"
  | .faceNoLoops f => s!"face-no-loops {f}"
  | .shellEuler s v => s!"shell-euler {s} {v}"
  | .vertexFans v n => s!"vertex-fans {v} {n}"

partial def nats (ts : List String) (k : Nat) (acc : Array Nat) : Option (Array Nat × List String) :=
  if k = 0 then some (acc, ts)
  else match ts with
    | t :: r => t.toNat?.bind fun n => nats r (k - 1) (acc.push n)
    | [] => none

abbrev Corners := Array (Nat × Nat × List (Nat × Nat))

def pairs : List Nat → List (Nat × Nat)
  | a :: c :: r => (a, c) :: pairs r
  | _ => []

partial def parseBody (ts : List String) (b : Body) (ks : Corners) :
    Except String (Body × Corners × List String) :=
  match ts with
  | "end" :: r => .ok (b, ks, r)
  | "K" :: r =>
    match nats r 3 #[] with
    | some (#[f, v, k], r) =>
      match nats r (2 * k) #[] with
      | some (xs, r) => parseBody r b (ks.push (f, v, pairs xs.toList))
      | none => .error "K pairs"
    | _ => .error "K"
  | "E" :: r =>
    match nats r 3 #[] with
    | some (#[s, e, k], r) =>
      match nats r k #[] with
      | some (us, r) => parseBody r { b with edges := b.edges.push (⟨s, e, us.toList⟩) } ks
      | none => .error "E uses"
    | _ => .error "E"
  | "C" :: r =>
    match nats r 3 #[] with
    | some (#[e, f, l], r) => parseBody r { b with coedges := b.coedges.push (⟨e, f == 1, l⟩) } ks
    | _ => .error "C"
  | "L" :: r =>
    match nats r 3 #[] with
    | some (#[a, n, f], r) => parseBody r { b with loops := b.loops.push (⟨a, n, f⟩) } ks
    | _ => .error "L"
  | "F" :: r =>
    match nats r 1 #[] with
    | some (#[k], r) =>
      match nats r k #[] with
      | some (ls, r) => parseBody r { b with faces := b.faces.push (ls.toList) } ks
      | none => .error "F loops"
    | _ => .error "F"
  | "S" :: r =>
    match nats r 1 #[] with
    | some (#[k], r) =>
      match nats r k #[] with
      | some (fs, r) => parseBody r { b with shells := b.shells.push (fs.toList) } ks
      | none => .error "S faces"
    | _ => .error "S"
  | t :: _ => .error s!"unexpected {t}"
  | [] => .error "no end"

partial def run (ts : List String) (out : IO.FS.Stream) : IO Unit := do
  match ts with
  | [] => pure ()
  | "body" :: n :: r =>
    match n.toNat? with
    | none => throw (IO.userError "body count")
    | some nv =>
      match parseBody r { nv, edges := #[], coedges := #[], loops := #[], faces := #[], shells := #[] } #[] with
      | .error e => throw (IO.userError e)
      | .ok (b, ks, r) =>
        out.putStrLn "body"
        let corners f v := (ks.find? (fun x => x.1 == f && x.2.1 == v)).elim [] (·.2.2)
        for p in b.check corners do
          out.putStrLn p.line
        out.putStrLn "end"
        run r out
  | t :: _ => throw (IO.userError s!"unexpected {t}")

def main : IO Unit := do
  let input ← (← IO.getStdin).readToEnd
  let ts := ((input.replace "\n" " ").replace "\t" " ").splitOn " " |>.filter (· ≠ "")
  run ts (← IO.getStdout)
