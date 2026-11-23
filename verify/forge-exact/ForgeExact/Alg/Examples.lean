import ForgeExact.Alg.Sum

/-! Checks of the executable algebraic numbers, run by every build. -/

namespace ForgeExact.Alg.Examples

open ForgeExact.Alg

def s2 : Num := .iso ⟨[-2, 0, 1], 1, 2⟩
def s3 : Num := .iso ⟨[-3, 0, 1], 1, 2⟩

#guard (⟨[-2, 0, 1], 1, 2⟩ : Iso).ok
#guard Num.cmp 50 s2 s3 == some .lt
#guard s2.sign [-2, 0, 1] 50 == some 0
#guard s2.sign [-141421356 / 100000000, 1] 60 == some 1
-- (√2 + √3)² − 10 = 2√6 − 5 < 0
#guard (match Num.add 60 s2 s3 with | some n => n.sign [-10, 0, 1] 80 | none => none) == some (-1)
-- √2 √3 = √6
#guard (match Num.mul 60 s2 s3 with | some n => n.sign [-6, 0, 1] 80 | none => none) == some 0
-- √2 − √2 = 0
#guard (match Num.add 60 s2 s2.neg with | some n => n.cmpRat 0 | none => .lt) == .eq

end ForgeExact.Alg.Examples
