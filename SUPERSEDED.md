# This branch is superseded: use cam/ui-parity-2

cam/ui-parity merged cam/2d-finish @f94a1e8d, which carried the withdrawn
claude/lang-rows-parity @a0a7cbfa (a forge-lang change pulled from the train). Do not train it.

cam/ui-parity-2 is the same work rebuilt without it: cam/ui-parity @8fb2cfdc + cam/2d-finish-2
@af799fe6 + the 2D-finish form rows and the coolant fix. `interface/forge-cam-lang/tests/superseded.rs`
fails on purpose so this branch cannot pass CI.
