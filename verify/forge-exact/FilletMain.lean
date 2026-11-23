import ForgeExact.Fillet.Check

/-- `fillet-check`: one rolled blend per line on stdin (verify/fillet-spec's dump), one judgement per
line on stdout (ForgeExact/Fillet/Check.lean). A line it cannot read gets `{"error": ...}`. -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  repeat
    let line ← stdin.getLine
    if line.isEmpty then break
    let line := line.trimAsciiEnd.toString
    if line.isEmpty then continue
    let out := match Lean.Json.parse line >>= ForgeExact.Fillet.Check.judge with
      | .ok j => j.compress
      | .error e => (Lean.Json.mkObj [("error", e)]).compress
    stdout.putStrLn out
    stdout.flush
