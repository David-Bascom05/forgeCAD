import ForgeLang.Struct

/-!
# P5 when the edit is structural

A loop count is an observed literal, so `Edits.lean` does not apply: changing it re-runs the loop.
What stays stable is this. Range-loop instances are a prefix-closed family: growing or shrinking
the count keeps every surviving instance, id and trace, exactly. Keyed loops are stable under
inserting or deleting any entry, anywhere. Range loops over a *list* are not
(`Counter/Ids.lean`): an insertion in the middle moves every later instance to a new id.
-/

namespace ForgeLang

theorem loopN_append : ∀ (ks ks' : List Nat) {n D σ ρ i b r}, loopN n D σ ρ i b (ks ++ ks') = .ok r →
    ∃ m1 l1 m2 l2, loopN n D σ ρ i b ks = .ok (m1, l1) ∧ loopN n D σ ρ i b ks' = .ok (m2, l2) ∧
      r = (m1 ++ m2, l1 ++ l2)
  | [], ks', n, D, σ, ρ, i, b, r, h => by
    cases n with
    | zero => simp [loopN] at h
    | succ n => exact ⟨[], [], r.1, r.2, by simp [loopN], by simpa using h, by simp⟩
  | k :: ks, ks', n, D, σ, ρ, i, b, r, h => by
    cases n with
    | zero => simp [loopN] at h
    | succ n =>
      simp only [List.cons_append, loopN, bind_ok] at h
      obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨m', l'⟩, h', h⟩ := h
      cases h
      obtain ⟨ma, la, mb, lb, ha, hb, e⟩ := loopN_append ks ks' h'
      cases e
      exact ⟨pre [.ix k] m1 ++ ma, l1 ++ la, mb, lb, by simp [loopN, h1, ha],
        loopN_mono hb (Nat.le_succ n), by simp⟩

theorem loopK_append : ∀ (kvs kvs' : List (Name × Expr)) {n D σ ρ i b r},
    loopK n D σ ρ i b (kvs ++ kvs') = .ok r →
    ∃ m1 l1 m2 l2, loopK n D σ ρ i b kvs = .ok (m1, l1) ∧ loopK n D σ ρ i b kvs' = .ok (m2, l2) ∧
      r = (m1 ++ m2, l1 ++ l2)
  | [], kvs', n, D, σ, ρ, i, b, r, h => by
    cases n with
    | zero => simp [loopK] at h
    | succ n => exact ⟨[], [], r.1, r.2, by simp [loopK], by simpa using h, by simp⟩
  | (k, e) :: kvs, kvs', n, D, σ, ρ, i, b, r, h => by
    cases n with
    | zero => simp [loopK] at h
    | succ n =>
      simp only [List.cons_append, loopK, bind_ok] at h
      obtain ⟨⟨t, l0⟩, h0, ⟨Δ1, m1, l1⟩, h1, ⟨m', l'⟩, h', h⟩ := h
      cases h
      obtain ⟨ma, la, mb, lb, ha, hb, e⟩ := loopK_append kvs kvs' h'
      cases e
      exact ⟨pre [.key k] m1 ++ ma, l0 ++ l1 ++ la, mb, lb, by simp [loopK, h0, h1, ha],
        loopK_mono hb (Nat.le_succ n), by simp⟩

/-- Changing a range loop's count from `k` to `k' ≥ k` keeps the first `k` instances exactly
(ids and traces) and appends the new ones. Read backwards, shrinking drops only the tail. -/
theorem count_prefix {n n' D σ ρ i b} {k k' : Nat} {m l m' l'} (hk : k ≤ k')
    (h : loopN n D σ ρ i b (List.range k) = .ok (m, l))
    (h' : loopN n' D σ ρ i b (List.range k') = .ok (m', l')) : ∃ m2, m' = m ++ m2 := by
  obtain ⟨d, rfl⟩ : ∃ d, k' = k + d := ⟨k' - k, by omega⟩
  induction d generalizing m' l' with
  | zero => cases loopN_unique h h'; exact ⟨[], by simp⟩
  | succ d ih =>
    rw [← Nat.add_assoc, List.range_succ] at h'
    obtain ⟨m1, l1, m2, l2, h1, -, e⟩ := loopN_append _ _ h'
    obtain ⟨m3, e3⟩ := ih (Nat.le_add_right k d) h1
    cases e
    exact ⟨m3 ++ m2, by simp [e3]⟩

/-- Inserting an entry anywhere in a keyed loop keeps every other instance exactly and puts the new
one's features where it was inserted. -/
theorem keyed_insert {n n' D σ ρ i b kvs1 kv kvs2 m l m' l'}
    (h : loopK n D σ ρ i b (kvs1 ++ kvs2) = .ok (m, l))
    (h' : loopK n' D σ ρ i b (kvs1 ++ kv :: kvs2) = .ok (m', l')) :
    ∃ m1 mk m2, m = m1 ++ m2 ∧ m' = m1 ++ mk ++ m2 ∧ (∃ lk, loopK n' D σ ρ i b [kv] = .ok (mk, lk)) := by
  obtain ⟨m1, l1, m2, l2, h1, h2, e⟩ := loopK_append _ _ h
  obtain ⟨m1', l1', m2', l2', h1', h2', e'⟩ := loopK_append _ _ h'
  obtain ⟨mk, lk, m2'', l2'', hk, h2'', e''⟩ := loopK_append [kv] kvs2 h2'
  cases loopK_unique h1 h1'
  cases loopK_unique h2 h2''
  simp only [Prod.mk.injEq] at e e' e''
  obtain ⟨rfl, -⟩ := e
  obtain ⟨rfl, -⟩ := e'
  obtain ⟨rfl, -⟩ := e''
  exact ⟨m1, mk, m2, rfl, by simp, lk, hk⟩

end ForgeLang
