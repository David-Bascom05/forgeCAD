import ForgeCertify.Input

/-- `forge-certify`: one certificate per line on stdin, one verdict per line on stdout
(ForgeCertify/Input.lean gives the format). -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  repeat
    let line ← stdin.getLine
    if line.isEmpty then break
    let line := line.trimAsciiEnd.toString
    if line.isEmpty then continue
    let out := match ForgeCertify.run line with
      | .ok j => j.compress
      | .error e => (Lean.Json.mkObj [("verdict", "scope"), ("reason", s!"unreadable: {e}")]).compress
    stdout.putStrLn out
    stdout.flush
