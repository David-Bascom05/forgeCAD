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

end ForgeLang.LL1.Lex
