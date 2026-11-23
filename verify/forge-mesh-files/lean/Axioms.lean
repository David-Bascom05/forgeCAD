import MeshFiles

/-! The claims rest on Lean's standard axioms only: `lake env lean Axioms.lean` prints each
theorem's axioms; none may be `sorryAx` or `Lean.ofReduceBool`. -/

#print axioms MeshFiles.readU32_u32le
#print axioms MeshFiles.readStl_writeStl
#print axioms MeshFiles.writeStl_length
#print axioms MeshFiles.readObjs_writeObjs
#print axioms MeshFiles.read_write
#print axioms MeshFiles.read_write_tris
#print axioms MeshFiles.readModel_writeModel
#print axioms MeshFiles.read_write3mf
