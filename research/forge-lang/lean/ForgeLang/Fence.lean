/-!
# The read fence: a file is read only when its real path is inside the project

A model of forge-lang's reads (`part`, `subassembly`, `use`, `import`, fonts, drawings, Bring Into
This File) as unified-doc-3 fences them: the path is resolved first, every symbolic link on the way
followed and `..` taken on the real path reached so far (as `canonicalize` does), and the file is
read only if the resolved path lies under the root (the project: the nearest folder holding `.git`,
else the document's own; a session's root when given).

* `read_inside`: whatever is read is the content of a file whose resolved path is under the root,
  however the path was written (`..`, absolute, through links, links to links).
* `outside_same`: a path resolving outside is refused the same whether or not a file is there (the
  refusal tells nothing about what lies outside).
* `link_out_refused`: a link inside the root to a file outside is refused.
* `root_contains_doc`: the project root found by walking up is an ancestor of the document's folder.
* `cycle_missing`: a cycle of links resolves to nothing (refused as missing), at any budget.
-/

namespace ForgeLang.Fence

abbrev Path := List String

inductive Node where
  | file (content : Nat)
  | link (target : Path) (absolute : Bool)
  | dir
  deriving DecidableEq

abbrev FS := Path → Option Node

/-- Resolve `rel` from the real folder `cur`, following links (each costs a step of `fuel`). -/
def resolve (fs : FS) : Nat → Path → List String → Option Path
  | 0, _, _ => none
  | _ + 1, cur, [] => some cur
  | n + 1, cur, c :: rest =>
    if c = "." then resolve fs n cur rest
    else if c = ".." then resolve fs n cur.dropLast rest
    else
      match fs (cur ++ [c]) with
      | some (.link t abs) => resolve fs n (if abs then [] else cur) (t ++ rest)
      | _ => resolve fs n (cur ++ [c]) rest

inductive Err where
  | missing | outside
  deriving DecidableEq

/-- Read `rel` from the document's folder `base` under `root`. -/
def read (fs : FS) (fuel : Nat) (root base : Path) (rel : List String) (abs : Bool) : Except Err Nat :=
  match resolve fs fuel (if abs then [] else base) rel with
  | none => .error .missing
  | some p =>
    if root.isPrefixOf p then
      match fs p with
      | some (.file c) => .ok c
      | _ => .error .missing
    else .error .outside

theorem read_inside {fs fuel root base rel abs c} (h : read fs fuel root base rel abs = .ok c) :
    ∃ p, resolve fs fuel (if abs then [] else base) rel = some p ∧ root.isPrefixOf p = true ∧
      fs p = some (.file c) := by
  unfold read at h
  split at h
  · cases h
  · rename_i p hp
    split at h
    · rename_i hr
      split at h
      · rename_i c' hf; cases h; exact ⟨p, hp, hr, hf⟩
      · cases h
    · cases h

/-- Outside is refused alike whether a file is there or not: change what is at the resolved path
(not on the way to it) and the answer is the same. -/
theorem outside_same {fs fs' : FS} {fuel root base rel abs p}
    (hp : resolve fs fuel (if abs then [] else base) rel = some p)
    (hp' : resolve fs' fuel (if abs then [] else base) rel = some p) (ho : root.isPrefixOf p = false) :
    read fs fuel root base rel abs = .error .outside ∧ read fs' fuel root base rel abs = .error .outside := by
  constructor <;> simp [read, hp, hp', ho]

/-- A link inside the root naming a file outside is refused. -/
theorem link_out_refused {fs : FS} {fuel root base name t p} (h1 : name ≠ ".") (h2 : name ≠ "..")
    (hl : fs (base ++ [name]) = some (.link t true)) (hr : resolve fs fuel [] t = some p)
    (ho : root.isPrefixOf p = false) :
    read fs (fuel + 1) root base [name] false = .error .outside := by
  have : resolve fs (fuel + 1) base [name] = some p := by
    simp only [resolve, h1, h2, ite_false, hl, ite_true, List.append_nil]
    exact hr
  simp [read, this, ho]

/-- The nearest ancestor of `dir` holding `.git`, else `dir` itself. -/
def projectRoot (fs : FS) (dir : Path) : Path :=
  match (List.range (dir.length + 1)).reverse.find? (fun k => (fs (dir.take k ++ [".git"])).isSome) with
  | some k => dir.take k
  | none => dir

theorem root_contains_doc (fs : FS) (dir : Path) : (projectRoot fs dir).isPrefixOf dir = true := by
  unfold projectRoot
  split
  · rename_i k _
    exact List.isPrefixOf_iff_prefix.mpr (List.take_prefix k dir)
  · exact List.isPrefixOf_iff_prefix.mpr (List.prefix_refl dir)

/-- Two links naming each other: nothing, at any budget. -/
def loopFS : FS := fun p =>
  if p = ["w", "a"] then some (.link ["w", "b"] true)
  else if p = ["w", "b"] then some (.link ["w", "a"] true)
  else none

theorem cycle_missing : ∀ n, resolve loopFS n ["w"] ["a"] = none := by
  -- the walk alternates between the two links, each step costing fuel
  have step : ∀ n, resolve loopFS n [] ["w", "a"] = none ∧ resolve loopFS n [] ["w", "b"] = none := by
    intro n
    induction n using Nat.strongRecOn with
    | _ n ih =>
      match n with
      | 0 => exact ⟨rfl, rfl⟩
      | 1 => exact ⟨rfl, rfl⟩
      | k + 2 =>
        have h := ih k (by omega)
        constructor
        · show resolve loopFS (k + 2) [] ["w", "a"] = none
          simp only [resolve, loopFS]
          simpa using h.2
        · show resolve loopFS (k + 2) [] ["w", "b"] = none
          simp only [resolve, loopFS]
          simpa using h.1
  intro n
  match n with
  | 0 => rfl
  | k + 1 =>
    simp only [resolve, loopFS]
    simpa using (step k).2

/-- The harness's tree (impl-proof-findings, unified-doc-3): project `P` (holding `.git`), the
document in `P/w`, `P/out/in` inside, `X/far` outside, `P/w/lfar -> ../../X/far`,
`P/w/lin -> ../out/in`, `P/out/back -> ../w`. -/
def tree : FS := fun p =>
  if p = ["P", ".git"] then some .dir
  else if p = ["P", "out", "in"] then some (.file 1)
  else if p = ["X", "far"] then some (.file 2)
  else if p = ["P", "w", "near"] then some (.file 3)
  else if p = ["P", "w", "lfar"] then some (.link ["..", "..", "X", "far"] false)
  else if p = ["P", "w", "lin"] then some (.link ["..", "out", "in"] false)
  else if p = ["P", "out", "back"] then some (.link ["..", "w"] false)
  else none

theorem tree_reads :
    projectRoot tree ["P", "w"] = ["P"] ∧
    read tree 20 ["P"] ["P", "w"] ["..", "out", "in"] false = .ok 1 ∧
    read tree 20 ["P"] ["P", "w"] ["..", "..", "X", "far"] false = .error .outside ∧
    read tree 20 ["P"] ["P", "w"] ["lfar"] false = .error .outside ∧
    read tree 20 ["P"] ["P", "w"] ["lin"] false = .ok 1 ∧
    read tree 20 ["P"] ["P", "w"] ["..", "out", "back", "near"] false = .ok 3 ∧
    read tree 20 ["P"] ["P", "w"] ["..", "..", "X", "missing"] false = .error .outside ∧
    read tree 20 ["P", "w"] ["P", "w"] ["lin"] false = .error .outside :=
  ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

end ForgeLang.Fence
