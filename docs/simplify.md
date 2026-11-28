# Simplify

Simplify builds a separate solid model from a Forge part, a solved Forge assembly, or a
STEP model. The derived `.fg` document stores the source path and rules and opens a
validated snapshot beside it. The source file stays untouched.

```sh
forgecad simplify preview board.step --holes 1 --components 2
forgecad simplify create board.step --holes 1 --components 2 -o board-clearance.fg
forgecad build board-clearance.fg
forgecad simplify refresh board-clearance.fg
```

- `--holes MM` closes recognized cylindrical through-bores at or below the diameter.
- `--pockets MM` and `--bosses MM` heal straight-edged planar details that meet a
  surrounding face at a complete inner rim. The limit is the detail’s box diagonal
  in that face’s frame.
- `--components MM` excludes small components at or below their local bounding-box
  diagonal. A single-body source is kept. Use `--exclude COMPONENT` to exclude a
  specific occurrence or `--keep COMPONENT` to protect it from automatic rules.
- `--envelope COMPONENT` replaces the component with its local bounding box at its
  original pose. Use it for clearance studies.
- `--keep-face COMPONENT=HEX` protects a named face. `--remove-face COMPONENT=HEX`
  requests a direct face edit. Repeat the option for all the faces of a feature.
  ForgeCAD's healer supports whole rim-bounded pockets and bosses and some isolated
  chamfers and blends. It refuses edits it cannot close into a solid.
- `--json` reports the occurrence names, recognized bores, face IDs, and proposed changes.
  Add `--faces` to a preview to list all source faces by name and surface type.
  The part name for a single native part is `part`. For repeated STEP product names,
  Simplify uses instance labels such as `R1` when the exporter supplies them and
  numbers any remaining duplicates.

After editing the source, run `refresh`. A failed refresh leaves the previous snapshot
available. When an imported source changes, review its explicit face and component
selections in a fresh preview. To approve a new set of rules and selections, pass them
to `refresh DERIVED.fg --replace-rules`. List every rule you want in that replacement.
STEP entity numbers can change between revisions.

Use `pause` and `resume` to control refresh. `detach` removes the source link and
leaves an ordinary part backed by the current snapshot. Keep the generated STEP assets
beside the document. Older snapshots may still be used by older document revisions.

This version exposes the workflow through the CLI and MCP. It stores the solved bodies as named
imports at their solved positions, with disjoint bodies preserved. Connector anchors, assembly
constraints, and harness metadata are not copied into the derived part.

The `simplify` MCP tool accepts `file`, `action`, an optional array of CLI rule
arguments, and `out` for creation. Use `forgecad mcp --root PROJECT --writable` for
operations that save files. Read-only sessions can preview.
