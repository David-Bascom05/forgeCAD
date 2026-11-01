#!/usr/bin/env python3
"""Whether the tests catch a check made too loose: loosens each comparison and each locality test of
kernel/forge-topo/src/check.rs in turn and runs forge-agent's check_local test, which must fail. Run from a scratch
copy of the tree, never your checkout: it edits check.rs. mutate.py [CARGO_TEST_COMMAND...]

Left out as they cannot change an answer: the length tests of `same_edge` (an edge not used twice is `edge-uses`, run
always) and of a loop's coedges (a loop equal to another's start and longer is `loop-broken`, or pinched, run always)."""
import subprocess, sys

P = "kernel/forge-topo/src/check.rs"
src = open(P).read()
# (text, what it becomes, which occurrence)
CUTS = [
    ("x.range == y.range", "true", 0),
    ("a.vertex(x.start).point == b.vertex(y.start).point", "true", 0),
    ("a.vertex(x.end).point == b.vertex(y.end).point", "true", 0),
    ("x.curve == y.curve", "true", 0),
    ("a.vertex(x.start).tol == b.vertex(y.start).tol", "true", 0),
    ("a.vertex(x.end).tol == b.vertex(y.end).tol", "true", 0),
    ("a.face(a.coedge_face(c)).surface == b.face(b.coedge_face(d)).surface", "true", 0),
    ("same_span(a, e, b, o)", "true", 0),
    ("x.same_sense == y.same_sense", "true", 0),
    ("x.surface == y.surface", "true", 0),
    ("x.loops.len() == y.loops.len()", "true", 0),
    ("c.forward == d.forward", "true", 0),
    ("same_span(a, c.edge, b, d.edge)", "true", 0),
    # a plane face's loops, one at a time
    ("c.forward == d.forward", "true", 1),
    ("same_span(a, c.edge, b, d.edge)", "true", 1),
    ("!matches!(x.surface, forge_geom::Surface::Plane(_))", "false", 0),
    ("x.same_sense != y.same_sense", "false", 0),
    ("x.surface != y.surface", "false", 0),
    ("edge_length(body, e) < 2.0", "false", 0),
    (">= 0.4", ">= 0.0", 0),
    # where material changed, and the shells
    ("new.chain(gone)", "new", 0),
    ("Some(r) => r.overlaps(&body.face_box(f)),", "Some(_) => false,", 0),
    ("want == 0 && clear", "clear", 0),
    ("same_shells &=", "let _ =", 0),
]
cmd = sys.argv[1:] or ["cargo", "test", "-q", "-p", "forge-agent", "--test", "check_local"]
survived = 0
for text, to, nth in CUTS:
    at = -1
    for _ in range(nth + 1):
        at = src.find(text, at + 1)
    assert at >= 0, text
    open(P, "w").write(src[:at] + to + src[at + len(text):])
    r = subprocess.run(cmd, capture_output=True, text=True)
    out = r.stdout + r.stderr
    caught = "test result: FAILED" in out
    if not caught and "test result" not in out:
        state = "NO BUILD"
    else:
        state = "caught  " if caught else "SURVIVED"
    print(state, f"[{nth}]", text, flush=True)
    survived += state != "caught  "
open(P, "w").write(src)
print("not caught:", survived)
sys.exit(1 if survived else 0)
