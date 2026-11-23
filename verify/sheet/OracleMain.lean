import Sheet.Oracle

/-- `lake exe sheet-oracle < dump`: the Lean model's verdicts on the kernel's published sheets
and region answers (Sheet/Oracle.lean has the format). -/
def main : IO Unit := do
  let input ← (← IO.getStdin).readToEnd
  for l in Sheet.Oracle.run input do
    IO.println l
