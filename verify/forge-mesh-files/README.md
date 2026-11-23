# The mesh files' claims (kernel/forge-mesh: stl.rs, obj.rs, threemf.rs, zip.rs)

What `forgecad export F stl|3mf|obj` writes is the mesh, and only the mesh: read back, the file
gives every triangle exactly as written, and every position exactly (STL's f32) or rounded as the
text formats write numbers (`num`, to 1e-6 mm). The triangles alone decide closed, oriented and
manifold (`open_edges`, `pinched_vertices`), so a file's mesh has those exactly when the written
one does. That the written one does is the tessellation's spec (verify/forge-mesh-check, on the
fundamentals branch: closed, oriented, the vertex-fan check, the chord bound).

`lean/` (core Lean only; `lake build`, then `lake env lean Axioms.lean`: standard axioms, no
`sorry`):

| Theorem | What it says |
|---|---|
| `readU32_u32le` | a count below 2³² written little endian reads back as itself |
| `readStl_writeStl` | binary STL: any 80-byte header and fewer than 2³² facets read back exactly: normals, corners, order |
| `writeStl_length` | its size is 84 + 50 n, which the reader checks a binary file by |
| `readObjs_writeObjs` | OBJ: the lines read back give each object its vertices and exactly its triangles (indices from 1 across the file, back to the object's own from 0) |
| `read_write`, `read_write_tris` | OBJ as text: positions rounded, triangles exact |
| `readModel_writeModel` | 3MF model part: every object (name, material, vertices, triangles) and every build item (its object by id, its transform, its name) |
| `read_write3mf` | 3MF as attribute values: positions rounded (`num`), transforms rounded (`matrix_num`, 1e-12), all else exact |

Assumed, not proven (std's, and each its own claim): an f32 bit-cast to four bytes and back is
itself (`Fixed`); a number formatted and parsed back is it rounded, an integer itself (`Text`);
XML escaping gives a name back as written. The Rust readers are modelled as the simplest readers
of what the writers write: obj.rs's renumbering by first use is this numbering for every object
`body_objects` makes.

## Levels (research/PROOF-STANDARD.md)

| Level | Mesh export has |
|---|---|
| L1 | the writers' claims above, over an abstract number type; the mesh's own spec is the tessellation's (forge-mesh-check) |
| L2 | the readers in kernel/forge-mesh (`read_stl`, `read_obj`, `read_3mf`, `zip::read`) are the executable reference the round trips are judged by |
| L3 | proven for every scene within the stated limits (fewer than 2³² triangles; items naming an object; no empty transform) |
| L4 | not yet: the Lean models are written from the Rust, not extracted from it (Aeneas) |
| L5 | none needed: the writers make no floating-point decisions (they format and cast) |
| L6 | interim, differential: every format round-tripped through our readers and checked by trimesh, numpy-stl and the 3MF Core schema (kernel/forge-mesh/tests/files.rs, interface/forge-cli/tests/mesh_export.rs, tools/meshcheck/run.sh), one hash over all four formats' bytes the same natively and on wasm32. DEFLATE and the zip around a 3MF have no Lean model: Python's zipfile checks every member's CRC, and forge-git's oracle has git inflate our DEFLATE streams |
