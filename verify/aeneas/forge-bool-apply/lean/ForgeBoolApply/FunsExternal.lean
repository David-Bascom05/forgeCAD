/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeBoolApply.Types

/-!
# The opaque functions of forge-bool-apply's translation (FunsExternal_Template.lean)

**std's**, each as std's own body (library source of the Charon toolchain, which matches the
kernel's 1.99 for these): the iterator adapters `filter`, `map` (and `Map::next`, `Map::fold`,
`Filter::fold`), the defaults `any` and `fold`, a slice iterator's `fold`, `&Vec`'s `into_iter`,
`Option::clone`, `Option::unwrap_or_else`, `Result::is_err`, `Result::map_or`, `..=end` slice
indexing, `[T]::first`, `binary_search`, `binary_search_by_key` (std's branch-free loop, so the
same index among equal keys), `rotate_right`, `Vec::dedup`.

- `[T]::sort_unstable`: an insertion sort through `Ord::cmp`. std's pattern-defeating quicksort
  gives the same slice whenever `cmp` is a total order whose `Equal` is equality, as it is for
  the one caller's `FaceId` (derived `Ord` on a `u32`).
- The raw-pointer `get_unchecked` methods of `..=end` fail (`undef`), as Aeneas's library models
  them for `Range` and `RangeTo`; nothing in `apply` calls them (they fill the trait instance).

**Geometry**: `Curve::clone` and `Surface::clone` are derived on values with no interior
mutability (their `Arc`s are shared, read-only): the same value.

**Models (to prove)**: `classify::extend_query` and `classify::drop_query_loops` keep face `f`'s
point-in-face index (`face.query`) in step with its loops. Each is modelled as writing an opaque
new `query` into face `f` and changing nothing else (`queryExtended`, `queryDropped`), the
writes' failure included in the opaque value. D8 reads no `query` (verify/forge-bool-proof LEDGER
D8.writes).
-/

open Aeneas Aeneas.Std Result ControlFlow

/-! ## Iterators -/

/-- One step of `Iterator::any`'s default method: the first `true` stops it. -/
def core.iter.traits.iterator.Iterator.anyStep {Self F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator Self Item)
    (fm : core.ops.function.FnMut F Item Bool)
    (st : Self × F) : Result (ControlFlow (Self × F) (Bool × Self)) := do
  let (x, it) ← inst.next st.1
  match x with
  | none => ok (done (false, it))
  | some x =>
    let (b, f) ← fm.call_mut st.2 x
    if b then ok (done (true, it)) else ok (cont (it, f))

/-- `Iterator::any`'s default method, over the iterator's own `next`. -/
def core.iter.traits.iterator.Iterator.any.default {Self F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator Self Item)
    (fm : core.ops.function.FnMut F Item Bool) :
    Self → F → Result (Bool × Self) :=
  fun it f => loop (core.iter.traits.iterator.Iterator.anyStep inst fm) (it, f)

/-- `Iterator::filter`: the adapter, holding the iterator and the predicate. -/
def core.iter.traits.iterator.Iterator.filter.default {Self P Item : Type _}
    (_ : core.iter.traits.iterator.Iterator Self Item)
    (_ : core.ops.function.FnMut P Item Bool) :
    Self → P → Result (core.iter.adapters.filter.Filter Self P) :=
  fun it p => ok ⟨it, p⟩

/-- `Iterator::map`: the adapter, holding the iterator and the closure. -/
def core.iter.traits.iterator.Iterator.map.default {Self B F Item : Type _}
    (_ : core.iter.traits.iterator.Iterator Self Item)
    (_ : core.ops.function.FnMut F Item B) :
    Self → F → Result (core.iter.adapters.map.Map Self F) :=
  fun it f => ok ⟨it, f⟩

/-- One step of `Iterator::fold`'s default: `while let Some(x) = self.next() { acc = f(acc, x) }`. -/
def core.iter.traits.iterator.Iterator.foldStep {Self B F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator Self Item)
    (fm : core.ops.function.FnMut F (B × Item) B)
    (st : Self × B × F) : Result (ControlFlow (Self × B × F) B) := do
  let (x, it) ← inst.next st.1
  match x with
  | none => ok (done st.2.1)
  | some x =>
    let (acc, f) ← fm.call_mut st.2.2 (st.2.1, x)
    ok (cont (it, acc, f))

/-- `Iterator::fold`'s default method. -/
def core.iter.traits.iterator.Iterator.fold.default {Self B F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator Self Item)
    (fm : core.ops.function.FnMut F (B × Item) B) :
    Self → B → F → Result B :=
  fun it init f => loop (core.iter.traits.iterator.Iterator.foldStep inst fm) (it, init, f)

/-- `Filter::fold`: the inner iterator's fold with `filter_fold` (the predicate on each item; the
kept ones into `fold`). -/
def core.iter.adapters.filter.Filter.Insts.CoreIterTraitsIteratorIterator.fold
    {I P Acc Fold Item : Type _} (inst : core.iter.traits.iterator.Iterator I Item)
    (pm : core.ops.function.FnMut P Item Bool)
    (fm : core.ops.function.FnMut Fold (Acc × Item) Acc)
    (it : core.iter.adapters.filter.Filter I P) (init : Acc) (g : Fold) : Result Acc :=
  loop (fun (st : I × Acc × P × Fold) => do
    let (x, i) ← inst.next st.1
    match x with
    | none => ok (done st.2.1)
    | some x =>
      let (keep, p) ← pm.call_mut st.2.2.1 x
      if keep then
        let (acc, g) ← fm.call_mut st.2.2.2 (st.2.1, x)
        ok (cont (i, acc, p, g))
      else ok (cont (i, st.2.1, p, st.2.2.2))) (it.iter, init, it.predicate, g)

/-- `Map::next`: the inner `next`, mapped through the closure. -/
def core.iter.adapters.map.Map.Insts.CoreIterTraitsIteratorIterator.next {B I F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator I Item)
    (fm : core.ops.function.FnMut F Item B)
    (m : core.iter.adapters.map.Map I F) :
    Result ((Option B) × (core.iter.adapters.map.Map I F)) := do
  let (x, i) ← inst.next m.iter
  match x with
  | none => ok (none, ⟨i, m.f⟩)
  | some x =>
    let (y, f) ← fm.call_mut m.f x
    ok (some y, ⟨i, f⟩)

/-- `Map::fold`: the inner iterator's fold with `map_fold` (`g(acc, f(x))`). -/
def core.iter.adapters.map.Map.Insts.CoreIterTraitsIteratorIterator.fold
    {B I F Acc G Item : Type _} (inst : core.iter.traits.iterator.Iterator I Item)
    (fm : core.ops.function.FnMut F Item B) (gm : core.ops.function.FnMut G (Acc × B) Acc)
    (m : core.iter.adapters.map.Map I F) (init : Acc) (g : G) : Result Acc :=
  loop (fun (st : I × Acc × F × G) => do
    let (x, i) ← inst.next st.1
    match x with
    | none => ok (done st.2.1)
    | some x =>
      let (y, f) ← fm.call_mut st.2.2.1 x
      let (acc, g) ← gm.call_mut st.2.2.2 (st.2.1, y)
      ok (cont (i, acc, f, g))) (m.iter, init, m.f, g)

/-- `Iterator::fold` on a slice iterator: each element left, in order. -/
def core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.fold {T B F : Type _}
    (fm : core.ops.function.FnMut F (B × T) B) (it : core.slice.iter.Iter T) (init : B) (f : F) :
    Result B :=
  if h : it.i < it.slice.val.length then do
    let (acc, f) ← fm.call_mut f (init, it.slice.val[it.i])
    core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.fold fm
      { it with i := it.i + 1 } acc f
  else ok init
termination_by it.slice.val.length - it.i
decreasing_by all_goals (simp_wf; omega)

/-- `<&Vec<T> as IntoIterator>::into_iter`: the slice's iterator from the start. -/
def SharedAVec.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter {T : Type _}
    (A : Type _) (v : alloc.vec.Vec T) : Result (core.slice.iter.Iter T) :=
  ok ⟨alloc.vec.Vec.deref v, 0⟩

/-! ## `Option`, `Result` -/

/-- `Option::clone`: the payload's clone. -/
def core.option.Option.Insts.CoreCloneClone.clone {T : Type _} (inst : core.clone.Clone T) :
    Option T → Result (Option T)
  | none => ok none
  | some x => do
    let y ← inst.clone x
    ok (some y)

/-- `Option::unwrap_or_else`: the payload, or the closure's value. -/
def core.option.Option.unwrap_or_else {T F : Type _} (inst : core.ops.function.FnOnce F Unit T) :
    Option T → F → Result T
  | some x, _ => ok x
  | none, f => inst.call_once f ()

/-- `Result::is_err`. -/
def core.result.Result.is_err {T E : Type _} : core.result.Result T E → Result Bool
  | .Ok _ => ok false
  | .Err _ => ok true

/-- `Result::map_or`: the default on `Err`, else the closure on the payload. -/
def core.result.Result.map_or {T E U F : Type _} (inst : core.ops.function.FnOnce F T U) :
    core.result.Result T E → U → F → Result U
  | .Ok t, _, f => inst.call_once f t
  | .Err _, d, _ => ok d

/-! ## Slices -/

/-- `(..=end).get(s)`: std's `(0..=end).get`, none unless `end < len`. -/
def core.ops.range.RangeToInclusiveUsize.Insts.CoreSliceIndexSliceIndexSliceSlice.get
    {T : Type _} (r : forge_bool.core.ops.range.RangeToInclusive Std.Usize) (s : Slice T) :
    Result (Option (Slice T)) :=
  if r.«end».val < s.length then
    ok (some (.from (s.val.take (r.«end».val + 1)) (by simp)))
  else ok none

/-- `(..=end).get_mut(s)`. -/
def core.ops.range.RangeToInclusiveUsize.Insts.CoreSliceIndexSliceIndexSliceSlice.get_mut
    {T : Type _} (r : forge_bool.core.ops.range.RangeToInclusive Std.Usize) (s : Slice T) :
    Result ((Option (Slice T)) × (Option (Slice T) → Slice T)) :=
  if r.«end».val < s.length then
    ok (some (.from (s.val.take (r.«end».val + 1)) (by simp)),
      fun s' => match s' with
        | none => s
        | some s' => .from (List.setSlice! s.val 0 s'.val) (by simp))
  else ok (none, fun _ => s)

/-- `(..=end).get_unchecked`: fails (`undef`), as Aeneas's library has `Range`'s. -/
def core.ops.range.RangeToInclusiveUsize.Insts.CoreSliceIndexSliceIndexSliceSlice.get_unchecked
    {T : Type _} (_ : forge_bool.core.ops.range.RangeToInclusive Std.Usize) (_ : ConstRawPtr (Slice T)) :
    Result (ConstRawPtr (Slice T)) := fail .undef

/-- `(..=end).get_unchecked_mut`: fails (`undef`), as Aeneas's library has `Range`'s. -/
def core.ops.range.RangeToInclusiveUsize.Insts.CoreSliceIndexSliceIndexSliceSlice.get_unchecked_mut
    {T : Type _} (_ : forge_bool.core.ops.range.RangeToInclusive Std.Usize) (_ : MutRawPtr (Slice T)) :
    Result (MutRawPtr (Slice T)) := fail .undef

/-- `&s[..=end]`: std's `RangeInclusive::index`, a panic unless `end < len`. -/
def core.ops.range.RangeToInclusiveUsize.Insts.CoreSliceIndexSliceIndexSliceSlice.index
    {T : Type _} (r : forge_bool.core.ops.range.RangeToInclusive Std.Usize) (s : Slice T) :
    Result (Slice T) :=
  if r.«end».val < s.length then ok (.from (s.val.take (r.«end».val + 1)) (by simp))
  else fail .panic

/-- `&mut s[..=end]`, with the write-back. -/
def core.ops.range.RangeToInclusiveUsize.Insts.CoreSliceIndexSliceIndexSliceSlice.index_mut
    {T : Type _} (r : forge_bool.core.ops.range.RangeToInclusive Std.Usize) (s : Slice T) :
    Result ((Slice T) × (Slice T → Slice T)) :=
  if r.«end».val < s.length then
    ok (.from (s.val.take (r.«end».val + 1)) (by simp),
      fun s' => .from (List.setSlice! s.val 0 s'.val) (by simp))
  else fail .panic

/-- `[T]::first`. -/
def core.slice.Slice.first {T : Type _} (s : Slice T) : Result (Option T) := ok s.val.head?

/-- The loop of std's `binary_search_by`: no early exit; `base` moves to `mid` unless the probe
is `Greater`, `size` halves (rounding up). -/
def core.slice.binarySearchLoop {T F : Type _} (fm : core.ops.function.FnMut F T Ordering)
    (s : List T) (f : F) (base size : Nat) : Result (Nat × F) :=
  if hs : size > 1 then do
    let half := size / 2
    let mid := base + half
    if h : mid < s.length then
      let (c, f) ← fm.call_mut f s[mid]
      let base := if c = .gt then base else mid
      core.slice.binarySearchLoop fm s f base (size - half)
    else fail .undef
  else ok (base, f)
termination_by size
decreasing_by omega

/-- A `usize` from a `Nat` (`undef` past `usize::MAX`: the callers' values are slice indices). -/
def core.slice.toUsize (n : Nat) : Result Std.Usize :=
  if h : n ≤ Usize.max then ok (Usize.ofNatCore n (by scalar_tac)) else fail .undef

/-- std's `binary_search_by` (library/core/src/slice/mod.rs): the loop, then one probe at
`base`: `Ok(base)` on `Equal`, else `Err(base + (cmp == Less))`. (The probes are in bounds; the
`undef` branches are `get_unchecked` out of range, which std's invariant excludes.) -/
def core.slice.binarySearchBy {T F : Type _} (fm : core.ops.function.FnMut F T Ordering)
    (s : Slice T) (f : F) : Result (core.result.Result Std.Usize Std.Usize) :=
  if s.length = 0 then ok (.Err 0#usize)
  else do
    let (base, f) ← core.slice.binarySearchLoop fm s.val f 0 s.length
    if h : base < s.val.length then
      let (c, _) ← fm.call_mut f s.val[base]
      if c = .eq then do
        let i ← core.slice.toUsize base
        ok (.Ok i)
      else do
        let i ← core.slice.toUsize (base + (if c = .lt then 1 else 0))
        ok (.Err i)
    else fail .undef

/-- The closure `|p| p.cmp(x)` of `binary_search`. -/
def core.slice.cmpWith {T : Type _} (inst : core.cmp.Ord T) (x : T) :
    core.ops.function.FnMut Unit T Ordering where
  FnOnceInst := { call_once := fun _ p => inst.cmp p x }
  call_mut := fun u p => do
    let c ← inst.cmp p x
    ok (c, u)

/-- `[T]::binary_search(x)`: `binary_search_by(|p| p.cmp(x))`. -/
def core.slice.Slice.binary_search {T : Type _} (inst : core.cmp.Ord T) (s : Slice T) (x : T) :
    Result (core.result.Result Std.Usize Std.Usize) :=
  core.slice.binarySearchBy (core.slice.cmpWith inst x) s ()

/-- The closure `|k| f(k).cmp(b)` of `binary_search_by_key`, carrying `f`. -/
def core.slice.keyCmp {T B F : Type _} (fm : core.ops.function.FnMut F T B)
    (inst : core.cmp.Ord B) (b : B) : core.ops.function.FnMut F T Ordering where
  FnOnceInst := { call_once := fun f k => do let (y, _) ← fm.call_mut f k; inst.cmp y b }
  call_mut := fun f k => do
    let (y, f) ← fm.call_mut f k
    let c ← inst.cmp y b
    ok (c, f)

/-- `[T]::binary_search_by_key(b, f)`: `binary_search_by(|k| f(k).cmp(b))`. -/
def core.slice.Slice.binary_search_by_key {T B F : Type _} (fm : core.ops.function.FnMut F T B)
    (inst : core.cmp.Ord B) (s : Slice T) (b : B) (f : F) :
    Result (core.result.Result Std.Usize Std.Usize) :=
  core.slice.binarySearchBy (core.slice.keyCmp fm inst b) s f

/-- Insert `x` into a sorted list, before the first element not `Less` than it. -/
def core.slice.insertSorted {T : Type _} (inst : core.cmp.Ord T) (x : T) :
    List T → Result (List T)
  | [] => ok [x]
  | y :: ys => do
    let c ← inst.cmp y x
    if c = .lt then do
      let r ← core.slice.insertSorted inst x ys
      ok (y :: r)
    else ok (x :: y :: ys)

/-- Insertion sort through `Ord::cmp`. -/
def core.slice.insertionSort {T : Type _} (inst : core.cmp.Ord T) : List T → Result (List T)
  | [] => ok []
  | x :: xs => do
    let r ← core.slice.insertionSort inst xs
    core.slice.insertSorted inst x r

/-- `[T]::sort_unstable` (see the module note: std's result whenever `Equal` is equality). -/
def core.slice.Slice.sort_unstable {T : Type _} (inst : core.cmp.Ord T) (s : Slice T) :
    Result (Slice T) := do
  let l ← core.slice.insertionSort inst s.val
  if h : l.length ≤ Usize.max then ok (.from l h) else fail .panic

/-- `[T]::rotate_right(k)`: a panic unless `k <= len`; the last `k` elements first. -/
def core.slice.Slice.rotate_right {T : Type _} (s : Slice T) (k : Std.Usize) : Result (Slice T) :=
  if k.val ≤ s.length then
    ok (.from (s.val.drop (s.length - k.val) ++ s.val.take (s.length - k.val)) (by simp))
  else fail .panic

/-- `dedup`'s walk: keep `x` unless `x == last kept` (std's `same_bucket(a, b)` with `a` the
candidate and `b` the element kept before it). -/
def alloc.vec.dedupList {T : Type _} (inst : core.cmp.PartialEq T T) :
    T → List T → Result (List T)
  | _, [] => ok []
  | prev, x :: xs => do
    let e ← inst.eq x prev
    if e then alloc.vec.dedupList inst prev xs
    else do
      let r ← alloc.vec.dedupList inst x xs
      ok (x :: r)

/-- `Vec::dedup`: consecutive equal elements collapsed to the first. -/
def alloc.vec.Vec.dedup {T : Type _} (A : Type _) (inst : core.cmp.PartialEq T T)
    (v : alloc.vec.Vec T) : Result (alloc.vec.Vec T) :=
  match v.val with
  | [] => ok v
  | x :: xs => do
    let l ← alloc.vec.dedupList inst x xs
    if h : (x :: l).length ≤ Usize.max then ok (alloc.vec.Vec.from (x :: l) h) else fail .panic

/-! ## Geometry: derived `Clone` on immutable values -/

def forge_geom.curve.Curve.Insts.CoreCloneClone.clone (c : forge_geom.curve.Curve) :
    Result forge_geom.curve.Curve := ok c

def forge_geom.surface.Surface.Insts.CoreCloneClone.clone (s : forge_geom.surface.Surface) :
    Result forge_geom.surface.Surface := ok s

/-! ## The point-in-face query cache: models (to prove) -/

namespace forge_bool

/-- The `query` `extend_query(body, f, lp)` leaves on face `f` (opaque), or its failure. -/
opaque queryExtended (b : forge_topo.body.Body) (f : forge_topo.body.FaceId)
    (lp : forge_topo.body.LoopId) : Result (forge_topo.body.Cached forge_topo.body.FaceQuery) :=
  ok none

/-- The `query` `drop_query_loops(body, f, gone)` leaves on face `f` (opaque), or its failure. -/
opaque queryDropped (b : forge_topo.body.Body) (f : forge_topo.body.FaceId)
    (gone : Slice forge_topo.body.LoopId) :
    Result (forge_topo.body.Cached forge_topo.body.FaceQuery) := ok none

/-- Face `f` with its `query` replaced: a panic when `f` is out of range (`body.faces[f]`). -/
def setQuery (b : forge_topo.body.Body) (f : forge_topo.body.FaceId)
    (q : forge_topo.body.Cached forge_topo.body.FaceQuery) : Result forge_topo.body.Body :=
  let i := (f : Std.U32).val
  if h : i < b.faces.val.length then
    let face := b.faces.val[i]
    let fs := b.faces.val.set i { face with query := q }
    have hfs : fs.length ≤ Usize.max := by simp [fs]
    ok { b with faces := alloc.vec.Vec.from fs hfs }
  else fail .panic

/-- `classify::extend_query`: a model, only face `f`'s `query` changes. -/
def classify.extend_query (b : forge_topo.body.Body) (f : forge_topo.body.FaceId)
    (lp : forge_topo.body.LoopId) : Result forge_topo.body.Body := do
  let q ← queryExtended b f lp
  setQuery b f q

/-- `classify::drop_query_loops`: a model, only face `f`'s `query` changes. -/
def classify.drop_query_loops (b : forge_topo.body.Body) (f : forge_topo.body.FaceId)
    (gone : Slice forge_topo.body.LoopId) : Result forge_topo.body.Body := do
  let q ← queryDropped b f gone
  setQuery b f q

end forge_bool

namespace forge_bool

/-- `forge_math::tol::LINEAR` as the translation reads it here (a constant, `ok`). -/
def forge_math.tol.LINEAR : Result F64 := ok _root_.forge_math.tol.LINEAR

end forge_bool
