/-!
# Reads: only files of the loader's own kind, and no read byte in any output

David's rule (unified-doc-3, replacing the folder fence): there is no folder fence; each loader
(`part`, `subassembly`, `use`, `import`, a font, a drawing, Bring Into This File) accepts only
its own kind of file, validated by parsing it, else the typed refusal `WrongKind`; and no byte of
any file read appears in any output (errors, JSON, logs, agent replies, exports). A canary
planted in a file that is not CAD must never surface.

The model: a loader is a parser `parse k : Bytes → Option Parsed`; links are followed (cycles
are `missing`); a read gives the parsed value or a refusal that holds the path and the kind but
no bytes. A document is any list of reads; its output is any function of their results.

* `read_ok_parses`: a read succeeds only on a file that parses as the loader's kind, to what the
  parser made of it.
* `output_through_parse`: outputs depend on the files only through what their parsers return:
  two file systems whose read files parse alike give the same output (non-interference).
* `canary`: so the bytes of a file that is not of its loader's kind (a canary in a `.txt`, a
  `.fg` that does not parse, a font that is no font) never change any output: any other bytes
  that also fail to parse give the same output, whatever the output function.
* `cycle_missing`: a cycle of links reads as missing, at any budget.

What is not modelled: a valid CAD file's own text (names, string literals) does reach outputs, as
it must; the rule is about files that are not of the loader's kind.
-/

namespace ForgeLang.Fence

abbrev Path := List String
abbrev Bytes := List Nat

inductive Node where
  | file (bytes : Bytes)
  | link (target : Path)
  | dir

abbrev FS := Path → Option Node

/-- Follow links from `p` (each costs a step of `fuel`); `none` for a cycle or a dangling link. -/
def follow (fs : FS) : Nat → Path → Option Bytes
  | 0, _ => none
  | n + 1, p =>
    match fs p with
    | some (.file b) => some b
    | some (.link t) => follow fs n t
    | _ => none

/-- A refusal says which file and which loader, never what is in the file. -/
inductive Refusal where
  | missing (p : Path)
  | wrongKind (k : Nat) (p : Path)
  deriving DecidableEq

section
variable {Parsed : Type} (parse : Nat → Bytes → Option Parsed)

def read (fs : FS) (fuel : Nat) (k : Nat) (p : Path) : Except Refusal Parsed :=
  match follow fs fuel p with
  | none => .error (.missing p)
  | some b =>
    match parse k b with
    | some v => .ok v
    | none => .error (.wrongKind k p)

/-- What a read's file came to: missing, or what its loader's parser made of its bytes. -/
def seen (fs : FS) (fuel : Nat) (k : Nat) (p : Path) : Option (Option Parsed) :=
  (follow fs fuel p).map (parse k)

theorem read_ok_parses {fs fuel k p v} (h : read parse fs fuel k p = .ok v) :
    ∃ b, follow fs fuel p = some b ∧ parse k b = some v := by
  unfold read at h
  split at h
  · cases h
  · rename_i b hb
    split at h
    · rename_i v' hv; cases h; exact ⟨b, hb, hv⟩
    · cases h

theorem read_of_seen {fs fs' : FS} {fuel k p} (h : seen parse fs fuel k p = seen parse fs' fuel k p) :
    read parse fs fuel k p = read parse fs' fuel k p := by
  unfold seen at h
  unfold read
  cases h1 : follow fs fuel p <;> cases h2 : follow fs' fuel p <;> simp_all

/-- Non-interference: a document reads `reads` (loader kind and path each) and outputs anything
computed from the results. Two file systems on which every read file parses alike give the same
output. -/
theorem output_through_parse {Out : Type} (out : List (Except Refusal Parsed) → Out)
    (reads : List (Nat × Path)) {fs fs' : FS} {fuel}
    (h : ∀ r ∈ reads, seen parse fs fuel r.1 r.2 = seen parse fs' fuel r.1 r.2) :
    out (reads.map fun r => read parse fs fuel r.1 r.2) = out (reads.map fun r => read parse fs' fuel r.1 r.2) := by
  congr 1
  apply List.map_congr_left
  intro r hr
  exact read_of_seen parse (h r hr)

/-- The canary: a file at `q` that its loaders reject. Any other bytes that they reject too give
the same output, for every document and every output function. -/
theorem canary {Out : Type} (out : List (Except Refusal Parsed) → Out) (reads : List (Nat × Path))
    (fs : FS) (q : Path) (b b' : Bytes) {fuel}
    (hb : ∀ r ∈ reads, parse r.1 b = none) (hb' : ∀ r ∈ reads, parse r.1 b' = none) :
    let put (x : Bytes) : FS := fun p => if p = q then some (.file x) else fs p
    out (reads.map fun r => read parse (put b) fuel r.1 r.2) =
      out (reads.map fun r => read parse (put b') fuel r.1 r.2) := by
  intro put
  apply output_through_parse parse out reads
  intro r hr
  -- the two systems differ only at `q`, whose bytes neither parse as `r`'s kind
  unfold seen
  have hf : ∀ n p, (follow (put b) n p).map (parse r.1) = (follow (put b') n p).map (parse r.1) := by
    intro n
    induction n with
    | zero => intro p; rfl
    | succ n ih =>
      intro p
      by_cases e : p = q
      · subst e; simp [follow, put, hb r hr, hb' r hr]
      · have h1 : put b p = fs p := by simp [put, e]
        have h2 : put b' p = fs p := by simp [put, e]
        simp only [follow, h1, h2]
        split
        · rfl
        · exact ih _
        · rfl
  exact hf fuel r.2

end

/-- A refusal that says where the file stopped parsing (unified-doc-3's `NotForge {line}`) is a
function of the file's bytes: two rejected files differing only in leading lines that parse
(comments, say) are told apart, so the bytes of a rejected file do reach the output (a little:
the line number). `canary` holds only for a refusal that carries no such number. -/
def stopLine (b : Bytes) : Nat := (b.takeWhile (· = 35)).length + 1

theorem line_leaks :
    let out (r : Option Nat) := r
    let refuse (b : Bytes) : Option Nat := some (stopLine b)
    out (refuse [35, 35, 7]) ≠ out (refuse [7]) := by decide

/-- Two links naming each other: missing, at any budget. -/
def loopFS : FS := fun p =>
  if p = ["a"] then some (.link ["b"]) else if p = ["b"] then some (.link ["a"]) else none

theorem cycle_missing : ∀ n, follow loopFS n ["a"] = none ∧ follow loopFS n ["b"] = none := by
  intro n
  induction n with
  | zero => exact ⟨rfl, rfl⟩
  | succ n ih => exact ⟨by simpa [follow, loopFS] using ih.2, by simpa [follow, loopFS] using ih.1⟩

end ForgeLang.Fence
