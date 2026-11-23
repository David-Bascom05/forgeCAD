import ForgeExact.DecideOnce.QGuards

/-- `curved-guards`: fails unless every representative curved input is certified (non-vacuity of
`curved_exact`, ForgeExact/DecideOnce/QGuards.lean). -/
def main : IO UInt32 := do
  let mut ok := true
  for (name, c) in ForgeExact.DecideOnce.Curved.Guards.curvedGuards do
    IO.println s!"{name}: {if c then "certified" else "NOT certified"}"
    if !c then ok := false
  return if ok then 0 else 1
