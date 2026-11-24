import ForgeLang.Families

/-!
# Rename in a machining document

`edit_ops::apply_rename` (and `ast_rename`) on a machining document (`calls_closed`) rewrites the
binding `a` and every read of it to `b`; a call's head is a built-in there, never a binding, so it
is left as written (`drill = drill(..)` renamed to `holes` is `holes = drill(..)`), and a param's
name (after `.` in `param op1.clear`) is not a read either.

`rename_run`: if `b` occurs nowhere in the document (not bound, not read: what `precheck_rename`'s
`NameTaken` and an accepted document give) and neither `a` nor `b` is a param's name, the renamed
document runs exactly as the original: the same state after every statement (so the same setups,
ops and programs), the same statements failing, and the final environment the old one with `a`
read as `b`. `rename_accepted`: an accepted document stays accepted.

Before the fix in `edit_ops.rs` the call head was renamed too, and the renamed document called a
built-in that does not exist; `renSt` is the fixed op.
-/

namespace ForgeLang
namespace Families

variable {S α : Type} (P : Prims α)

/-- `a` read as `b`. -/
def ren (a b : Name) (z : Name) : Name := if z = a then b else z

mutual
def renE (a b : Name) : Ex → Ex
  | .lit n => .lit n
  | .read x => .read (ren a b x)
  | .op u v => .op (renE a b u) (renE a b v)
  | .call f as => .call f (renL a b as)

def renL (a b : Name) : List Ex → List Ex
  | [] => []
  | e :: es => renE a b e :: renL a b es
end

/-- The op on a statement: its binding and reads; not a call's head, not a param's name. -/
def renSt (a b : Name) : St → St
  | .bind x e => .bind (ren a b x) (renE a b e)
  | .bindCall x ps f as => .bindCall (ren a b x) (ps.map fun p => (p.1, renE a b p.2)) f (renL a b as)
  | .skip => .skip

mutual
/-- The names an expression reads. -/
def readsE : Ex → List Name
  | .lit _ => []
  | .read x => [x]
  | .op u v => readsE u ++ readsE v
  | .call _ as => readsL as

def readsL : List Ex → List Name
  | [] => []
  | e :: es => readsE e ++ readsL es
end

/-- `b` is fresh in the statement, and neither `a` nor `b` is one of its params' names. -/
def Fresh (a b : Name) : St → Prop
  | .bind x e => x ≠ b ∧ b ∉ readsE e
  | .bindCall x ps _ as => x ≠ b ∧ (∀ p ∈ ps, p.1 ≠ a ∧ p.1 ≠ b ∧ b ∉ readsE p.2) ∧ b ∉ readsL as
  | .skip => True

/-- The renamed environment reads `ren x` as the old one reads `x`, for every `x` but `b`. -/
def Rel (a b : Name) (ρ ρ' : Env α) : Prop := ∀ x, x ≠ b → ρ'.lookup (ren a b x) = ρ.lookup x

/-- Both succeed alike, or both fail. -/
inductive Same {β : Type} : Except Err β → Except Err β → Prop
  | ok (v : β) : Same (.ok v) (.ok v)
  | err (e e' : Err) : Same (.error e) (.error e')

theorem Same.refl {β : Type} (z : Except Err β) : Same z z := by
  cases z with
  | ok v => exact .ok v
  | error e => exact .err e e

/-- Two lists, entry by entry related. -/
inductive All2 {β : Type} (R : β → β → Prop) : List β → List β → Prop
  | nil : All2 R [] []
  | cons {x y : β} {xs ys : List β} : R x y → All2 R xs ys → All2 R (x :: xs) (y :: ys)

theorem All2.mem_right {β : Type} {R : β → β → Prop} :
    ∀ {xs ys : List β}, All2 R xs ys → ∀ x ∈ xs, ∃ y ∈ ys, R x y
  | _, _, .nil, _, h => by simp at h
  | _, _, .cons hr h2, x, hx => by
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ⟨_, List.mem_cons_self, hr⟩
    · obtain ⟨y, hy, r⟩ := All2.mem_right h2 x hx
      exact ⟨y, List.mem_cons_of_mem _ hy, r⟩

theorem Same.elim {β : Type} {x y : Except Err β} (h : Same x y) :
    (∃ v, x = .ok v ∧ y = .ok v) ∨ (∃ e e', x = .error e ∧ y = .error e') := by
  cases h with
  | ok v => exact .inl ⟨v, rfl, rfl⟩
  | err e e' => exact .inr ⟨e, e', rfl, rfl⟩

theorem ren_inj {a b x y : Name} (hx : x ≠ b) (hy : y ≠ b) (h : ren a b x = ren a b y) : x = y := by
  unfold ren at h
  by_cases ha : x = a <;> by_cases hb : y = a <;> simp only [ha, hb, ↓reduceIte] at h
  · rw [ha, hb]
  · exact absurd h.symm hy
  · exact absurd h hx
  · exact h

theorem rel_nil (a b : Name) : Rel a b ([] : Env α) [] := fun _ _ => rfl

/-- A binding, renamed. -/
theorem rel_cons {a b : Name} {ρ ρ' : Env α} (h : Rel a b ρ ρ') (k : Name) (v : α) (hk : k ≠ b) :
    Rel a b ((k, v) :: ρ) ((ren a b k, v) :: ρ') := by
  intro x hx
  by_cases e : x = k
  · subst e; simp [List.lookup]
  · have : (ren a b x == ren a b k) = false := by
      simp only [beq_eq_false_iff_ne]
      exact fun h' => e (ren_inj hx hk h')
    have e' : (x == k) = false := by simpa using e
    simp only [List.lookup, this, e']
    exact h x hx

/-- A param's binding, kept as it is (its name is neither `a` nor `b`). -/
theorem rel_cons_keep {a b : Name} {ρ ρ' : Env α} (h : Rel a b ρ ρ') (k : Name) (v : α)
    (ha : k ≠ a) (hb : k ≠ b) : Rel a b ((k, v) :: ρ) ((k, v) :: ρ') := by
  have hk : ren a b k = k := by simp [ren, ha]
  have := rel_cons h k v hb
  rwa [hk] at this

mutual
theorem eval_ren (E : ETab S α) (s : S) {a b : Name} {ρ ρ' : Env α} (hr : Rel a b ρ ρ') :
    ∀ (e : Ex), b ∉ readsE e → Same (eval P E s ρ' (renE a b e)) (eval P E s ρ e)
  | .lit n, _ => by simp only [renE, eval]; exact .ok _
  | .read x, hb => by
    have hx : x ≠ b := fun h => hb (by simp [readsE, h])
    simp only [renE, eval, hr x hx]
    cases ρ.lookup x with
    | some v => exact .ok v
    | none => exact .err _ _
  | .op u v, hb => by
    have hu : b ∉ readsE u := fun h => hb (by simp [readsE, h])
    have hv : b ∉ readsE v := fun h => hb (by simp [readsE, h])
    simp only [renE, eval]
    rcases (eval_ren E s hr u hu).elim with ⟨x, h1, h2⟩ | ⟨e1, e2, h1, h2⟩
    · rw [h1, h2]
      rcases (eval_ren E s hr v hv).elim with ⟨y, h3, h4⟩ | ⟨e3, e4, h3, h4⟩
      · rw [h3, h4]
        exact Same.refl _
      · rw [h3, h4]; exact .err _ _
    · rw [h1, h2]; exact .err _ _
  | .call f as, hb => by
    have has : b ∉ readsL as := fun h => hb (by simpa [readsE] using h)
    simp only [renE, eval]
    rcases (evalL_ren E s hr as has).elim with ⟨vs, h1, h2⟩ | ⟨e1, e2, h1, h2⟩
    · rw [h1, h2]
      cases E.lookup f with
      | none => exact .err _ _
      | some g =>
        exact Same.refl _
    · rw [h1, h2]; exact .err _ _

theorem evalL_ren (E : ETab S α) (s : S) {a b : Name} {ρ ρ' : Env α} (hr : Rel a b ρ ρ') :
    ∀ (es : List Ex), b ∉ readsL es → Same (evalL P E s ρ' (renL a b es)) (evalL P E s ρ es)
  | [], _ => by simp only [renL, evalL]; exact .ok _
  | e :: es, hb => by
    have he : b ∉ readsE e := fun h => hb (by simp [readsL, h])
    have hes : b ∉ readsL es := fun h => hb (by simp [readsL, h])
    simp only [renL, evalL]
    rcases (eval_ren E s hr e he).elim with ⟨v, h1, h2⟩ | ⟨e1, e2, h1, h2⟩
    · rw [h1, h2]
      rcases (evalL_ren E s hr es hes).elim with ⟨vs, h3, h4⟩ | ⟨e3, e4, h3, h4⟩
      · rw [h3, h4]; exact .ok _
      · rw [h3, h4]; exact .err _ _
    · rw [h1, h2]; exact .err _ _
end

/-- A statement's params, each name kept: the same success, environments still related. -/
theorem locals_ren (E : ETab S α) (s : S) {a b : Name} {ρ ρ' : Env α} (hr : Rel a b ρ ρ') :
    ∀ (ps : List (Name × Ex)), (∀ p ∈ ps, p.1 ≠ a ∧ p.1 ≠ b ∧ b ∉ readsE p.2) →
      (∃ ρl ρl', locals P E s ρ ps = .ok ρl ∧
        locals P E s ρ' (ps.map fun p => (p.1, renE a b p.2)) = .ok ρl' ∧ Rel a b ρl ρl') ∨
      ((∃ e, locals P E s ρ ps = .error e) ∧
        ∃ e', locals P E s ρ' (ps.map fun p => (p.1, renE a b p.2)) = .error e')
  | [], _ => .inl ⟨ρ, ρ', rfl, rfl, hr⟩
  | (k, e) :: ps, h => by
    obtain ⟨hka, hkb, he⟩ := h (k, e) List.mem_cons_self
    simp only [List.map_cons, locals]
    rcases (eval_ren P E s hr e he).elim with ⟨v, g1, g2⟩ | ⟨e1, e2, g1, g2⟩
    · rw [g1, g2]
      rcases locals_ren E s hr ps (fun p hp => h p (List.mem_cons_of_mem _ hp)) with
        ⟨ρl, ρl', h1, h2, h3⟩ | ⟨⟨e3, h1⟩, ⟨e4, h2⟩⟩
      · simp only [h1, h2]
        exact .inl ⟨_, _, rfl, rfl, rel_cons_keep h3 k v hka hkb⟩
      · simp only [h1, h2]
        exact .inr ⟨⟨_, rfl⟩, ⟨_, rfl⟩⟩
    · rw [g1, g2]; exact .inr ⟨⟨_, rfl⟩, ⟨_, rfl⟩⟩

/-- One statement, renamed: the same state, related environments, the same success. -/
theorem stmt_ren (E : ETab S α) (T : TTab S α) (s : S) {a b : Name} {ρ ρ' : Env α}
    (hr : Rel a b ρ ρ') (st : St) (hf : Fresh a b st) :
    (stmt P E T s ρ' (renSt a b st)).1 = (stmt P E T s ρ st).1 ∧
      Rel a b (stmt P E T s ρ st).2.1 (stmt P E T s ρ' (renSt a b st)).2.1 ∧
      Same (stmt P E T s ρ' (renSt a b st)).2.2 (stmt P E T s ρ st).2.2 := by
  cases st with
  | skip => exact ⟨rfl, hr, .ok _⟩
  | bind x e =>
    obtain ⟨hx, he⟩ := hf
    simp only [renSt, stmt]
    rcases (eval_ren P E s hr e he).elim with ⟨v, g1, g2⟩ | ⟨e1, e2, g1, g2⟩
    · rw [g1, g2]; exact ⟨rfl, rel_cons hr x v hx, .ok _⟩
    · rw [g1, g2]; exact ⟨rfl, hr, .err _ _⟩
  | bindCall x ps f as =>
    obtain ⟨hx, hps, has⟩ := hf
    simp only [renSt, stmt]
    rcases locals_ren P E s hr ps hps with ⟨ρl, ρl', h1, h2, h3⟩ | ⟨⟨e3, h1⟩, ⟨e4, h2⟩⟩
    · simp only [h1, h2]
      rcases (evalL_ren P E s h3 as has).elim with ⟨vs, g1, g2⟩ | ⟨e1, e2, g1, g2⟩
      · simp only [g1, g2]
        cases T.lookup f with
        | none => exact ⟨rfl, hr, .err _ _⟩
        | some g =>
          simp only
          rcases g s vs with ⟨s', r⟩
          cases r with
          | ok v => exact ⟨rfl, rel_cons hr x v hx, .ok _⟩
          | error e => exact ⟨rfl, hr, .err _ _⟩
      · simp only [g1, g2]; exact ⟨trivial, hr, .err _ _⟩
    · simp only [h1, h2]
      exact ⟨trivial, hr, .err _ _⟩

/-- The renamed document runs as the original: the same state after it, related environments,
and statement by statement the same success. -/
theorem rename_run (E : ETab S α) (T : TTab S α) {a b : Name} :
    ∀ (sts : List St) (s : S) {ρ ρ' : Env α}, Rel a b ρ ρ' → (∀ st ∈ sts, Fresh a b st) →
      (run P E T s ρ' (sts.map (renSt a b))).1 = (run P E T s ρ sts).1 ∧
        Rel a b (run P E T s ρ sts).2.1 (run P E T s ρ' (sts.map (renSt a b))).2.1 ∧
        All2 Same (run P E T s ρ' (sts.map (renSt a b))).2.2 (run P E T s ρ sts).2.2
  | [], s, ρ, ρ', hr, _ => ⟨rfl, hr, .nil⟩
  | st :: sts, s, ρ, ρ', hr, hf => by
    obtain ⟨h1, h2, h3⟩ := stmt_ren P E T s hr st (hf st List.mem_cons_self)
    obtain ⟨g1, g2, g3⟩ := rename_run E T sts (stmt P E T s ρ st).1 h2
      (fun t ht => hf t (List.mem_cons_of_mem _ ht))
    simp only [List.map_cons, run]
    rw [h1]
    exact ⟨g1, g2, .cons h3 g3⟩

/-- An accepted machining document stays accepted when renamed, to the same state. -/
theorem rename_accepted (E : ETab S α) (T : TTab S α) {a b : Name} (sts : List St) (s : S)
    (hf : ∀ st ∈ sts, Fresh a b st) (ha : Accepted (run P E T s [] sts).2.2) :
    (run P E T s [] (sts.map (renSt a b))).1 = (run P E T s [] sts).1 ∧
      Accepted (run P E T s [] (sts.map (renSt a b))).2.2 := by
  obtain ⟨h1, -, h3⟩ := rename_run P E T sts s (rel_nil a b) hf
  refine ⟨h1, ?_⟩
  intro r hr
  obtain ⟨r0, hr0, hs⟩ := All2.mem_right h3 r hr
  have := ha r0 hr0
  subst this
  rcases hs.elim with ⟨v, h1, -⟩ | ⟨e, e', -, h2⟩
  · rw [h1]
  · cases h2

end Families
end ForgeLang
