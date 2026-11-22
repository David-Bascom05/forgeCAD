import ForgeExact.Ref.Input

/-- `forge-exact`: one history per line on stdin, one answer per line on stdout
(ForgeExact/Ref/Input.lean gives the format). A line it cannot read gets `{"error": ...}`. -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  repeat
    let line ← stdin.getLine
    if line.isEmpty then break
    let line := line.trimAsciiEnd.toString
    if line.isEmpty then continue
    let out := match ForgeExact.Ref.run line with
      | .ok j => j.compress
      | .error e => (Lean.Json.mkObj [("error", e)]).compress
    stdout.putStrLn out
    stdout.flush
