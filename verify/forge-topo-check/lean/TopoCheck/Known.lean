import TopoCheck.Model

/-!
# T.all: `check_known`'s combinatorial part is `Body.check` (ruling (b): no Rust change)

check.rs `check_known` runs the combinatorial checks with geometric ones between them:

```
check_links;  if out nonempty: return out
check_edges; check_loops; check_geometry; stray_edges
if out empty: check_sides
if out empty and more than one shell: check_shells
check_names; check_euler
if no EdgeUses or LoopBroken in out: check_shell_euler; check_fans
check_shell_faces; check_strays
```

Each check appends to one `Vec<Problem>`. `known` is that sequence over the outputs of each call,
given as lists of `RP G`: a problem the model decides (`.topo p`) or one it does not (`.geo g`,
`G` opaque). The topological calls are the translated functions, each proven to append exactly
its model part through the sink's model view (`links_refines`, `edges_refines`, `loops_refines`,
`shell_euler_refines`, `fans_refines_base`, `shell_faces_refines`, `strays_refines`; their `P`
is `topoOf`, which drops what the model does not decide, such as `EdgeDegenerate`).

**`known_topo`**: whatever the geometric calls emit, provided they emit nothing the model decides,
the model-decided problems of `known` are exactly `Body.check corners`, in order.

**What stays opaque** (not covered here): the geometric calls' outputs (`check_geometry`,
`stray_edges`, `check_sides`, `check_shells`, `check_names`, `check_euler`, and `check_edges`'s
`EdgeDegenerate`), and the geometric half of the `out.is_empty()` gates. Those gates decide only
whether `check_sides` and `check_shells` run, and those emit only geometric problems, so the
combinatorial output does not depend on them. Two facts are read off check.rs by inspection, not
proven against a translation of `check_known` itself: the call order above, and that only the
topological calls push the problems the model decides (in particular `EdgeUses` and
`LoopBroken`, the `blocks` gate's).
-/

namespace TopoCheck

/-- A problem `check_known` pushes: one the model decides, or a geometric one (opaque). -/
inductive RP (G : Type) where
  | topo (p : Problem)
  | geo (g : G)

variable {G : Type}

/-- The problems the model decides, in order. -/
def topoOf (l : List (RP G)) : List Problem :=
  l.filterMap fun | .topo p => some p | .geo _ => none

/-- check.rs's gate: an `EdgeUses` or a `LoopBroken`. -/
def blocksR : RP G → Bool
  | .topo p => Body.blocks p
  | .geo _ => false

/-- Geometric only: nothing the model decides. -/
def GeoOnly (l : List (RP G)) : Prop := ∀ x ∈ l, ∃ g, x = .geo g

/-- `if out.is_empty() { check_sides }`. -/
def whenEmpty (o x : List (RP G)) : List (RP G) := if o.isEmpty then o ++ x else o

/-- `if out.is_empty() && shells > 1 { check_shells }`. -/
def whenEmptyMany (ns : Nat) (o x : List (RP G)) : List (RP G) :=
  if o.isEmpty && decide (ns > 1) then o ++ x else o

/-- `if no EdgeUses or LoopBroken { check_shell_euler; check_fans }`. -/
def unlessBlocked (o x y : List (RP G)) : List (RP G) :=
  if !(o.any blocksR) then o ++ x ++ y else o

/-- `check_known`, over each call's output. `ns` is the number of shells. -/
def known (ns : Nat) (links edges loops geometry strayEdges sides shells names euler shellEuler fans
    shellFaces strays : List (RP G)) : List (RP G) :=
  if !links.isEmpty then links
  else
    unlessBlocked (whenEmptyMany ns (whenEmpty (links ++ edges ++ loops ++ geometry ++ strayEdges)
      sides) shells ++ names ++ euler) shellEuler fans ++ shellFaces ++ strays

theorem topoOf_append (a b : List (RP G)) : topoOf (a ++ b) = topoOf a ++ topoOf b := by
  simp [topoOf, List.filterMap_append]

theorem topoOf_geo {l : List (RP G)} (h : GeoOnly l) : topoOf l = [] := by
  induction l with
  | nil => rfl
  | cons x xs ih =>
    obtain ⟨g, rfl⟩ := h x (List.mem_cons_self ..)
    simp only [topoOf, List.filterMap_cons] at ih ⊢
    exact ih fun y hy => h y (List.mem_cons_of_mem _ hy)

theorem any_blocksR (l : List (RP G)) : l.any blocksR = (topoOf l).any Body.blocks := by
  induction l with
  | nil => rfl
  | cons x xs ih =>
    cases x <;> simp [topoOf, List.filterMap_cons, blocksR, List.any_cons] at ih ⊢ <;> rw [ih]

/-- The links' output is all the model's, so it is empty exactly when the model's is. -/
theorem isEmpty_of_topo {l : List (RP G)} {m : List Problem} (h : topoOf l = m)
    (hall : ∀ x ∈ l, ∃ p, x = .topo p) : l.isEmpty = m.isEmpty := by
  cases l with
  | nil => subst h; rfl
  | cons x xs =>
    obtain ⟨p, rfl⟩ := hall x (List.mem_cons_self ..)
    simp only [topoOf, List.filterMap_cons] at h
    subst h; rfl

/-- **T.all.** Whatever the geometric calls emit (nothing the model decides), the problems of
`check_known` that the model decides are `Body.check corners`, in its order. -/
theorem known_topo (b : Body) (corners : Nat → Nat → List (Nat × Nat)) (ns : Nat)
    {links edges loops geometry strayEdges sides shells names euler shellEuler fans shellFaces
      strays : List (RP G)}
    (hlinks : topoOf links = b.checkLinks) (hlinksT : ∀ x ∈ links, ∃ p, x = .topo p)
    (hedges : topoOf edges = b.checkEdges) (hloops : topoOf loops = b.checkLoops)
    (hse : topoOf shellEuler = b.checkShellEuler) (hfans : topoOf fans = b.checkFans corners)
    (hsf : topoOf shellFaces = b.checkShellFaces) (hst : topoOf strays = b.checkStrays)
    (hg1 : GeoOnly geometry) (hg2 : GeoOnly strayEdges) (hg3 : GeoOnly sides)
    (hg4 : GeoOnly shells) (hg5 : GeoOnly names) (hg6 : GeoOnly euler) :
    topoOf (known ns links edges loops geometry strayEdges sides shells names euler shellEuler fans
      shellFaces strays) = b.check corners := by
  have hE := isEmpty_of_topo hlinks hlinksT
  unfold known Body.check
  by_cases hl : b.checkLinks.isEmpty
  · have hl' : links.isEmpty = true := by rw [hE]; exact hl
    have hnil : links = [] := List.isEmpty_iff.mp hl'
    simp only [hl, hl', Bool.not_true, Bool.false_eq_true, ↓reduceIte]
    have h1 : topoOf (links ++ edges ++ loops ++ geometry ++ strayEdges) = (b.checkEdges ++ b.checkLoops) := by
      rw [hnil]; simp only [List.nil_append, topoOf_append, hedges, hloops, topoOf_geo hg1,
        topoOf_geo hg2, List.append_nil]
    have h2 : ∀ o : List (RP G), topoOf o = (b.checkEdges ++ b.checkLoops) → topoOf (whenEmpty o sides) = (b.checkEdges ++ b.checkLoops) := by
      intro o h; unfold whenEmpty; split
      · rw [topoOf_append, h, topoOf_geo hg3, List.append_nil]
      · exact h
    have h3 : ∀ o : List (RP G), topoOf o = (b.checkEdges ++ b.checkLoops) → topoOf (whenEmptyMany ns o shells) = (b.checkEdges ++ b.checkLoops) := by
      intro o h; unfold whenEmptyMany; split
      · rw [topoOf_append, h, topoOf_geo hg4, List.append_nil]
      · exact h
    have h4 : ∀ o : List (RP G), topoOf o = (b.checkEdges ++ b.checkLoops) → topoOf (o ++ names ++ euler) = (b.checkEdges ++ b.checkLoops) := by
      intro o h
      rw [topoOf_append, topoOf_append, h, topoOf_geo hg5, topoOf_geo hg6]; simp
    have H := h4 _ (h3 _ (h2 _ h1))
    generalize whenEmptyMany ns (whenEmpty (links ++ edges ++ loops ++ geometry ++ strayEdges)
      sides) shells ++ names ++ euler = o4 at H ⊢
    have hany : o4.any blocksR = (b.checkEdges ++ b.checkLoops).any Body.blocks := by rw [any_blocksR, H]
    unfold unlessBlocked
    rw [hany]
    by_cases hb : (b.checkEdges ++ b.checkLoops).any Body.blocks = true
    · simp only [hb, Bool.not_true, Bool.false_eq_true, ↓reduceIte]
      rw [topoOf_append, topoOf_append, H, hsf, hst]
    · simp only [Bool.not_eq_true] at hb
      simp only [hb, Bool.not_false, ↓reduceIte]
      rw [topoOf_append, topoOf_append, topoOf_append, topoOf_append, H, hse, hfans, hsf, hst]
      simp
  · have hl' : links.isEmpty = false := by rw [hE]; simpa using hl
    simp only [hl, hl', Bool.not_false, ↓reduceIte]
    exact hlinks

end TopoCheck
