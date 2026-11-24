#!/usr/bin/env python3
"""The registry of runtime checks removed or replaced because a proof says they cannot fire
(verify/REMOVED-CHECKS.toml), checked in seconds, with no build:

* each Rust fn listed still exists, and its source (the fn's text, whitespace-normalized) hashes
  to the entry's `reviewed_at`: a change to the code a proof stands for forces a re-review (the
  reviewer re-reads the lemma against the new code and updates the hash);
* each lemma listed is still declared in its Lean project and listed in that project's axiom report
  (whose own CI job fails on a broken proof or a non-standard axiom; this only checks the name is
  there, so the proof job covers it);
* each guarding test listed exists as a `#[test]` and is not `#[ignore]`d (it then runs in the
  existing test jobs);
* an entry whose lemma is about a hand model (level L1 or L2), not the translated code (L4), lists
  at least one sampling test and one mutation test, and owes "code = model";
* an entry with no lemma yet (a check replaced on a hypothesis) names it (`hyp`) and owes "lemma".

Usage: tools/ci/removed_checks.py [--hash ID] [--registry FILE]
  --hash ID       print an entry's current source hash (to set `reviewed_at` after a review)
  --registry FILE another registry, its paths relative to its own directory (the self-test)
"""
import hashlib
import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "verify" / "REMOVED-CHECKS.toml"
if "--registry" in sys.argv:
    k = sys.argv.index("--registry")
    REGISTRY = pathlib.Path(sys.argv[k + 1]).resolve()
    ROOT = REGISTRY.parent
    del sys.argv[k:k + 2]


def fn_source(path: pathlib.Path, name: str) -> str | None:
    """The text of `fn name` in a Rust file: its signature to its closing brace (by brace depth,
    skipping string and char literals and comments)."""
    text = path.read_text()
    m = re.search(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+)*fn\s+" + re.escape(name) + r"\b", text)
    if not m:
        return None
    i = text.index("{", m.end())
    depth, j, n = 0, i, len(text)
    while j < n:
        c = text[j]
        if text.startswith("//", j):
            j = text.find("\n", j)
            j = n if j < 0 else j
            continue
        if text.startswith("/*", j):
            j = text.find("*/", j) + 2
            continue
        if c == '"':
            j += 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
        elif c == "'" and re.match(r"'(\\.|[^\\'])'", text[j:j + 4]):
            j = text.index("'", j + 1)
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return text[m.start():j + 1]
        j += 1
    return None


def source_hash(src: str) -> str:
    return hashlib.sha256(" ".join(src.split()).encode()).hexdigest()[:16]


def lean_declared(project: pathlib.Path, full: str) -> bool:
    short = full.rsplit(".", 1)[-1]
    pat = re.compile(r"(?m)^\s*(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def)\s+" + re.escape(short) + r"\b")
    return any(pat.search(p.read_text()) for p in project.rglob("*.lean") if ".lake" not in p.parts)


def in_axiom_report(project: pathlib.Path, full: str) -> bool:
    pat = re.compile(r"(?m)^#print axioms\s+" + re.escape(full) + r"\s*$")
    return any(pat.search(p.read_text()) for p in project.rglob("*xioms*.lean") if ".lake" not in p.parts)


def test_state(path: pathlib.Path, name: str) -> str:
    """'ok', 'missing', 'not a test' or 'ignored'."""
    text = path.read_text()
    m = re.search(r"(?m)^[ \t]*(?:pub\s+)?fn\s+" + re.escape(name) + r"\s*\(", text)
    if not m:
        return "missing"
    # the attributes just above the fn
    above = text[:m.start()].rstrip().split("\n")
    attrs = []
    for line in reversed(above):
        s = line.strip()
        if s.startswith("#[") or s.startswith("///") or s.startswith("//"):
            attrs.append(s)
        else:
            break
    if not any(a.startswith("#[test]") or a.startswith("#[tokio::test") or "proptest" in a for a in attrs):
        return "not a test"
    if any(a.startswith("#[ignore") for a in attrs):
        return "ignored"
    return "ok"


def main() -> int:
    reg = tomllib.loads(REGISTRY.read_text())
    entries = reg.get("check", [])
    if len(sys.argv) == 3 and sys.argv[1] == "--hash":
        for e in entries:
            if e["id"] == sys.argv[2]:
                src = fn_source(ROOT / e["rust"]["file"], e["rust"]["fn"])
                print(source_hash(src) if src else "fn not found")
                return 0
        print("no such id")
        return 2
    errors = []
    ids = set()
    for e in entries:
        eid = e.get("id", "?")
        err = lambda msg: errors.append(f"{eid}: {msg}")
        if eid in ids:
            err("duplicate id")
        ids.add(eid)
        for key in ("id", "removed", "rust", "level", "lemmas", "tests", "reviewed_at", "owner"):
            if key not in e:
                err(f"missing `{key}`")
        if any(k not in e for k in ("rust", "level", "lemmas", "tests", "reviewed_at")):
            continue
        # the code the proof stands for
        path = ROOT / e["rust"]["file"]
        src = fn_source(path, e["rust"]["fn"]) if path.exists() else None
        if src is None:
            err(f"fn {e['rust']['fn']} not found in {e['rust']['file']}")
        elif source_hash(src) != e["reviewed_at"]:
            err(f"{e['rust']['file']}::{e['rust']['fn']} changed since its review "
                f"(reviewed_at {e['reviewed_at']}, now {source_hash(src)}): re-read the lemma "
                f"against the new code, then set reviewed_at = \"{source_hash(src)}\"")
        # the lemmas
        if not e["lemmas"]:
            # a check replaced on a hypothesis not yet proven: it says which, and owes the lemma
            if not e.get("hyp") or "lemma" not in e.get("owed", []):
                err('no lemma: name the hypothesis it rests on (`hyp`) and list "lemma" in `owed`')
        for lm in e["lemmas"]:
            proj = ROOT / lm["project"]
            if not proj.is_dir():
                err(f"Lean project {lm['project']} not found")
            elif not lean_declared(proj, lm["name"]):
                err(f"lemma {lm['name']} not declared in {lm['project']}")
            elif not in_axiom_report(proj, lm["name"]):
                err(f"lemma {lm['name']} not in {lm['project']}'s axiom report "
                    f"(add `#print axioms {lm['name']}`, so its proof job checks it)")
        # the guarding tests
        kinds = set()
        for t in e["tests"]:
            st = test_state(ROOT / t["file"], t["name"]) if (ROOT / t["file"]).exists() else "missing"
            if st != "ok":
                err(f"test {t['file']}::{t['name']}: {st}")
            kinds.add(t.get("kind"))
        if e["level"] not in ("L1", "L2", "L4"):
            err(f"level {e['level']!r}: L1, L2 (a hand model) or L4 (the translated code)")
        if e["level"] in ("L1", "L2"):
            if "sampling" not in kinds:
                err("a hand-model entry needs a sampling test (the new path against the old)")
            if "mutation" not in kinds:
                err("a hand-model entry needs a mutation test (breaking the code fails a test)")
            if "code = model" not in e.get("owed", []):
                err('a hand-model entry owes "code = model": list it in `owed`')
    for msg in errors:
        print(f"::error::{msg}")
    print(f"{len(entries)} entries, {len(errors)} problems")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
