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
* `checked_distinct`: the check `push_made` makes, one look-up per feature in the kernel names
  so far (`hidCheck`), passes only when no two ids share a kernel name; `checked_complete`: it
  refuses only then. `run_kernel_names`: a run that evaluated and passed the check gives
  distinct features distinct kernel names. `clash_refused`: the two above are refused, both
  named.

| Assumption | Counterexample without it |
|---|---|
| The features of one part have distinct history ids | Now checked: `hidCheck`, as `Eval::push_made` makes it, a typed error `HistoryIdClash` naming both ids. Before, `historyId_collides`: in fg, `qolagjaapenjgkgca = extrude(plan.outline, t)`, `qpibohgembfgikgid = extrude(plan.c, t)` and `fillet(edges(of: qolagjaapenjgkgca, crease: "convex"), 1)` built with no error, filleted the other feature's edges too (31 faces, 11904.3 mm³, against 29 faces and 11925.3 with the names `base`, `boss`), and the part was reported `valid: false` with `DuplicateName` problems only |
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

/-! ## The check -/

/-- The check `Eval::push_made` makes as each feature is pushed: the ids so far, each with its
kernel name; an id whose kernel name an earlier, different id has is refused, naming both. -/
def hidCheck {α : Type} [DecidableEq α] (kid : α → Nat) : List α → List α → Except (α × α) (List α)
  | seen, [] => .ok seen
  | seen, x :: xs => match seen.find? (fun y => kid y == kid x) with
    | some y => if y = x then hidCheck kid seen xs else .error (y, x)
    | none => hidCheck kid (x :: seen) xs

/-- No two of `l` share a kernel name. -/
def KidInj {α : Type} (kid : α → Nat) (l : List α) : Prop := ∀ a ∈ l, ∀ b ∈ l, kid a = kid b → a = b

theorem hidCheck_ok {α : Type} [DecidableEq α] (kid : α → Nat) : ∀ (xs seen out : List α),
    hidCheck kid seen xs = .ok out → KidInj kid seen →
      KidInj kid out ∧ (∀ x ∈ xs, x ∈ out) ∧ ∀ x ∈ seen, x ∈ out
  | [], seen, out, h, hs => by
    simp only [hidCheck, Except.ok.injEq] at h
    subst h
    exact ⟨hs, by simp, fun _ h => h⟩
  | x :: xs, seen, out, h, hs => by
    simp only [hidCheck] at h
    split at h
    · rename_i y hy
      split at h
      · rename_i e
        subst e
        have hy' : y ∈ seen := List.mem_of_find?_eq_some hy
        obtain ⟨h1, h2, h3⟩ := hidCheck_ok kid xs seen out h hs
        refine ⟨h1, ?_, h3⟩
        intro z hz
        rcases List.mem_cons.1 hz with rfl | hz
        · exact h3 _ hy'
        · exact h2 z hz
      · cases h
    · rename_i hn
      have hn' := List.find?_eq_none.1 hn
      have hs' : KidInj kid (x :: seen) := by
        intro a ha b hb e
        rcases List.mem_cons.1 ha with rfl | ha' <;> rcases List.mem_cons.1 hb with rfl | hb'
        · rfl
        · exact (hn' b hb' (by simp [e])).elim
        · exact (hn' a ha' (by simp [e])).elim
        · exact hs a ha' b hb' e
      obtain ⟨h1, h2, h3⟩ := hidCheck_ok kid xs (x :: seen) out h hs'
      refine ⟨h1, ?_, fun z hz => h3 z (List.mem_cons_of_mem _ hz)⟩
      intro z hz
      rcases List.mem_cons.1 hz with rfl | hz
      · exact h3 _ (List.mem_cons_self ..)
      · exact h2 z hz

/-- With the check, distinct ids never share a kernel name. -/
theorem checked_distinct {α : Type} [DecidableEq α] (kid : α → Nat) {xs out : List α}
    (h : hidCheck kid [] xs = .ok out) : KidInj kid xs := by
  obtain ⟨h1, h2, -⟩ := hidCheck_ok kid xs [] out h (by simp [KidInj])
  exact fun a ha b hb e => h1 a (h2 a ha) b (h2 b hb) e

/-- The check refuses only a real clash: ids with distinct kernel names pass. -/
theorem checked_complete {α : Type} [DecidableEq α] (kid : α → Nat) : ∀ (xs seen : List α),
    KidInj kid (seen ++ xs) → ∃ out, hidCheck kid seen xs = .ok out
  | [], seen, _ => ⟨seen, rfl⟩
  | x :: xs, seen, hi => by
    simp only [hidCheck]
    split
    · rename_i y hy
      have e : kid y = kid x := by simpa using List.find?_some hy
      have hy' : y ∈ seen := List.mem_of_find?_eq_some hy
      have : y = x := hi y (by simp [hy']) x (by simp) e
      have sub : ∀ z, z ∈ seen ++ xs → z ∈ seen ++ x :: xs := by
        intro z hz
        simp only [List.mem_append, List.mem_cons] at hz ⊢
        rcases hz with h | h
        · exact Or.inl h
        · exact Or.inr (Or.inr h)
      simp only [this, ↓reduceIte]
      exact checked_complete kid xs seen (fun a ha b hb => hi a (sub a ha) b (sub b hb))
    · have sub : ∀ z, z ∈ (x :: seen) ++ xs → z ∈ seen ++ x :: xs := by
        intro z hz
        simp only [List.mem_append, List.mem_cons] at hz ⊢
        rcases hz with (h | h) | h
        · exact Or.inr (Or.inl h)
        · exact Or.inl h
        · exact Or.inr (Or.inr h)
      exact checked_complete kid xs (x :: seen) (fun a ha b hb => hi a (sub a ha) b (sub b hb))

theorem nodup_map_of_inj {α β : Type} (f : α → β) : ∀ (l : List α),
    (∀ a ∈ l, ∀ b ∈ l, f a = f b → a = b) → l.Nodup → (l.map f).Nodup
  | [], _, _ => List.nodup_nil
  | x :: xs, hi, hn => by
    rw [List.nodup_cons] at hn
    rw [List.map_cons, List.nodup_cons]
    refine ⟨?_, nodup_map_of_inj f xs (fun a ha b hb => hi a (List.mem_cons_of_mem _ ha) b
      (List.mem_cons_of_mem _ hb)) hn.2⟩
    intro hm
    obtain ⟨y, hy, e⟩ := List.mem_map.1 hm
    exact hn.1 (hi y (List.mem_cons_of_mem _ hy) x (List.mem_cons_self ..) e ▸ hy)

/-- A run that evaluated and passed the check: its features' kernel names are distinct, one per
feature, as its ids are (P1, `run_ids_unique`). -/
theorem run_kernel_names {n : Nat} {d : Doc} {m : Model} {l : Log} (kid : FId → Nat) {out}
    (h : run n d = .ok (m, l)) (hc : hidCheck kid [] (m.map Prod.fst) = .ok out) :
    ((m.map Prod.fst).map kid).Nodup :=
  nodup_map_of_inj kid _ (checked_distinct kid hc) (run_ids_unique h)

/-- A feature id's text (`error.rs`'s `path_text`, a loop key's brackets simplified). -/
def idText : FId → String
  | [] => ""
  | s :: ss => (match s with
      | .nm x => x
      | .ix i => s!"[{i}]"
      | .key k => s!"[{k}]") ++ (if ss.isEmpty then "" else ".") ++ idText ss

/-- The kernel name of a feature id. -/
def kidOf (f : FId) : Nat := historyId (bytesOf (idText f))

/-- `two_features_run`'s ids, checked: refused, both named. -/
theorem clash_refused :
    hidCheck kidOf [] [[.nm "qolagjaapenjgkgca"], [.nm "qpibohgembfgikgid"]] =
      .error ([.nm "qolagjaapenjgkgca"], [.nm "qpibohgembfgikgid"]) := by
  have e : kidOf [.nm "qolagjaapenjgkgca"] = kidOf [.nm "qpibohgembfgikgid"] := by
    simp only [kidOf, idText, List.isEmpty_nil, ite_true, String.append_empty]
    exact historyId_collides.2.1
  simp only [hidCheck, List.find?_nil, List.find?_cons, e, beq_self_eq_true]
  rfl

end ForgeLang
