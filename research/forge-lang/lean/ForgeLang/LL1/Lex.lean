/-!
# The lexer, as a function, and what P6 needs of it

A model of `lex_into` in interface/forge-lang/src/syntax.rs, on bytes. It is a function, so lexing
is deterministic. Each step takes a non-empty piece off the front of the input (a token or the
trivia between tokens), so:

* `lex_lossless`: the pieces, in order, are the input: whitespace and comments are kept, nothing
  is dropped or invented (with `LL1.lossless` this is the byte-identical-edit claim).
* `word_maximal`: a name or keyword is never followed by a name byte (maximal munch).
* `num_literal`: a literal of the shape SetParam writes (sign, digits, fraction, exponent, unit)
  lexes back as exactly one number token, whatever follows that cannot continue a number.
* `reparse_lex`: an edit of one plain number (the same sign) and of the trivia after it on its
  line, as `syntax::reparse_number` takes it, lexes to the same pieces before the number, the new
  number, trivia, and then the same pieces as before, each moved by the change in length. Where
  exactly one of the two trivia holds a `#`, this needs the rest of the line to be trivia too;
  without that the rest of a comment becomes code, or code a comment.

The keywords and operators are parameters (`Cfg`); `Impl.lean` holds the parser's own lists, and
tests/parse_diff.rs runs this model (`lake exe fglex`) against the Rust lexer.
-/

namespace ForgeLang.LL1.Lex

abbrev Bytes := List Nat

def isDigit (c : Nat) : Bool := 48 ≤ c && c ≤ 57
def isAlpha (c : Nat) : Bool := (65 ≤ c && c ≤ 90) || (97 ≤ c && c ≤ 122)
def isWord (c : Nat) : Bool := isAlpha c || isDigit c || c == 95
def isSpace (c : Nat) : Bool := c == 32 || c == 9 || c == 13

def NL : Nat := 10
def HASH : Nat := 35
def QUOTE : Nat := 34
def MINUS : Nat := 45
def DOT : Nat := 46

inductive Unit where
  | none | mm | deg
  deriving DecidableEq, Repr

inductive Kind where
  | num (u : Unit)
  | str
  | id
  | kw (k : Nat)
  | op (o : Nat)
  | nl
  | space
  | comment
  | newline
  deriving DecidableEq, Repr

inductive Err where
  | badNumber | unknownUnit | openString | badChar
  deriving DecidableEq, Repr

structure Cfg where
  kws : List Bytes
  /-- In the order `op_at` tries them: the longer ones first. -/
  ops : List Bytes

/-- A value ends with this token, so a `-` after it is an operator. -/
def Kind.isValue (C : Cfg) : Kind → Bool
  | .num _ | .str | .id => true
  | .kw k => C.kws[k]? == some [116, 114, 117, 101] || C.kws[k]? == some [102, 97, 108, 115, 101]
  | .op o => C.ops[o]? == some [41] || C.ops[o]? == some [93] || C.ops[o]? == some [125]
  | _ => false

/-! Scanning, each a split of the input: `(taken, rest)` with `taken ++ rest` the input. -/

def takeWhile (p : Nat → Bool) : Bytes → Bytes × Bytes
  | [] => ([], [])
  | c :: s => if p c then (c :: (takeWhile p s).1, (takeWhile p s).2) else ([], c :: s)

/-- Digits, and dots not followed by a dot. -/
def mantissa : Bytes → Bytes × Bytes
  | [] => ([], [])
  | c :: s =>
    if isDigit c || (c == DOT && s.head? != some DOT) then (c :: (mantissa s).1, (mantissa s).2)
    else ([], c :: s)

/-- `e` or `E`, then a digit or sign, then digits. -/
def exponent : Bytes → Bytes × Bytes
  | e :: d :: s =>
    if (e == 69 || e == 101) && (isDigit d || d == 45 || d == 43) then
      (e :: d :: (takeWhile isDigit s).1, (takeWhile isDigit s).2)
    else ([], e :: d :: s)
  | s => ([], s)

def upToQuote : Bytes → Bytes × Bytes := takeWhile fun c => c != QUOTE && c != NL

/-- What Rust's `str::parse::<f64>` takes from the scanned text (no sign, no inf or nan here):
at most one dot, and an exponent, if any, with a digit after its sign. -/
def floatOk (t : Bytes) : Bool :=
  let (m, e) := takeWhile (fun c => c != 69 && c != 101) t
  (m.filter (· == DOT)).length ≤ 1 && m.any isDigit &&
  match e with
  | [] => true
  | _ :: s => let s := match s with | c :: r => if c == 45 || c == 43 then r else c :: r | [] => []
              !s.isEmpty && s.all isDigit

def unitOf (u : Bytes) : Option Unit :=
  if u = [] then some .none else if u = [109, 109] then some .mm else if u = [100, 101, 103] then some .deg else none

/-- The first operator of `ops` the input starts with, and its index. -/
def opAt : Nat → List Bytes → Bytes → Option (Nat × Bytes)
  | _, [], _ => none
  | k, o :: os, s => if o ≠ [] ∧ o.isPrefixOf s then some (k, o) else opAt (k + 1) os s

def utf8Len (c : Nat) : Nat := if c < 128 then 1 else if c < 224 then 2 else if c < 240 then 3 else 4

/-- The mantissa of a number starting with byte `c`: with a leading `-` (`neg`), the digits after it. -/
def mant (c : Nat) (s : Bytes) : Bytes × Bytes :=
  if c == MINUS then (c :: (mantissa s).1, (mantissa s).2) else mantissa (c :: s)

/-- A number's bytes before its unit, and what follows them. -/
def numText (c : Nat) (s : Bytes) : Bytes := (mant c s).1 ++ (exponent (mant c s).2).1
def numRest (c : Nat) (s : Bytes) : Bytes := (exponent (mant c s).2).2
def numBody (c : Nat) (s : Bytes) : Bytes := if c == MINUS then (numText c s).tail else numText c s
def numUnit (c : Nat) (s : Bytes) : Bytes × Bytes := takeWhile isAlpha (numRest c s)

/-- A number at the front (a leading `-` already allowed): its unit, bytes and the rest. -/
def number (c : Nat) (s : Bytes) : Except (Err × Nat × Nat) (Kind × Bytes × Bytes) :=
  match floatOk (numBody c s), unitOf (numUnit c s).1 with
  | false, _ => .error (.badNumber, 0, (numText c s).length)
  | true, some un => .ok (.num un, numText c s ++ (numUnit c s).1, (numUnit c s).2)
  | true, none => .error (.unknownUnit, (numText c s).length, (numText c s).length + (numUnit c s).1.length)

def bump (o : Bytes) (depth : Nat) : Nat :=
  if o = [40] || o = [91] then depth + 1 else if o = [41] || o = [93] then depth - 1 else depth

/-- One piece off the front of the input `c :: s`: its kind, its bytes, what is left and the new
bracket depth; or an error with its span relative to here. -/
def step (C : Cfg) (prevValue : Bool) (depth : Nat) (c : Nat) (s : Bytes) :
    Except (Err × Nat × Nat) (Kind × Bytes × Bytes × Nat) :=
  if c == NL then .ok (if depth == 0 then .nl else .newline, [c], s, depth)
  else if isSpace c then .ok (.space, c :: (takeWhile isSpace s).1, (takeWhile isSpace s).2, depth)
  else if c == HASH then .ok (.comment, c :: (takeWhile (· != NL) s).1, (takeWhile (· != NL) s).2, depth)
  else if isDigit c || (c == MINUS && (s.head?.map isDigit).getD false && !prevValue) then
    match number c s with
    | .ok (k, t, r) => .ok (k, t, r, depth)
    | .error e => .error e
  else if isAlpha c || c == 95 then
    .ok (match C.kws.findIdx? (· == c :: (takeWhile isWord s).1) with | some k => .kw k | none => .id,
      c :: (takeWhile isWord s).1, (takeWhile isWord s).2, depth)
  else if c == QUOTE then
    match (upToQuote s).2 with
    | d :: b' => if d == QUOTE then .ok (.str, c :: (upToQuote s).1 ++ [d], b', depth)
                 else .error (.openString, 0, 1 + (upToQuote s).1.length)
    | [] => .error (.openString, 0, 1 + (upToQuote s).1.length)
  else match opAt 0 C.ops (c :: s) with
    | some (k, o) => .ok (.op k, o, (c :: s).drop o.length, bump o depth)
    | none => .error (.badChar, 0, utf8Len c)

structure Piece where
  kind : Kind
  text : Bytes
  deriving DecidableEq, Repr

def isTrivia : Kind → Bool
  | .space | .comment | .newline => true
  | _ => false

/-- Lex from byte offset `pos`, the last token's kind `prev`, bracket depth `depth`. -/
def go (C : Cfg) : Nat → Nat → Option Kind → Nat → Bytes → Except (Err × Nat × Nat) (List Piece)
  | 0, _, _, _, _ => .error (.badChar, 0, 0)
  | _ + 1, _, _, _, [] => .ok []
  | n + 1, pos, prev, depth, c :: s =>
    match step C ((prev.map (Kind.isValue C)).getD false) depth c s with
    | .error (e, lo, hi) => .error (e, pos + lo, pos + hi)
    | .ok (k, t, r, d) =>
      match go C n (pos + t.length) (if isTrivia k then prev else some k) d r with
      | .ok ps => .ok (⟨k, t⟩ :: ps)
      | .error e => .error e

def lex (C : Cfg) (s : Bytes) : Except (Err × Nat × Nat) (List Piece) := go C (s.length + 1) 0 none 0 s

/-! ## Every step takes a non-empty piece off the front -/

theorem takeWhile_app (p : Nat → Bool) : ∀ s, (takeWhile p s).1 ++ (takeWhile p s).2 = s
  | [] => rfl
  | c :: s => by
    by_cases h : p c = true
    · simp [takeWhile, h, takeWhile_app p s]
    · simp [takeWhile, h]

theorem mantissa_app : ∀ s, (mantissa s).1 ++ (mantissa s).2 = s
  | [] => rfl
  | c :: s => by
    by_cases h : (isDigit c || (c == DOT && s.head? != some DOT)) = true
    · simp only [mantissa, h, ite_true, List.cons_append, mantissa_app s]
    · simp only [mantissa, h, Bool.false_eq_true, ite_false, List.nil_append]

theorem exponent_app : ∀ s, (exponent s).1 ++ (exponent s).2 = s
  | e :: d :: s => by
    by_cases h : ((e == 69 || e == 101) && (isDigit d || d == 45 || d == 43)) = true
    · simp only [exponent, h, ite_true, List.cons_append, takeWhile_app]
    · simp only [exponent, h, Bool.false_eq_true, ite_false, List.nil_append]
  | [] => rfl
  | [_] => rfl

theorem mant_app (c : Nat) (s : Bytes) : (mant c s).1 ++ (mant c s).2 = c :: s := by
  unfold mant; split
  · simp [mantissa_app]
  · exact mantissa_app _

theorem mant_ne {c s} (hc : isDigit c = true ∨ c = MINUS) : (mant c s).1 ≠ [] := by
  unfold mant; split
  · simp
  · rename_i h
    have hd : isDigit c = true := by
      rcases hc with hc | hc
      · exact hc
      · subst hc; simp at h
    simp [mantissa, hd]

theorem number_ok {c s k t r} (h : number c s = .ok (k, t, r)) :
    ∃ u, k = .num u ∧ t = numText c s ++ (numUnit c s).1 ∧ r = (numUnit c s).2 := by
  unfold number at h
  cases hf : floatOk (numBody c s) <;> cases hu : unitOf (numUnit c s).1 <;> simp [hf, hu] at h
  obtain ⟨rfl, rfl, rfl⟩ := h
  exact ⟨_, rfl, rfl, rfl⟩

theorem number_split {c s k t r} (hc : isDigit c = true ∨ c = MINUS) (h : number c s = .ok (k, t, r)) :
    t ≠ [] ∧ t ++ r = c :: s := by
  obtain ⟨_, _, rfl, rfl⟩ := number_ok h
  refine ⟨by simp [numText, mant_ne hc], ?_⟩
  rw [List.append_assoc, numUnit, takeWhile_app, numRest, numText, List.append_assoc, exponent_app, mant_app]

theorem number_kind {c s k t r} (h : number c s = .ok (k, t, r)) : ∃ u, k = .num u :=
  let ⟨u, h, _⟩ := number_ok h; ⟨u, h⟩

theorem opAt_prefix : ∀ {k os s j o}, opAt k os s = some (j, o) → o ≠ [] ∧ o.isPrefixOf s = true
  | _, [], _, _, _, h => by simp [opAt] at h
  | k, o' :: os, s, j, o, h => by
    unfold opAt at h
    split at h
    · rename_i hc; cases h; exact hc
    · exact opAt_prefix h

theorem prefix_app {o s : Bytes} (h : o.isPrefixOf s = true) : o ++ s.drop o.length = s := by
  induction o generalizing s with
  | nil => simp
  | cons a o ih =>
    cases s with
    | nil => simp [List.isPrefixOf] at h
    | cons b s =>
      simp only [List.isPrefixOf, Bool.and_eq_true, beq_iff_eq] at h
      obtain ⟨rfl, h⟩ := h
      simp [ih h]

theorem upToQuote_app (s : Bytes) : (upToQuote s).1 ++ (upToQuote s).2 = s := takeWhile_app _ s

/-- A step's piece is non-empty and the input is the piece then the rest. -/
theorem step_split {C pv d c s k t r d'} (h : step C pv d c s = .ok (k, t, r, d')) : t ≠ [] ∧ t ++ r = c :: s := by
  unfold step at h
  split at h
  · cases h; simp
  split at h
  · cases h; simp [takeWhile_app]
  split at h
  · cases h; simp [takeWhile_app]
  split at h
  · rename_i hn
    have hc : isDigit c = true ∨ c = MINUS := by
      simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at hn
      rcases hn with h | ⟨⟨h, _⟩, _⟩
      · exact Or.inl h
      · exact Or.inr h
    split at h
    · rename_i k' t' r' hnum
      cases h
      exact number_split hc hnum
    · cases h
  split at h
  · cases h; simp [takeWhile_app]
  split at h
  · split at h
    · rename_i d0 b' hb
      split at h
      · rename_i hq
        cases h
        have := upToQuote_app s
        rw [hb] at this
        simp only [beq_iff_eq] at hq
        subst hq
        refine ⟨by simp, ?_⟩
        conv => rhs; rw [← this]
        simp
      · cases h
    · cases h
  · split at h
    · rename_i j o ho
      cases h
      obtain ⟨hne, hp⟩ := opAt_prefix ho
      exact ⟨hne, prefix_app hp⟩
    · cases h

/-! ## Lossless -/

theorem go_lossless {C} : ∀ {n pos prev depth s ps}, go C n pos prev depth s = .ok ps → (ps.map Piece.text).flatten = s
  | 0, _, _, _, _, _, h => by simp [go] at h
  | _ + 1, _, _, _, [], _, h => by simp [go] at h; subst h; rfl
  | n + 1, pos, prev, depth, c :: s, ps, h => by
    unfold go at h
    split at h
    · cases h
    · rename_i k t r d hs
      split at h
      · rename_i qs hq
        cases h
        simp only [List.map_cons, List.flatten_cons]
        rw [go_lossless hq]
        exact (step_split hs).2
      · cases h

/-- P6's lexer assumption: the tokens and the trivia between them are the source, byte for byte. -/
theorem lex_lossless {C s ps} (h : lex C s = .ok ps) : (ps.map Piece.text).flatten = s := go_lossless h

/-! ## Maximal munch for words -/

theorem takeWhile_stop (p : Nat → Bool) : ∀ s, ∀ c ∈ (takeWhile p s).2.head?, p c = false
  | [] => by simp [takeWhile]
  | c :: s => by
    by_cases h : p c = true
    · simpa [takeWhile, h] using takeWhile_stop p s
    · simp [takeWhile, h]

/-- A name or keyword piece takes every name byte there is: what follows does not start with one. -/
theorem word_maximal {C pv d c s k t r d'} (h : step C pv d c s = .ok (k, t, r, d'))
    (hk : k = .id ∨ ∃ j, k = .kw j) : ∀ x ∈ r.head?, isWord x = false := by
  have nk : ∀ {k'}, (k' = Kind.id ∨ ∃ j, k' = .kw j) → k' ≠ .nl ∧ k' ≠ .newline ∧ k' ≠ .space ∧ k' ≠ .comment ∧ k' ≠ .str ∧
      (∀ u, k' ≠ .num u) ∧ (∀ o, k' ≠ .op o) := by
    rintro k' (rfl | ⟨j, rfl⟩) <;> simp
  obtain ⟨h1, h2, h3, h4, h5, h6, h7⟩ := nk hk
  unfold step at h
  split at h
  · cases h; split at h1 <;> simp_all
  split at h
  · cases h; exact absurd rfl h3
  split at h
  · cases h; exact absurd rfl h4
  split at h
  · split at h
    · rename_i k' t' r' hnum
      cases h
      obtain ⟨u, hu⟩ := number_kind hnum
      exact absurd hu (h6 u)
    · cases h
  split at h
  · cases h; exact takeWhile_stop isWord s
  split at h
  · split at h
    · split at h
      · cases h; exact absurd rfl h5
      · cases h
    · cases h
  · split at h
    · cases h; exact absurd rfl (h7 _)
    · cases h

/-! ## SetParam's literals lex back as one number -/

/-- The shape of a literal SetParam writes: `-`? digits (`.` digits)? (`e` `-`? digits)? unit. -/
structure Lit where
  neg : Bool
  int : Bytes
  frac : Option Bytes
  exp : Option (Bool × Bytes)
  unit : Unit

def unitText : Unit → Bytes
  | .none => []
  | .mm => [109, 109]
  | .deg => [100, 101, 103]

def Lit.mant (l : Lit) : Bytes := l.int ++ (match l.frac with | some f => DOT :: f | none => [])
def Lit.expPart (l : Lit) : Bytes :=
  match l.exp with | some (n, e) => 101 :: (if n then [MINUS] else []) ++ e | none => []
def Lit.text (l : Lit) : Bytes := (if l.neg then [MINUS] else []) ++ l.mant ++ l.expPart ++ unitText l.unit

/-- Digits where digits are written. -/
def Lit.Ok (l : Lit) : Prop :=
  l.int ≠ [] ∧ l.int.all isDigit = true ∧ (∀ f, l.frac = some f → f ≠ [] ∧ f.all isDigit = true) ∧
  (∀ n e, l.exp = some (n, e) → e ≠ [] ∧ e.all isDigit = true)

/-- What follows cannot continue a number: not a digit, letter or dot. -/
def Stops (r : Bytes) : Prop := ∀ c ∈ r.head?, isDigit c = false ∧ isAlpha c = false ∧ c ≠ DOT

theorem takeWhile_all (p : Nat → Bool) : ∀ (a r : Bytes), a.all p = true → (∀ c ∈ r.head?, p c = false) →
    takeWhile p (a ++ r) = (a, r)
  | [], [], _, _ => rfl
  | [], c :: r, _, h => by simp at h; simp [takeWhile, h]
  | x :: a, r, ha, h => by
    simp only [List.all_cons, Bool.and_eq_true] at ha
    simp [takeWhile, ha.1, takeWhile_all p a r ha.2 h]

theorem mantissa_digits : ∀ (a s : Bytes), a.all isDigit = true → mantissa (a ++ s) = (a ++ (mantissa s).1, (mantissa s).2)
  | [], s, _ => by simp
  | x :: a, s, ha => by
    simp only [List.all_cons, Bool.and_eq_true] at ha
    simp [mantissa, ha.1, mantissa_digits a s ha.2]

theorem mantissa_stop {s : Bytes} (h : ∀ c ∈ s.head?, isDigit c = false ∧ c ≠ DOT) : mantissa s = ([], s) := by
  cases s with
  | nil => rfl
  | cons c s =>
    simp at h
    simp [mantissa, h.1, h.2]

theorem isDigit_ne_dot {c} (h : isDigit c = true) : c ≠ DOT := by
  intro e; subst e; simp [isDigit, DOT] at h

theorem mant_scan {l : Lit} (hl : l.Ok) {X : Bytes} (hX : ∀ c ∈ X.head?, isDigit c = false ∧ c ≠ DOT) :
    mantissa (l.mant ++ X) = (l.mant, X) := by
  obtain ⟨_, hi, hf, _⟩ := hl
  unfold Lit.mant
  rw [List.append_assoc, mantissa_digits _ _ hi]
  cases hfr : l.frac with
  | none => simp [mantissa_stop hX]
  | some f =>
    obtain ⟨hne, hfd⟩ := hf f hfr
    obtain ⟨d, f', rfl⟩ : ∃ d f', f = d :: f' := ⟨f.head hne, f.tail, by simp⟩
    simp only [List.all_cons, Bool.and_eq_true] at hfd
    have h1 : mantissa (d :: (f' ++ X)) = (d :: f', X) := by
      rw [show d :: (f' ++ X) = (d :: f') ++ X from rfl, mantissa_digits _ _ (by simp [hfd.1, hfd.2]), mantissa_stop hX]
      simp
    have hd46 : d ≠ DOT := isDigit_ne_dot hfd.1
    simp only [List.cons_append]
    rw [mantissa, if_pos (by simp [isDigit, DOT] at hd46 ⊢; omega), h1]

theorem unit_head (u : Unit) (r : Bytes) (hr : Stops r) :
    ∀ c ∈ (unitText u ++ r).head?, isDigit c = false ∧ c ≠ DOT := by
  intro c hc
  cases u with
  | none => have := hr c (by simpa [unitText] using hc); exact ⟨this.1, this.2.2⟩
  | mm => simp [unitText] at hc; subst hc; decide
  | deg => simp [unitText] at hc; subst hc; decide

theorem exp_scan {l : Lit} (hl : l.Ok) (Y : Bytes) (hY : ∀ c ∈ Y.head?, isDigit c = false) (hYe : ∀ c ∈ Y.head?, c ≠ 69 ∧ c ≠ 101) :
    exponent (l.expPart ++ Y) = (l.expPart, Y) := by
  obtain ⟨_, _, _, he⟩ := hl
  unfold Lit.expPart
  cases hx : l.exp with
  | none =>
    simp only [List.nil_append]
    match Y, hYe with
    | [], _ => rfl
    | [_], _ => rfl
    | c :: d :: s, h =>
      have := h c (by simp)
      simp [exponent, this.1, this.2]
  | some p =>
    obtain ⟨n, e⟩ := p
    obtain ⟨hne, hed⟩ := he n e hx
    obtain ⟨d, e', rfl⟩ : ∃ d e', e = d :: e' := ⟨e.head hne, e.tail, by simp⟩
    simp only [List.all_cons, Bool.and_eq_true] at hed
    cases n
    · simp [exponent, hed.1, takeWhile_all isDigit e' Y hed.2 hY]
    · have := takeWhile_all isDigit (d :: e') Y (by simp [hed.1, hed.2]) hY
      simp only [List.cons_append] at this
      simp [exponent, MINUS, this]

theorem floatOk_lit {l : Lit} (hl : l.Ok) : floatOk (l.mant ++ l.expPart) = true := by
  obtain ⟨hi0, hi, hf, he⟩ := hl
  have hm : (l.mant).all (fun c => c != 69 && c != 101) = true := by
    unfold Lit.mant
    have h1 : ∀ x ∈ l.int, x ≠ 69 ∧ x ≠ 101 := fun x hx => by
      have := List.all_eq_true.1 hi x hx; simp [isDigit] at this; omega
    cases hfr : l.frac with
    | none => simp; intro x hx; exact h1 x hx
    | some f =>
      have h2 : ∀ x ∈ f, x ≠ 69 ∧ x ≠ 101 := fun x hx => by
        have := List.all_eq_true.1 (hf f hfr).2 x hx; simp [isDigit] at this; omega
      simp [DOT]
      exact ⟨fun x hx => h1 x hx, fun x hx => h2 x hx⟩
  have hsplit : takeWhile (fun c => c != 69 && c != 101) (l.mant ++ l.expPart) = (l.mant, l.expPart) := by
    apply takeWhile_all _ _ _ hm
    unfold Lit.expPart; cases l.exp <;> simp
  unfold floatOk
  rw [hsplit]
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  refine ⟨⟨?_, ?_⟩, ?_⟩
  · unfold Lit.mant
    have h0 : (l.int.filter (· == DOT)) = [] := by
      rw [List.filter_eq_nil_iff]; intro x hx
      have := List.all_eq_true.1 hi x hx; simp [isDigit, DOT] at this ⊢; omega
    cases hfr : l.frac with
    | none => simp [h0]
    | some f =>
      have h3 : (f.filter (· == DOT)) = [] := by
        rw [List.filter_eq_nil_iff]; intro x hx
        have := List.all_eq_true.1 (hf f hfr).2 x hx; simp [isDigit, DOT] at this ⊢; omega
      have e0 := congrArg List.length h0
      have e3 := congrArg List.length h3
      simp only [DOT, List.length_nil] at e0 e3
      simp [List.filter_append, DOT, e0, e3]
  · unfold Lit.mant
    obtain ⟨x, xs, hx⟩ : ∃ x xs, l.int = x :: xs := ⟨l.int.head hi0, l.int.tail, by simp⟩
    rw [hx] at hi ⊢
    simp only [List.all_cons, Bool.and_eq_true] at hi
    simp [hi.1]
  · unfold Lit.expPart
    cases hx : l.exp with
    | none => rfl
    | some p =>
      obtain ⟨n, e⟩ := p
      obtain ⟨hne, hed⟩ := he n e hx
      obtain ⟨d, e', rfl⟩ : ∃ d e', e = d :: e' := ⟨e.head hne, e.tail, by simp⟩
      simp only [List.all_cons, Bool.and_eq_true] at hed
      have hd : ¬ (d = 45 ∨ d = 43) := by have := hed.1; simp [isDigit] at this; omega
      cases n <;> simp [MINUS, hed.1, hed.2, hd]

/-- SetParam's literals lex back as one token: a literal of `Lit`'s shape, followed by anything
that cannot continue a number, is one number piece with its unit, and the rest is left as it was.
A negative literal needs a place where no value precedes it (a literal position: after `=`, `(`,
`,`, an operator or a keyword). -/
theorem num_literal (C : Cfg) (pv : Bool) (d : Nat) (l : Lit) (hl : l.Ok) (r : Bytes) (hr : Stops r)
    (hpv : l.neg = true → pv = false) :
    ∃ c s, l.text ++ r = c :: s ∧ step C pv d c s = .ok (.num l.unit, l.text, r, d) := by
  have hY : ∀ c ∈ (unitText l.unit ++ r).head?, isDigit c = false := fun c hc => (unit_head _ r hr c hc).1
  have hYe : ∀ c ∈ (unitText l.unit ++ r).head?, c ≠ 69 ∧ c ≠ 101 := by
    cases hu : l.unit <;> simp [unitText]
    intro c hc; have := (hr c (by simp [hc])).2.1; simp [isAlpha] at this; omega
  have hE := exp_scan hl _ hY hYe
  have hX : ∀ c ∈ (l.expPart ++ (unitText l.unit ++ r)).head?, isDigit c = false ∧ c ≠ DOT := by
    unfold Lit.expPart; cases l.exp
    · exact unit_head _ r hr
    · simp [isDigit, DOT]
  have hM := mant_scan hl hX
  have hU : takeWhile isAlpha (unitText l.unit ++ r) = (unitText l.unit, r) := by
    apply takeWhile_all
    · cases l.unit <;> simp [unitText, isAlpha]
    · intro c hc; exact (hr c hc).2.1
  have hunit : unitOf (unitText l.unit) = some l.unit := by cases l.unit <;> rfl
  obtain ⟨hi0, hi, _, _⟩ := hl
  obtain ⟨x, xs, hx⟩ : ∃ x xs, l.int = x :: xs := ⟨l.int.head hi0, l.int.tail, by simp⟩
  have hxd : isDigit x = true := by rw [hx] at hi; simp at hi; exact hi.1
  have hm1 : l.mant = x :: (xs ++ (match l.frac with | some f => DOT :: f | none => [])) := by
    simp [Lit.mant, hx]
  have nNL : ∀ {c}, isDigit c = true ∨ c = MINUS → (c == NL) = false ∧ isSpace c = false ∧ (c == HASH) = false := by
    intro c hc; rcases hc with h | rfl
    · simp [isDigit] at h; simp [NL, isSpace, HASH]; omega
    · simp [NL, isSpace, HASH, MINUS]
  cases hn : l.neg
  · -- the literal starts with its first digit
    refine ⟨x, xs ++ (match l.frac with | some f => DOT :: f | none => []) ++ l.expPart ++ unitText l.unit ++ r, ?_, ?_⟩
    · simp [Lit.text, hn, hm1]
    · obtain ⟨a1, a2, a3⟩ := nNL (Or.inl hxd)
      have hrest : x :: (xs ++ (match l.frac with | some f => DOT :: f | none => []) ++ l.expPart ++ unitText l.unit ++ r) =
          l.mant ++ (l.expPart ++ (unitText l.unit ++ r)) := by simp [hm1]
      have hmant : mant x (xs ++ (match l.frac with | some f => DOT :: f | none => []) ++ l.expPart ++ unitText l.unit ++ r) =
          (l.mant, l.expPart ++ (unitText l.unit ++ r)) := by
        have : (x == MINUS) = false := by simp [isDigit] at hxd; simp [MINUS]; omega
        simp only [mant, this, Bool.false_eq_true, ite_false]
        rw [hrest, hM]
      have hnum : number x (xs ++ (match l.frac with | some f => DOT :: f | none => []) ++ l.expPart ++ unitText l.unit ++ r) =
          .ok (.num l.unit, l.text, r) := by
        have hb : (x == MINUS) = false := by simp [isDigit] at hxd; simp [MINUS]; omega
        unfold number numBody numUnit numRest numText
        rw [hmant]
        simp only [hE, hb, Bool.false_eq_true, ite_false, floatOk_lit ⟨hi0, hi, by assumption, by assumption⟩, hU, hunit]
        simp [Lit.text, hn]
      unfold step
      simp only [a1, a2, a3, hxd, Bool.true_or, Bool.false_eq_true, ite_false, ite_true, hnum]
  · -- `-`, then the digits
    have hpv' := hpv hn
    refine ⟨MINUS, l.mant ++ l.expPart ++ unitText l.unit ++ r, ?_, ?_⟩
    · simp [Lit.text, hn]
    · obtain ⟨a1, a2, a3⟩ := nNL (Or.inr rfl)
      have hmant : mant MINUS (l.mant ++ l.expPart ++ unitText l.unit ++ r) =
          (MINUS :: l.mant, l.expPart ++ (unitText l.unit ++ r)) := by
        simp only [mant, beq_self_eq_true, ite_true, List.append_assoc]
        rw [hM]
      have hnum : number MINUS (l.mant ++ l.expPart ++ unitText l.unit ++ r) = .ok (.num l.unit, l.text, r) := by
        unfold number numBody numUnit numRest numText
        rw [hmant]
        simp only [hE, beq_self_eq_true, ite_true, List.cons_append, List.tail_cons,
          floatOk_lit ⟨hi0, hi, by assumption, by assumption⟩, hU, hunit]
        simp [Lit.text, hn]
      have hg : (isDigit MINUS || (MINUS == MINUS && ((l.mant ++ l.expPart ++ unitText l.unit ++ r).head?.map isDigit).getD false && !pv)) = true := by
        simp [hpv', hm1, hxd]
      unfold step
      simp only [a1, a2, a3, hg, Bool.false_eq_true, ite_false, ite_true, hnum]

/-! ## An edit of one number: what the lexer gives the new text -/

/-- The input starts with a digit. -/
def DStart (y : Bytes) : Prop := ∃ a t, y = a :: t ∧ isDigit a = true

/-- Two inputs that start a number alike: both with a digit, or both with `-` then a digit. -/
def Res (Y Y' : Bytes) : Prop :=
  (DStart Y ∧ DStart Y') ∨ (∃ y y', Y = MINUS :: y ∧ Y' = MINUS :: y' ∧ DStart y ∧ DStart y')

/-- A test that does not tell digits apart. -/
def DU (p : Nat → Bool) : Prop := ∀ x y, isDigit x = true → isDigit y = true → p x = p y

/-- No operator has a digit in it. -/
def DigitFree (C : Cfg) : Prop := ∀ o ∈ C.ops, ∀ x ∈ o, isDigit x = false

theorem res_head {Y Y'} (h : Res Y Y') :
    ∃ a a' y y', Y = a :: y ∧ Y' = a' :: y' ∧ (a = a' ∨ (isDigit a = true ∧ isDigit a' = true)) ∧
      (isDigit a = true ∨ a = MINUS) := by
  rcases h with ⟨⟨a, y, rfl, ha⟩, ⟨a', y', rfl, ha'⟩⟩ | ⟨y, y', rfl, rfl, _, _⟩
  · exact ⟨a, a', y, y', rfl, rfl, Or.inr ⟨ha, ha'⟩, Or.inl ha⟩
  · exact ⟨MINUS, MINUS, y, y', rfl, rfl, Or.inl rfl, Or.inr rfl⟩

theorem res_du {p} (hp : DU p) {a a'} (h : a = a' ∨ (isDigit a = true ∧ isDigit a' = true)) : p a = p a' := by
  rcases h with rfl | ⟨h1, h2⟩
  · rfl
  · exact hp _ _ h1 h2

theorem du_isDigit : DU isDigit := fun _ _ h1 h2 => by rw [h1, h2]

theorem du_of_ne {p : Nat → Bool} (h : ∀ x, isDigit x = true → p x = false) : DU p :=
  fun x y hx hy => by rw [h x hx, h y hy]

theorem digit_ne {x c : Nat} (hx : isDigit x = true) (hc : isDigit c = false) : (x == c) = false := by
  by_cases h : x = c
  · subst h; rw [hx] at hc; cases hc
  · simp [h]

theorem res_not {Y Y'} (h : Res Y Y') {c : Nat} (hc : isDigit c = false) (hm : c ≠ MINUS) :
    ∃ a a' y y', Y = a :: y ∧ Y' = a' :: y' ∧ (a == c) = false ∧ (a' == c) = false := by
  obtain ⟨a, a', y, y', rfl, rfl, he, hd⟩ := res_head h
  refine ⟨a, a', y, y', rfl, rfl, ?_, ?_⟩
  · rcases hd with hd | rfl
    · exact digit_ne hd hc
    · simpa using fun e => hm e.symm
  · rcases he with rfl | ⟨_, hd'⟩
    · rcases hd with hd | rfl
      · exact digit_ne hd hc
      · simpa using fun e => hm e.symm
    · exact digit_ne hd' hc

/-! Each scan, on `r₀ ++ Y` and `r₀ ++ Y'`, where what it takes lies in `r₀`. -/

theorem tw_res {p} (hp : DU p) {Y Y'} (hY : Res Y Y') : ∀ (r₀ a b : Bytes), takeWhile p (r₀ ++ Y) = (a, b) →
    a.length ≤ r₀.length → ∃ r₁, r₀ = a ++ r₁ ∧ b = r₁ ++ Y ∧ takeWhile p (r₀ ++ Y') = (a, r₁ ++ Y')
  | [], a, b, h, hl => by
    obtain ⟨c, c', y, y', rfl, rfl, he, _⟩ := res_head hY
    have ha : a = [] := List.eq_nil_of_length_eq_zero (by simpa using hl)
    subst ha
    have hpc : p c = false := by
      cases hq : p c
      · rfl
      · simp [takeWhile, hq] at h
    have hpc' : p c' = false := by rw [← res_du hp he]; exact hpc
    simp only [List.nil_append, takeWhile, hpc, Bool.false_eq_true, ite_false, Prod.mk.injEq] at h
    exact ⟨[], rfl, by simp [h.2], by simp [takeWhile, hpc']⟩
  | x :: r₀, a, b, h, hl => by
    by_cases hx : p x = true
    · rw [List.cons_append, takeWhile.eq_2, ite_eq_left hx] at h
      obtain ⟨ha, hb⟩ := Prod.mk.inj h
      subst ha
      obtain ⟨r₁, h1, h2, h3⟩ := tw_res hp hY r₀ _ _ rfl (by simpa using hl)
      refine ⟨r₁, by rw [List.cons_append]; exact congrArg _ h1, by rw [← hb, h2], ?_⟩
      rw [List.cons_append, takeWhile.eq_2, ite_eq_left hx, h3]
    · rw [List.cons_append, takeWhile.eq_2, ite_eq_right hx] at h
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj h
      exact ⟨x :: r₀, rfl, rfl, by rw [List.cons_append, takeWhile.eq_2, ite_eq_right hx]⟩

theorem head_dot {Y Y'} (hY : Res Y Y') (r₀ : Bytes) :
    ((r₀ ++ Y).head? != some DOT) = ((r₀ ++ Y').head? != some DOT) := by
  cases r₀ with
  | cons x r => rfl
  | nil =>
    obtain ⟨a, a', y, y', rfl, rfl, ha, ha'⟩ := res_not hY (c := DOT) (by decide) (by decide)
    show (some a != some DOT) = (some a' != some DOT)
    have e : ∀ z : Nat, (some z != some DOT) = !(z == DOT) := fun z => rfl
    rw [e, e, ha, ha']

theorem mantissa_minus (y : Bytes) : mantissa (MINUS :: y) = ([], MINUS :: y) := by
  simp [mantissa.eq_2, isDigit, MINUS, DOT]

theorem mantissa_res {Y Y'} (hY : Res Y Y') : ∀ (r₀ a b : Bytes), mantissa (r₀ ++ Y) = (a, b) →
    a.length ≤ r₀.length → ∃ r₁, r₀ = a ++ r₁ ∧ b = r₁ ++ Y ∧ mantissa (r₀ ++ Y') = (a, r₁ ++ Y')
  | [], a, b, h, hl => by
    have ha : a = [] := List.eq_nil_of_length_eq_zero (by simpa using hl)
    subst ha
    obtain ⟨c, c', y, y', rfl, rfl, he, hd⟩ := res_head hY
    simp only [List.nil_append] at h ⊢
    rcases hd with hd | rfl
    · simp [mantissa.eq_2, hd] at h
    · rcases he with he | ⟨h1, _⟩
      · subst he
        rw [mantissa_minus] at h ⊢
        cases h
        exact ⟨[], rfl, rfl, rfl⟩
      · simp [isDigit, MINUS] at h1
  | x :: r₀, a, b, h, hl => by
    have hd := head_dot hY r₀
    by_cases hx : (isDigit x || (x == DOT && (r₀ ++ Y).head? != some DOT)) = true
    · have hx' : (isDigit x || (x == DOT && (r₀ ++ Y').head? != some DOT)) = true := by rw [← hd]; exact hx
      rw [List.cons_append, mantissa.eq_2, ite_eq_left hx] at h
      obtain ⟨ha, hb⟩ := Prod.mk.inj h
      subst ha
      obtain ⟨r₁, h1, h2, h3⟩ := mantissa_res hY r₀ _ _ rfl (by simpa using hl)
      refine ⟨r₁, by rw [List.cons_append]; exact congrArg _ h1, by rw [← hb, h2], ?_⟩
      rw [List.cons_append, mantissa.eq_2, ite_eq_left hx', h3]
    · have hx' : ¬ (isDigit x || (x == DOT && (r₀ ++ Y').head? != some DOT)) = true := by rw [← hd]; exact hx
      simp only [List.cons_append, mantissa.eq_2, hx, Bool.false_eq_true, ite_false, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact ⟨x :: r₀, rfl, rfl, by rw [List.cons_append, mantissa.eq_2, ite_eq_right hx']⟩

theorem du_of {p : Nat → Bool} (b : Bool) (h : ∀ x, isDigit x = true → p x = b) : DU p :=
  fun x y hx hy => by rw [h x hx, h y hy]

theorem ne_digit {x a : Nat} (hx : isDigit x = false) (ha : isDigit a = true) : (x == a) = false := by
  by_cases h : x = a
  · subst h; rw [hx] at ha; cases ha
  · simp [h]

theorem head_du {p} (hp : DU p) {Y Y'} (hY : Res Y Y') (r₀ : Bytes) :
    (r₀ ++ Y).head?.map p = (r₀ ++ Y').head?.map p := by
  cases r₀ with
  | cons x r => rfl
  | nil =>
    obtain ⟨a, a', y, y', rfl, rfl, he, _⟩ := res_head hY
    simp [res_du hp he]

theorem exponent_nonE {c : Nat} (s : Bytes) (h1 : c ≠ 69) (h2 : c ≠ 101) : exponent (c :: s) = ([], c :: s) := by
  cases s <;> simp [exponent, h1, h2]

theorem res_nonE {Y Y'} (hY : Res Y Y') : exponent Y = ([], Y) ∧ exponent Y' = ([], Y') := by
  obtain ⟨a, a', y, y', rfl, rfl, he, hd⟩ := res_head hY
  have ne : ∀ {z}, (isDigit z = true ∨ z = MINUS) → z ≠ 69 ∧ z ≠ 101 := by
    intro z hz; rcases hz with hz | rfl
    · simp [isDigit] at hz; omega
    · decide
  have h' : isDigit a' = true ∨ a' = MINUS := by
    rcases he with rfl | ⟨_, h⟩
    · exact hd
    · exact Or.inl h
  exact ⟨exponent_nonE _ (ne hd).1 (ne hd).2, exponent_nonE _ (ne h').1 (ne h').2⟩

theorem exponent_res {Y Y'} (hY : Res Y Y') (r₀ a b : Bytes) (h : exponent (r₀ ++ Y) = (a, b))
    (hl : a.length ≤ r₀.length) : ∃ r₁, r₀ = a ++ r₁ ∧ b = r₁ ++ Y ∧ exponent (r₀ ++ Y') = (a, r₁ ++ Y') := by
  match r₀, hl, h with
  | [], hl, h =>
    obtain ⟨e1, e2⟩ := res_nonE hY
    simp only [List.nil_append] at h ⊢
    rw [e1] at h
    obtain ⟨rfl, rfl⟩ := Prod.mk.inj h
    exact ⟨[], rfl, rfl, e2⟩
  | [x], hl, h =>
    obtain ⟨c, c', y, y', rfl, rfl, he, hd⟩ := res_head hY
    have hc : (isDigit c || c == 45 || c == 43) = (isDigit c' || c' == 45 || c' == 43) := by
      rcases he with rfl | ⟨h1, h2⟩
      · rfl
      · have a1 : (c == 45) = false := digit_ne h1 (by decide)
        have a2 : (c == 43) = false := digit_ne h1 (by decide)
        have a3 : (c' == 45) = false := digit_ne h2 (by decide)
        have a4 : (c' == 43) = false := digit_ne h2 (by decide)
        rw [h1, h2, a1, a2, a3, a4]
    simp only [List.cons_append, List.nil_append] at h ⊢
    by_cases hx : ((x == 69 || x == 101) && (isDigit c || c == 45 || c == 43)) = true
    · simp only [exponent, hx, ite_true] at h
      obtain ⟨rfl, _⟩ := Prod.mk.inj h
      simp at hl
    · have hx' : ¬ ((x == 69 || x == 101) && (isDigit c' || c' == 45 || c' == 43)) = true := by rw [← hc]; exact hx
      simp only [exponent, hx, Bool.false_eq_true, ite_false] at h
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj h
      exact ⟨[x], rfl, rfl, by simp only [exponent, hx', Bool.false_eq_true, ite_false]; rfl⟩
  | x :: z :: r, hl, h =>
    simp only [List.cons_append] at h ⊢
    by_cases hx : ((x == 69 || x == 101) && (isDigit z || z == 45 || z == 43)) = true
    · simp only [exponent, hx, ite_true] at h ⊢
      obtain ⟨ha, hb⟩ := Prod.mk.inj h
      subst ha
      obtain ⟨r₁, h1, h2, h3⟩ := tw_res du_isDigit hY r _ _ rfl (by simp at hl; omega)
      refine ⟨r₁, by simp [← h1], by rw [← hb, h2], by rw [h3]⟩
    · simp only [exponent, hx, Bool.false_eq_true, ite_false] at h ⊢
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj h
      exact ⟨x :: z :: r, rfl, rfl, rfl⟩

theorem prefix_res {Y Y'} (hY : Res Y Y') : ∀ (o r₀ : Bytes), (∀ x ∈ o, isDigit x = false) →
    o.isPrefixOf (r₀ ++ Y) = o.isPrefixOf (r₀ ++ Y')
  | [], _, _ => by simp [List.isPrefixOf]
  | x :: o, w :: r₀, ho => by
    simp only [List.cons_append, List.isPrefixOf]
    rw [prefix_res hY o r₀ (fun z hz => ho z (by simp [hz]))]
  | x :: o, [], ho => by
    have hx := ho x (by simp)
    rcases hY with ⟨⟨a, t, rfl, ha⟩, ⟨a', t', rfl, ha'⟩⟩ | ⟨y, y', rfl, rfl, ⟨a, t, rfl, ha⟩, ⟨a', t', rfl, ha'⟩⟩
    · simp [List.isPrefixOf, ne_digit hx ha, ne_digit hx ha']
    · cases o with
      | nil => simp [List.isPrefixOf]
      | cons z o =>
        have hz := ho z (by simp)
        simp [List.isPrefixOf, ne_digit hz ha, ne_digit hz ha']

theorem opAt_res {Y Y'} (hY : Res Y Y') (r₀ : Bytes) : ∀ (os : List Bytes) (k : Nat),
    (∀ o ∈ os, ∀ x ∈ o, isDigit x = false) → opAt k os (r₀ ++ Y) = opAt k os (r₀ ++ Y')
  | [], _, _ => rfl
  | o :: os, k, hC => by
    simp only [opAt]
    rw [prefix_res hY o r₀ (hC o (by simp)), opAt_res hY r₀ os (k + 1) (fun o' h => hC o' (by simp [h]))]

theorem mant_res {Y Y'} (hY : Res Y Y') (c : Nat) (r₀ a b : Bytes) (h : mant c (r₀ ++ Y) = (a, b))
    (hl : a.length ≤ r₀.length + 1) :
    ∃ r₁, c :: r₀ = a ++ r₁ ∧ b = r₁ ++ Y ∧ mant c (r₀ ++ Y') = (a, r₁ ++ Y') := by
  unfold mant at h ⊢
  by_cases hm : (c == MINUS) = true
  · rw [ite_eq_left hm] at h ⊢
    obtain ⟨ha, hb⟩ := Prod.mk.inj h
    subst ha
    obtain ⟨r₁, h1, h2, h3⟩ := mantissa_res hY r₀ _ _ rfl (by simpa using hl)
    exact ⟨r₁, by rw [List.cons_append]; exact congrArg _ h1, by rw [← hb, h2], by rw [h3]⟩
  · rw [ite_eq_right hm] at h ⊢
    exact mantissa_res hY (c :: r₀) a b h hl

theorem du_isAlpha : DU isAlpha := du_of false fun x hx => by simp [isDigit] at hx; simp [isAlpha]; omega

theorem number_res {Y Y'} (hY : Res Y Y') {c : Nat} {r₀ : Bytes} {k t r}
    (h : number c (r₀ ++ Y) = .ok (k, t, r)) (hl : t.length ≤ r₀.length + 1) :
    ∃ r₁, c :: r₀ = t ++ r₁ ∧ r = r₁ ++ Y ∧ number c (r₀ ++ Y') = .ok (k, t, r₁ ++ Y') := by
  obtain ⟨u, rfl, rfl, rfl⟩ := number_ok h
  generalize hm : mant c (r₀ ++ Y) = m
  obtain ⟨ma, mb⟩ := m
  generalize he : exponent mb = e
  obtain ⟨ea, eb⟩ := e
  generalize hu : takeWhile isAlpha eb = w
  obtain ⟨ua, ub⟩ := w
  have ht : numText c (r₀ ++ Y) ++ (numUnit c (r₀ ++ Y)).1 = ma ++ ea ++ ua := by
    simp [numText, numUnit, numRest, hm, he, hu]
  have hr : (numUnit c (r₀ ++ Y)).2 = ub := by simp [numUnit, numRest, hm, he, hu]
  rw [ht] at hl ⊢
  rw [hr]
  simp only [List.length_append] at hl
  obtain ⟨r₁, h1, h2, h3⟩ := mant_res hY c r₀ ma mb hm (by omega)
  subst h2
  have hl1 : (c :: r₀).length = ma.length + r₁.length := by rw [h1, List.length_append]
  obtain ⟨r₂, h4, h5, h6⟩ := exponent_res hY r₁ ea eb he (by simp at hl1; omega)
  subst h5
  have hl2 : r₁.length = ea.length + r₂.length := by rw [h4, List.length_append]
  obtain ⟨r₃, h7, h8, h9⟩ := tw_res du_isAlpha hY r₂ ua ub hu (by simp at hl1; omega)
  refine ⟨r₃, by rw [h1, h4, h7]; simp, h8, ?_⟩
  have nt : numText c (r₀ ++ Y') = numText c (r₀ ++ Y) := by simp [numText, hm, he, h3, h6]
  have nu : numUnit c (r₀ ++ Y') = (ua, r₃ ++ Y') := by simp [numUnit, numRest, h3, h6, h9]
  have nb : numBody c (r₀ ++ Y') = numBody c (r₀ ++ Y) := by simp [numBody, nt]
  have nu0 : (numUnit c (r₀ ++ Y)).1 = ua := by simp [numUnit, numRest, hm, he, hu]
  unfold number at h ⊢
  rw [nb, nu, nt]
  rw [nu0] at h
  revert h
  have ntv : numText c (r₀ ++ Y) = ma ++ ea := by simp [numText, hm, he]
  cases floatOk (numBody c (r₀ ++ Y)) <;> cases unitOf ua <;> simp [ntv]

theorem du_isSpace : DU isSpace := du_of false fun x hx => by simp [isDigit] at hx; simp [isSpace]; omega
theorem du_notNL : DU (· != NL) := du_of true fun x hx => by simp [isDigit] at hx; simp [NL]; omega
theorem du_isWord : DU isWord := du_of true fun x hx => by simp [isWord, hx]
theorem du_quote : DU (fun c => c != QUOTE && c != NL) :=
  du_of true fun x hx => by simp [isDigit] at hx; simp [QUOTE, NL]; omega

theorem step_res {C : Cfg} (hC : DigitFree C) {Y Y'} (hY : Res Y Y') {pv d c} {r₀ : Bytes} {k t r d'}
    (h : step C pv d c (r₀ ++ Y) = .ok (k, t, r, d')) (hl : t.length ≤ r₀.length + 1) :
    ∃ r₁, c :: r₀ = t ++ r₁ ∧ r = r₁ ++ Y ∧ step C pv d c (r₀ ++ Y') = .ok (k, t, r₁ ++ Y', d') := by
  unfold step at h ⊢
  by_cases h1 : (c == NL) = true
  · rw [ite_eq_left h1] at h ⊢
    simp only [Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    exact ⟨r₀, rfl, rfl, rfl⟩
  rw [ite_eq_right h1] at h ⊢
  by_cases h2 : isSpace c = true
  · rw [ite_eq_left h2] at h ⊢
    simp only [Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    obtain ⟨r₁, e1, e2, e3⟩ := tw_res du_isSpace hY r₀ _ _ rfl (by simpa using hl)
    exact ⟨r₁, by rw [List.cons_append]; exact congrArg _ e1, e2, by rw [e3]⟩
  rw [ite_eq_right h2] at h ⊢
  by_cases h3 : (c == HASH) = true
  · rw [ite_eq_left h3] at h ⊢
    simp only [Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    obtain ⟨r₁, e1, e2, e3⟩ := tw_res du_notNL hY r₀ _ _ rfl (by simpa using hl)
    exact ⟨r₁, by rw [List.cons_append]; exact congrArg _ e1, e2, by rw [e3]⟩
  rw [ite_eq_right h3] at h ⊢
  have hhd := head_du du_isDigit hY r₀
  by_cases h4 : (isDigit c || (c == MINUS && ((r₀ ++ Y).head?.map isDigit).getD false && !pv)) = true
  · have h4' : (isDigit c || (c == MINUS && ((r₀ ++ Y').head?.map isDigit).getD false && !pv)) = true := by
      rw [← hhd]; exact h4
    rw [ite_eq_left h4] at h
    rw [ite_eq_left h4']
    generalize hn : number c (r₀ ++ Y) = res at h
    rcases res with e | ⟨k₁, t₁, r₂⟩
    · cases h
    · simp only [Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl, rfl, rfl⟩ := h
      obtain ⟨r₁, e1, e2, e3⟩ := number_res hY hn hl
      exact ⟨r₁, e1, e2, by rw [e3]⟩
  have h4' : ¬ (isDigit c || (c == MINUS && ((r₀ ++ Y').head?.map isDigit).getD false && !pv)) = true := by
    rw [← hhd]; exact h4
  rw [ite_eq_right h4] at h
  rw [ite_eq_right h4']
  by_cases h5 : (isAlpha c || c == 95) = true
  · rw [ite_eq_left h5] at h ⊢
    simp only [Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    obtain ⟨r₁, e1, e2, e3⟩ := tw_res du_isWord hY r₀ _ _ rfl (by simpa using hl)
    exact ⟨r₁, by rw [List.cons_append]; exact congrArg _ e1, e2, by rw [e3]⟩
  rw [ite_eq_right h5] at h ⊢
  by_cases h6 : (c == QUOTE) = true
  · rw [ite_eq_left h6] at h ⊢
    generalize hq : upToQuote (r₀ ++ Y) = w at h
    obtain ⟨wa, wb⟩ := w
    have hq' := hq
    unfold upToQuote at hq'
    cases wb with
    | nil => cases h
    | cons x b' =>
      by_cases hx : (x == QUOTE) = true
      · simp only [ite_eq_left hx, Except.ok.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl, rfl⟩ := h
        simp only [List.length_cons, List.length_append] at hl
        obtain ⟨r₁, e1, e2, e3⟩ := tw_res du_quote hY r₀ _ _ hq' (by omega)
        have hr1 : r₁ ≠ [] := by
          intro e; subst e; simp at e1; subst e1; omega
        obtain ⟨y, r₁', rfl⟩ : ∃ y r₁', r₁ = y :: r₁' := ⟨r₁.head hr1, r₁.tail, by simp⟩
        simp only [List.cons_append, List.cons.injEq] at e2
        obtain ⟨rfl, rfl⟩ := e2
        refine ⟨r₁', by rw [e1]; simp, rfl, ?_⟩
        have hq'' : upToQuote (r₀ ++ Y') = (wa, x :: (r₁' ++ Y')) := by unfold upToQuote; rw [e3]; rfl
        rw [hq'']
        simp [hx]
      · simp only [ite_eq_right hx] at h; cases h
  rw [ite_eq_right h6] at h ⊢
  have ho := opAt_res hY (c :: r₀) C.ops 0 hC
  simp only [List.cons_append] at ho
  rw [← ho]
  generalize hop : opAt 0 C.ops (c :: (r₀ ++ Y)) = w at h ⊢
  rcases w with _ | ⟨k₁, o⟩
  · cases h
  · simp only [Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    obtain ⟨_, hp⟩ := opAt_prefix hop
    have hpa := prefix_app hp
    have hlen : o.length ≤ (c :: r₀).length := by simpa using hl
    have dY : ∀ Z : Bytes, (c :: (r₀ ++ Z)).drop o.length = (c :: r₀).drop o.length ++ Z := fun Z => by
      rw [← List.cons_append, List.drop_append_of_le_length hlen]
    rw [dY] at hpa
    refine ⟨(c :: r₀).drop o.length, ?_, dY Y, by dsimp only; rw [dY]⟩
    have : (o ++ (c :: r₀).drop o.length) ++ Y = (c :: r₀) ++ Y := by rw [List.append_assoc, hpa]; rfl
    exact (List.append_cancel_right this).symm

/-! ## The lexer from a position -/

/-- `go` with all the fuel it needs. -/
def lx (C : Cfg) (pos : Nat) (prev : Option Kind) (depth : Nat) (s : Bytes) :
    Except (Err × Nat × Nat) (List Piece) :=
  go C (s.length + 1) pos prev depth s

theorem lex_eq_lx (C : Cfg) (s : Bytes) : lex C s = lx C 0 none 0 s := rfl

theorem step_rest_lt {C pv d c s k t r d'} (h : step C pv d c s = .ok (k, t, r, d')) : r.length < s.length + 1 := by
  obtain ⟨ht, he⟩ := step_split h
  have := congrArg List.length he
  simp only [List.length_append, List.length_cons] at this
  have : 0 < t.length := by cases t with | nil => exact absurd rfl ht | cons _ _ => simp
  omega

theorem go_fuel {C : Cfg} : ∀ (n m pos : Nat) (prev : Option Kind) (depth : Nat) (s : Bytes),
    s.length < n → s.length < m → go C n pos prev depth s = go C m pos prev depth s
  | 0, _, _, _, _, _, h, _ => absurd h (Nat.not_lt_zero _)
  | _ + 1, 0, _, _, _, _, _, h => absurd h (Nat.not_lt_zero _)
  | n + 1, m + 1, pos, prev, depth, [], _, _ => by simp [go]
  | n + 1, m + 1, pos, prev, depth, c :: s, hn, hm => by
    simp only [go]
    cases hs : step C ((prev.map (Kind.isValue C)).getD false) depth c s with
    | error e => rfl
    | ok v =>
      obtain ⟨k, t, r, d⟩ := v
      have hr := step_rest_lt hs
      simp only [List.length_cons] at hn hm
      simp only
      rw [go_fuel n m _ _ _ r (by omega) (by omega)]

theorem lx_cons (C : Cfg) (pos : Nat) (prev : Option Kind) (depth c : Nat) (s : Bytes) :
    lx C pos prev depth (c :: s) =
      match step C ((prev.map (Kind.isValue C)).getD false) depth c s with
      | .error (e, lo, hi) => .error (e, pos + lo, pos + hi)
      | .ok (k, t, r, d) => (lx C (pos + t.length) (if isTrivia k then prev else some k) d r).map (⟨k, t⟩ :: ·) := by
  unfold lx
  simp only [List.length_cons, go]
  cases hs : step C ((prev.map (Kind.isValue C)).getD false) depth c s with
  | error e => rfl
  | ok v =>
    obtain ⟨k, t, r, d⟩ := v
    have hr := step_rest_lt hs
    simp only
    rw [go_fuel (s.length + 1) (r.length + 1) _ _ _ r (by omega) (by omega)]
    cases go C (r.length + 1) (pos + t.length) (if isTrivia k = true then prev else some k) d r <;> rfl

theorem lx_step {C : Cfg} {pos : Nat} {prev : Option Kind} {depth c : Nat} {s : Bytes} {k t r d}
    (h : step C ((prev.map (Kind.isValue C)).getD false) depth c s = .ok (k, t, r, d)) :
    lx C pos prev depth (c :: s) = (lx C (pos + t.length) (if isTrivia k then prev else some k) d r).map (⟨k, t⟩ :: ·) := by
  rw [lx_cons, h]

theorem map_ok {α ε : Type} {f : α → α} {x : Except ε α} {b : α} (h : x.map f = .ok b) : ∃ a, x = .ok a ∧ b = f a := by
  cases x with
  | error e => cases h
  | ok a => exact ⟨a, rfl, by simp [Except.map] at h; exact h.symm⟩

theorem map_map_app {ε : Type} (x : Except ε (List Piece)) (p : Piece) (l : List Piece) :
    (x.map (l ++ ·)).map (p :: ·) = x.map ((p :: l) ++ ·) := by cases x <;> rfl

theorem map2 {ε : Type} {x y : Except ε (List Piece)} (p q : Piece) (h : x = y) :
    (x.map (q :: ·)).map (p :: ·) = y.map ([p, q] ++ ·) := by subst h; cases x <;> rfl

theorem map_nil {ε : Type} (x : Except ε (List Piece)) : x.map ([] ++ ·) = x := by cases x <;> rfl

/-- The pieces of a prefix `P` of the input are cut the same whatever follows `P`, among inputs
that start a number alike. -/
theorem lx_prefix {C : Cfg} (hC : DigitFree C) {Y : Bytes} : ∀ (pre rest : List Piece) (P : Bytes)
    (pos : Nat) (prev : Option Kind) (depth : Nat),
    lx C pos prev depth (P ++ Y) = .ok (pre ++ rest) → (pre.map Piece.text).flatten = P →
    ∃ prev' depth', lx C (pos + P.length) prev' depth' Y = .ok rest ∧
      ∀ Y', Res Y Y' → lx C pos prev depth (P ++ Y') = (lx C (pos + P.length) prev' depth' Y').map (pre ++ ·)
  | [], rest, P, pos, prev, depth, h, hP => by
    simp only [List.map_nil, List.flatten_nil] at hP
    subst hP
    exact ⟨prev, depth, by simpa using h, fun Y' _ => by simp only [List.nil_append]; cases lx C pos prev depth Y' <;> rfl⟩
  | ⟨k₀, t₀⟩ :: pre, rest, P, pos, prev, depth, h, hP => by
    simp only [List.map_cons, List.flatten_cons] at hP
    subst hP
    generalize hP' : (pre.map Piece.text).flatten = P' at h ⊢
    cases hPY : t₀ ++ P' ++ Y with
    | nil =>
      rw [hPY] at h; simp [lx, go] at h
    | cons c s =>
      rw [hPY, lx_cons] at h
      cases hs : step C ((prev.map (Kind.isValue C)).getD false) depth c s with
      | error e => rw [hs] at h; cases h
      | ok v =>
        obtain ⟨k, t, r, d₁⟩ := v
        rw [hs] at h
        obtain ⟨ps, hps, he⟩ := map_ok h
        simp only [List.cons_append, List.cons.injEq, Piece.mk.injEq] at he
        obtain ⟨⟨rfl, rfl⟩, rfl⟩ := he
        obtain ⟨ht, hsplit⟩ := step_split hs
        obtain ⟨x, t', rfl⟩ : ∃ x t', t₀ = x :: t' := ⟨t₀.head ht, t₀.tail, by simp⟩
        simp only [List.cons_append, List.cons.injEq] at hPY hsplit
        obtain ⟨rfl, hs'⟩ := hPY
        obtain ⟨_, hsplit⟩ := hsplit
        have hr : r = P' ++ Y := by
          rw [← hs', List.append_assoc] at hsplit
          exact List.append_cancel_left hsplit
        rw [hr] at hps
        obtain ⟨prev', depth', h1, h2⟩ := lx_prefix hC pre rest P' _ _ _ hps hP'
        have hlen : pos + (x :: t' ++ P').length = pos + (x :: t').length + P'.length := by
          simp only [List.length_append]; omega
        refine ⟨prev', depth', by rw [hlen]; exact h1, fun Y' hY => ?_⟩
        rw [← hs'] at hs
        obtain ⟨r₁, e1, _, e3⟩ := step_res hC hY hs (by simp only [List.length_cons, List.length_append]; omega)
        have hr1 : r₁ = P' := by
          simp only [List.cons_append, List.cons.injEq] at e1
          exact (List.append_cancel_left e1.2).symm
        subst hr1
        have : x :: t' ++ r₁ ++ Y' = x :: ((t' ++ r₁) ++ Y') := by simp
        rw [this, lx_step e3, h2 Y' hY, map_map_app, hlen]

/-- What came before matters to the lexer only as whether it was a value. -/
theorem go_prev {C : Cfg} : ∀ (n pos : Nat) (prev prev' : Option Kind) (depth : Nat) (s : Bytes),
    (prev.map (Kind.isValue C)).getD false = (prev'.map (Kind.isValue C)).getD false →
    go C n pos prev depth s = go C n pos prev' depth s
  | 0, _, _, _, _, _, _ => rfl
  | _ + 1, _, _, _, _, [], _ => rfl
  | n + 1, pos, prev, prev', depth, c :: s, hv => by
    simp only [go]
    rw [hv]
    cases step C ((prev'.map (Kind.isValue C)).getD false) depth c s with
    | error e => rfl
    | ok v =>
      obtain ⟨k, t, r, d⟩ := v
      simp only
      by_cases hk : isTrivia k = true
      · rw [ite_eq_left hk, ite_eq_left hk, go_prev n _ prev prev' d r hv]
      · rw [ite_eq_right hk, ite_eq_right hk]

theorem lx_prev {C : Cfg} {pos : Nat} {prev prev' : Option Kind} {depth : Nat} (s : Bytes)
    (hv : (prev.map (Kind.isValue C)).getD false = (prev'.map (Kind.isValue C)).getD false) :
    lx C pos prev depth s = lx C pos prev' depth s := go_prev _ _ _ _ _ _ hv

/-- Where the lexer starts counting changes only the positions of errors. -/
theorem go_pos {C : Cfg} : ∀ (n pos pos' : Nat) (prev : Option Kind) (depth : Nat) (s : Bytes) (ps : List Piece),
    go C n pos prev depth s = .ok ps → go C n pos' prev depth s = .ok ps
  | 0, _, _, _, _, _, _, h => by simp [go] at h
  | _ + 1, _, _, _, _, [], _, h => by simpa [go] using h
  | n + 1, pos, pos', prev, depth, c :: s, ps, h => by
    simp only [go] at h ⊢
    cases hs : step C ((prev.map (Kind.isValue C)).getD false) depth c s with
    | error e => rw [hs] at h; obtain ⟨e, lo, hi⟩ := e; cases h
    | ok v =>
      obtain ⟨k, t, r, d⟩ := v
      rw [hs] at h
      simp only at h ⊢
      cases hg : go C n (pos + t.length) (if isTrivia k = true then prev else some k) d r with
      | error e => rw [hg] at h; cases h
      | ok qs =>
        rw [hg] at h
        rw [go_pos n _ (pos' + t.length) _ _ r qs hg]
        exact h

theorem lx_pos {C : Cfg} {pos pos' : Nat} {prev : Option Kind} {depth : Nat} {s : Bytes} {ps : List Piece}
    (h : lx C pos prev depth s = .ok ps) : lx C pos' prev depth s = .ok ps := go_pos _ _ _ _ _ _ _ h

/-! ## Trivia to the line's end -/

/-- Spaces, then maybe a comment, and no line break: what `reparse_number` lets follow a number. -/
def TrivLine (T : Bytes) : Prop :=
  ∃ sp c, T = sp ++ c ∧ sp.all isSpace = true ∧ (c = [] ∨ ∃ c', c = HASH :: c' ∧ c'.all (· != NL) = true)

/-- What the lexer takes after `W` up to the end of `W`'s last piece: the comment's rest, or the spaces. -/
def lineSkip (W S : Bytes) : Bytes × Bytes :=
  if HASH ∈ W then takeWhile (· != NL) S else takeWhile isSpace S

theorem takeWhile_pre (p : Nat → Bool) : ∀ (u S : Bytes), u.all p = true →
    takeWhile p (u ++ S) = (u ++ (takeWhile p S).1, (takeWhile p S).2)
  | [], S, _ => by simp
  | x :: u, S, h => by
    simp only [List.all_cons, Bool.and_eq_true] at h
    rw [List.cons_append, takeWhile.eq_2, ite_eq_left h.1, takeWhile_pre p u S h.2]
    rfl

theorem space_ne {c : Nat} (h : isSpace c = true) : (c == NL) = false := by
  simp [isSpace] at h; simp [NL]; omega

theorem lx_space_step {C : Cfg} {pos : Nat} {prev : Option Kind} {depth c : Nat} {s : Bytes} (hc : isSpace c = true) :
    lx C pos prev depth (c :: s) =
      (lx C (pos + (c :: (takeWhile isSpace s).1).length) prev depth (takeWhile isSpace s).2).map
        (⟨.space, c :: (takeWhile isSpace s).1⟩ :: ·) := by
  have hs : step C ((prev.map (Kind.isValue C)).getD false) depth c s =
      .ok (.space, c :: (takeWhile isSpace s).1, (takeWhile isSpace s).2, depth) := by
    unfold step
    rw [ite_eq_right (by simp [space_ne hc]), ite_eq_left hc]
  rw [lx_step hs]
  rfl

theorem lx_comment_step {C : Cfg} {pos : Nat} {prev : Option Kind} {depth : Nat} {s : Bytes} :
    lx C pos prev depth (HASH :: s) =
      (lx C (pos + (HASH :: (takeWhile (· != NL) s).1).length) prev depth (takeWhile (· != NL) s).2).map
        (⟨.comment, HASH :: (takeWhile (· != NL) s).1⟩ :: ·) := by
  have hs : step C ((prev.map (Kind.isValue C)).getD false) depth HASH s =
      .ok (.comment, HASH :: (takeWhile (· != NL) s).1, (takeWhile (· != NL) s).2, depth) := by
    unfold step
    rw [ite_eq_right (by decide), ite_eq_right (by decide), ite_eq_left (by decide)]
  rw [lx_step hs]
  rfl

theorem takeWhile_nil_of {p : Nat → Bool} {x : Nat} (s : Bytes) (h : p x = false) : takeWhile p (x :: s) = ([], x :: s) := by
  rw [takeWhile.eq_2, ite_eq_right (by simp [h])]

/-- After a line's trivia `W`: its pieces, all trivia, then the lexer goes on where they end. -/
theorem lx_trivline {C : Cfg} {W : Bytes} (hW : TrivLine W) (S : Bytes) :
    ∃ tr : List Piece, (∀ p ∈ tr, isTrivia p.kind = true) ∧ (tr.map Piece.text).flatten = W ++ (lineSkip W S).1 ∧
      ∀ pos prev depth, lx C pos prev depth (W ++ S) =
        (lx C (pos + (W ++ (lineSkip W S).1).length) prev depth (lineSkip W S).2).map (tr ++ ·) := by
  obtain ⟨sp, c, rfl, hsp, hc⟩ := hW
  have hsp' : ∀ x ∈ sp, isSpace x = true := fun x hx => List.all_eq_true.1 hsp x hx
  rcases hc with rfl | ⟨c', rfl, hc'⟩
  · have hh : ¬ HASH ∈ sp ++ [] := by
      intro h; simp at h; have := hsp' _ h; simp [isSpace, HASH] at this
    have hl : lineSkip (sp ++ []) S = takeWhile isSpace S := by unfold lineSkip; exact ite_eq_right hh
    rw [hl]
    simp only [List.append_nil]
    generalize hw : takeWhile isSpace S = w
    obtain ⟨a, b⟩ := w
    have hab : a ++ b = S := by have := takeWhile_app isSpace S; rw [hw] at this; exact this
    cases sp with
    | nil =>
      cases a with
      | nil =>
        refine ⟨[], by simp, by simp, fun pos prev depth => ?_⟩
        simp only [List.nil_append] at hab ⊢
        subst hab
        simp only [List.length_nil, Nat.add_zero]
        cases lx C pos prev depth b <;> rfl
      | cons x a' =>
        cases S with
        | nil => simp [takeWhile] at hw
        | cons y S' =>
          by_cases hy : isSpace y = true
          · rw [takeWhile.eq_2, ite_eq_left hy] at hw
            obtain ⟨⟨rfl, ha⟩, hb⟩ := (Prod.mk.inj hw).imp (List.cons.inj) id
            refine ⟨[⟨.space, y :: a'⟩], by simp [isTrivia], by simp, fun pos prev depth => ?_⟩
            simp only [List.nil_append]
            rw [lx_space_step hy, ha, hb]
            cases lx C (pos + (y :: a').length) prev depth b <;> rfl
          · rw [takeWhile.eq_2, ite_eq_right hy] at hw
            cases (Prod.mk.inj hw).1
    | cons y sp' =>
      have hy : isSpace y = true := hsp' y (by simp)
      have hsp2 : sp'.all isSpace = true := by simp only [List.all_cons, Bool.and_eq_true] at hsp; exact hsp.2
      refine ⟨[⟨.space, y :: sp' ++ a⟩], by simp [isTrivia], by simp, fun pos prev depth => ?_⟩
      rw [List.cons_append, lx_space_step hy, takeWhile_pre isSpace sp' S hsp2, hw]
      rfl
  · have hh : HASH ∈ sp ++ HASH :: c' := by simp
    have hl : lineSkip (sp ++ HASH :: c') S = takeWhile (· != NL) S := by unfold lineSkip; exact ite_eq_left hh
    rw [hl]
    generalize hw : takeWhile (· != NL) S = w
    obtain ⟨a, b⟩ := w
    have hcom : ∀ pos prev depth, lx C pos prev depth (HASH :: c' ++ S) =
        (lx C (pos + (HASH :: c' ++ a).length) prev depth b).map (⟨.comment, HASH :: c' ++ a⟩ :: ·) := by
      intro pos prev depth
      rw [List.cons_append, lx_comment_step, takeWhile_pre _ c' S hc', hw]
      rfl
    cases sp with
    | nil =>
      refine ⟨[⟨.comment, HASH :: c' ++ a⟩], by simp [isTrivia], by simp, fun pos prev depth => ?_⟩
      simp only [List.nil_append]
      rw [hcom]
      rfl
    | cons y sp' =>
      have hy : isSpace y = true := hsp' y (by simp)
      have hsp2 : sp'.all isSpace = true := by simp only [List.all_cons, Bool.and_eq_true] at hsp; exact hsp.2
      refine ⟨[⟨.space, y :: sp'⟩, ⟨.comment, HASH :: c' ++ a⟩], by simp [isTrivia], by simp, fun pos prev depth => ?_⟩
      have htw : takeWhile isSpace (sp' ++ HASH :: c' ++ S) = (sp', HASH :: c' ++ S) := by
        rw [List.append_assoc, takeWhile_pre isSpace sp' _ hsp2, List.cons_append,
          takeWhile_nil_of _ (by decide : isSpace HASH = false)]
        simp
      rw [List.cons_append, List.cons_append, lx_space_step hy, htw, hcom]
      refine map2 _ _ ?_
      congr 1
      simp only [List.length_append, List.length_cons]; omega

theorem trivline_app {T V : Bytes} (hT : TrivLine T) (hV : TrivLine V) : TrivLine (T ++ V) := by
  obtain ⟨sp, c, rfl, hsp, hc⟩ := hT
  obtain ⟨sp₂, c₂, rfl, hsp₂, hc₂⟩ := hV
  have hsp₂' : sp₂.all (· != NL) = true := List.all_eq_true.2 fun x hx => by
    have := List.all_eq_true.1 hsp₂ x hx; simp [isSpace] at this; simp [NL]; omega
  have hc₂' : c₂.all (· != NL) = true := by
    rcases hc₂ with rfl | ⟨c', rfl, h⟩
    · rfl
    · simp only [List.all_cons, h, Bool.and_true]; decide
  rcases hc with rfl | ⟨c', rfl, h⟩
  · exact ⟨sp ++ sp₂, c₂, by simp, by simp [hsp, hsp₂], hc₂⟩
  · refine ⟨sp, HASH :: (c' ++ sp₂ ++ c₂), by simp, hsp, Or.inr ⟨_, rfl, ?_⟩⟩
    simp only [List.all_append, h, hsp₂', hc₂', Bool.and_self]

/-- A number piece leaves the bracket depth alone, and starts with `-` only where no value precedes. -/
theorem step_num {C pv d c s u t r d'} (h : step C pv d c s = .ok (.num u, t, r, d')) :
    d' = d ∧ (c = MINUS → pv = false) := by
  unfold step at h
  by_cases h1 : (c == NL) = true
  · rw [ite_eq_left h1] at h
    by_cases hd : (d == 0) = true <;> simp [hd] at h
  rw [ite_eq_right h1] at h
  by_cases h2 : isSpace c = true
  · rw [ite_eq_left h2] at h; simp at h
  rw [ite_eq_right h2] at h
  by_cases h3 : (c == HASH) = true
  · rw [ite_eq_left h3] at h; simp at h
  rw [ite_eq_right h3] at h
  by_cases h4 : (isDigit c || (c == MINUS && (s.head?.map isDigit).getD false && !pv)) = true
  · rw [ite_eq_left h4] at h
    generalize number c s = res at h
    rcases res with e | ⟨k₁, t₁, r₂⟩
    · cases h
    · simp only [Except.ok.injEq, Prod.mk.injEq] at h
      refine ⟨h.2.2.2.symm, fun hc => ?_⟩
      subst hc
      simp [isDigit, MINUS] at h4
      exact h4.2
  rw [ite_eq_right h4] at h
  by_cases h5 : (isAlpha c || c == 95) = true
  · rw [ite_eq_left h5] at h
    generalize C.kws.findIdx? _ = o at h
    cases o <;> simp at h
  rw [ite_eq_right h5] at h
  by_cases h6 : (c == QUOTE) = true
  · rw [ite_eq_left h6] at h
    generalize (upToQuote s).2 = w at h
    cases w with
    | nil => cases h
    | cons x b => by_cases hx : (x == QUOTE) = true <;> simp [hx] at h
  rw [ite_eq_right h6] at h
  generalize opAt 0 C.ops (c :: s) = w at h
  rcases w with _ | ⟨k₁, o⟩ <;> simp at h

theorem lit_int {L : Lit} (hl : L.Ok) : ∃ x z, isDigit x = true ∧ L.mant = x :: z := by
  obtain ⟨hi0, hi, _, _⟩ := hl
  obtain ⟨x, xs, hx⟩ : ∃ x xs, L.int = x :: xs := ⟨L.int.head hi0, L.int.tail, by simp⟩
  rw [hx] at hi
  simp only [List.all_cons, Bool.and_eq_true] at hi
  exact ⟨x, L.mant.tail, hi.1, by simp only [Lit.mant, hx, List.cons_append, List.tail_cons]⟩

theorem lit_text {L : Lit} (hl : L.Ok) :
    ∃ x z, isDigit x = true ∧ L.text = (if L.neg then [MINUS] else []) ++ x :: z := by
  obtain ⟨x, z, hx, hm⟩ := lit_int hl
  exact ⟨x, z ++ L.expPart ++ unitText L.unit, hx, by simp [Lit.text, hm]⟩

/-- Two plain numbers with the same sign start alike, whatever follows them. -/
theorem res_lit {L₁ L₂ : Lit} (hl₁ : L₁.Ok) (hl₂ : L₂.Ok) (hneg : L₁.neg = L₂.neg) (X X' : Bytes) :
    Res (L₁.text ++ X) (L₂.text ++ X') := by
  obtain ⟨x₁, z₁, hx₁, ht₁⟩ := lit_text hl₁
  obtain ⟨x₂, z₂, hx₂, ht₂⟩ := lit_text hl₂
  rw [ht₁, ht₂, ← hneg]
  cases L₁.neg
  · exact Or.inl ⟨⟨x₁, z₁ ++ X, by simp, hx₁⟩, ⟨x₂, z₂ ++ X', by simp, hx₂⟩⟩
  · exact Or.inr ⟨x₁ :: z₁ ++ X, x₂ :: z₂ ++ X', by simp, by simp, ⟨x₁, z₁ ++ X, by simp, hx₁⟩,
      ⟨x₂, z₂ ++ X', by simp, hx₂⟩⟩

theorem lineSkip_end (W : Bytes) {S₁ : Bytes} (h : S₁ = [] ∨ S₁.head? = some NL) : lineSkip W S₁ = ([], S₁) := by
  unfold lineSkip
  rcases h with rfl | h
  · split <;> rfl
  · obtain ⟨z, S', rfl⟩ : ∃ z S', S₁ = z :: S' := by
      cases S₁ with
      | nil => simp at h
      | cons z S' => exact ⟨z, S', rfl⟩
    simp only [List.head?_cons, Option.some.injEq] at h
    subst h
    split
    · exact takeWhile_nil_of _ (by decide)
    · exact takeWhile_nil_of _ (by decide)

/-- **The lexer, on an edit of one number.** The old text is `P`, a plain number `L₁` lexed as one
number piece, the trivia `T₁` and the rest `S`; the new text has the plain number `L₂` (same sign)
and the trivia `T₂` in their place. If `T₁` and `T₂` both hold a comment or both do not (or the
rest of `S`'s line is trivia), the new text lexes to the same pieces before the number, the new
number's piece, trivia, and then the same pieces as before, `rest`: the pieces after the trivia
are the old ones, each `|L₂ T₂| − |L₁ T₁|` bytes later. -/
theorem reparse_lex {C : Cfg} (hC : DigitFree C) {P T₁ T₂ S : Bytes} {L₁ L₂ : Lit} (hl₁ : L₁.Ok) (hl₂ : L₂.Ok)
    (hneg : L₁.neg = L₂.neg) (hs₂ : Stops (T₂ ++ S)) (ht₁ : TrivLine T₁) (ht₂ : TrivLine T₂)
    (hh : (HASH ∈ T₁ ↔ HASH ∈ T₂) ∨ ∃ V S₁, S = V ++ S₁ ∧ TrivLine V ∧ (S₁ = [] ∨ S₁.head? = some NL))
    {pre post : List Piece} {u₁ : Unit}
    (hlex : lex C (P ++ L₁.text ++ T₁ ++ S) = .ok (pre ++ ⟨.num u₁, L₁.text⟩ :: post))
    (hpre : (pre.map Piece.text).flatten = P) :
    ∃ tr₁ tr₂ rest S₀, post = tr₁ ++ rest ∧
      lex C (P ++ L₂.text ++ T₂ ++ S) = .ok (pre ++ ⟨.num L₂.unit, L₂.text⟩ :: (tr₂ ++ rest)) ∧
      (∀ p ∈ tr₁ ++ tr₂, isTrivia p.kind = true) ∧
      (tr₁.map Piece.text).flatten = T₁ ++ S₀ ∧ (tr₂.map Piece.text).flatten = T₂ ++ S₀ := by
  -- the trivia after each number
  obtain ⟨tr₁, tr₂, S₀, S₁, a1, a2, b1, b2, e₁, e₂⟩ : ∃ tr₁ tr₂ S₀ S₁,
      (∀ p ∈ tr₁, isTrivia p.kind = true) ∧ (∀ p ∈ tr₂, isTrivia p.kind = true) ∧
      (tr₁.map Piece.text).flatten = T₁ ++ S₀ ∧ (tr₂.map Piece.text).flatten = T₂ ++ S₀ ∧
      (∀ pos prev depth, lx C pos prev depth (T₁ ++ S) =
        (lx C (pos + (T₁ ++ S₀).length) prev depth S₁).map (tr₁ ++ ·)) ∧
      (∀ pos prev depth, lx C pos prev depth (T₂ ++ S) =
        (lx C (pos + (T₂ ++ S₀).length) prev depth S₁).map (tr₂ ++ ·)) := by
    rcases hh with hh | ⟨V, S₁, rfl, hV, hS₁⟩
    · obtain ⟨tr₁, a1, b1, c1⟩ := lx_trivline (C := C) ht₁ S
      obtain ⟨tr₂, a2, b2, c2⟩ := lx_trivline (C := C) ht₂ S
      have hls : lineSkip T₂ S = lineSkip T₁ S := by
        unfold lineSkip
        by_cases h : HASH ∈ T₁
        · rw [ite_eq_left h, ite_eq_left (hh.1 h)]
        · rw [ite_eq_right h, ite_eq_right (fun h' => h (hh.2 h'))]
      rw [hls] at b2 c2
      exact ⟨tr₁, tr₂, (lineSkip T₁ S).1, (lineSkip T₁ S).2, a1, a2, b1, b2, c1, c2⟩
    · obtain ⟨tr₁, a1, b1, c1⟩ := lx_trivline (C := C) (trivline_app ht₁ hV) S₁
      obtain ⟨tr₂, a2, b2, c2⟩ := lx_trivline (C := C) (trivline_app ht₂ hV) S₁
      rw [lineSkip_end _ hS₁] at b1 c1 b2 c2
      refine ⟨tr₁, tr₂, V, S₁, a1, a2, by simpa using b1, by simpa using b2,
        fun pos prev depth => ?_, fun pos prev depth => ?_⟩
      · rw [← List.append_assoc, c1]; simp
      · rw [← List.append_assoc, c2]; simp
  rw [lex_eq_lx] at hlex ⊢
  rw [show P ++ L₁.text ++ T₁ ++ S = P ++ (L₁.text ++ (T₁ ++ S)) by simp] at hlex
  obtain ⟨prev', d', h1, h2⟩ := lx_prefix hC pre _ P 0 none 0 hlex hpre
  -- the old number's step
  obtain ⟨x₁, z₁, hx₁, ht₁⟩ := lit_text hl₁
  obtain ⟨c, s, hcs⟩ : ∃ c s, L₁.text ++ (T₁ ++ S) = c :: s := by
    rw [ht₁]; cases L₁.neg
    · exact ⟨x₁, z₁ ++ (T₁ ++ S), by simp⟩
    · exact ⟨MINUS, x₁ :: z₁ ++ (T₁ ++ S), by simp⟩
  have hc : L₁.neg = true → c = MINUS := by
    intro hn; rw [ht₁, hn] at hcs; simp at hcs; exact hcs.1.symm
  rw [hcs, lx_cons] at h1
  cases hs : step C ((prev'.map (Kind.isValue C)).getD false) d' c s with
  | error e => rw [hs] at h1; obtain ⟨e, lo, hi⟩ := e; cases h1
  | ok v =>
    obtain ⟨k, t, r, d₁⟩ := v
    rw [hs] at h1
    obtain ⟨ps, hps, he⟩ := map_ok h1
    simp only [List.cons.injEq, Piece.mk.injEq] at he
    obtain ⟨⟨rfl, rfl⟩, rfl⟩ := he
    obtain ⟨hd1, hpv⟩ := step_num hs
    subst hd1
    have hr : r = T₁ ++ S := by
      have := (step_split hs).2
      rw [← hcs] at this
      exact List.append_cancel_left this
    simp only [isTrivia, Bool.false_eq_true, ite_false] at hps
    rw [hr, e₁] at hps
    obtain ⟨rest, hrest, hpost⟩ := map_ok hps
    refine ⟨tr₁, tr₂, rest, S₀, hpost, ?_, fun p hp => ?_, b1, b2⟩
    · have hres : Res (L₁.text ++ (T₁ ++ S)) (L₂.text ++ (T₂ ++ S)) := res_lit hl₁ hl₂ hneg _ _
      rw [show P ++ L₂.text ++ T₂ ++ S = P ++ (L₂.text ++ (T₂ ++ S)) by simp, h2 _ hres]
      obtain ⟨c₂, s₂, hcs₂, hst₂⟩ := num_literal C ((prev'.map (Kind.isValue C)).getD false) d₁ L₂ hl₂
        (T₂ ++ S) hs₂ (fun hn => hpv (hc (hneg ▸ hn)))
      rw [hcs₂, lx_step hst₂]
      simp only [isTrivia, Bool.false_eq_true, ite_false]
      rw [e₂, lx_prev (prev := some (.num L₂.unit)) (prev' := some (.num u₁)) _ (by simp [Kind.isValue]),
        lx_pos hrest]
      rfl
    · rcases List.mem_append.1 hp with hp | hp
      · exact a1 p hp
      · exact a2 p hp

end ForgeLang.LL1.Lex
