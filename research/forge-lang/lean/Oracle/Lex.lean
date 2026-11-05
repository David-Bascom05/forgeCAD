import ForgeLang.LL1.Impl

/-! The lexer oracle for `interface/forge-lang/tests/parse_diff.rs`: each stdin line is a source
text in hex; the answer is its pieces as `kind:length`, or `e<error> lo hi`. It runs the model
`Lex.lex` on the parser's keywords and operators (`Impl.lexCfg`), nothing else. -/

open ForgeLang.LL1

def code : Lex.Kind → String
  | .num .none => "n0" | .num .mm => "n1" | .num .deg => "n2"
  | .str => "s" | .id => "i" | .kw k => s!"k{k}" | .op o => s!"o{o}"
  | .nl => "l" | .space => "w" | .comment => "c" | .newline => "j"

def errCode : Lex.Err → String
  | .badNumber => "eN" | .unknownUnit => "eU" | .openString => "eS" | .badChar => "eC"

def hexVal (c : Char) : Nat :=
  if c.isDigit then c.toNat - 48 else c.toNat - 87

def unhex : List Char → List Nat
  | a :: b :: r => (16 * hexVal a + hexVal b) :: unhex r
  | _ => []

def answer (s : List Nat) : String :=
  match Lex.lex Impl.lexCfg s with
  | .ok ps => " ".intercalate (ps.map fun p => s!"{code p.kind}:{p.text.length}")
  | .error (e, lo, hi) => s!"{errCode e} {lo} {hi}"

partial def loop (h : IO.FS.Stream) (o : IO.FS.Stream) : IO Unit := do
  let line ← h.getLine
  if line.isEmpty then return
  o.putStrLn (answer (unhex ((line.replace "\n" "").toList)))
  loop h o

def main : IO Unit := do
  loop (← IO.getStdin) (← IO.getStdout)
