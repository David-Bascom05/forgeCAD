/-!
# P6: grammars as data, and an LL(1) check

A grammar is a start symbol and a list of rules `A := α`; terminals and nonterminals are numbers
(names are kept beside them for reports). The check is a Boolean function, so a syntax change is a
data change: re-run `decide`/`native_decide` and the theorems in `LL1/Proof.lean` apply as they are.

The check does not trust how its tables were computed. `tables` iterates FIRST, FOLLOW and
nullability for a while; `check` then verifies, rule by rule, that the tables are *closed* (each
satisfies the inclusions that define it) and that every choice is made on one token. The proofs use
only that verification, so an unconverged table makes the check fail, never lie.
-/

namespace ForgeLang.LL1

abbrev Tok := Nat

inductive Sym where
  | t : Tok → Sym
  | n : Nat → Sym
  deriving DecidableEq, Repr

structure Grammar where
  start : Nat
  rules : List (Nat × List Sym)

/-- The tables a check is about: nullable nonterminals, FIRST and FOLLOW (`none` is end of input). -/
structure Tables where
  nul : List Nat
  fst : Nat → List Tok
  fol : Nat → List (Option Tok)

namespace Tables
variable (T : Tables)

def nullSym : Sym → Bool
  | .t _ => false
  | .n A => decide (A ∈ T.nul)

def firstSym : Sym → List Tok
  | .t a => [a]
  | .n A => T.fst A

def nullSeq (α : List Sym) : Bool := α.all T.nullSym

def firstSeq : List Sym → List Tok
  | [] => []
  | X :: β => T.firstSym X ++ (if T.nullSym X then firstSeq β else [])

/-- The lookaheads that select `A := α`. -/
def predict (A : Nat) (α : List Sym) : List (Option Tok) :=
  (T.firstSeq α).map some ++ (if T.nullSeq α then T.fol A else [])

end Tables

/-! ## What the check verifies, as propositions -/

structure Closed (G : Grammar) (T : Tables) : Prop where
  nul : ∀ A α, (A, α) ∈ G.rules → T.nullSeq α = true → A ∈ T.nul
  fst : ∀ A α, (A, α) ∈ G.rules → ∀ a ∈ T.firstSeq α, a ∈ T.fst A
  fol : ∀ A α, (A, α) ∈ G.rules → ∀ pre B post, α = pre ++ .n B :: post →
    (∀ a ∈ T.firstSeq post, some a ∈ T.fol B) ∧ (T.nullSeq post = true → ∀ o ∈ T.fol A, o ∈ T.fol B)
  eof : none ∈ T.fol G.start

def LL1 (G : Grammar) (T : Tables) : Prop :=
  ∀ (i j : Nat) (A : Nat) (α β : List Sym), G.rules[i]? = some (A, α) → G.rules[j]? = some (A, β) → i ≠ j →
    ∀ o ∈ T.predict A α, o ∉ T.predict A β

/-! ## The same, as Booleans -/

def splits : List Sym → List (List Sym × Nat × List Sym)
  | [] => []
  | X :: β =>
    let rest := (splits β).map fun (p, B, q) => (X :: p, B, q)
    match X with
    | .n B => ([], B, β) :: rest
    | .t _ => rest

def closedB (G : Grammar) (T : Tables) : Bool :=
  G.rules.all (fun (A, α) =>
    (!T.nullSeq α || decide (A ∈ T.nul)) &&
    (T.firstSeq α).all (fun a => decide (a ∈ T.fst A)) &&
    (splits α).all (fun (_, B, post) =>
      (T.firstSeq post).all (fun a => decide (some a ∈ T.fol B)) &&
      (!T.nullSeq post || (T.fol A).all (fun o => decide (o ∈ T.fol B))))) &&
  decide (none ∈ T.fol G.start)

def disjoint (xs ys : List (Option Tok)) : Bool := xs.all fun o => !decide (o ∈ ys)

def ll1B (G : Grammar) (T : Tables) : Bool :=
  let idx := List.range G.rules.length
  idx.all fun i => idx.all fun j =>
    i == j ||
    match G.rules[i]?, G.rules[j]? with
    | some (A, α), some (B, β) => A != B || disjoint (T.predict A α) (T.predict A β)
    | _, _ => true

/-! ## Computing the tables (any method would do; `check` verifies the result) -/

def nts (G : Grammar) : List Nat := (G.rules.map Prod.fst).eraseDups

def tblOf (xs : List (Nat × List α)) (A : Nat) : List α := (xs.lookup A).getD []

def stepNul (G : Grammar) (nul : List Nat) : List Nat :=
  (nts G).filter fun A => decide (A ∈ nul) ||
    G.rules.any fun (B, α) => B == A && α.all (fun X => match X with | .t _ => false | .n C => decide (C ∈ nul))

def stepFst (G : Grammar) (T : Tables) : List (Nat × List Tok) :=
  (nts G).map fun A => (A, ((T.fst A) ++
    (G.rules.filter (fun r : Nat × List Sym => r.1 == A)).flatMap (fun r => T.firstSeq r.2)).eraseDups)

def stepFol (G : Grammar) (T : Tables) : List (Nat × List (Option Tok)) :=
  (nts G).map fun B => (B, ((T.fol B) ++ (if B = G.start then [none] else []) ++
    G.rules.flatMap (fun r : Nat × List Sym => (splits r.2).flatMap fun sp : List Sym × Nat × List Sym =>
      let A := r.1; let C := sp.2.1; let post := sp.2.2
      if C = B then (T.firstSeq post).map some ++ (if T.nullSeq post then T.fol A else []) else [])).eraseDups)

def iterate {α : Type} (f : α → α) : Nat → α → α
  | 0, x => x
  | k + 1, x => iterate f k (f x)

def tables (G : Grammar) (k : Nat := 64) : Tables :=
  let nul := iterate (stepNul G) k []
  let fst := iterate (fun F => stepFst G ⟨nul, tblOf F, fun _ => []⟩) k []
  let fol := iterate (fun F => stepFol G ⟨nul, tblOf fst, tblOf F⟩) k []
  ⟨nul, tblOf fst, tblOf fol⟩

/-- The check. -/
def check (G : Grammar) (T : Tables := tables G) : Bool := closedB G T && ll1B G T

/-- For reports: every pair of alternatives of one nonterminal that share a lookahead. -/
def conflicts (G : Grammar) (T : Tables := tables G) : List (Nat × Nat × Nat × List (Option Tok)) :=
  let idx := List.range G.rules.length
  idx.flatMap fun i => idx.filterMap fun j =>
    if i < j then
      match G.rules[i]?, G.rules[j]? with
      | some (A, α), some (B, β) =>
        if A = B then
          let common := (T.predict A α).filter (· ∈ T.predict A β)
          if common.isEmpty then none else some (A, i, j, common)
        else none
      | _, _ => none
    else none

end ForgeLang.LL1
