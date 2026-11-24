/-
forge-cam-oracle: slab::dist's candidates over ℚ, exactly as proven in ForgeCam/Dist2.lean, for
forge-cam-geom's L6 differential tests.

Run: `lake env lean --run Oracle.lean < cases` (interpreted: no native build of Mathlib).
Reads lines from stdin, writes one answer per line, numbers as "n" or "n/d" (exact; an f64 is
a dyadic, so write it as m/2^k):
  segseg ax ay ux uy cx cy vx vy  → segSegLo: the least squared distance between the segments
                                    a + t·u and c + s·v (t, s ∈ [0, 1]) if they are apart (slab's
                                    `apart`), else 0. A lower bound always; exact when apart.
  ptseg qx qy ax ay dx dy         → ptSegSq: the least squared distance from q to a + t·d.
  apart ax ay ux uy cx cy vx vy   → true or false.
A line it cannot read gets "error: ..." and the next line is read.
-/
import ForgeCam.Dist2

open ForgeCam.Dist2

def parseRat (s : String) : Option ℚ :=
  match s.splitOn "/" with
  | [n] => (n.toInt?).map (fun n => (n : ℚ))
  | [n, d] => do
    let n ← n.toInt?
    let d ← d.toNat?
    if d = 0 then none else some (mkRat n d)
  | _ => none

def showRat (q : ℚ) : String :=
  if q.den = 1 then toString q.num else s!"{q.num}/{q.den}"

def answer (line : String) : String :=
  let ws := (line.splitOn " ").filter (· ≠ "")
  match ws with
  | [] => ""
  | cmd :: args =>
    match args.mapM parseRat with
    | none => "error: a number is not n or n/d"
    | some xs =>
      match cmd, xs with
      | "segseg", [ax, ay, ux, uy, cx, cy, vx, vy] =>
        showRat (segSegLo (ax, ay) (ux, uy) (cx, cy) (vx, vy))
      | "ptseg", [qx, qy, ax, ay, dx, dy] => showRat (ptSegSq (qx, qy) (ax, ay) (dx, dy))
      | "apart", [ax, ay, ux, uy, cx, cy, vx, vy] =>
        if decide (apart (ax, ay) (ux, uy) (cx, cy) (vx, vy)) then "true" else "false"
      | _, _ => "error: segseg (8 numbers), ptseg (6) or apart (8)"

partial def loop (h : IO.FS.Stream) (o : IO.FS.Stream) : IO Unit := do
  let line ← h.getLine
  if line.isEmpty then return
  let l := String.ofList (line.toList.filter (fun c => c ≠ '\n' && c ≠ '\r'))
  if !l.isEmpty then o.putStrLn (answer l)
  loop h o

def main : IO Unit := do
  loop (← IO.getStdin) (← IO.getStdout)
