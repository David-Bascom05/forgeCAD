# Slow exact boolean (for CAM-CORE)

`boolean(&A, &B, Op::Diff)` with the regions in slow-boolean-k2.txt took 6.13 s (release, this
VM, cam/check-api + cam/machine-train4 base). A = (pass-2 loop of flange_cam's `top` clear,
grown by r = 5, minus the loop shrunk by r) ∩ the stock box; B = the next pass's region grown by
r. From ops::pocket_in's drop-loop test (cam/quality), which now skips such loops untested.
The first line of the .txt is the measured time.
