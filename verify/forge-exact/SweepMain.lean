import ForgeExact.Swept.LoftRef

/-- `sweep-ref`: one swept feature per line on stdin (ForgeExact/Swept/Ref.lean gives the
format), one answer per line on stdout. -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  repeat
    let line ← stdin.getLine
    if line.isEmpty then break
    let line := line.trimAsciiEnd.toString
    if line.isEmpty then continue
    let out := match ForgeExact.Swept.LoftRef.run line with
      | .ok j => j.compress
      | .error e => (Lean.Json.mkObj [("error", e)]).compress
    stdout.putStrLn out
    stdout.flush
