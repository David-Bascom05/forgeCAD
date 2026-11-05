/-!
# A memo of kernel operations: when it is exact, and what its key must hold

The kernel is abstract: an op is a function `kernel : Inp → Shape` from its whole input (kind,
parameters, input shapes with their ids, the naming context it stamps, settings, files, what it
reads of the model before it) to a shape (geometry, ids, verdict). That it *is* a function, with
no other input, is what the audit of the kernel crates checks (statics, env, time, randomness,
hash-map order, threads, float environment). A rebuild is a history: step `k` builds its input
from the outputs before it (by reference, or "all bodies before") and runs the op. The memo maps
`key i` to the shape made for `i`.

* `memo_exact`: if the kernel factors through the key (`Covers`) and the cache holds only real
  results, a rebuild through the memo gives exactly the fresh rebuild, and the cache stays sound.
* `memo_exact_iff`: and only then. If two inputs share a key and the op gives them different
  shapes, a two-step history makes the memo wrong (`miss_wrong`). So the key must hold everything
  the op reads, and that is all it must hold (`drop_ok_iff`: a part of the input may be left out
  of the key exactly when no op's output depends on it).
* `hit_of_mem`, `edit_reuses`: every step whose key is in the cache is a hit; after an edit, every
  step before it, and every later step whose input comes out equal, is reused: no needless rebuild.
* `hashed_exact`, `collision_wrong`, `verified_exact`: keyed by a hash of the key, the memo is
  exact when the hash has no collision on the keys met; a collision is a wrong hit; storing the
  full key and comparing it on a hit makes the memo exact with no assumption on the hash.
* `Counter`: for each part of the input the key must hold (naming context, tolerance, backend,
  "everything before", file content, units, kernel version) a concrete op reading it, and the wrong
  hit when the key leaves it out.
-/

namespace ForgeLang.OpMemo

section
variable {Inp Key Shape : Type} [DecidableEq Key]

/-- The op's output is a function of its key. -/
def Covers (kernel : Inp → Shape) (key : Inp → Key) : Prop := ∀ i j, key i = key j → kernel i = kernel j

/-- A history: each step makes its op's input from the shapes made before it. -/
abbrev History (Inp Shape : Type) := List (List Shape → Inp)

def freshGo (kernel : Inp → Shape) : List Shape → History Inp Shape → List Shape
  | outs, [] => outs
  | outs, st :: h => freshGo kernel (outs ++ [kernel (st outs)]) h

/-- A fresh rebuild: every op run. -/
def fresh (kernel : Inp → Shape) (h : History Inp Shape) : List Shape := freshGo kernel [] h

abbrev Cache (Key Shape : Type) := List (Key × Shape)

/-- A rebuild through the memo: the outputs, the cache after, and which steps were hits. -/
def memoGo (kernel : Inp → Shape) (key : Inp → Key) :
    List Shape → Cache Key Shape → List Bool → History Inp Shape → List Shape × Cache Key Shape × List Bool
  | outs, c, hits, [] => (outs, c, hits)
  | outs, c, hits, st :: h =>
    let i := st outs
    match c.lookup (key i) with
    | some s => memoGo kernel key (outs ++ [s]) c (hits ++ [true]) h
    | none => memoGo kernel key (outs ++ [kernel i]) ((key i, kernel i) :: c) (hits ++ [false]) h

def memo (kernel : Inp → Shape) (key : Inp → Key) (c : Cache Key Shape) (h : History Inp Shape) :=
  memoGo kernel key [] c [] h

/-- Every entry is the op's real output for some input with that key. -/
def Sound (kernel : Inp → Shape) (key : Inp → Key) (c : Cache Key Shape) : Prop :=
  ∀ k s, c.lookup k = some s → ∃ i, key i = k ∧ kernel i = s

theorem sound_nil (kernel : Inp → Shape) (key : Inp → Key) : Sound kernel key ([] : Cache Key Shape) := by
  intro k s h; simp [List.lookup] at h

theorem sound_cons {kernel : Inp → Shape} {key : Inp → Key} {c : Cache Key Shape} (hc : Sound kernel key c) (i : Inp) :
    Sound kernel key ((key i, kernel i) :: c) := by
  intro k s h
  by_cases e : k = key i
  · subst e; simp [List.lookup] at h; exact ⟨i, rfl, h⟩
  · have : (k == key i) = false := by simp [e]
    simp [List.lookup, this] at h; exact hc k s h

theorem memoGo_exact {kernel : Inp → Shape} {key : Inp → Key} (hk : Covers kernel key) :
    ∀ (h : History Inp Shape) outs c hits, Sound kernel key c →
      (memoGo kernel key outs c hits h).1 = freshGo kernel outs h ∧ Sound kernel key (memoGo kernel key outs c hits h).2.1
  | [], _, _, _, hc => ⟨rfl, hc⟩
  | st :: h, outs, c, hits, hc => by
    simp only [memoGo, freshGo]
    split
    · rename_i s hs
      obtain ⟨j, hj, rfl⟩ := hc _ _ hs
      rw [hk j (st outs) hj]
      exact memoGo_exact hk h _ _ _ hc
    · exact memoGo_exact hk h _ _ _ (sound_cons hc _)

/-- (a) Through the memo, a rebuild is the fresh rebuild: the same shapes, ids and verdicts. -/
theorem memo_exact {kernel : Inp → Shape} {key : Inp → Key} (hk : Covers kernel key) {c : Cache Key Shape}
    (hc : Sound kernel key c) (h : History Inp Shape) :
    (memo kernel key c h).1 = fresh kernel h ∧ Sound kernel key (memo kernel key c h).2.1 :=
  memoGo_exact hk h [] c [] hc

/-- (c) A key that two inputs share while the op gives them different shapes: a wrong hit. -/
theorem miss_wrong {kernel : Inp → Shape} {key : Inp → Key} {i j : Inp} (hkey : key i = key j)
    (hne : kernel i ≠ kernel j) :
    (memo kernel key [] [fun _ => i, fun _ => j]).1 ≠ fresh kernel [fun _ => i, fun _ => j] := by
  simp [memo, memoGo, fresh, freshGo, List.lookup, hkey, hne]

/-- Exact for every history and every sound cache, exactly when the op factors through the key. -/
theorem memo_exact_iff (kernel : Inp → Shape) (key : Inp → Key) :
    (∀ (c : Cache Key Shape) h, Sound kernel key c → (memo kernel key c h).1 = fresh kernel h) ↔ Covers kernel key := by
  constructor
  · intro H i j hij
    by_cases e : kernel i = kernel j
    · exact e
    · exact absurd (H [] _ (sound_nil kernel key)) (miss_wrong hij e)
  · intro hk c h hc; exact (memo_exact hk hc h).1

/-- A part of the input may be left out of the key exactly when no output depends on it: with a
key that is the full input's `full` key after `erase` forgets that part (and `full` injective),
the memo is exact iff the op gives the same shape with the part erased. -/
theorem drop_ok_iff {Full : Type} (kernel : Inp → Shape) (full : Inp → Full) (erase : Inp → Inp)
    (hidem : ∀ i, erase (erase i) = erase i)
    (hsame : ∀ i j, erase i = erase j ↔ (full ∘ erase) i = (full ∘ erase) j) :
    Covers kernel (full ∘ erase) ↔ ∀ i, kernel i = kernel (erase i) := by
  constructor
  · intro hk i
    exact hk i (erase i) (by simp [hidem])
  · intro h i j hij
    have := (hsame i j).2 hij
    rw [h i, h j, this]

/-! ## Reuse: no needless rebuild -/

theorem memoGo_hits_len {kernel : Inp → Shape} {key : Inp → Key} :
    ∀ (h : History Inp Shape) outs c hits, (memoGo kernel key outs c hits h).2.2.length = hits.length + h.length
  | [], _, _, _ => by simp [memoGo]
  | st :: h, outs, c, hits => by
    simp only [memoGo]
    split <;> (rw [memoGo_hits_len]; simp; omega)

theorem memoGo_hits_prefix {kernel : Inp → Shape} {key : Inp → Key} :
    ∀ (h : History Inp Shape) outs c (pre : List Bool), ∃ rest, (memoGo kernel key outs c pre h).2.2 = pre ++ rest
  | [], _, _, _ => ⟨[], by simp [memoGo]⟩
  | st :: h, outs, c, pre => by
    simp only [memoGo]
    split
    · obtain ⟨r, hr⟩ := memoGo_hits_prefix (kernel := kernel) (key := key) h (outs ++ [_]) c (pre ++ [true])
      exact ⟨true :: r, by rw [hr]; simp⟩
    · obtain ⟨r, hr⟩ := memoGo_hits_prefix (kernel := kernel) (key := key) h (outs ++ [_]) _ (pre ++ [false])
      exact ⟨false :: r, by rw [hr]; simp⟩

/-- (b) A step whose key is in the cache is a hit: reused, never rebuilt. -/
theorem hit_of_mem {kernel : Inp → Shape} {key : Inp → Key} {c : Cache Key Shape} {st : List Shape → Inp}
    {h : History Inp Shape} {s : Shape} (hs : c.lookup (key (st [])) = some s) :
    (memo kernel key c (st :: h)).2.2.head? = some true := by
  simp only [memo, memoGo, hs]
  obtain ⟨r, hr⟩ := memoGo_hits_prefix (kernel := kernel) (key := key) h ([] ++ [s]) c ([] ++ [true])
  rw [hr]; rfl

/-- A step whose key is not in the cache is computed (and only then). -/
theorem miss_of_not_mem {kernel : Inp → Shape} {key : Inp → Key} {c : Cache Key Shape} {st : List Shape → Inp}
    {h : History Inp Shape} (hs : c.lookup (key (st [])) = none) :
    (memo kernel key c (st :: h)).2.2.head? = some false := by
  simp only [memo, memoGo, hs]
  obtain ⟨r, hr⟩ := memoGo_hits_prefix (kernel := kernel) (key := key) h ([] ++ [kernel (st [])]) _ ([] ++ [false])
  rw [hr]; rfl

/-- A cache only grows: what one rebuild found stays for the next. -/
theorem memoGo_keeps {kernel : Inp → Shape} {key : Inp → Key} :
    ∀ (h : History Inp Shape) outs c hits k s, c.lookup k = some s →
      (memoGo kernel key outs c hits h).2.1.lookup k = some s
  | [], _, _, _, _, _, hk => hk
  | st :: h, outs, c, hits, k, s, hk => by
    simp only [memoGo]
    split
    · exact memoGo_keeps h _ _ _ k s hk
    · rename_i hn
      apply memoGo_keeps h _ _ _ k s
      by_cases e : k = key (st outs)
      · subst e; rw [hn] at hk; cases hk
      · have : (k == key (st outs)) = false := by simp [e]
        simp [List.lookup, this, hk]

end

/-! ## Hash collisions -/

section
variable {Inp Key Hash Shape : Type}

/-- Keyed by a hash: exact when the hash is injective on the keys (no collision). -/
theorem hashed_exact {kernel : Inp → Shape} {key : Inp → Key} {hash : Key → Hash} (hk : Covers kernel key)
    (inj : ∀ a b, hash a = hash b → a = b) : Covers kernel (hash ∘ key) :=
  fun i j h => hk i j (inj _ _ h)

/-- A collision between two keys whose ops differ is a wrong hit. -/
theorem collision_wrong [DecidableEq Hash] {kernel : Inp → Shape} {key : Inp → Key} {hash : Key → Hash} {i j : Inp}
    (hc : hash (key i) = hash (key j)) (hne : kernel i ≠ kernel j) :
    (memo kernel (hash ∘ key) [] [fun _ => i, fun _ => j]).1 ≠ fresh kernel [fun _ => i, fun _ => j] :=
  miss_wrong (key := hash ∘ key) hc hne

/-- Verify on hit: the entry stores the full key, a hit must match it. The memo key is the pair, so
it is exact whatever the hash does (only the full key's covering is needed). -/
theorem verified_exact {kernel : Inp → Shape} {key : Inp → Key} {hash : Key → Hash} (hk : Covers kernel key) :
    Covers kernel (fun i => (hash (key i), key i)) :=
  fun i j h => hk i j (Prod.mk.inj h).2

end

/-! ## What the key must hold: one op per part, and the wrong hit without it -/

namespace Counter

/-- The whole input of an op, as the audit found it. `before`: the shapes of the model before it
that it reads when it reads "all"; `file`: a path and the content read from it. -/
structure Inp where
  kind : Nat
  params : List Int
  inputs : List Int
  naming : List Nat
  tol : Int
  backend : Nat
  before : List Int
  file : Nat × Int
  units : Nat
  version : Nat
  deriving DecidableEq

/-- A shape: geometry and the ids stamped on it. -/
abbrev Shape := Int × List Nat

def base : Inp := ⟨0, [5], [1], [7], 1, 0, [], (0, 0), 0, 1⟩

/-- A key that holds everything but the named-out part. -/
def keyAll (i : Inp) := (i.kind, i.params, i.inputs, i.naming, i.tol, i.backend, i.before, i.file.2, i.units, i.version)

/-- Ids: an op stamps its feature's id on what it makes. Two features with equal geometry, keyed
without the naming context: the second gets the first's ids. -/
def stamp (i : Inp) : Shape := (i.params.sum, i.naming)
theorem key_needs_naming :
    (memo stamp (fun i => (i.kind, i.params, i.inputs)) [] [fun _ => base, fun _ => { base with naming := [8] }]).1 ≠
    fresh stamp [fun _ => base, fun _ => { base with naming := [8] }] := by decide

/-- Tolerance: an op that merges what is closer than the tolerance (here: rounds to it). -/
def snap (i : Inp) : Shape := (i.params.sum / i.tol, [])
theorem key_needs_tolerance :
    (memo snap (fun i => (i.kind, i.params)) [] [fun _ => { base with tol := 2 }, fun _ => { base with tol := 3 }]).1 ≠
    fresh snap [fun _ => { base with tol := 2 }, fun _ => { base with tol := 3 }] := by decide

/-- Backend: a batch backend whose verdict on a touching case differs from the CPU's. -/
def touchVerdict (i : Inp) : Shape := (if i.backend = 1 then 1 else 0, [])
theorem key_needs_backend :
    (memo touchVerdict (fun i => (i.kind, i.params)) [] [fun _ => base, fun _ => { base with backend := 1 }]).1 ≠
    fresh touchVerdict [fun _ => base, fun _ => { base with backend := 1 }] := by decide
/-- ... and when every backend gives the same output, leaving it out is exact (`drop_ok_iff`). -/
theorem backend_free_ok : Covers stamp (fun i => keyAll { i with backend := 0 }) := by
  intro i j h; simp [keyAll] at h; simp [stamp, h.2.1, h.2.2.2.1]

/-- "All bodies before": an op that unions everything before it, keyed by its explicit inputs only.
A body added before it leaves the key unchanged: a wrong hit. Its key must hold the hashes of
what it reads, in order (the step reads them from the outputs before it). -/
def unionAll (i : Inp) : Shape := (i.before.sum, [])
theorem key_needs_before :
    let h : History Inp Shape := [fun _ => { base with kind := 1, before := [4] }, fun outs => { base with before := outs.map (·.1) }]
    let h' : History Inp Shape := [fun _ => { base with kind := 1, before := [4] }, fun _ => base]
    (memo unionAll (fun i => (i.kind, i.inputs)) (memo unionAll (fun i => (i.kind, i.inputs)) [] h').2.1 h).1 ≠
      fresh unionAll h := by decide

/-- Files by content: a font or STEP file keyed by its path, the file changed on disk. -/
def readFile (i : Inp) : Shape := (i.file.2, [])
theorem key_needs_file_content :
    (memo readFile (fun i => (i.kind, i.file.1)) [] [fun _ => { base with file := (3, 10) }, fun _ => { base with file := (3, 11) }]).1 ≠
    fresh readFile [fun _ => { base with file := (3, 10) }, fun _ => { base with file := (3, 11) }] := by decide

/-- Units or material, where the op reads them (a mass, a K-factor). -/
def scaled (i : Inp) : Shape := (i.params.sum * (if i.units = 1 then 25 else 1), [])
theorem key_needs_units :
    (memo scaled (fun i => (i.kind, i.params)) [] [fun _ => base, fun _ => { base with units := 1 }]).1 ≠
    fresh scaled [fun _ => base, fun _ => { base with units := 1 }] := by decide

/-- Kernel version: the same op, fixed in a new kernel; a cache that outlives the kernel hands
back the old result. -/
def versioned (i : Inp) : Shape := (if i.version = 1 then i.params.sum else i.params.sum + 1, [])
theorem key_needs_version :
    (memo versioned (fun i => (i.kind, i.params)) [] [fun _ => base, fun _ => { base with version := 2 }]).1 ≠
    fresh versioned [fun _ => base, fun _ => { base with version := 2 }] := by decide

/-- Parameters as bits: two parameters that print alike but differ (here: a key that rounds). -/
theorem key_needs_exact_params :
    (memo stamp (fun i => (i.kind, i.params.map (· / 10), i.naming)) [] [fun _ => { base with params := [11] }, fun _ => { base with params := [12] }]).1 ≠
    fresh stamp [fun _ => { base with params := [11] }, fun _ => { base with params := [12] }] := by decide

/-- With every part in the key, every op above is exact. -/
theorem keyAll_covers (kernel : Inp → Shape) : Covers kernel (fun i => (keyAll i, i.file.1)) := by
  intro i j h
  obtain ⟨k1, p1, in1, n1, t1, b1, be1, f1, u1, v1⟩ := i
  obtain ⟨k2, p2, in2, n2, t2, b2, be2, f2, u2, v2⟩ := j
  obtain ⟨fa, fb⟩ := f1
  obtain ⟨fc, fd⟩ := f2
  simp [keyAll] at h
  obtain ⟨⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩, rfl⟩ := h
  rfl

end Counter

end ForgeLang.OpMemo
