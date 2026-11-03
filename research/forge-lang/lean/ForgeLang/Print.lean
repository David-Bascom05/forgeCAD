import ForgeLang.Struct

/-!
# P4, the text side: everything outside the edited span is byte-identical

The printer writes a *skeleton*: tokens, with a hole where each literal goes. The text is the
skeleton with the literal table filled in. The skeleton does not depend on the table, so:

* SetParam rewrites the one token at the literal's site and nothing else (`setLit_text`), provided
  the site occurs once. In the language that is free: a site is the literal's path.
* Add, Delete and Move cut or splice whole statements (`body_text_*`).

That the edited text parses back to the edited document is P6 (`ASSUMPTIONS.md`): this file proves
the edit is local, the grammar argument proves the parse is the intended one.
-/

namespace ForgeLang

inductive Tok where
  | sym : String → Tok
  | id : Name → Tok
  | num : Int → Tok
  | hole : Site → Tok
  deriving DecidableEq, Repr

def segToks : Seg → List Tok
  | .nm x => [.sym ".", .id x]
  | .ix k => [.sym "[", .num k, .sym "]"]
  | .key k => [.sym "[", .id k, .sym "]"]

mutual
def skelE : Expr → List Tok
  | .lit s => [.hole s]
  | .cst c => [.sym "const", .num c]
  | .var x => [.id x]
  | .add a b => [.sym "("] ++ skelE a ++ [.sym "+"] ++ skelE b ++ [.sym ")"]
  | .mul a b => [.sym "("] ++ skelE a ++ [.sym "*"] ++ skelE b ++ [.sym ")"]
  | .eq a b => [.sym "("] ++ skelE a ++ [.sym "=="] ++ skelE b ++ [.sym ")"]
  | .ite c a b => [.sym "("] ++ skelE c ++ [.sym "?"] ++ skelE a ++ [.sym ":"] ++ skelE b ++ [.sym ")"]
  | .sel es k => [.sym "["] ++ skelEs es ++ [.sym "]", .sym "["] ++ skelE k ++ [.sym "]"]
  | .call f a => [.id f, .sym "("] ++ skelE a ++ [.sym ")"]
  | .ref x rel => [.id x] ++ (rel.map segToks).flatten
  | .refIx x k rel => [.id x, .sym "["] ++ skelE k ++ [.sym "]"] ++ (rel.map segToks).flatten
def skelEs : List Expr → List Tok
  | [] => []
  | e :: es => skelE e ++ [.sym ","] ++ skelEs es
end

mutual
def skelS : Stmt → List Tok
  | .let_ x e => [.sym "let", .id x, .sym "="] ++ skelE e ++ [.sym ";"]
  | .set x e => [.id x, .sym "="] ++ skelE e ++ [.sym ";"]
  | .feat x e => [.sym "feat", .id x, .sym "="] ++ skelE e ++ [.sym ";"]
  | .forN x i c b => [.id x, .sym "=", .sym "for", .id i, .sym "in", .sym "0", .sym ".."] ++ skelE c ++
      [.sym "{"] ++ skelB b ++ [.sym "}"]
  | .forK x i kvs b => [.id x, .sym "=", .sym "for", .id i, .sym "in", .sym "{"] ++ skelKV kvs ++
      [.sym "}", .sym "{"] ++ skelB b ++ [.sym "}"]
  | .ifs c a b => [.sym "if"] ++ skelE c ++ [.sym "{"] ++ skelB a ++ [.sym "}", .sym "else", .sym "{"] ++
      skelB b ++ [.sym "}"]
  | .use x p a => [.id x, .sym "=", .id p, .sym "("] ++ skelE a ++ [.sym ")", .sym ";"]
def skelB : List Stmt → List Tok
  | [] => []
  | s :: ss => skelS s ++ skelB ss
def skelKV : List (Name × Expr) → List Tok
  | [] => []
  | (k, e) :: kvs => [.id k, .sym ":"] ++ skelE e ++ [.sym ","] ++ skelKV kvs
end

def render (σ : Site → Int) : Tok → Tok
  | .hole s => .num (σ s)
  | t => t

/-- The document's body as text. -/
def Doc.text (d : Doc) : List Tok := (skelB d.body).map (render d.σ)

theorem count_one {a : Tok} : ∀ {l : List Tok}, l.count a = 1 → ∃ X Y, l = X ++ a :: Y ∧ a ∉ X ∧ a ∉ Y
  | [], h => by simp at h
  | b :: l, h => by
    by_cases hb : b = a
    · subst hb
      rw [List.count_cons_self] at h
      refine ⟨[], l, rfl, by simp, ?_⟩
      intro hm
      have := List.count_pos_iff.2 hm
      omega
    · rw [List.count_cons_of_ne hb] at h
      obtain ⟨X, Y, rfl, hX, hY⟩ := count_one h
      exact ⟨b :: X, Y, rfl, by simp [Ne.symm hb, hX], hY⟩

theorem render_off {σ : Site → Int} {p : Site} {v : Int} {X : List Tok} (hX : Tok.hole p ∉ X) :
    X.map (render (upd σ p v)) = X.map (render σ) := by
  apply List.map_congr_left
  intro t ht
  cases t with
  | hole s =>
    have : s ≠ p := fun e => hX (e ▸ ht)
    simp [render, upd, this]
  | _ => rfl

/-- SetParam on the text: one token changes, from the old number to the new; the rest is
byte-identical. -/
theorem setLit_text {d : Doc} {p : Site} (v : Int) (h : (skelB d.body).count (.hole p) = 1) :
    ∃ A B, d.text = A ++ [.num (d.σ p)] ++ B ∧ (d.setLit p v).text = A ++ [.num v] ++ B := by
  obtain ⟨X, Y, e, hX, hY⟩ := count_one h
  refine ⟨X.map (render d.σ), Y.map (render d.σ), ?_, ?_⟩
  · simp [Doc.text, e, render]
  · simp only [Doc.text, Doc.setLit, e, List.map_append, List.map_cons, render_off hX, render_off hY]
    simp [render, upd]

theorem skelB_append (xs ys : List Stmt) : skelB (xs ++ ys) = skelB xs ++ skelB ys := by
  induction xs with
  | nil => simp [skelB]
  | cons s xs ih => simp [skelB, ih]

/-- AddFeature / DeleteFeature on the text: the statement's tokens are spliced in or cut out. -/
theorem body_text_insert (σ : Site → Int) (xs ys : List Stmt) (s : Stmt) :
    (skelB (xs ++ s :: ys)).map (render σ) =
      (skelB xs).map (render σ) ++ (skelS s).map (render σ) ++ (skelB ys).map (render σ) := by
  simp [skelB_append, skelB]

/-- MoveFeature on the text: two statements' tokens swap places. -/
theorem body_text_swap (σ : Site → Int) (xs ys : List Stmt) (s t : Stmt) :
    (skelB (xs ++ t :: s :: ys)).map (render σ) =
      (skelB xs).map (render σ) ++ (skelS t).map (render σ) ++ (skelS s).map (render σ) ++
        (skelB ys).map (render σ) := by
  simp [skelB_append, skelB]

end ForgeLang
