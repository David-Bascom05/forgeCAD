/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeTopoBody.Types

/-!
# The opaque functions of forge-topo-body's translation (FunsExternal_Template.lean)

std's, each as std's own body:
- `OnceLock::new`, `get_mut`, `take` on `OnceLock<T>` as `Option<T>` (TypesExternal.lean);
- `Vec::retain`: the elements the predicate keeps, in order, the predicate called on each in turn;
- `Iterator::size_hint`'s default: `(0, None)`;
- `Vec::resize_with`: grown by `new_len - len` calls of the closure, in order, or truncated.

`Body::rebuild_shells` is not translated (it regroups the shells; its union-find walks
`windows` and `map` iterators): a model, the body with only its shells and its faces' shell ids
changed (`rebuild_shells_model`), to prove (verify/forge-bool-proof LEDGER).
-/

open Aeneas Aeneas.Std Result

def std.sync.once_lock.OnceLock.new (T : Type _) : Result (std.sync.once_lock.OnceLock T) := ok none

def std.sync.once_lock.OnceLock.get_mut {T : Type _} (o : std.sync.once_lock.OnceLock T) :
    Result ((Option T) × (Option T → std.sync.once_lock.OnceLock T)) :=
  ok (o, fun o' => o')

def std.sync.once_lock.OnceLock.get {T : Type _} (o : std.sync.once_lock.OnceLock T) :
    Result (Option T) := ok o

/-- `OnceLock::set(&self, v)`: `Ok(())` if it was empty, else `Err(v)`. The write itself goes
through `&self` (interior mutability), which the translation cannot carry back: the translated
`Body` keeps the cache as it was. Only `Cached::set` (a face's box) calls it, and no statement reads
a cache. -/
def std.sync.once_lock.OnceLock.set {T : Type _} (o : std.sync.once_lock.OnceLock T) (v : T) :
    Result (core.result.Result Unit T) :=
  match o with
  | none => ok (.Ok ())
  | some _ => ok (.Err v)

def std.sync.once_lock.OnceLock.take {T : Type _} (o : std.sync.once_lock.OnceLock T) :
    Result ((Option T) × (std.sync.once_lock.OnceLock T)) :=
  ok (o, none)

/-- The elements `inst` keeps, in order. -/
def retainList {T F : Type _} (inst : core.ops.function.FnMut F T Bool) :
    List T → F → Result (List T)
  | [], _ => ok []
  | x :: xs, f => do
    let (b, f) ← inst.call_mut f x
    let ys ← retainList inst xs f
    ok (if b then x :: ys else ys)

def alloc.vec.Vec.retain {T : Type _} (A : Type _) {F : Type _}
    (inst : core.ops.function.FnMut F T Bool) (v : alloc.vec.Vec T) (f : F) :
    Result (alloc.vec.Vec T) := do
  let l ← retainList inst v.val f
  if h : l.length ≤ Usize.max then ok (alloc.vec.Vec.from l h) else fail .panic

def core.iter.traits.iterator.Iterator.size_hint.default {Self : Type _} {Clause0_Item : Type _}
    (_ : core.iter.traits.iterator.Iterator Self Clause0_Item) :
    Self → Result (Std.Usize × (Option Std.Usize)) :=
  fun _ => ok (0#usize, none)


/-- `n` values of the closure, called in turn. -/
def alloc.vec.repeatWith {T F : Type _} (fm : core.ops.function.FnMut F Unit T) :
    Nat → F → Result (List T × F)
  | 0, f => ok ([], f)
  | n + 1, f => do
    let (x, f) ← fm.call_mut f ()
    let (xs, f) ← alloc.vec.repeatWith fm n f
    ok (x :: xs, f)

/-- `Vec::resize_with(new_len, f)`, std's body: `extend_trusted(repeat_with(f).take(new_len - len))`
when it grows, else `truncate(new_len)`. -/
def alloc.vec.Vec.resize_with {T : Type _} (A : Type _) {F : Type _}
    (fm : core.ops.function.FnMut F Unit T) (v : alloc.vec.Vec T) (n : Std.Usize) (f : F) :
    Result (alloc.vec.Vec T) :=
  if v.val.length < n.val then do
    let (xs, _) ← alloc.vec.repeatWith fm (n.val - v.val.length) f
    if h : (v.val ++ xs).length ≤ Usize.max then ok (alloc.vec.Vec.from (v.val ++ xs) h)
    else fail .panic
  else ok (alloc.vec.Vec.from (v.val.take n.val) (by simp <;> scalar_tac))

namespace forge_topo

/-! ## forge-math's constants, bound to forge-math's translation (as in ForgeGeom) -/

def forge_math.tol.LINEAR : Result F64 := ok _root_.forge_math.tol.LINEAR

/-! ## `Coedges`, a loop's coedges in order: its `Iterator` methods, as body.rs writes them

(Kept out of the translation: its `next` goes through `bool::then` with a closure, and its
`size_hint` is a method the library's `Iterator` has no field for.) -/

/-- `(self.at < self.end).then(|| { self.at += 1; CoedgeId(self.at - 1) })`. -/
def body.Coedges.Insts.CoreIterTraitsIteratorIteratorCoedgeId.next (it : body.Coedges) :
    Result ((Option body.CoedgeId) × body.Coedges) :=
  if it.at < it.end then do
    let a ← it.at + 1#u32
    let c ← a - 1#u32
    ok (some c, { it with «at» := a })
  else ok (none, it)

/-- `let n = (self.end - self.at) as usize; (n, Some(n))`. -/
def body.Coedges.Insts.CoreIterTraitsIteratorIteratorCoedgeId.size_hint (it : body.Coedges) :
    Result (Std.Usize × (Option Std.Usize)) := do
  let d ← it.end - it.at
  let n := UScalar.cast .Usize d
  ok (n, some n)

end forge_topo

namespace forge_topo

/-! ## `Body::rebuild_shells`: a model (to prove)

body.rs regroups the live faces into shells by connectivity (`connected`, a union-find over the
live edges' `windows(2)` and a `map`), clears `shells`, and sets each live face's `shell`. The
model keeps exactly that shape: the new shell list and each live face's shell id are opaque
functions of the body (which grouping is left unstated), and nothing else changes. So the faces'
loops, surfaces and senses, the edges, coedges and loops are as they were: all D8.writes,
D8.sense and D8.lone read. Ledger: "model, to prove". -/

/-- The shells `rebuild_shells` makes (opaque: which grouping is not stated). -/
opaque rebuiltShells (b : body.Body) : alloc.vec.Vec body.Shell := alloc.vec.Vec.new body.Shell

/-- The shell `rebuild_shells` puts live face `f` in (opaque). -/
opaque rebuiltShellOf (b : body.Body) (f : Nat) : body.ShellId := (0#u32 : Std.U32)

def body.Body.rebuild_shells (self : body.Body) : Result body.Body :=
  let fs := self.faces.val.mapIdx fun i f =>
    if f.alive then { f with shell := rebuiltShellOf self i } else f
  if h : fs.length ≤ Usize.max then
    ok { self with shells := rebuiltShells self, faces := alloc.vec.Vec.from fs h }
  else fail .panic

end forge_topo
