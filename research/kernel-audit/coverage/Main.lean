import Coverage.Model

open Coverage

/-- One line per pattern: label, the model's outcome (both sample sides), the exact outcome. -/
def main : IO Unit := do
  for p in Pat.all do
    let k0 := kernel p {} false
    let k1 := kernel p {} true
    let k := if k0 == k1 then k0.word else s!"{k0.word}|{k1.word}"
    IO.println s!"{p.label}\t{k}\t{(exact p).word}"
