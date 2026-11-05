import ForgeLang.LL1.Impl

/-! The parse oracle for `interface/forge-lang/tests/parse_diff.rs`: each stdin line is a token
string (terminal numbers of table.rs); the answer line is `1` and the tree's rules in preorder, or
`0`. It runs the proven `parse` on `Impl.grammar` and `Impl.certT`, nothing else. -/

open ForgeLang.LL1

mutual
def preT : Tree → List Nat
  | .leaf _ => []
  | .node r f => r :: preF f
def preF : Forest → List Nat
  | .nil => []
  | .cons t f => preT t ++ preF f
end

/-- Fuel: every tree of an input is found within it (a tree's depth is at most its leaves times
one more than the nonterminals, as no nonterminal derives itself without a token in between);
`0` is only answered when a run with four times as much also has no tree. -/
def answer (w : List Nat) : String :=
  let n := (w.length + 2) * (Impl.NN + 2)
  let go (k : Nat) : Option Tree := match parse Impl.grammar Impl.certT k (.n Impl.grammar.start) w with
    | some (t, []) => some t
    | _ => none
  match (go n).orElse fun _ => go (4 * n) with
  | some t => "1 " ++ " ".intercalate ((preT t).map toString)
  | none => "0"

partial def loop (h : IO.FS.Stream) (o : IO.FS.Stream) : IO Unit := do
  let line ← h.getLine
  if line.isEmpty then return
  let w := ((line.replace "\n" "").splitOn " ").filter (· ≠ "") |>.map String.toNat!
  o.putStrLn (answer w)
  loop h o

def main : IO Unit := do
  loop (← IO.getStdin) (← IO.getStdout)
