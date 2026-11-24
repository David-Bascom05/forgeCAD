import ForgeLang.LL1.Impl

/-!
# The machining documents are sentences of the one grammar

A `machining` document (forge-cam-lang, forge-turn-lang) is read by forge-lang's own parser
(`syntax::parse`); its first statement is the bare name `machining`, an expression statement
(`stmt := expr assigntail`, `assigntail := %empty`), and an `assembly` file's optional first line is
the same. So the machining and assembly documents add no syntax: the LL(1) theorems of `Impl`
(`grammar_ll1`, `impl_unambiguous`, `source_to_tree`, `impl_roundtrip`) and the parser refinement
of `Driver` hold of them unchanged, and `tests/parse_diff.rs` compares the Rust parser with this
model on every `.fg` in the repository, examples/machined/ included.

What this file adds is evidence that the headers and the built-ins' calls are taken as intended,
on the generated grammar itself: text to tokens by the lexer model (`Lex.lex` on the parser's
keywords and operators), tokens to an accepted parse (`parsesAll`), so `source_to_tree` gives the
document's one tree. `Families.families_not_keywords` is the general half: no built-in of these
families and no header word is a keyword, so each lexes as an identifier wherever it stands.
-/

namespace ForgeLang.LL1.Documents

open Lex

/-- A string's bytes (ASCII). -/
def b (s : String) : List Nat := s.toList.map Char.toNat

/-- The parser's input for a text, if the lexer takes it. -/
def toks (s : String) : Option (List Nat) :=
  match lex Impl.lexCfg (b s) with
  | .ok ps => some (Impl.terminals ps)
  | .error _ => none

/-- The text lexes and its tokens parse, whole. -/
def accepts (s : String) : Bool :=
  match toks s with
  | some w => parsesAll Impl.grammar Impl.certT 4000 w
  | none => false

/-- A milling document: the header, a shared param, each kind of statement built-in, the nested
built-ins `box` and `vise`, a hole feature of the part read by `.`, a list of setups. -/
theorem milling_doc_accepted : accepts "machining
shared tol = 0.01
f     = part(\"flange.fg\", group: \"N\")
m     = machine(\"generic-vmc3\")
t1    = tool(\"face-50\", t: 1, stickout: 30, holder: \"bt40-er32\")
op1   = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: tol,
              fixture: vise(\"kurt-dx6\", across: \"x\", clamp: 8))
param op1.clear = 12
face  = facing(op1, tool: t1, step: 1)
bolts = drill(op1, f.bolts, plan: \"auto\")
prog  = program([op1], post: \"fanuc\", number: 1001)
" = true := by decide +kernel

/-- A turning document (examples/machined/shaft_turn.fg, its comments left out). -/
theorem turning_doc_accepted : accepts "machining
shared bar = 32
f     = part(\"shaft.fg\")
t1    = lathe_tool(\"od-rough-cnmg\", t: 1)
jaws  = chuck(face: -70, grip: 30, jaw: 15)
op1   = lathe_setup(f, bar: bar, front: 2, chuck: jaws, machine: \"generic-lathe\", at: 100,
                    offsets: ((1, 0, 0), (2, 0, 0), (4, 0, 0), (5, 0, 0)))
face  = turn_face(op1, tool: t1, to: 0, step: 1)
rough = turn_rough(op1, tool: t1, leave: 0.3, step: 2, to: -56)
thr   = turn_thread(op1, tool: t1, d: 20, pitch: 1.5, from: 3, to: -30, passes: 8)
cut   = part_off(op1, tool: t1, at: -55)
prog  = lathe_program(op1, post: \"fanuc-lathe\", number: 1)
" = true := by decide +kernel

/-- An assembly's optional header line, then a statement. -/
theorem assembly_header_accepted : accepts "assembly
a = part(\"a.fg\")
" = true := by decide +kernel

/-- The header must be a statement of its own: `machining f = part(..)` on one line is refused. -/
theorem header_glued_refused : accepts "machining f = part(\"a.fg\")
" = false := by decide +kernel

end ForgeLang.LL1.Documents
