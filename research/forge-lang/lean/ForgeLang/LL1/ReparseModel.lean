import ForgeLang.LL1.Reparse

/-!
# `reparse_number`, as a function, and its correctness

`reparseNumber` is a model of `syntax::reparse_number` on bytes. It finds the edit's common
prefix and suffix, then the number token the change starts in. It reads a plain number at that
place in each text (`litOf`, Rust's `lit`) and checks the trivia after each (`triv`, Rust's
`trivia`), plus the same `#` presence (the check `reparse_number` lacks). It answers the old
parse with that token's text replaced and the later tokens moved. A parse here is the tree and the
tokens with their offsets (`parseText`), which is what the spans are made of.

`reparse_correct`: wherever it answers, its answer is the parse of the new text. The model looks
for the number among the tokens; Rust looks among the number expressions, which are some of
those tokens. So Rust refuses at least where the model does, and gives the model's answer where
both answer.
-/

namespace ForgeLang.LL1.Reparse
open Lex

/-! ## Bytes -/

/-- The length of the common prefix. -/
def cpl : Bytes → Bytes → Nat
  | x :: a, y :: b => if x = y then cpl a b + 1 else 0
  | _, _ => 0

theorem cpl_take : ∀ a b : Bytes, a.take (cpl a b) = b.take (cpl a b)
  | x :: a, y :: b => by
    unfold cpl
    by_cases h : x = y
    · subst h; rw [if_pos rfl]; simp [cpl_take a b]
    · rw [if_neg h]; rfl
  | [], _ => by simp [cpl]
  | _ :: _, [] => by simp [cpl]

theorem cpl_le : ∀ a b : Bytes, cpl a b ≤ a.length ∧ cpl a b ≤ b.length
  | x :: a, y :: b => by
    unfold cpl
    by_cases h : x = y
    · rw [if_pos h]; have := cpl_le a b; simp; omega
    · rw [if_neg h]; simp
  | [], _ => by simp [cpl]
  | _ :: _, [] => by simp [cpl]

theorem take_le_eq {a b : Bytes} {n m : Nat} (h : a.take n = b.take n) (hm : m ≤ n) : a.take m = b.take m := by
  have := congrArg (List.take m) h
  rwa [List.take_take, List.take_take, Nat.min_eq_left hm] at this

/-- The common suffix of length `q`, from the common prefix of the reversed texts. -/
theorem suffix_eq {a b : Bytes} {q : Nat} (h : a.reverse.take q = b.reverse.take q) :
    a.drop (a.length - q) = b.drop (b.length - q) := by
  rw [List.take_reverse, List.take_reverse] at h
  exact List.reverse_inj.mp h

/-- Spaces, tabs or CRs, then maybe `#` and the rest, with no line break: Rust's `trivia`. -/
def triv (s : Bytes) : Bool := (takeWhile (· != HASH) s).1.all isSpace && !s.contains NL

theorem takeWhile_all_p (p : Nat → Bool) : ∀ s : Bytes, (takeWhile p s).1.all p = true
  | [] => rfl
  | c :: s => by
    by_cases h : p c = true
    · rw [takeWhile.eq_2, ite_eq_left h, List.all_cons, h, takeWhile_all_p p s]; rfl
    · rw [takeWhile.eq_2, ite_eq_right h]; rfl

theorem triv_spec {s : Bytes} (h : triv s = true) : TrivLine s := by
  simp only [triv, Bool.and_eq_true, Bool.not_eq_true'] at h
  obtain ⟨hsp, hnl⟩ := h
  have happ := takeWhile_app (· != HASH) s
  generalize hw : takeWhile (· != HASH) s = w at happ hsp
  obtain ⟨sp, c⟩ := w
  have hstop := takeWhile_stop (· != HASH) s
  rw [hw] at hstop
  refine ⟨sp, c, happ.symm, hsp, ?_⟩
  cases c with
  | nil => exact Or.inl rfl
  | cons x c' =>
    have hx : x = HASH := by have := hstop x (by simp); simpa using this
    subst hx
    refine Or.inr ⟨c', rfl, List.all_eq_true.2 fun y hy => ?_⟩
    have : y ∈ s := by rw [← happ]; simp [hy]
    have hne : y ≠ NL := by
      intro e
      have h2 : s.contains y = true := by simpa using this
      rw [e, hnl] at h2; cases h2
    simpa using hne

/-! ## A plain number: Rust's `lit` -/

def signOf : Bytes → Bool × Bytes
  | c :: t => if c = MINUS then (true, t) else (false, c :: t)
  | [] => (false, [])

def fracOf : Bytes → Option (Option Bytes × Bytes)
  | c :: t => if c = DOT then
      (if (takeWhile isDigit t).1 = [] then none else some (some (takeWhile isDigit t).1, (takeWhile isDigit t).2))
    else some (none, c :: t)
  | [] => some (none, [])

def unitPre (s : Bytes) : Lex.Unit × Bytes :=
  if [109, 109].isPrefixOf s then (.mm, s.drop 2)
  else if [100, 101, 103].isPrefixOf s then (.deg, s.drop 3) else (.none, s)

/-- A byte a number could run into: a letter, a digit, `_` or `.`. -/
def contByte (c : Nat) : Bool := isAlpha c || isDigit c || c == 95 || c == DOT

/-- `-`?, digits, maybe `.` digits, maybe `mm` or `deg`, and then no byte a number runs into. -/
def litOf (s : Bytes) : Option (Lit × Bytes) :=
  let int := (takeWhile isDigit (signOf s).2).1
  if int = [] then none else
  match fracOf (takeWhile isDigit (signOf s).2).2 with
  | none => none
  | some (fr, s3) =>
    if ((unitPre s3).2.head?.map contByte).getD false then none
    else some (⟨(signOf s).1, int, fr, none, (unitPre s3).1⟩, (unitPre s3).2)

theorem signOf_app (s : Bytes) : (if (signOf s).1 then [MINUS] else []) ++ (signOf s).2 = s := by
  cases s with
  | nil => rfl
  | cons c t => unfold signOf; by_cases h : c = MINUS <;> simp [h]

def fracText : Option Bytes → Bytes
  | some f => DOT :: f
  | none => []

theorem fracOf_spec {s : Bytes} {fr r} (h : fracOf s = some (fr, r)) :
    fracText fr ++ r = s ∧
      ∀ f, fr = some f → f ≠ [] ∧ f.all isDigit = true := by
  cases s with
  | nil => simp [fracOf] at h; obtain ⟨rfl, rfl⟩ := h; simp [fracText]
  | cons c t =>
    simp only [fracOf] at h
    by_cases hc : c = DOT
    · rw [if_pos hc] at h
      by_cases he : (takeWhile isDigit t).1 = []
      · rw [if_pos he] at h; cases h
      · rw [if_neg he] at h
        simp only [Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        refine ⟨by simp [fracText, hc, takeWhile_app], fun f hf => ?_⟩
        cases hf
        exact ⟨he, takeWhile_all_p _ _⟩
    · rw [if_neg hc] at h
      simp only [Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      simp [fracText]

theorem unitPre_app (s : Bytes) : unitText (unitPre s).1 ++ (unitPre s).2 = s := by
  unfold unitPre
  split
  · rename_i h; exact prefix_app h
  · split
    · rename_i _ h; exact prefix_app h
    · rfl

theorem litOf_spec {s : Bytes} {L : Lit} {r : Bytes} (h : litOf s = some (L, r)) :
    L.Ok ∧ L.text ++ r = s ∧ Stops r := by
  unfold litOf at h
  simp only at h
  by_cases hi : (takeWhile isDigit (signOf s).2).1 = []
  · rw [if_pos hi] at h; cases h
  rw [if_neg hi] at h
  generalize hf : fracOf (takeWhile isDigit (signOf s).2).2 = fo at h
  rcases fo with _ | ⟨fr, s3⟩
  · cases h
  simp only at h
  by_cases hc : ((unitPre s3).2.head?.map contByte).getD false = true
  · rw [if_pos hc] at h; cases h
  rw [if_neg hc] at h
  simp only [Option.some.injEq, Prod.mk.injEq] at h
  obtain ⟨rfl, rfl⟩ := h
  obtain ⟨hfa, hfd⟩ := fracOf_spec hf
  refine ⟨⟨hi, takeWhile_all_p _ _, hfd, by simp⟩, ?_, ?_⟩
  · simp only [Lit.text, Lit.mant, Lit.expPart, List.append_nil, List.append_assoc]
    change _ ++ (_ ++ (fracText fr ++ _)) = s
    rw [unitPre_app, hfa, takeWhile_app, signOf_app]
  · intro c hcm
    have : contByte c = false := by
      cases hh : (unitPre s3).2 with
      | nil => rw [hh] at hcm; cases hcm
      | cons x t =>
        rw [hh] at hcm hc; simp at hcm; subst hcm; simpa using hc
    simp only [contByte, Bool.or_eq_false_iff] at this
    obtain ⟨⟨⟨h1, h2⟩, _⟩, h4⟩ := this
    exact ⟨h2, h1, by simpa using h4⟩

/-! ## The model -/

def isNum : Kind → Bool
  | .num _ => true
  | _ => false

/-- Rust's `number_at`: the first number token whose bytes hold `p` (or end there), and the tokens
before and after it. -/
def findNum : List (Piece × Nat) → Nat → Option (List (Piece × Nat) × (Piece × Nat) × List (Piece × Nat))
  | [], _ => none
  | (pc, lo) :: ts, p =>
    if isNum pc.kind = true ∧ lo ≤ p ∧ p ≤ lo + pc.text.length then some ([], (pc, lo), ts)
    else (findNum ts p).map fun (xs, x, ys) => ((pc, lo) :: xs, x, ys)

/-- A text's parse, as far as `reparse_number` is concerned: the tree, and the tokens with their
offsets (`None` where the text does not lex or parse). -/
def parseText (C : Cfg) (term : Kind → Option Nat) (eof : Nat) (G : Grammar) (T : Tables) (n : Nat)
    (s : Bytes) : Option (Tree × List (Piece × Nat)) :=
  match lex C s with
  | .ok ps =>
    match parse G T n (.n G.start) (ps.filterMap (fun p => term p.kind) ++ [eof]) with
    | some (t, []) => some (t, tokens 0 ps)
    | _ => none
  | .error _ => none

/-- **`syntax::reparse_number`, on bytes.** `r` is the old text's parse. `none` where Rust answers
false (parse again); otherwise the patched parse. The checks follow the Rust one for one, plus
the last (the same `#` presence in both changed regions). -/
def reparseNumber (a b : Bytes) (r : Tree × List (Piece × Nat)) : Option (Tree × List (Piece × Nat)) :=
  if a = b then some r else
  let p := cpl a b
  let q := min (cpl a.reverse b.reverse) (min a.length b.length - p)
  match findNum r.2 p with
  | none => none
  | some (xs, (pc, lo), ys) =>
    match litOf (a.drop lo), litOf (b.drop lo) with
    | some (L₁, _), some (L₂, _) =>
      let hi := lo + pc.text.length
      let newHi := lo + L₂.text.length
      let oldEnd := max (a.length - q) hi
      let newEnd := max (b.length - q) newHi
      if lo + L₁.text.length = hi ∧ L₁.neg = L₂.neg ∧ newEnd + a.length = oldEnd + b.length ∧
          triv ((a.drop hi).take (oldEnd - hi)) = true ∧ triv ((b.drop newHi).take (newEnd - newHi)) = true ∧
          ((a.drop hi).take (oldEnd - hi)).contains HASH = ((b.drop newHi).take (newEnd - newHi)).contains HASH then
        some (r.1, xs ++ (⟨.num L₂.unit, L₂.text⟩, lo) :: ys.map fun q => (q.1, q.2 + newEnd - oldEnd))
      else none
    | _, _ => none

/-! ## Correctness -/

theorem findNum_spec : ∀ {ts : List (Piece × Nat)} {p xs pc lo ys}, findNum ts p = some (xs, (pc, lo), ys) →
    ts = xs ++ (pc, lo) :: ys ∧ isNum pc.kind = true ∧ lo ≤ p
  | [], _, _, _, _, _, h => by simp [findNum] at h
  | (pc', lo') :: ts, p, xs, pc, lo, ys, h => by
    unfold findNum at h
    split at h
    · rename_i hc
      simp only [Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, ⟨rfl, rfl⟩, rfl⟩ := h
      exact ⟨rfl, hc.1, hc.2.1⟩
    · generalize hf : findNum ts p = F at h
      rcases F with _ | ⟨xs', x, ys'⟩
      · cases h
      · simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        obtain ⟨h1, h2, h3⟩ := findNum_spec hf
        exact ⟨by rw [h1]; rfl, h2, h3⟩

theorem tokens_triv_cons {p : Piece} (b : Nat) (ps : List Piece) (hp : isTrivia p.kind = true) :
    tokens b (p :: ps) = tokens (b + p.text.length) ps := by
  simp [tokens, placed, hp]

/-- A token of the list is a piece of the input, at the offset the text before it has. -/
theorem tokens_split : ∀ (ps : List Piece) (b : Nat) {xs pc lo ys}, tokens b ps = xs ++ (pc, lo) :: ys →
    ∃ pre post, ps = pre ++ pc :: post ∧ b + ((pre.map Piece.text).flatten).length = lo ∧
      tokens b pre = xs ∧ tokens (lo + pc.text.length) post = ys
  | [], b, xs, pc, lo, ys, h => by simp [tokens, placed] at h
  | p :: ps, b, xs, pc, lo, ys, h => by
    by_cases hp : isTrivia p.kind = true
    · rw [tokens_triv_cons _ _ hp] at h
      obtain ⟨pre, post, h1, h2, h3, h4⟩ := tokens_split ps _ h
      refine ⟨p :: pre, post, by rw [h1]; rfl, by simp only [List.map_cons, List.flatten_cons, List.length_append] at h2 ⊢; omega, ?_, h4⟩
      rw [tokens_triv_cons _ _ hp, h3]
    · have hp' : isTrivia p.kind = false := by simpa using hp
      rw [tokens_tok _ _ hp'] at h
      cases xs with
      | nil =>
        simp only [List.nil_append, List.cons.injEq, Prod.mk.injEq] at h
        obtain ⟨⟨rfl, rfl⟩, rfl⟩ := h
        exact ⟨[], ps, rfl, by simp, rfl, rfl⟩
      | cons x xs =>
        simp only [List.cons_append, List.cons.injEq] at h
        obtain ⟨rfl, h⟩ := h
        obtain ⟨pre, post, h1, h2, h3, h4⟩ := tokens_split ps _ h
        refine ⟨p :: pre, post, by rw [h1]; rfl, by simp only [List.map_cons, List.flatten_cons, List.length_append] at h2 ⊢; omega, ?_, h4⟩
        rw [tokens_tok _ _ hp', h3]

theorem prefix_eq_of_len {l a b c : Bytes} (h1 : a ++ b = l) (h2 : l = c ++ d) (hl : a.length = c.length) : a = c := by
  have e1 : l.take a.length = a := by rw [← h1, List.take_left]
  have e2 : l.take c.length = c := by rw [h2, List.take_left]
  rw [← e1, hl, e2]

theorem drop_of_app {l a r : Bytes} (h : a ++ r = l) : r = l.drop a.length := by
  rw [← h, List.drop_left]

/-- **`reparse_number` is correct**, on the model: wherever it answers, its answer is the new
text's parse, tree and tokens with offsets. For any lexer configuration with no digit in an
operator, and any terminal map that drops trivia and does not look at a number's unit. -/
theorem reparse_correct {C : Cfg} (hC : DigitFree C) {term : Kind → Option Nat}
    (htriv : ∀ k, isTrivia k = true → term k = none) (hnum : ∀ u u', term (.num u) = term (.num u'))
    {eof : Nat} {G : Grammar} {T : Tables} {n : Nat} {a b : Bytes} {r t : Tree × List (Piece × Nat)}
    (hr : parseText C term eof G T n a = some r) (ht : reparseNumber a b r = some t) :
    parseText C term eof G T n b = some t := by
  unfold reparseNumber at ht
  by_cases hab : a = b
  · subst hab; rw [if_pos rfl] at ht; cases ht; exact hr
  rw [if_neg hab] at ht
  simp only at ht
  -- the old parse
  unfold parseText at hr
  generalize hl : lex C a = la at hr
  rcases la with _ | ps
  · cases hr
  simp only at hr
  generalize hp : parse G T n (.n G.start) (ps.filterMap (fun p => term p.kind) ++ [eof]) = pa at hr
  rcases pa with _ | ⟨tr, w⟩
  · cases hr
  cases w with
  | cons _ _ => cases hr
  | nil =>
  simp only [Option.some.injEq] at hr
  subst hr
  simp only at ht
  -- the number token
  generalize hf : findNum (tokens 0 ps) (cpl a b) = F at ht
  rcases F with _ | ⟨xs, ⟨pc, lo⟩, ys⟩
  · cases ht
  simp only at ht
  obtain ⟨htok, hnumk, hlo⟩ := findNum_spec hf
  generalize h1 : litOf (a.drop lo) = O1 at ht
  generalize h2 : litOf (b.drop lo) = O2 at ht
  rcases O1 with _ | ⟨L₁, r₁⟩
  · simp at ht
  rcases O2 with _ | ⟨L₂, r₂⟩
  · simp at ht
  simp only at ht
  split at ht
  case isFalse => cases ht
  rename_i hc
  obtain ⟨hlen, hneg, hd, ht1, ht2, hh⟩ := hc
  simp only [Option.some.injEq] at ht
  subst ht
  generalize hq : min (cpl a.reverse b.reverse) (min a.length b.length - cpl a b) = q at hd ht1 ht2 hh ⊢
  have hq1 : q ≤ cpl a.reverse b.reverse := by rw [← hq]; exact Nat.min_le_left _ _
  have hcl := cpl_le a b
  have hq2 : q ≤ a.length ∧ q ≤ b.length := by
    have := Nat.min_le_right (cpl a.reverse b.reverse) (min a.length b.length - cpl a b)
    rw [hq] at this; constructor <;> omega
  -- the old pieces around the number
  obtain ⟨pre, post, hps, hpos, hxs, hys⟩ := tokens_split ps 0 htok
  have hloss := lex_lossless hl
  obtain ⟨u₁, hk⟩ : ∃ u, pc.kind = .num u := by
    cases h : pc.kind <;> simp [isNum, h] at hnumk ⊢
  obtain ⟨hok₁, hap₁, hst₁⟩ := litOf_spec h1
  obtain ⟨hok₂, hap₂, _⟩ := litOf_spec h2
  generalize hP : (pre.map Piece.text).flatten = P at hpos
  have hlo' : P.length = lo := by simpa using hpos
  have haP : a = P ++ (pc.text ++ (post.map Piece.text).flatten) := by
    rw [← hloss, hps, ← hP]; simp
  have hadrop : a.drop lo = pc.text ++ (post.map Piece.text).flatten := by
    rw [haP, ← hlo', List.drop_left]
  have hpcL : L₁.text = pc.text := prefix_eq_of_len hap₁ hadrop (by omega)
  have hpc : pc = ⟨.num u₁, L₁.text⟩ := by cases pc; simp_all
  (try rw [← hpcL] at hd); (try rw [← hpcL] at ht1); (try rw [← hpcL] at ht2); (try rw [← hpcL] at hh)
  try rw [← hpcL]
  have hlo_a : lo ≤ a.length := by omega
  have hlo_b : lo ≤ b.length := by omega
  have hr₁ : r₁ = a.drop (lo + L₁.text.length) := by rw [drop_of_app hap₁, List.drop_drop]
  have hr₂ : r₂ = b.drop (lo + L₂.text.length) := by rw [drop_of_app hap₂, List.drop_drop]
  have hla : lo + L₁.text.length ≤ a.length := by
    have := congrArg List.length hap₁; simp only [List.length_append, List.length_drop] at this; omega
  have hlb : lo + L₂.text.length ≤ b.length := by
    have := congrArg List.length hap₂; simp only [List.length_append, List.length_drop] at this; omega
  have hbP : b.take lo = P := by
    have := take_le_eq (cpl_take a b) hlo
    rw [← this, haP, ← hlo', List.take_left]
  -- the ends of the changed regions
  generalize hoe : max (a.length - q) (lo + L₁.text.length) = oe at hd ht1 ht2 hh ⊢
  generalize hne : max (b.length - q) (lo + L₂.text.length) = ne at hd ht1 ht2 hh ⊢
  have hoe1 : a.length - q ≤ oe ∧ lo + L₁.text.length ≤ oe ∧ oe ≤ a.length := by
    rw [← hoe]; exact ⟨Nat.le_max_left _ _, Nat.le_max_right _ _, Nat.max_le.2 ⟨by omega, hla⟩⟩
  have hne1 : b.length - q ≤ ne ∧ lo + L₂.text.length ≤ ne ∧ ne ≤ b.length := by
    rw [← hne]; exact ⟨Nat.le_max_left _ _, Nat.le_max_right _ _, Nat.max_le.2 ⟨by omega, hlb⟩⟩
  generalize hT₁ : (a.drop (lo + L₁.text.length)).take (oe - (lo + L₁.text.length)) = T₁ at ht1 hh
  generalize hT₂ : (b.drop (lo + L₂.text.length)).take (ne - (lo + L₂.text.length)) = T₂ at ht2 hh
  have hlT₁ : T₁.length = oe - (lo + L₁.text.length) := by
    rw [← hT₁, List.length_take, List.length_drop]; omega
  have hlT₂ : T₂.length = ne - (lo + L₂.text.length) := by
    rw [← hT₂, List.length_take, List.length_drop]; omega
  have hdA : a.drop (lo + L₁.text.length) = T₁ ++ a.drop oe := by
    rw [← hT₁]
    conv => lhs; rw [← List.take_append_drop (oe - (lo + L₁.text.length)) (a.drop (lo + L₁.text.length))]
    rw [List.drop_drop, Nat.add_sub_cancel' hoe1.2.1]
  have hdB : b.drop (lo + L₂.text.length) = T₂ ++ b.drop ne := by
    rw [← hT₂]
    conv => lhs; rw [← List.take_append_drop (ne - (lo + L₂.text.length)) (b.drop (lo + L₂.text.length))]
    rw [List.drop_drop, Nat.add_sub_cancel' hne1.2.1]
  -- the common suffix
  have hsuf := suffix_eq (take_le_eq (cpl_take a.reverse b.reverse) hq1)
  have hS : b.drop ne = a.drop oe := by
    have e1 : a.drop oe = (a.drop (a.length - q)).drop (oe - (a.length - q)) := by
      rw [List.drop_drop, Nat.add_sub_cancel' hoe1.1]
    have e2 : b.drop ne = (b.drop (b.length - q)).drop (ne - (b.length - q)) := by
      rw [List.drop_drop, Nat.add_sub_cancel' hne1.1]
    rw [e1, e2, hsuf]
    congr 1
    omega
  generalize hSd : a.drop oe = S at hdA hS
  rw [hS] at hdB
  have ha : a = P ++ L₁.text ++ T₁ ++ S := by
    rw [← List.take_append_drop lo a, haP, ← hlo', List.take_left, ← hadrop, ← hap₁, hr₁, hdA]
    simp
  have hb : b = P ++ L₂.text ++ T₂ ++ S := by
    rw [← List.take_append_drop lo b, hbP, ← hap₂, hr₂, hdB]
    simp
  have hedit : Edit T₁ T₂ S L₁ L₂ :=
    ⟨hok₁, hok₂, hneg, by rw [← hdB, ← hr₂]; exact (litOf_spec h2).2.2, triv_spec ht1, triv_spec ht2,
      Or.inl (by rw [← List.contains_iff_mem, ← List.contains_iff_mem, hh])⟩
  have hlex : lex C (P ++ L₁.text ++ T₁ ++ S) = .ok (pre ++ ⟨.num u₁, L₁.text⟩ :: post) := by
    rw [← ha, hl, hps, hpc]
  obtain ⟨ps', hnew, hterm, base₁, base₂, rest, e1, e2, e3⟩ := reparse_tokens hC hedit hlex hP
  unfold parseText
  rw [hb, hnew]
  simp only
  rw [hterm term htriv hnum, ← hpc, ← hps, hp]
  simp only [Option.some.injEq, Prod.mk.injEq, true_and]
  rw [← hpc, ← hps, htok, ← hxs, hlo'] at e1
  have hys' : ys = tokens base₁ rest := by
    have := List.append_cancel_left e1
    simp only [List.cons.injEq] at this
    exact this.2
  rw [e2, hxs, hlo', hys', ← Nat.zero_add base₁, tokens_shift 0 base₁, ← Nat.zero_add base₂, tokens_shift 0 base₂,
    List.map_map]
  congr 2
  refine List.map_congr_left fun x _ => ?_
  simp only [Function.comp, Prod.mk.injEq, true_and]
  simp only [List.length_append] at e3
  omega

end ForgeLang.LL1.Reparse

namespace ForgeLang.LL1.Impl
open Lex Reparse

/-- A text's parse on the parser's lexer and grammar. -/
def parseFg (n : Nat) (s : Lex.Bytes) : Option (Tree × List (Piece × Nat)) :=
  parseText lexCfg terminalOf tEOF grammar certT n s

/-- **`reparse_number` is correct** on the parser's grammar: if the model patches the old parse,
the patch is the new text's parse. -/
theorem reparse_number_correct {n : Nat} {a b : Lex.Bytes} {r t : Tree × List (Piece × Nat)}
    (hr : parseFg n a = some r) (ht : reparseNumber a b r = some t) : parseFg n b = some t :=
  reparse_correct lexCfg_digitFree terminalOf_trivia (fun _ _ => rfl) hr ht

private def bytes (s : String) : Lex.Bytes := s.toUTF8.toList.map (·.toNat)

private def patchOf (a b : String) : Option (Option (Tree × List (Piece × Nat))) :=
  (parseFg 10000 (bytes a)).map fun r => reparseNumber (bytes a) (bytes b) r

-- the cases that broke the Rust: refused here
#guard (patchOf "a = 5 #+ 2\nb = 1\n" "a = 6 + 2\nb = 1\n").map Option.isNone == some true
#guard (patchOf "a = 5 + 2\n" "a = 6 #+ 2\n").map Option.isNone == some true
#guard (patchOf "a = 5 #1+2\n" "a = 6 1+2\n").map Option.isNone == some true
#guard (patchOf "a = f(5, 2)\n" "a = f(6 #, 2)\n").map Option.isNone == some true
-- ordinary edits: patched, to the new parse
-- ordinary edits: patched, to the new parse's tokens (the tree is the theorem's business; `Tree` has no `BEq`)
#guard (patchOf "a = 5 + 2\nb = 1\n" "a = 50 + 2\nb = 1\n").map (fun t => t.isSome &&
  t.map (·.2) == (parseFg 10000 (bytes "a = 50 + 2\nb = 1\n")).map (·.2)) == some true
#guard (patchOf "shared t = 6mm # wall\nx = t\n" "shared t = 3.5mm # wall\nx = t\n").map (fun t => t.isSome &&
  t.map (·.2) == (parseFg 10000 (bytes "shared t = 3.5mm # wall\nx = t\n")).map (·.2)) == some true
#guard (patchOf "a = -5\n" "a = -12.25deg\n").map (fun t => t.isSome &&
  t.map (·.2) == (parseFg 10000 (bytes "a = -12.25deg\n")).map (·.2)) == some true

end ForgeLang.LL1.Impl

namespace ForgeLang.LL1.ImplUnifiedDoc
open Lex Reparse

theorem reparse_number_correct {n : Nat} {a b : Lex.Bytes} {r t : Tree × List (Piece × Nat)}
    (hr : parseText lexCfg terminalOf tEOF grammar certT n a = some r) (ht : reparseNumber a b r = some t) :
    parseText lexCfg terminalOf tEOF grammar certT n b = some t :=
  reparse_correct lexCfg_digitFree terminalOf_trivia (fun _ _ => rfl) hr ht

end ForgeLang.LL1.ImplUnifiedDoc
