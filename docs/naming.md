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
  an edge between two such faces from the set of the two (Capoyleas' second mechanism). A loop
  of two segments meets itself twice, so its joints also take the segment starting there.
- Names are 64-bit hashes: `Name::with` mixes a word in one step (a murmur3 finaliser),
  `with_str` hashes the word first, `with_set` sums its members' mixes (order-free, no sort).
- **Feature ids are stable.** Each feature line has an id from when it was made, kept in the
  part file (`id N` before the line where it is not the line's place among the features) and
  passed by forge-doc to the regen (`forge_model::History`), so inserting, deleting or moving
  lines renames nothing. A file written by hand without ids gets each line's place (or, where a line
  already says that id is its own, the next unused one). An id is never used twice: when lines
  with the highest ids are deleted, the file starts `next N`, so a handle to a deleted feature
  fails rather than find a later one's entities.
- Booleans (sequential and n-ary alike, since 2026-10-31):
  - a new edge, a piece of the curve where faces `a` and `b` meet, is named from the pair, the
    branch of the intersection, and the two edges its ends cut (not its place along the curve);
    a seam edge names nothing (it lies where a surface's parameters start);
  - a new vertex is named from the curve pieces that end there (each with which end), or the
    least-named edge cut there; pieces or vertices that still tie are ordered along the axis
    they spread most along (`spread_rank`), which no small move changes;
  - an edge kept along a face of the other body it lay flush on is named as the cut it becomes
    when the bodies overlap by a hair, and its vertices likewise (`flush_edges_as_cuts`);
  - a piece of a split edge is named from its end vertices, a piece of a split face from the least
    of its boundary pieces' names with their directions (`Name::piece`, so `is_piece_of` can tell
    which face it came from);
  - a face or edge cut down to one kept piece keeps its own name (it was trimmed, not split);
  - names no longer carry the feature's id, so the n-ary and sequential regens agree wherever
    they split alike.
  This is Capoyleas' scheme with their third mechanism (geometric qualifiers) replaced by
  neighbours, and Kripac's split tracking replaced by naming pieces from their own boundaries.
- **Lineage: neighbours are named as wholes** (since 2026-10-31). A piece's name keeps the high
  32 bits of the whole it came from (`Name::whole`), through any number of splits, and its
  low half from what tells it from its siblings only (its ends), so a piece of
  a piece is named as if cut from the whole at once. Everything named from a neighbour takes
  the neighbour's lineage, not its name: a boolean's new edge from its faces' and end edges'
  lineages, a new vertex from what meets *at* it (each curve piece's face pair with the edge it
  cuts there, not the piece's other end), a fillet's or full round's ends from the end faces'
  lineages and the vertex, a chamfer's contact edges from its faces' lineages. So a later
  feature splitting a face or edge somewhere renames nothing it did not reach: an edge it never
  touched keeps its name though both its faces are now pieces. Names stay unique because pieces
  that would collide are told apart where they meet: their own ends, the vertex, or, for ties,
  where they lie (Capoyleas' disambiguation, applied only when two pieces collide).
- **No orientation or rounding in names** (since 2026-10-31). A new edge no longer carries the
  index of its intersection branch (the order a surface-surface intersection returns branches
  in turns with the part); it is named by its faces and the edges it runs between either way
  round, and where the curve crosses several edges at one point (a vertex, or edges of both
  faces) by all of them, since which cut ends the piece there is chance. Pieces or vertices
  still alike are ranked by where they lie in the first face's surface frame (the part's own,
  so it turns and moves with the part), along the axis on which they spread most, coordinates
  within rounding counting as equal: nearly level points are never ordered by noise. A vertex
  that ties on what meets at it is first told apart by the whole curve pieces ending there.
- **Where three tools meet, and flush crossings** (since 2026-11-01). The n-ary regen cuts each
  new edge into spans where tools older than its own later tool cross it, as the sequential
  regen meets the point (the older curve is an edge by then, the newer curves end on it), and
  names spans, their pieces and the vertices as that regen does (R34); a span lying inside an
  older tool is one the sequential regen never makes and marks nothing. Pieces of one pair of
  faces as wholes are ranked together wherever the faces were split, and new vertices tie only
  with vertices the result keeps (n-ary). A flush edge is named exactly as the cut it becomes
  (its faces as wholes and the edges it runs between, either way round); flush edges still
  alike (a revolve's two seams) are ranked by where they lie. A vertex where only imprints of
  flush faces cross is marked by the imprinted edges together, so crossings of one edge by
  several features' imprints are told apart, whichever regen made them. Faces a fillet joins
  across a flush seam keep the lesser name.
- Body splits name what the cut trimmed on both sides as pieces by their ends, split faces
  name pieces by their boundaries and new vertices by the edge they cut, and names moved off
  another body's (copies, splits, tools of several bodies) keep their lineage (`Name::salted`).
  None of these depend on order any more.

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
| order/index names (FreeCAD `FaceN`, [6]) | boolean vertices, curve pieces, edge pieces and face regions numbered by discovery order: wrong matches under small edits | none from booleans; small-edit hunts (1500 edits) show wrong matches only in the two order classes below |
| split and merge [1, 6] | a face trimmed to one piece renamed `split k`; seq and n-ary disagreed | trimmed faces and edges keep their names on both paths |
| ambiguity among pieces [2] | by order | by boundary; ties by position |
| history change (insert, delete, reorder) [3, 6] | everything after renamed; deletes hand names to the next feature | stable ids: inserts rename nothing; a delete hands nothing on. Open: swapping two features that overlap reorders the booleans, and faces split by both are named from the pieces the earlier one left (fillet:pit-four, pit-five: 49 of 200 hunt edits, seed 3) |
| seams of periodic surfaces | n/a | fixed: seams name nothing; halves of a closed curve go by where they lie (shaft-post, boss-foot-periodic, and the fillets along them) |
| two seams lying almost together | n/a | open: the slivers between them are told apart only by order (shaft-cross-hole) |
| repeats (a thread's turns) | n/a | open: told apart by order along the axis; a turn appearing at an end shifts the rest |
| flush faces parting or meeting | n/a | fixed for flat faces: a foot edge and its vertices keep their names whether the block stands flush or is sunk a hair in (lost names in topology-keeping edits roughly halved) |
| two-segment loops (a half disc) | n/a | fixed: both joints took one name (DuplicateName, invalid bodies) |
| orientation (the whole history turned, moved or rounded) | a circular pattern's edge renamed under an inexact rotation (M11: branch order and which of five coincident edges a cut found) | the turned part names every entity alike, seq and n-ary (`turning_the_whole_part_renames_nothing`). A mirror is no pure re-orientation: a right-handed mirrored sketch puts its segments on the other sides and its seam elsewhere, so names follow those roles |
| splits of neighbours (an insert splitting a face or edge elsewhere) | 39 edges, 91 vertices, 1 face renamed though untouched (60 seeds of `namehunt --inserts`, 14 137 inserts), and a face fillet or full round on a body with tombstones resolved its faces by stale ids and rounded the wrong edge | 0 edges, 2 vertices, 0 faces (14 296 inserts) |

`namehunt --inserts` puts holes, pockets, slots and bosses (`names::run_inserts`) in at random
places above each finishing feature of the shift corpus (finishing parts, fillet variants,
several bodies) and judges every entity: one the insert left where it was must keep its name.
An edge a first feature splits into two kept pieces and a later one trims back to one would,
without the first feature, have been trimmed whole and kept its own name. So a body remembers
the full names of the edges it split (`Body::split`), and when a boolean trims a piece of one
to the last piece left of it, that piece takes the edge's name back (`Body::sole_survivors`;
only wholes this boolean trimmed are looked at, so it costs nothing elsewhere). Copies, body
splits and salted bodies carry the list; ops that build a body afresh drop it. What is left
are two vertices whose geometry stayed but whose role changed: a fillet cap's corner the insert
reshaped, and the seam vertex of a closed edge the insert cut open (chamfer-box-shell).

Seq and n-ary still name differently where a face splits into several kept pieces: the
sequential regen sees the pieces earlier features left, the n-ary sees the original faces.
Lineage cut the names that differ by about a third on the suite parts (mixed 4453 to 3589 of
6253, shaft 3044 to 1895 of 5049; coil, upto and upto-flat now agree). Where they still differ
the two paths make different topology, not different names: the sequential regen keeps the
imprints of tools flush on one plane (the arms of a circular pattern on a rod's end face), the
n-ary merges them. Full
agreement needs names defined against the original tool faces (Kripac's graph, or carrying each
face's origin through booleans).

## Handles

`@`, the kind (`f` face, `e` edge, `v` vertex), and at least 8 hex digits of the name, more
where two entities of one kind share them. A handle whose entity an upstream edit took away
finds another entity only if that one shares the kind and all the digits: about one in 2^32
per entity of the part, so in practice it fails. Old short handles are refused with a hint.

## Arena order

A feature's result names nothing after the order in its input's arenas (`tests/arena_order.rs`):
a fillet or chamfer takes its edges by name, whatever order a selector gave them; ties among new
vertices break in the frame of the least-named face, not the first in the arena; a chamfer numbers what
it makes at a corner from the corner's least-named face. Measured in research/names-arena.

## Open questions

1. Fillet laws run from the vertex whose name sorts first (`forge_ops::law_start`). Any change
   to naming can turn a stored variable radius end for end; a geometric rule (or storing the
   start) would make it stable.
2. Seq and n-ary still differ where a face splits into several kept pieces (above).
3. Arena order: jack's wrapped text still names otherwise on a reversed stage, around a cylinder's
   seam; cause not found. Frame's `htb` revolve names a vertex otherwise on a body rebuilt by the skip path
   (research/rebuild), and neither reversing the arenas nor a 1e-13 nudge reproduces it.

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
