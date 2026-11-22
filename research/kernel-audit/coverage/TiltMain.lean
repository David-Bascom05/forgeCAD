import Coverage.Tilt

/-- `tilt θ...` (θ in 1e-13 rad): the modelled gaps, one `family θ` line each. -/
def main (args : List String) : IO Unit := do
  for (f, θ) in Coverage.Tilt.predicted (args.filterMap String.toNat?) do
    IO.println s!"{f}\t{θ}"
