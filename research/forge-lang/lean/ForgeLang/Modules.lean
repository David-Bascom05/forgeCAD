import ForgeLang.Scoped

/-!
# Modules: `use`, a file's default part, cycles

A file is a module: the components it defines, the files it uses, and its top level as its
*default part*, a zero-arg component whose params are the file's shared params (`part(FILE, with:
{..})` sets them). Loading follows `use` edges with a stack of the files being loaded.

* `load_total`: with a budget of one more than the number of files, loading never runs out. It
  links, or it stops with the typed error `useCycle` (a file reached again while it is loading) or
  `noFile`. So `use` keeps evaluation total.
* `link_inert`: what `use` adds is definitions, never statements. Linking components that the
  document does not place changes nothing, so a used module's top level is not placed.
* `ids_rooted`: every feature id starts with the name of a statement of the document (through
  `if` blocks), so a used file's features can only appear under an occurrence the document names.
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

abbrev FileName := String

structure Module where
  uses : List FileName
  /-- the top level as a zero-arg part: shared params as params, nothing visible -/
  part : Comp
  comps : List (Name × Comp)

/-- Load a file and, depth first, every file it uses. `stack`: the files being loaded. -/
def load (F : FileName → Option Module) : Nat → List FileName → FileName → R (List (FileName × Module))
  | 0, _, _ => .error .fuel
  | n + 1, stack, f =>
    if f ∈ stack then .error (.useCycle f) else
    match F f with
    | none => .error (.noFile f)
    | some M => do
      let rest ← M.uses.mapM (load F n (f :: stack))
      .ok ((f, M) :: rest.flatten)

theorem nodup_len {α : Type} [DecidableEq α] : ∀ (l fs : List α), l.Nodup → (∀ x ∈ l, x ∈ fs) →
    l.length ≤ fs.length
  | [], _, _, _ => Nat.zero_le _
  | x :: l, fs, hn, hs => by
    have hx : x ∈ fs := hs x (by simp)
    have hxl : x ∉ l := (List.nodup_cons.1 hn).1
    have ih := nodup_len l (fs.erase x) (List.nodup_cons.1 hn).2 (fun y hy => by
      have hyx : y ≠ x := fun e => hxl (e ▸ hy)
      exact (List.mem_erase_of_ne hyx).2 (hs y (by simp [hy])))
    rw [List.length_erase_of_mem hx] at ih
    have : 0 < fs.length := List.length_pos_of_mem hx
    simp; omega

theorem mapM_no_fuel {α β : Type} (g : α → R β) : ∀ (l : List α), (∀ a ∈ l, g a ≠ .error .fuel) →
    l.mapM g ≠ .error .fuel
  | [], _ => by simp [List.mapM_nil, pure, Except.pure]
  | a :: l, h => by
    rw [List.mapM_cons]
    have ha := h a (by simp)
    have hl := mapM_no_fuel g l (fun b hb => h b (by simp [hb]))
    cases hg : g a with
    | error e => rw [hg] at ha; simpa [bind, Except.bind] using ha
    | ok b =>
      cases hm : l.mapM g with
      | error e => rw [hm] at hl; simpa [bind, Except.bind, hm] using hl
      | ok bs => simp [bind, Except.bind, pure, Except.pure]

/-- With a budget above the number of files not yet on the stack, loading never runs out. -/
theorem load_no_fuel (F : FileName → Option Module) (fs : List FileName)
    (hfs : ∀ f M, F f = some M → f ∈ fs) :
    ∀ n stack f, stack.Nodup → (∀ g ∈ stack, g ∈ fs) → fs.length - stack.length < n →
      load F n stack f ≠ .error .fuel := by
  intro n
  induction n with
  | zero => intro _ _ _ _ h; omega
  | succ n ih =>
    intro stack f hn hs hl
    simp only [load]
    split
    · simp
    · rename_i hf
      cases hF : F f with
      | none => simp
      | some M =>
        simp only
        have hfm := hfs f M hF
        have hlt : stack.length < fs.length := by
          have := nodup_len (f :: stack) fs (List.nodup_cons.2 ⟨hf, hn⟩)
            (fun g hg => by rcases List.mem_cons.1 hg with rfl | hg; exact hfm; exact hs g hg)
          simp at this; omega
        have hm := mapM_no_fuel (load F n (f :: stack)) M.uses (fun g _ =>
          ih (f :: stack) g (List.nodup_cons.2 ⟨hf, hn⟩)
            (fun g hg => by rcases List.mem_cons.1 hg with rfl | hg; exact hfm; exact hs g hg)
            (by simp; omega))
        cases hr : M.uses.mapM (load F n (f :: stack)) with
        | error e => rw [hr] at hm; simpa [bind, Except.bind] using hm
        | ok _ => simp [bind, Except.bind]

theorem mapM_err {α β : Type} {g : α → R β} {P : Err → Prop} : ∀ {l : List α} {e : Err},
    (∀ a ∈ l, ∀ e, g a = .error e → P e) → l.mapM g = .error e → P e
  | [], e, _, h => by simp [List.mapM_nil, pure, Except.pure] at h
  | a :: l, e, hg, h => by
    rw [List.mapM_cons] at h
    cases ha : g a with
    | error e' =>
      rw [ha] at h
      simp only [bind, Except.bind] at h
      cases h; exact hg a (by simp) _ ha
    | ok b =>
      rw [ha] at h
      simp only [bind, Except.bind] at h
      cases hm : l.mapM g with
      | error e' =>
        rw [hm] at h; cases h
        exact mapM_err (fun a' h' => hg a' (by simp [h'])) hm
      | ok bs => rw [hm] at h; cases h

/-- Loading stops only with these errors. -/
theorem load_errs (F : FileName → Option Module) : ∀ n stack f e, load F n stack f = .error e →
    e = .fuel ∨ (∃ g, e = .useCycle g) ∨ (∃ g, e = .noFile g) := by
  intro n
  induction n with
  | zero => intro _ _ e h; simp [load] at h; exact .inl h.symm
  | succ n ih =>
    intro stack f e h
    simp only [load] at h
    split at h
    · cases h; exact .inr (.inl ⟨f, rfl⟩)
    · cases hF : F f with
      | none => rw [hF] at h; cases h; exact .inr (.inr ⟨f, rfl⟩)
      | some M =>
        rw [hF] at h
        simp only at h
        cases hr : M.uses.mapM (load F n (f :: stack)) with
        | error e' =>
          rw [hr] at h
          simp only [bind, Except.bind] at h
          cases h
          exact mapM_err (fun g _ e h => ih _ g e h) hr
        | ok _ => rw [hr] at h; simp [bind, Except.bind] at h

/-- `use` keeps evaluation total: with a budget of one more than the number of files, loading
links or stops with a typed `useCycle` or `noFile`, never `fuel`. -/
theorem load_total (F : FileName → Option Module) (fs : List FileName) (hfs : ∀ f M, F f = some M → f ∈ fs)
    (f : FileName) {e : Err} (h : load F (fs.length + 1) [] f = .error e) :
    (∃ g, e = .useCycle g) ∨ (∃ g, e = .noFile g) := by
  rcases load_errs F _ _ _ _ h with h' | h' | h'
  · subst h'
    exact absurd h (load_no_fuel F fs hfs _ [] f List.nodup_nil (by simp) (by simp))
  · exact .inl h'
  · exact .inr h'

/-! ## Linking adds definitions, never statements -/

def linkAll (L : List (Name × Comp)) (D : Defs) : Defs := L.foldr (fun p D => D.withComp p.1 p.2) D

theorem linkAll_procs (L : List (Name × Comp)) (D : Defs) : (linkAll L D).procs = D.procs := by
  induction L with
  | nil => rfl
  | cons p L ih => simp [linkAll] at ih ⊢; exact ih

theorem linkAll_comps (L : List (Name × Comp)) (D : Defs) : (linkAll L D).comps = L ++ D.comps := by
  induction L with
  | nil => rfl
  | cons p L ih => simp [linkAll, Defs.withComp] at ih ⊢; exact ih

/-- Linking components the document does not place (a used module's components and default part)
changes no run. -/
theorem link_inert {D : Defs} {body : List Stmt} :
    ∀ (L : List (Name × Comp)), (∀ c ∈ L.map Prod.fst, c ∉ placesB body ∧ NoPlace c D ∧ ∀ p ∈ L, c ∉ placesB p.2.body) →
      ∀ n σ ρ, execS n (linkAll L D) σ ρ body = execS n D σ ρ body
  | [], _, _, _, _ => rfl
  | (c, C) :: L, h, n, σ, ρ => by
    have hc := h c (by simp)
    have np : NoPlace c (linkAll L D) := by
      refine ⟨by rw [linkAll_procs]; exact hc.2.1.1, ?_⟩
      rw [linkAll_comps]
      intro q hq
      rcases List.mem_append.1 hq with hq | hq
      · exact hc.2.2 q (by simp [hq])
      · exact hc.2.1.2 q hq
    show execS n ((linkAll L D).withComp c C) σ ρ body = _
    rw [(congr np n).s hc.1]
    exact link_inert L (fun c' hc' => by
      have := h c' (by simp [hc'])
      exact ⟨this.1, this.2.1, fun p hp => this.2.2 p (by simp [hp])⟩) n σ ρ

/-! ## Ids are rooted at the document's own statements -/

mutual
def rootsS : Stmt → List Name
  | .let_ _ _ | .set _ _ => []
  | .feat x _ | .forN x _ _ _ | .forK x _ _ _ | .use x _ _ | .occ x _ _ => [x]
  | .ifs _ a b => rootsB a ++ rootsB b
def rootsB : List Stmt → List Name
  | [] => []
  | s :: ss => rootsS s ++ rootsB ss
end

def Rooted (N : List Name) (m : Model) : Prop := ∀ e ∈ m, ∃ x ∈ N, e.1.head? = some (.nm x)

theorem rooted_pre (x : Name) (m : Model) : Rooted [x] (pre [.nm x] m) := by
  intro e he
  obtain ⟨e', _, rfl⟩ := List.mem_map.1 he
  exact ⟨x, by simp, by simp⟩

theorem Rooted.mono {N N' : List Name} {m : Model} (h : Rooted N m) (hs : ∀ x ∈ N, x ∈ N') : Rooted N' m :=
  fun e he => let ⟨x, hx, h⟩ := h e he; ⟨x, hs x hx, h⟩

theorem ids_rooted : ∀ n, (∀ {D σ ρ s Δ m l}, execOne n D σ ρ s = .ok (Δ, m, l) → Rooted (rootsS s) m) ∧
    (∀ {D σ ρ ss Δ m l}, execS n D σ ρ ss = .ok (Δ, m, l) → Rooted (rootsB ss) m) := by
  intro n
  induction n with
  | zero => exact ⟨fun h => by simp [execOne] at h, fun h => by simp [execS] at h⟩
  | succ n ih =>
    refine ⟨fun {D σ ρ s Δ m l} h => ?_, fun {D σ ρ ss Δ m l} h => ?_⟩
    · cases s with
      | let_ x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, h⟩ := h; cases h; intro e he; cases he
      | set x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, _, _, h⟩ := h; cases h; intro e he; cases he
      | feat x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, h⟩ := h; cases h
        intro e he; simp at he; subst he; exact ⟨x, by simp [rootsS], rfl⟩
      | forN x i c b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, _, _, _, _, h⟩ := h; cases h; exact rooted_pre x _
      | forK x i kvs b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, h⟩ := h; cases h; exact rooted_pre x _
      | ifs c a b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, ⟨_, mm, _⟩, hb, h⟩ := h
        cases h
        have := ih.2 hb
        split at this
        · exact this.mono fun x hx => by simp [rootsS, hx]
        · exact this.mono fun x hx => by simp [rootsS, hx]
      | use x p a =>
        simp only [execOne, bind_ok] at h
        obtain ⟨_, _, h⟩ := h
        split at h
        · rw [bind_ok] at h; obtain ⟨_, _, h⟩ := h; cases h; exact rooted_pre x _
        · cases h
      | occ x c args =>
        simp only [execOne] at h
        split at h
        · split at h
          · simp only [bind_ok] at h
            obtain ⟨_, _, _, _, _, _, h⟩ := h; cases h; exact rooted_pre x _
          · cases h
        · cases h
    · cases ss with
      | nil => simp [execS] at h; obtain ⟨-, rfl, -⟩ := h; intro e he; cases he
      | cons s ss =>
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        intro e he
        rcases List.mem_append.1 he with he | he
        · obtain ⟨x, hx, hh⟩ := ih.1 h1 e he; exact ⟨x, by simp [rootsB, hx], hh⟩
        · obtain ⟨x, hx, hh⟩ := ih.2 h2 e he; exact ⟨x, by simp [rootsB, hx], hh⟩

/-! ## Examples -/

def emptyPart : Comp := { params := [], body := [], vis := [] }

/-- `a.fg` uses `b.fg`, which uses `a.fg`: the typed error, at once. This is also what
ExtractComponent produces if the new file `use`s the source file for a function the block calls:
functions the block calls must be copied, or moved to a third module. -/
def cycleFiles : FileName → Option Module
  | "a.fg" => some { uses := ["b.fg"], part := emptyPart, comps := [] }
  | "b.fg" => some { uses := ["a.fg"], part := emptyPart, comps := [] }
  | _ => none

theorem use_cycle : load cycleFiles 3 [] "a.fg" = .error (.useCycle "a.fg") := rfl

/-- `part(FILE, with: {k: ..})` with `k` not one of the file's shared params is a typed error. -/
def mugPart : Comp := { params := [("d", .lit 0)], body := [.feat "body" (.var "d")], vis := [] }

theorem with_unknown_param :
    run 20 { defs := { comps := [("mug.fg", mugPart)] }, body := [.occ "m" "mug.fg" [("h", .lit 1)]], σ := fun _ => 1 } =
      .error (.noParam "mug.fg") ∧
    run 20 { defs := { comps := [("mug.fg", mugPart)] }, body := [.occ "m" "mug.fg" [("d", .lit 1)]], σ := fun _ => 1 } =
      .ok ([([.nm "m", .nm "body"], .site 1)], []) := ⟨rfl, rfl⟩

end ForgeLang
