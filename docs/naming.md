# Naming faces, edges and vertices

Every face, edge and vertex has a `forge_topo::Name`, a 64-bit hash of how it was made. Later
features (fillet and chamfer edges, shell and draft faces, up-to faces, agent handles) refer to
entities by name. This page says how names are made, what the literature knows about the
problem, and where our scheme stands against its known failure classes, as measured by
`forge_model::names` (the fuzzer: `cargo run --release -p forge-model --example namehunt`).

## The problem

The topological (persistent) naming problem: after an upstream edit and a regeneration, a
reference must find the entity that corresponds to the one it named, or fail clearly. Bidarra
et al. split it into two questions: how to name an entity in one B-rep, and how to map names from
the B-rep before the edit to the one after [5]. Systems either rebuild names from the history on
every regeneration (generative naming: Kripac, Capoyleas et al., FreeCAD, ForgeCAD) or match the
new B-rep against the old one after the fact (matching: Agbodan et al.).

## Prior work

- **Kripac 1997** [1]. Names faces from the feature and its generating sketch entity, and keeps a
  FaceIdGraph of how each face splits and merges through the history; a stored name is matched
  against the graph after regeneration, with heuristics when a face splits differently. Later
  authors note the matching rules are underspecified [7].
- **Capoyleas, Chen & Hoffmann 1996** [2]. Three mechanisms in Erep: a name from the generating
  operation and profile element, a name from the adjacent named entities (an edge as the faces it
  bounds), and disambiguation of several pieces by geometric qualifiers relative to the feature
  (which side, which direction). No one of them is complete; they are combined.
- **Agbodan, Marcheix & Pierra 2000, 2003; Marcheix 2006** [3, 4]. Hierarchical names from form
  features and the topology they evolve through, plus matching of entities by their invariant
  topological neighbourhood when the history changes; extended to shells. Built on Open Cascade
  and in a TopSolid prototype.
- **Bidarra, Nyirenda & Bronsvoort 2005** [5]. Semantic feature models: references go to feature
  faces by their role in the feature, not to B-rep entities, which removes most splits from the
  problem.
- **Farjana & Han 2018** [6] survey these and classify failures: renaming after topology change,
  split and merge, ambiguity among pieces, and order dependence.
- **FreeCAD (RealThunder's element map, in FreeCAD 1.0)** [8]. Replaced `Face1`-style index names
  with history-encoded names: each element's name records the operation and the names of the
  input elements it was modified or generated from; split pieces get a suffix with a counter. The
  FreeCAD wiki is explicit that the scheme does not remove every ambiguity.
- **Onshape** uses query-based references re-evaluated against the regenerated part. I could not
  reach a technical description during this survey; nothing below depends on it.

## Our scheme

- An extruded, revolved or swept face is named from its feature and its profile segment's id;
  an edge between two such faces from the set of the two (Capoyleas' second mechanism).
- **Feature ids are positions in the history.** Inserting, deleting or reordering a feature
  renames everything after it (see below).
- Booleans (sequential and n-ary alike, since 2026-10-31):
  - a new edge, a piece of the curve where faces `a` and `b` meet, is named from the pair, the
    branch of the intersection, and the two edges its ends cut (not its place along the curve);
  - a new vertex is named from the curve pieces that end there (each with which end), or the
    edges cut there, with ties broken by where it lies;
  - a piece of a split edge is named from its end vertices, a piece of a split face from the least
    of its boundary pieces' names with their directions (`Name::piece`, so `is_piece_of` can tell
    which face it came from);
  - a face or edge cut down to one kept piece keeps its own name (it was trimmed, not split);
  - names no longer carry the feature's id, so the n-ary and sequential regens agree wherever
    they split alike.
  This is Capoyleas' scheme with their third mechanism (geometric qualifiers) replaced by
  neighbours, and Kripac's split tracking replaced by naming pieces from their own boundaries.

## Measured against the failure classes

`namehunt` makes random edits above a reference point (resize, move, insert an unrelated hole,
delete, swap independent features) over the suite parts, random histories and the fillet and
finishing corpora, and tracks each referenced entity geometrically through the edit in small
steps. Classes: *kept*; *gone* (truly gone, name gone); *on a piece* (it split, the name is on a
piece); *lost* (entity survives, name gone); *renamed* (entity untouched, name gone); *wrong*
(name now on a different entity); *jumped* (the edit flipped the entity past tracking; the name
went to something that is not a piece of it); *duplicate*.

| class (literature) | before this work | now |
|---|---|---|
| order/index names (FreeCAD `FaceN`, [6]) | boolean vertices, curve pieces, edge pieces and face regions numbered by discovery order: wrong matches under small edits | none from booleans; small-edit hunts (900 edits) show wrong matches only in the seam class below |
| split and merge [1, 6] | a face trimmed to one piece renamed `split k`; seq and n-ary disagreed | trimmed faces and edges keep their names on both paths |
| ambiguity among pieces [2] | by order | by boundary; ties by position |
| history change (insert, delete, reorder) [3, 6] | everything after renamed; deletes hand names to the next feature | unchanged: needs stable feature ids (open) |
| seams of periodic surfaces | n/a | open: a closed intersection curve is cut where a cylinder's seam lies, so a tiny move can swap the halves' names (shaft-post, shaft-cross-hole) |
| flush faces parting or meeting | n/a | open: a block standing flush on a plate names its foot edges as its own; sunk 0.01 mm in, they become intersection edges with new names (lost, never wrong) |
| fillet faces on closed edges | n/a | open: fillet piece names follow the edge's seam (boss-foot-periodic) |

Seq and n-ary still name differently where a face splits into several kept pieces: the
sequential regen sees the pieces earlier features left, the n-ary sees the original faces. Full
agreement needs names defined against the original tool faces (Kripac's graph, or carrying each
face's origin through booleans).

## Open questions (for David)

1. Stable feature ids: give each feature an id that survives insert, delete and reorder (stored
   in the part, e.g. hidden in the `.fc`), instead of its position.
2. Handles are 4+ hex digits of the name. When an upstream edit loses a name, another entity's
   name can share those digits and the handle silently resolves to it (about one in 16^4 per
   entity of the part). Longer handles, or a kind letter, would cut that.
3. Seams: name pieces of closed curves (and the fillets along them) by a geometric qualifier
   relative to the feature (Capoyleas' third mechanism) rather than where the seam falls.
4. Flush configurations: name an edge between a tool's face and a coincident face of the part
   the same whether they touch or overlap.

## References

1. J. Kripac, "A mechanism for persistently naming topological entities in history-based
   parametric solid models", Computer-Aided Design 29(2):113-122, 1997.
2. V. Capoyleas, X. Chen, C. M. Hoffmann, "Generic naming in generative, constraint-based design",
   Computer-Aided Design 28(1):17-26, 1996. https://www.cs.purdue.edu/cgvlab/www/resources/papers/Capoyleas-Computer_aided_design-1996-Generic_naming_in_generative.pdf
3. D. Agbodan, D. Marcheix, G. Pierra, "Persistent naming for parametric models", WSCG 2000;
   D. Agbodan et al., "A topological entity matching technique for geometric parametric models",
   ICSMA 2003. https://www.lias-lab.fr/publications/7076/2003-ICSMA-AGBODAN.pdf
4. D. Marcheix, "A persistent naming of shells", CAD/CAM 2006. https://www.lias-lab.fr/publications/7550/2006-CAD-CAM-Marcheix.pdf
5. R. Bidarra, P. J. Nyirenda, W. F. Bronsvoort, "A feature-based solution to the persistent
   naming problem", Computer-Aided Design and Applications, 2005.
6. S. H. Farjana, S. Han, "Mechanisms of persistent identification of topological entities in CAD
   systems: a review", Alexandria Engineering Journal, 2018.
7. J. Mun, S. Han, "Identification of topological entities and naming mapping for parametric CAD
   model exchanges", 2005. https://koreascience.kr/article/JAKO200503018236985.pub
8. FreeCAD wiki, "Topological naming problem" (https://wiki.freecad.org/Topological_naming_problem);
   Zheng Lei (realthunder), "Topological Naming" (https://github.com/realthunder/FreeCAD_assembly3/wiki/Topological-Naming).
