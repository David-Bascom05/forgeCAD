import ForgeBoolFallback

-- The axiom report CI checks: only propext, Classical.choice and Quot.sound, no sorryAx.
#print axioms ForgeBoolFallback.Batch.applyK_sound
#print axioms ForgeBoolFallback.Batch.cpuFold_sound
#print axioms ForgeBoolFallback.Batch.cpuCode_sound_all
#print axioms ForgeBoolFallback.Batch.grid_fold_eq
#print axioms ForgeBoolFallback.Batch.cpuCode_sound_grid
#print axioms ForgeBoolFallback.Ladder.plan_ok
#print axioms ForgeBoolFallback.Ladder.plan_err
#print axioms ForgeBoolFallback.Ladder.plan_correct
#print axioms ForgeBoolFallback.Member.csg_sound
#print axioms ForgeBoolFallback.Member.sound_mem
#print axioms ForgeBoolFallback.Walk.splitOpen_never
#print axioms ForgeBoolFallback.Walk.walk_ne_nil
#print axioms ForgeBoolFallback.Walk.first_half_of_boundary
#print axioms ForgeBoolFallback.Unneeded.withFallback_ok_iff
