/-!
# Little-endian integers and fixed-width fields (kernel/forge-mesh/src/stl.rs, zip.rs)

A byte is a `Nat` (the writers only ever put values below 256 there). `u32le` is Rust's
`u32::to_le_bytes`; `readU32` is `u32::from_le_bytes` on the next four bytes.
-/

namespace MeshFiles

def u32le (n : Nat) : List Nat :=
  [n % 256, n / 256 % 256, n / 65536 % 256, n / 16777216 % 256]

def readU32 : List Nat → Option (Nat × List Nat)
  | b0 :: b1 :: b2 :: b3 :: rest => some (b0 + 256 * b1 + 65536 * b2 + 16777216 * b3, rest)
  | _ => none

theorem u32le_length (n : Nat) : (u32le n).length = 4 := rfl

theorem u32le_bytes (n : Nat) : ∀ b ∈ u32le n, b < 256 := by
  intro b hb
  simp only [u32le, List.mem_cons, List.not_mem_nil, or_false] at hb
  rcases hb with h | h | h | h <;> subst h <;> omega

/-- A count below 2³² reads back as itself, whatever follows. -/
theorem readU32_u32le (n : Nat) (h : n < 4294967296) (rest : List Nat) :
    readU32 (u32le n ++ rest) = some (n, rest) := by
  simp only [u32le, List.cons_append, List.nil_append, readU32, Option.some.injEq,
    Prod.mk.injEq, and_true]
  omega

/-- A codec for a fixed-width field: Rust's `f32::to_le_bytes` / `from_le_bytes`, which are
inverse bit casts (std). It is the one assumption STL's round trip rests on. -/
structure Fixed (α : Type) (w : Nat) where
  enc : α → List Nat
  dec : List Nat → α
  len : ∀ x, (enc x).length = w
  dec_enc : ∀ x, dec (enc x) = x

theorem take_of_length {l r : List Nat} {n : Nat} (h : l.length = n) : (l ++ r).take n = l := by
  subst h; simp

theorem drop_of_length {l r : List Nat} {n : Nat} (h : l.length = n) : (l ++ r).drop n = r := by
  subst h; simp

end MeshFiles
