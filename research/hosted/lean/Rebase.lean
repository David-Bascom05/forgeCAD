import ForgeLang.RenameDefs
import ForgeLang.Ren

set_option autoImplicit false

/-!
# Rebase through a Rename, in the ForgeLang core model (research/forge-lang/lean)

The hosted protocol (DESIGN.md §4.2, `interface/forge-sync/src/rebase.rs`) has one transform:
an op made before a committed `Rename {from: a, to: b}` has its names substituted `a` to `b`
(`Op.rn`), then is re-validated on the head. Every other op is re-applied as it is. This file
proves, against the core model, that the substitution is the right one: the rebased op on the
renamed document is the original op's result, renamed.

The model of the protocol's ops (`Op`, `apply`) is on top-level statements, addressed by the name
the statement binds (`Stmt.binder`), as the Rust op addresses a feature by its id:

* `setParam x k v`: the `k`-th literal site of the statement `x` binds (`codeS`) set to `v`. In
  the language a param is a path (statement, argument, list index); the core's sites are abstract
  labels, so the path is modelled as (statement name, position among its literals). This stands
  for both kinds of Rust `ParamKey`: a feature's param (owner renamed) and a shared param (its own
  name renamed); in the core both are the name of a top-level statement.
* `add before s`, `delete x`, `move x before`, `replace x s`: insert `s` before the statement
  `before` binds (at the end for `none`), remove `x`'s statement, move it before `before`,
  replace it by `s`. `replace` is also the model of SetArg (a statement's text changed).
* `rename f t`: the model's Rename (`rnB`/`rnDefs`, `RenameDefs.lean`: every binding and read of
  `f`, definitions included), accepted when `f` is bound at top level and `t` is not.

`apply` refuses (`none`) when an addressed name binds nothing; it does not evaluate. Whether the
result evaluates is the model's other theorems' business (`rename_defs`, `Struct.lean`,
`Complete.lean`); `rebased_evaluates` composes the two for the rename.

Proven here:

* `rebase_rename`: the Rename transform lemma, for every op kind. If the rename `a → b` is
  accepted on `d` and the op is accepted on `d` (it was made there), the transformed op
  (`Op.rn a b op = some op'`) is accepted on the renamed document, and its result is the op's
  result renamed: from `apply d op = some d'`, `apply (d.rename a b) op' = some (d'.rename a b)`.
  One side condition, for a Rename op only: its target is not `b` (two renames to one name; see
  below). The equality is of documents: bodies, definitions and literal tables (a rename keeps
  every site, `codeS_mS`, so a SetParam reaches the same literal).
  Per op kind, in the stronger form that also covers refused ops (`apply` of the rebased op is
  `(apply d op).map (rename a b)`, provided the op's own names are not `b`): `setParam_rn`,
  `add_rn`, `delete_rn`, `move_rn`, `replace_rn`. The proviso matters: an op naming `b`, made
  where `b` was free, is refused on its base but would reach `a`'s renamed statement on the head.
  The protocol never sends such an op (the client applied it on its base first), which is why
  `rebase_rename` assumes the op was accepted there.
* `rename_race_differ`: two Renames of one name `a` (to `b`, to `c`) do not commute: the two
  server orders give different documents. `rename_absorbed`: after `a → b` a rename of `a`
  changes nothing, and `rename_after_rename_refused`: re-validated on the head it is refused
  (`a` binds nothing). Together they justify refusing the later one (`RenameConflict`).
* `rename_same_target_refused`: two Renames of different names to one target `b`: the later is
  refused on the head (`b` is taken). Rust refuses it by re-validation (`NameTaken`).
* `rename_comm`: renames of different names commute (`a → b`, `c → e` with `a ≠ c`, `b ≠ c`,
  `e ≠ a`; in particular when all four are distinct).
* `converge`, `converge_chunks`: protocol convergence (S): a client that applies the server's
  total order with the same deterministic `apply`, from any snapshot and in any chunking, ends at
  the server's document. By construction: `apply` is a function. Stated for any state type and
  any `apply`, so it covers the model's `apply` and the Rust one alike.
* `client_replay_rename`: the one-step client replay case: a client that applied its op
  optimistically on `d` and then receives the server's (rename, rebased op) ends where the
  server is, by renaming its own optimistic result.

What the model cannot express:

* Nested ids (`bolt[2].hub`): ops here address top-level statements only. Rust's `id` renames the
  first segment of a path, which is the top-level name, so the transform is the same; the model's
  `rnModel`/`renFirst` (`RenamePart.lean`) says ids change only in that first segment.
* Names inside statement texts are expressible: an added or replacing statement is a `Stmt`, and
  `rnS` is the model of forge-lang's rename on it. Rust's fallback for text that does not lex
  (left as it is, refused by validation) has no counterpart: a `Stmt` always parses.
* LiftLiteral and Respell address byte spans, which the core does not have; Rust refuses them
  when stale (`StaleSpan`). Suppress, AddLocalParam, MakeShared, SplitInstance and SetProperty
  are not ops here.
* Validity of the result (it evaluates) is not part of `apply`; it is re-checked on the head, as
  in Rust (`apply_checked`).

Compile from research/forge-lang/lean: `lake env lean ../../hosted/lean/Rebase.lean`.
-/

namespace ForgeLang
namespace Rebase

/-! ## Renaming as a map of names -/

mutual
/-- `rnE` with any map of names in place of `rn a b`. -/
def mE (f : Name → Name) : Expr → Expr
  | .lit s => .lit s
  | .cst c => .cst c
  | .var x => .var (f x)
  | .add u v => .add (mE f u) (mE f v)
  | .mul u v => .mul (mE f u) (mE f v)
  | .bin o u v => .bin o (mE f u) (mE f v)
  | .eq u v => .eq (mE f u) (mE f v)
  | .lt u v => .lt (mE f u) (mE f v)
  | .ite c u v => .ite (mE f c) (mE f u) (mE f v)
  | .sel es k => .sel (mEs f es) (mE f k)
  | .call g u => .call g (mE f u)
  | .ref x rel => .ref (f x) rel
  | .refIx x k rel => .refIx (f x) (mE f k) rel
def mEs (f : Name → Name) : List Expr → List Expr
  | [] => []
  | e :: es => mE f e :: mEs f es
end

mutual
def mS (f : Name → Name) : Stmt → Stmt
  | .let_ x e => .let_ (f x) (mE f e)
  | .set x e => .set (f x) (mE f e)
  | .feat x e => .feat (f x) (mE f e)
  | .forN x i c body => .forN (f x) i (mE f c) (mB f body)
  | .forK x i kvs body => .forK (f x) i (mKV f kvs) (mB f body)
  | .ifs c u v => .ifs (mE f c) (mB f u) (mB f v)
  | .use x p e => .use (f x) p (mE f e)
  | .occ x c args => .occ (f x) c (mKV f args)
def mB (f : Name → Name) : List Stmt → List Stmt
  | [] => []
  | s :: ss => mS f s :: mB f ss
def mKV (f : Name → Name) : List (Name × Expr) → List (Name × Expr)
  | [] => []
  | (k, e) :: kvs => (k, mE f e) :: mKV f kvs
end

mutual
theorem rnE_eq (a b : Name) : ∀ e : Expr, rnE a b e = mE (rn a b) e
  | .lit _ => by simp [rnE, mE]
  | .cst _ => by simp [rnE, mE]
  | .var _ => by simp [rnE, mE]
  | .add u v => by simp [rnE, mE, rnE_eq a b u, rnE_eq a b v]
  | .mul u v => by simp [rnE, mE, rnE_eq a b u, rnE_eq a b v]
  | .bin _ u v => by simp [rnE, mE, rnE_eq a b u, rnE_eq a b v]
  | .eq u v => by simp [rnE, mE, rnE_eq a b u, rnE_eq a b v]
  | .lt u v => by simp [rnE, mE, rnE_eq a b u, rnE_eq a b v]
  | .ite c u v => by simp [rnE, mE, rnE_eq a b c, rnE_eq a b u, rnE_eq a b v]
  | .sel es k => by simp [rnE, mE, rnEs_eq a b es, rnE_eq a b k]
  | .call _ u => by simp [rnE, mE, rnE_eq a b u]
  | .ref _ _ => by simp [rnE, mE]
  | .refIx _ k _ => by simp [rnE, mE, rnE_eq a b k]
theorem rnEs_eq (a b : Name) : ∀ es : List Expr, rnEs a b es = mEs (rn a b) es
  | [] => by simp [rnEs, mEs]
  | e :: es => by simp [rnEs, mEs, rnE_eq a b e, rnEs_eq a b es]
end

mutual
theorem rnS_eq (a b : Name) : ∀ s : Stmt, rnS a b s = mS (rn a b) s
  | .let_ _ e => by simp [rnS, mS, rnE_eq]
  | .set _ e => by simp [rnS, mS, rnE_eq]
  | .feat _ e => by simp [rnS, mS, rnE_eq]
  | .forN _ _ c body => by simp [rnS, mS, rnE_eq, rnB_eq a b body]
  | .forK _ _ kvs body => by simp [rnS, mS, rnKV_eq a b kvs, rnB_eq a b body]
  | .ifs c u v => by simp [rnS, mS, rnE_eq, rnB_eq a b u, rnB_eq a b v]
  | .use _ _ e => by simp [rnS, mS, rnE_eq]
  | .occ _ _ args => by simp [rnS, mS, rnKV_eq a b args]
theorem rnB_eq (a b : Name) : ∀ ss : List Stmt, rnB a b ss = mB (rn a b) ss
  | [] => by simp [rnB, mB]
  | s :: ss => by simp [rnB, mB, rnS_eq a b s, rnB_eq a b ss]
theorem rnKV_eq (a b : Name) : ∀ kvs : List (Name × Expr), rnKV a b kvs = mKV (rn a b) kvs
  | [] => by simp [rnKV, mKV]
  | (_, e) :: kvs => by simp [rnKV, mKV, rnE_eq, rnKV_eq a b kvs]
end

mutual
theorem mE_comp (f g : Name → Name) : ∀ e : Expr, mE f (mE g e) = mE (fun z => f (g z)) e
  | .lit _ => by simp [mE]
  | .cst _ => by simp [mE]
  | .var _ => by simp [mE]
  | .add u v => by simp [mE, mE_comp f g u, mE_comp f g v]
  | .mul u v => by simp [mE, mE_comp f g u, mE_comp f g v]
  | .bin _ u v => by simp [mE, mE_comp f g u, mE_comp f g v]
  | .eq u v => by simp [mE, mE_comp f g u, mE_comp f g v]
  | .lt u v => by simp [mE, mE_comp f g u, mE_comp f g v]
  | .ite c u v => by simp [mE, mE_comp f g c, mE_comp f g u, mE_comp f g v]
  | .sel es k => by simp [mE, mEs_comp f g es, mE_comp f g k]
  | .call _ u => by simp [mE, mE_comp f g u]
  | .ref _ _ => by simp [mE]
  | .refIx _ k _ => by simp [mE, mE_comp f g k]
theorem mEs_comp (f g : Name → Name) : ∀ es : List Expr, mEs f (mEs g es) = mEs (fun z => f (g z)) es
  | [] => by simp [mEs]
  | e :: es => by simp [mEs, mE_comp f g e, mEs_comp f g es]
end

mutual
theorem mS_comp (f g : Name → Name) : ∀ s : Stmt, mS f (mS g s) = mS (fun z => f (g z)) s
  | .let_ _ e => by simp [mS, mE_comp]
  | .set _ e => by simp [mS, mE_comp]
  | .feat _ e => by simp [mS, mE_comp]
  | .forN _ _ c body => by simp [mS, mE_comp, mB_comp f g body]
  | .forK _ _ kvs body => by simp [mS, mKV_comp f g kvs, mB_comp f g body]
  | .ifs c u v => by simp [mS, mE_comp, mB_comp f g u, mB_comp f g v]
  | .use _ _ e => by simp [mS, mE_comp]
  | .occ _ _ args => by simp [mS, mKV_comp f g args]
theorem mB_comp (f g : Name → Name) : ∀ ss : List Stmt, mB f (mB g ss) = mB (fun z => f (g z)) ss
  | [] => by simp [mB]
  | s :: ss => by simp [mB, mS_comp f g s, mB_comp f g ss]
theorem mKV_comp (f g : Name → Name) :
    ∀ kvs : List (Name × Expr), mKV f (mKV g kvs) = mKV (fun z => f (g z)) kvs
  | [] => by simp [mKV]
  | (_, e) :: kvs => by simp [mKV, mE_comp, mKV_comp f g kvs]
end

mutual
theorem codeE_mE (f : Name → Name) : ∀ e : Expr, codeE (mE f e) = codeE e
  | .lit _ => by simp [mE, codeE]
  | .cst _ => by simp [mE, codeE]
  | .var _ => by simp [mE, codeE]
  | .add u v => by simp [mE, codeE, codeE_mE f u, codeE_mE f v]
  | .mul u v => by simp [mE, codeE, codeE_mE f u, codeE_mE f v]
  | .bin _ u v => by simp [mE, codeE, codeE_mE f u, codeE_mE f v]
  | .eq u v => by simp [mE, codeE, codeE_mE f u, codeE_mE f v]
  | .lt u v => by simp [mE, codeE, codeE_mE f u, codeE_mE f v]
  | .ite c u v => by simp [mE, codeE, codeE_mE f c, codeE_mE f u, codeE_mE f v]
  | .sel es k => by simp [mE, codeE, codeEs_mEs f es, codeE_mE f k]
  | .call _ u => by simp [mE, codeE, codeE_mE f u]
  | .ref _ _ => by simp [mE, codeE]
  | .refIx _ k _ => by simp [mE, codeE, codeE_mE f k]
theorem codeEs_mEs (f : Name → Name) : ∀ es : List Expr, codeEs (mEs f es) = codeEs es
  | [] => by simp [mEs, codeEs]
  | e :: es => by simp [mEs, codeEs, codeE_mE f e, codeEs_mEs f es]
end

mutual
theorem codeS_mS (f : Name → Name) : ∀ s : Stmt, codeS (mS f s) = codeS s
  | .let_ _ e => by simp [mS, codeS, codeE_mE]
  | .set _ e => by simp [mS, codeS, codeE_mE]
  | .feat _ e => by simp [mS, codeS, codeE_mE]
  | .forN _ _ c body => by simp [mS, codeS, codeE_mE, codeB_mB f body]
  | .forK _ _ kvs body => by simp [mS, codeS, codeKV_mKV f kvs, codeB_mB f body]
  | .ifs c u v => by simp [mS, codeS, codeE_mE, codeB_mB f u, codeB_mB f v]
  | .use _ _ e => by simp [mS, codeS, codeE_mE]
  | .occ _ _ args => by simp [mS, codeS, codeKV_mKV f args]
theorem codeB_mB (f : Name → Name) : ∀ ss : List Stmt, codeB (mB f ss) = codeB ss
  | [] => by simp [mB, codeB]
  | s :: ss => by simp [mB, codeB, codeS_mS f s, codeB_mB f ss]
theorem codeKV_mKV (f : Name → Name) : ∀ kvs : List (Name × Expr), codeKV (mKV f kvs) = codeKV kvs
  | [] => by simp [mKV, codeKV]
  | (_, e) :: kvs => by simp [mKV, codeKV, codeE_mE, codeKV_mKV f kvs]
end

theorem mB_map (f : Name → Name) : ∀ ss : List Stmt, mB f ss = ss.map (mS f)
  | [] => by simp [mB]
  | s :: ss => by simp [mB, mB_map f ss]

theorem binder_mS (f : Name → Name) (s : Stmt) : (mS f s).binder = s.binder.map f := by
  cases s <;> rfl

theorem binders_mB (f : Name → Name) : ∀ ss : List Stmt, binders (mB f ss) = (binders ss).map f
  | [] => by simp [mB, binders]
  | s :: ss => by
    have ih := binders_mB f ss
    simp only [binders] at ih ⊢
    simp only [mB, List.filterMap_cons, binder_mS, ih]
    cases s.binder <;> simp

/-! ## Definitions and documents -/

def mDefs (f : Name → Name) (D : Defs) : Defs :=
  { fns := D.fns.map fun q => (q.1, (q.2.1, mE f q.2.2)),
    procs := D.procs.map fun p => (p.1, (p.2.1, mB f p.2.2)),
    comps := D.comps.map fun c =>
      (c.1, { params := mKV f c.2.params, body := mB f c.2.body, vis := c.2.vis.map f }),
    track := D.track }

theorem rnDefs_eq (a b : Name) (D : Defs) : rnDefs a b D = mDefs (rn a b) D := by
  simp [rnDefs, mDefs, rnComp, rnE_eq, rnB_eq, rnKV_eq]

theorem mDefs_comp (f g : Name → Name) (D : Defs) :
    mDefs f (mDefs g D) = mDefs (fun z => f (g z)) D := by
  simp [mDefs, mE_comp, mB_comp, mKV_comp, Function.comp_def]

/-- Rename `a` to `b` in a document: its body and its definitions (`rename_defs`). The literal
table is not touched: no site changes. -/
def _root_.ForgeLang.Doc.rename (a b : Name) (d : Doc) : Doc :=
  { defs := rnDefs a b d.defs, body := rnB a b d.body, σ := d.σ }

def mDoc (f : Name → Name) (d : Doc) : Doc := { defs := mDefs f d.defs, body := mB f d.body, σ := d.σ }

theorem rename_eq (a b : Name) : Doc.rename a b = mDoc (rn a b) := by
  funext d; simp [Doc.rename, mDoc, rnDefs_eq, rnB_eq]

theorem mDoc_comp (f g : Name → Name) (d : Doc) : mDoc f (mDoc g d) = mDoc (fun z => f (g z)) d := by
  simp [mDoc, mDefs_comp, mB_comp]

theorem mDoc_congr {f g : Name → Name} (h : ∀ z, f z = g z) (d : Doc) : mDoc f d = mDoc g d := by
  rw [funext h]

/-! ## Top-level statements addressed by name -/

/-- The statement that binds `x` (the first; under single assignment, the only one but `set`s). -/
def find (x : Name) : List Stmt → Option Stmt
  | [] => none
  | s :: ss => if s.binder = some x then some s else find x ss

/-- The block without `x`'s statement. -/
def del (x : Name) : List Stmt → Option (List Stmt)
  | [] => none
  | s :: ss => if s.binder = some x then some ss else (del x ss).map (s :: ·)

/-- `x`'s statement replaced by `t`. -/
def repl (x : Name) (t : Stmt) : List Stmt → Option (List Stmt)
  | [] => none
  | s :: ss => if s.binder = some x then some (t :: ss) else (repl x t ss).map (s :: ·)

/-- `t` inserted before `y`'s statement. -/
def ins (y : Name) (t : Stmt) : List Stmt → Option (List Stmt)
  | [] => none
  | s :: ss => if s.binder = some y then some (t :: s :: ss) else (ins y t ss).map (s :: ·)

/-- `t` inserted before `before`'s statement, or at the end. -/
def insAt : Option Name → Stmt → List Stmt → Option (List Stmt)
  | none, t, ss => some (ss ++ [t])
  | some y, t, ss => ins y t ss

/-- `x`'s statement moved before `before`'s (or to the end). -/
def move (x : Name) (before : Option Name) (ss : List Stmt) : Option (List Stmt) :=
  (find x ss).bind fun t => (del x ss).bind fun r => insAt before t r

/-- `f` tells `x` apart from every other name `ss` binds. -/
def Sep (f : Name → Name) (x : Name) (ss : List Stmt) : Prop :=
  ∀ s ∈ ss, ∀ y, s.binder = some y → f y = f x → y = x

theorem Sep.tail {f : Name → Name} {x : Name} {s : Stmt} {ss : List Stmt} (h : Sep f x (s :: ss)) :
    Sep f x ss := fun t ht => h t (List.mem_cons_of_mem _ ht)

theorem Sep.mono {f : Name → Name} {x : Name} {ss r : List Stmt} (h : Sep f x ss)
    (hr : ∀ s ∈ r, s ∈ ss) : Sep f x r := fun t ht => h t (hr t ht)

theorem binder_iff {f : Name → Name} {x : Name} {s : Stmt}
    (h : ∀ y, s.binder = some y → f y = f x → y = x) :
    (mS f s).binder = some (f x) ↔ s.binder = some x := by
  rw [binder_mS]
  cases hs : s.binder with
  | none => simp
  | some y =>
    simp only [Option.map_some, Option.some.injEq]
    exact ⟨h y hs, fun e => e ▸ rfl⟩

theorem find_map {f : Name → Name} {x : Name} : ∀ {ss : List Stmt}, Sep f x ss →
    find (f x) (ss.map (mS f)) = (find x ss).map (mS f)
  | [], _ => rfl
  | s :: ss, h => by
    have hs := binder_iff (h s List.mem_cons_self)
    by_cases e : s.binder = some x
    · simp [find, e, hs.2 e]
    · simp [find, e, mt hs.1 e, find_map h.tail]

theorem del_map {f : Name → Name} {x : Name} : ∀ {ss : List Stmt}, Sep f x ss →
    del (f x) (ss.map (mS f)) = (del x ss).map (List.map (mS f))
  | [], _ => rfl
  | s :: ss, h => by
    have hs := binder_iff (h s List.mem_cons_self)
    by_cases e : s.binder = some x
    · simp [del, e, hs.2 e]
    · simp [del, e, mt hs.1 e, del_map h.tail, Option.map_map, Function.comp_def]

theorem repl_map {f : Name → Name} {x : Name} {t : Stmt} : ∀ {ss : List Stmt}, Sep f x ss →
    repl (f x) (mS f t) (ss.map (mS f)) = (repl x t ss).map (List.map (mS f))
  | [], _ => rfl
  | s :: ss, h => by
    have hs := binder_iff (h s List.mem_cons_self)
    by_cases e : s.binder = some x
    · simp [repl, e, hs.2 e]
    · simp [repl, e, mt hs.1 e, repl_map h.tail, Option.map_map, Function.comp_def]

theorem ins_map {f : Name → Name} {y : Name} {t : Stmt} : ∀ {ss : List Stmt}, Sep f y ss →
    ins (f y) (mS f t) (ss.map (mS f)) = (ins y t ss).map (List.map (mS f))
  | [], _ => rfl
  | s :: ss, h => by
    have hs := binder_iff (h s List.mem_cons_self)
    by_cases e : s.binder = some y
    · simp [ins, e, hs.2 e]
    · simp [ins, e, mt hs.1 e, ins_map h.tail, Option.map_map, Function.comp_def]

theorem insAt_map {f : Name → Name} {before : Option Name} {t : Stmt} {ss : List Stmt}
    (h : ∀ y, before = some y → Sep f y ss) :
    insAt (before.map f) (mS f t) (ss.map (mS f)) = (insAt before t ss).map (List.map (mS f)) := by
  cases before with
  | none => simp [insAt]
  | some y => exact ins_map (h y rfl)

theorem del_sub {x : Name} : ∀ {ss r : List Stmt}, del x ss = some r → ∀ s ∈ r, s ∈ ss
  | [], _, h => by simp [del] at h
  | s :: ss, r, h => by
    simp only [del] at h
    split at h
    · cases h; intro t ht; exact List.mem_cons_of_mem _ ht
    · cases hd : del x ss with
      | none => simp [hd] at h
      | some r' =>
        simp only [hd, Option.map_some, Option.some.injEq] at h
        subst h
        intro t ht
        rcases List.mem_cons.1 ht with rfl | ht
        · exact List.mem_cons_self
        · exact List.mem_cons_of_mem _ (del_sub hd t ht)

theorem move_map {f : Name → Name} {x : Name} {before : Option Name} {ss : List Stmt}
    (hx : Sep f x ss) (hb : ∀ y, before = some y → Sep f y ss) :
    move (f x) (before.map f) (ss.map (mS f)) = (move x before ss).map (List.map (mS f)) := by
  unfold move
  rw [find_map hx, del_map hx]
  cases find x ss with
  | none => rfl
  | some t =>
    cases hd : del x ss with
    | none => rfl
    | some r =>
      simp only [Option.map_some, Option.bind_some]
      exact insAt_map fun y hy => (hb y hy).mono (del_sub hd)

/-! ### What an accepted op names is bound -/

theorem find_some {x : Name} : ∀ {ss : List Stmt} {t : Stmt}, find x ss = some t →
    ∃ s ∈ ss, s.binder = some x
  | [], _, h => by simp [find] at h
  | s :: ss, t, h => by
    simp only [find] at h
    split at h
    · exact ⟨s, List.mem_cons_self, by assumption⟩
    · obtain ⟨u, hu, e⟩ := find_some h; exact ⟨u, List.mem_cons_of_mem _ hu, e⟩

theorem del_some {x : Name} : ∀ {ss r : List Stmt}, del x ss = some r → ∃ s ∈ ss, s.binder = some x
  | [], _, h => by simp [del] at h
  | s :: ss, r, h => by
    simp only [del] at h
    split at h
    · exact ⟨s, List.mem_cons_self, by assumption⟩
    · cases hd : del x ss with
      | none => simp [hd] at h
      | some r' => obtain ⟨u, hu, e⟩ := del_some hd; exact ⟨u, List.mem_cons_of_mem _ hu, e⟩

theorem repl_some {x : Name} {t : Stmt} : ∀ {ss r : List Stmt}, repl x t ss = some r →
    ∃ s ∈ ss, s.binder = some x
  | [], _, h => by simp [repl] at h
  | s :: ss, r, h => by
    simp only [repl] at h
    split at h
    · exact ⟨s, List.mem_cons_self, by assumption⟩
    · cases hd : repl x t ss with
      | none => simp [hd] at h
      | some r' => obtain ⟨u, hu, e⟩ := repl_some hd; exact ⟨u, List.mem_cons_of_mem _ hu, e⟩

theorem ins_some {y : Name} {t : Stmt} : ∀ {ss r : List Stmt}, ins y t ss = some r →
    ∃ s ∈ ss, s.binder = some y
  | [], _, h => by simp [ins] at h
  | s :: ss, r, h => by
    simp only [ins] at h
    split at h
    · exact ⟨s, List.mem_cons_self, by assumption⟩
    · cases hd : ins y t ss with
      | none => simp [hd] at h
      | some r' => obtain ⟨u, hu, e⟩ := ins_some hd; exact ⟨u, List.mem_cons_of_mem _ hu, e⟩

theorem mem_binders {ss : List Stmt} {x : Name} : x ∈ binders ss ↔ ∃ s ∈ ss, s.binder = some x := by
  simp [binders, List.mem_filterMap]

/-- With `b` bound nowhere at top level, `rn a b` tells every name but `b` apart. -/
theorem sep_rn {a b x : Name} {ss : List Stmt} (hb : b ∉ binders ss) (hx : x ≠ b) :
    Sep (rn a b) x ss := fun s hs y hy e =>
  rn_inj (fun hyb => hb (mem_binders.2 ⟨s, hs, by rw [← hyb]; exact hy⟩)) hx e

theorem ne_of_bound {b x : Name} {ss : List Stmt} (hb : b ∉ binders ss)
    (h : ∃ s ∈ ss, s.binder = some x) : x ≠ b := fun e => hb (e ▸ mem_binders.2 h)

/-! ## Ops -/

/-- The protocol's ops on names, as the core can state them (see the header). -/
inductive Op where
  | setParam (x : Name) (k : Nat) (v : Int)
  | add (before : Option Name) (s : Stmt)
  | delete (x : Name)
  | move (x : Name) (before : Option Name)
  | replace (x : Name) (s : Stmt)
  | rename (f t : Name)

def withBody (d : Doc) (body : List Stmt) : Doc := { d with body := body }

/-- An op applied to a document; `none` is a refusal. -/
def apply (d : Doc) : Op → Option Doc
  | .setParam x k v => ((find x d.body).bind fun s => (codeS s)[k]?).map fun p => d.setLit p v
  | .add before s => (insAt before s d.body).map (withBody d)
  | .delete x => (del x d.body).map (withBody d)
  | .move x before => (move x before d.body).map (withBody d)
  | .replace x s => (repl x s d.body).map (withBody d)
  | .rename f t => if f ∈ binders d.body ∧ t ∉ binders d.body then some (d.rename f t) else none

/-- The transform (`rebase.rs` `rename_in`): an op made before `Rename {a, b}`, its names
substituted. A Rename of the same name is refused (`RenameConflict`); any other Rename is kept. -/
def Op.rn (a b : Name) : Op → Option Op
  | .setParam x k v => some (.setParam (ForgeLang.rn a b x) k v)
  | .add before s => some (.add (before.map (ForgeLang.rn a b)) (rnS a b s))
  | .delete x => some (.delete (ForgeLang.rn a b x))
  | .move x before => some (.move (ForgeLang.rn a b x) (before.map (ForgeLang.rn a b)))
  | .replace x s => some (.replace (ForgeLang.rn a b x) (rnS a b s))
  | .rename f t => if f = a then none else some (.rename f t)

theorem withBody_m (f : Name → Name) (d : Doc) (r : List Stmt) :
    withBody (mDoc f d) (r.map (mS f)) = mDoc f (withBody d r) := by
  simp [withBody, mDoc, mB_map]

theorem binders_rename (a b : Name) (d : Doc) :
    binders (d.rename a b).body = (binders d.body).map (rn a b) := by
  simp [Doc.rename, rnB_eq, binders_mB]

theorem body_m (f : Name → Name) (d : Doc) : (mDoc f d).body = d.body.map (mS f) := by
  simp [mDoc, mB_map]

/-! ## The transform lemma, op kind by op kind

Each in the form `apply (rename a b d) (rn a b op) = (apply d op).map (rename a b)`: accepted
before, accepted after with the renamed result; refused before, refused after. Hypotheses: `b`
is bound nowhere at top level (the rename was accepted) and the names the op addresses are not
`b` (true of any op accepted on `d`). -/

theorem setParam_rn {d : Doc} {a b x : Name} (k : Nat) (v : Int) (hb : b ∉ binders d.body)
    (hx : x ≠ b) :
    apply (d.rename a b) (.setParam (rn a b x) k v) =
      (apply d (.setParam x k v)).map (Doc.rename a b) := by
  rw [rename_eq]
  simp only [apply]
  rw [body_m, find_map (sep_rn hb hx)]
  cases find x d.body with
  | none => rfl
  | some s =>
    simp only [Option.map_some, Option.bind_some, codeS_mS]
    cases (codeS s)[k]? <;> rfl

theorem add_rn {d : Doc} {a b : Name} {before : Option Name} (s : Stmt) (hb : b ∉ binders d.body)
    (hx : before ≠ some b) :
    apply (d.rename a b) (.add (before.map (rn a b)) (rnS a b s)) =
      (apply d (.add before s)).map (Doc.rename a b) := by
  rw [rename_eq, rnS_eq]
  have h := insAt_map (f := rn a b) (before := before) (t := s) (ss := d.body)
    fun y hy => sep_rn hb fun e => hx (by rw [hy, e])
  simp only [apply, Option.map_map]
  rw [body_m, h]
  cases insAt before s d.body <;> simp [withBody_m]

theorem delete_rn {d : Doc} {a b x : Name} (hb : b ∉ binders d.body) (hx : x ≠ b) :
    apply (d.rename a b) (.delete (rn a b x)) = (apply d (.delete x)).map (Doc.rename a b) := by
  rw [rename_eq]
  simp only [apply, Option.map_map]
  rw [body_m, del_map (sep_rn hb hx)]
  cases del x d.body <;> simp [withBody_m]

theorem replace_rn {d : Doc} {a b x : Name} (s : Stmt) (hb : b ∉ binders d.body) (hx : x ≠ b) :
    apply (d.rename a b) (.replace (rn a b x) (rnS a b s)) =
      (apply d (.replace x s)).map (Doc.rename a b) := by
  rw [rename_eq, rnS_eq]
  simp only [apply, Option.map_map]
  rw [body_m, repl_map (sep_rn hb hx)]
  cases repl x s d.body <;> simp [withBody_m]

theorem move_rn {d : Doc} {a b x : Name} {before : Option Name} (hb : b ∉ binders d.body)
    (hx : x ≠ b) (hy : before ≠ some b) :
    apply (d.rename a b) (.move (rn a b x) (before.map (rn a b))) =
      (apply d (.move x before)).map (Doc.rename a b) := by
  rw [rename_eq]
  simp only [apply, Option.map_map]
  rw [body_m, move_map (sep_rn hb hx) fun y e => sep_rn hb fun e' => hy (by rw [e, e'])]
  cases move x before d.body <;> simp [withBody_m]

/-! ## Renames against renames -/

/-- Renames of different names commute: `a → b` then `c → e` is `c → e` then `a → b`, when
`a ≠ c`, `b ≠ c` and `e ≠ a` (in particular when all four names are distinct). -/
theorem rename_comm (d : Doc) {a b c e : Name} (h1 : a ≠ c) (h2 : b ≠ c) (h3 : e ≠ a) :
    (d.rename a b).rename c e = (d.rename c e).rename a b := by
  simp only [rename_eq, mDoc_comp]
  apply mDoc_congr
  intro z
  unfold rn
  by_cases hza : z = a
  · subst hza; simp [h1, h2]
  · by_cases hzc : z = c
    · subst hzc; simp [Ne.symm h1, h3]
    · simp [hza, hzc]

/-- After `a → b`, a rename of `a` changes nothing: the name is gone. -/
theorem rename_absorbed (d : Doc) {a b : Name} (c : Name) (hab : a ≠ b) :
    (d.rename a b).rename a c = d.rename a b := by
  simp only [rename_eq, mDoc_comp]
  apply mDoc_congr
  intro z
  unfold rn
  by_cases hza : z = a
  · simp [hza, Ne.symm hab]
  · simp [hza]

/-- Re-validated on the head, a later rename of `a` is refused: `a` binds nothing there. -/
theorem rename_after_rename_refused (d : Doc) {a b : Name} (c : Name) (hab : a ≠ b) :
    apply (d.rename a b) (.rename a c) = none := by
  have : a ∉ binders (d.rename a b).body := by
    rw [binders_rename]
    intro h
    obtain ⟨y, -, e⟩ := List.mem_map.1 h
    unfold rn at e
    split at e
    · exact hab e.symm
    · contradiction
  simp [apply, this]

/-- Two Renames of one name `a`, to `b` and to `c`, both accepted on the same document: the two
server orders give different documents. They do not commute, so one must be refused. -/
theorem rename_race_differ {d : Doc} {a b c : Name} (hab : a ≠ b) (hac : a ≠ c) (hbc : b ≠ c)
    (ha : a ∈ binders d.body) (hc : c ∉ binders d.body) :
    (d.rename a b).rename a c ≠ (d.rename a c).rename a b := by
  rw [rename_absorbed d c hab, rename_absorbed d b hac]
  intro e
  have hb := congrArg (fun d : Doc => binders d.body) e
  simp only [binders_rename] at hb
  have hcR : c ∈ (binders d.body).map (rn a c) := List.mem_map.2 ⟨a, ha, by simp [rn]⟩
  rw [← hb] at hcR
  obtain ⟨y, hy, ey⟩ := List.mem_map.1 hcR
  unfold rn at ey
  split at ey
  · exact hbc ey
  · exact hc (ey ▸ hy)

/-- Two Renames of different names to one target `b`: once `a → b` is in, `f → b` is refused
on the head (`b` is taken), by re-validation. -/
theorem rename_same_target_refused (d : Doc) {a b : Name} (f : Name) (ha : a ∈ binders d.body) :
    apply (d.rename a b) (.rename f b) = none := by
  have : b ∈ binders (d.rename a b).body := by
    rw [binders_rename]; exact List.mem_map.2 ⟨a, ha, by simp [rn]⟩
  simp [apply, this]

/-! ## The transform lemma for every op -/

/-- **Rebase through a Rename.** The rename `a → b` and `op` were both accepted on `d` (the
client made `op` there; the server committed the rename first). The transformed op is accepted
on the head and gives the op's result, renamed. Side condition, for a Rename op only: its target
is not `b` (else it is refused on the head: `rename_same_target_refused`). -/
theorem rebase_rename {d d' : Doc} {a b : Name} {op op' : Op}
    (hr : apply d (.rename a b) = some (d.rename a b)) (h : apply d op = some d')
    (ho : op.rn a b = some op') (ht : ∀ f t, op = .rename f t → t ≠ b) :
    apply (d.rename a b) op' = some (d'.rename a b) := by
  have hab : a ∈ binders d.body ∧ b ∉ binders d.body := by
    simp only [apply] at hr; split at hr
    · assumption
    · cases hr
  obtain ⟨ha, hb⟩ := hab
  cases op with
  | setParam x k v =>
    simp only [Op.rn, Option.some.injEq] at ho; subst ho
    have hx : x ≠ b := by
      simp only [apply] at h
      cases hf : find x d.body with
      | none => simp [hf] at h
      | some s => exact ne_of_bound hb (find_some hf)
    rw [setParam_rn k v hb hx, h]; rfl
  | add before s =>
    simp only [Op.rn, Option.some.injEq] at ho; subst ho
    have hx : before ≠ some b := by
      rintro rfl
      simp only [apply, insAt] at h
      cases hi : ins b s d.body with
      | none => simp [hi] at h
      | some r => exact ne_of_bound hb (ins_some hi) rfl
    rw [add_rn s hb hx, h]; rfl
  | delete x =>
    simp only [Op.rn, Option.some.injEq] at ho; subst ho
    have hx : x ≠ b := by
      simp only [apply] at h
      cases hf : del x d.body with
      | none => simp [hf] at h
      | some s => exact ne_of_bound hb (del_some hf)
    rw [delete_rn hb hx, h]; rfl
  | replace x s =>
    simp only [Op.rn, Option.some.injEq] at ho; subst ho
    have hx : x ≠ b := by
      simp only [apply] at h
      cases hf : repl x s d.body with
      | none => simp [hf] at h
      | some s => exact ne_of_bound hb (repl_some hf)
    rw [replace_rn s hb hx, h]; rfl
  | move x before =>
    simp only [Op.rn, Option.some.injEq] at ho; subst ho
    have hxy : x ≠ b ∧ before ≠ some b := by
      simp only [apply] at h
      cases hm : move x before d.body with
      | none => simp [hm] at h
      | some r =>
        unfold move at hm
        cases hf : find x d.body with
        | none => simp [hf] at hm
        | some t =>
          cases hd : del x d.body with
          | none => simp [hf, hd] at hm
          | some r' =>
            simp only [hf, hd, Option.bind_some] at hm
            refine ⟨ne_of_bound hb (find_some hf), ?_⟩
            rintro rfl
            obtain ⟨u, hu, e⟩ := ins_some hm
            exact ne_of_bound hb ⟨u, del_sub hd u hu, e⟩ rfl
    rw [move_rn hb hxy.1 hxy.2, h]; rfl
  | rename f t =>
    simp only [Op.rn] at ho
    split at ho
    · cases ho
    · rename_i hfa
      simp only [Option.some.injEq] at ho; subst ho
      simp only [apply] at h
      split at h
      · rename_i hft
        simp only [Option.some.injEq] at h; subst h
        have htb := ht f t rfl
        have hf' : f ∈ binders (d.rename a b).body := by
          rw [binders_rename]; exact List.mem_map.2 ⟨f, hft.1, by simp [rn, hfa]⟩
        have ht' : t ∉ binders (d.rename a b).body := by
          rw [binders_rename]
          intro hm
          obtain ⟨y, hy, e⟩ := List.mem_map.1 hm
          unfold rn at e
          split at e
          · exact htb e.symm
          · exact hft.2 (e ▸ hy)
        have hbf : b ≠ f := fun e => hb (e ▸ hft.1)
        have hta : t ≠ a := fun e => hft.2 (e ▸ ha)
        simp only [apply, hf', ht', not_false_eq_true, and_self, ite_true]
        rw [rename_comm d (Ne.symm hfa) hbf hta]
      · cases h

/-- With `rename_defs`: if the op's result evaluates and the rename's hypotheses hold of it, the
rebased op's result on the head evaluates, at the same fuel, to the same traces and log, ids
renamed in their first segment only. -/
theorem rebased_evaluates {d d' : Doc} {a b : Name} {op op' : Op} {n : Nat} {Δ : Env} {m : Model}
    {l : Log} (hr : apply d (.rename a b) = some (d.rename a b)) (h : apply d op = some d')
    (ho : op.rn a b = some op') (ht : ∀ f t, op = .rename f t → t ≠ b)
    (hD : DefsRn a b d'.defs) (hT : TopB a b d'.body) (hfv : b ∉ fvB d'.body)
    (he : execS n d'.defs d'.σ [] d'.body = .ok (Δ, m, l)) :
    ∃ d'', apply (d.rename a b) op' = some d'' ∧
      execS n d''.defs d''.σ [] d''.body = .ok (rnEnv a b Δ, rnModel a b m, l) :=
  ⟨d'.rename a b, rebase_rename hr h ho ht, rename_defs a b hD hT hfv he⟩

/-! ## Convergence (S) -/

/-- A sequence of ops applied in order; the first refusal refuses the whole. -/
def applyAll {S O : Type} (ap : S → O → Option S) : S → List O → Option S
  | s, [] => some s
  | s, o :: os => (ap s o).bind fun s' => applyAll ap s' os

theorem applyAll_append {S O : Type} (ap : S → O → Option S) : ∀ (s : S) (xs ys : List O),
    applyAll ap s (xs ++ ys) = (applyAll ap s xs).bind fun s' => applyAll ap s' ys
  | s, [], ys => rfl
  | s, o :: xs, ys => by
    simp only [List.cons_append, applyAll]
    cases ap s o with
    | none => rfl
    | some s' => exact applyAll_append ap s' xs ys

/-- **Convergence.** The server's document is its first document with its total order `L`
applied. A client that took a snapshot after the first `k` ops and then applies the rest, with
the same deterministic `apply`, ends at the server's document. -/
theorem converge {S O : Type} (ap : S → O → Option S) (s0 : S) (L : List O) (k : Nat) :
    (applyAll ap s0 (L.take k)).bind (fun s => applyAll ap s (L.drop k)) = applyAll ap s0 L := by
  rw [← applyAll_append, List.take_append_drop]

/-- The same, whatever chunks the server's order arrives in. -/
theorem converge_chunks {S O : Type} (ap : S → O → Option S) : ∀ (s0 : Option S) (cs : List (List O)),
    cs.foldl (fun acc c => acc.bind fun s => applyAll ap s c) s0 =
      s0.bind fun s => applyAll ap s cs.flatten
  | s0, [] => by cases s0 <;> rfl
  | s0, c :: cs => by
    rw [List.foldl_cons, converge_chunks ap _ cs, List.flatten_cons]
    cases s0 with
    | none => rfl
    | some s =>
      simp only [Option.bind_some, applyAll_append]

/-- Client replay, one step: a client applied `op` on `d` (its optimistic `d'`); the server
committed `a → b` first and then the rebased `op'`. Replaying the server's order from `d` gives
the client's own result, renamed. -/
theorem client_replay_rename {d d' : Doc} {a b : Name} {op op' : Op}
    (hr : apply d (.rename a b) = some (d.rename a b)) (h : apply d op = some d')
    (ho : op.rn a b = some op') (ht : ∀ f t, op = .rename f t → t ≠ b) :
    applyAll apply d [.rename a b, op'] = some (d'.rename a b) := by
  simp [applyAll, hr, rebase_rename hr h ho ht]

end Rebase
end ForgeLang
