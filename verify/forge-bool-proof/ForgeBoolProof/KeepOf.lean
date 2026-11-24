import ForgeBoolProof.Orient

/-!
# boolean.rs `keep_of`: the keep rule and the flip, together

```
fn keep_of(op: Op, side: Side, c: Class) -> Option<bool> {
    match (op, side, c) {
        (Op::Union, _, Class::Out) | (Op::Union, 0, Class::OnSame) => Some(false),
        (Op::Subtract, 0, Class::Out) | (Op::Subtract, 0, Class::OnOpposite) => Some(false),
        (Op::Subtract, 1, Class::In) => Some(true),
        (Op::Intersect, _, Class::In) | (Op::Intersect, 0, Class::OnSame) => Some(false),
        _ => None,
    }
}
```

`keepOf` is that function, read by hand (side 0 is A: `src = true`; ledger: interim until
TRANSLATE's translation of `keep_of` replaces it, when the refinement is this file's statement).
`apply` writes a kept spec's face with sense `sense != flip` (boolean.rs `apply`: `add_face(...,
sense != flip, ...)`).

**`keepOf_spec`**: `keep_of` keeps exactly what `keepRule` keeps, and its flip is the difference's
reversal of B's regions; so the sense `apply` writes is `writtenSense` (`written_eq`), the sense
D8.orient (`kept_sense`) proves points into the kept rooms. This is the sense half of D8.sense, on
the code's side.
-/

namespace ForgeBoolProof

/-- boolean.rs `keep_of` (side 0, A, is `s = true`). -/
def keepOf : Op → Bool → Class → Option Bool
  | .union, _, .outside => some false
  | .union, true, .onSame => some false
  | .diff, true, .outside => some false
  | .diff, true, .onOpposite => some false
  | .diff, false, .inside => some true
  | .inter, _, .inside => some false
  | .inter, true, .onSame => some false
  | _, _, _ => none

/-- **`keep_of` is the keep rule, with the difference's flip.** -/
theorem keepOf_spec (op : Op) (s : Bool) (c : Class) :
    keepOf op s c = if keepRule op s c then some (op.isDiff && !s) else none := by
  cases op <;> cases s <;> cases c <;> rfl

/-- **The sense `apply` writes** (`sense != flip`) is the written sense. -/
theorem written_eq (op : Op) (s : Bool) (c : Class) (sn flip : Bool)
    (h : keepOf op s c = some flip) : (sn != flip) = writtenSense op s sn := by
  rw [keepOf_spec] at h
  split at h
  · cases h; unfold writtenSense; cases sn <;> cases op.isDiff <;> cases s <;> rfl
  · cases h

end ForgeBoolProof
