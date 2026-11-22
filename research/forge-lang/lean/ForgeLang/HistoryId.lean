import ForgeLang.Edits

/-!
# Kernel names from feature ids (`interface/forge-lang/src/build.rs`, `history_id`)

A feature's id is a path (`error.rs`'s `path_text`: segments joined by `.`, a segment's loop keys
in `[..]`). The kernel names the entities a feature makes from `history_id(path)`: 64-bit FNV-1a
of the path's bytes, then `(h >> 1) | 1`.

* `historyId_stable`: the kernel name is a function of the id text, so every theorem that keeps
  an id (`setParam_ids`, Add, Delete, Move, `count_prefix`, `keyed_insert`, Suppress's kept
  fragments) keeps the kernel names of that feature's entities. Ids come from names, call paths
  and loop keys (P5), never from evaluation order, and so do kernel names.
* `historyId_collides`: it is not injective, even on identifiers. `qolagjaapenjgkgca` and
  `qpibohgembfgikgid` have the same history id (found by a distinguished-point search over
  17-letter names, about 2^31 hashes). Both are legal feature names, and a run with both
  (`two_features_run`) has unique ids (P1) but one kernel name.

| Assumption | Counterexample without it |
|---|---|
| The features of one part have distinct history ids (unchecked) | `historyId_collides`. In fg: `qolagjaapenjgkgca = extrude(plan.outline, t)`, `qpibohgembfgikgid = extrude(plan.c, t)` and `fillet(edges(of: qolagjaapenjgkgca, crease: "convex"), 1)` builds with no error, fillets the other feature's edges too (31 faces, 11904.3 mm³, against 29 faces and 11925.3 with the names `base`, `boss`), and the part is reported `valid: false` with `DuplicateName` problems only. Fix: check the ids of a run for history-id clashes (one pass, a typed error like `dupId`), or name entities by the path itself |
| `path_text` injective on ids | Holds for the language: names and labels are identifiers, indices integers, so no segment holds `.`, `[`, `]` or `,`. Not mechanised |
-/

namespace ForgeLang

def fnvOffset : Nat := 0xcbf29ce484222325
def fnvPrime : Nat := 0x100000001b3

/-- `history_id` on the path's bytes. -/
def historyId (bytes : List Nat) : Nat :=
  let h := bytes.foldl (fun h b => ((h ^^^ b) * fnvPrime) % 2 ^ 64) fnvOffset
  (h >>> 1) ||| 1

def bytesOf (s : String) : List Nat := s.toList.map Char.toNat

/-- A function of the id's text: equal ids, equal kernel names, whatever else the edit changed. -/
theorem historyId_stable {s t : String} (h : s = t) : historyId (bytesOf s) = historyId (bytesOf t) := by
  rw [h]

theorem historyId_collides :
    "qolagjaapenjgkgca" ≠ "qpibohgembfgikgid" ∧
    historyId (bytesOf "qolagjaapenjgkgca") = historyId (bytesOf "qpibohgembfgikgid") ∧
    historyId (bytesOf "qolagjaapenjgkgca") = 3315298033150976027 := by
  decide

/-- The two as features of one document: it runs, its ids are unique, its kernel names are not. -/
theorem two_features_run :
    run 4 ({ body := [.feat "qolagjaapenjgkgca" (.lit 0), .feat "qpibohgembfgikgid" (.lit 0)], σ := fun _ => 6 } : Doc) =
      .ok ([([.nm "qolagjaapenjgkgca"], .site 0), ([.nm "qpibohgembfgikgid"], .site 0)], []) := rfl

end ForgeLang
