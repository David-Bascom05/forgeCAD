#!/usr/bin/env python3
"""Per-function translation status of a crate, from a tools/aeneas/survey-modules.sh run.

    SURVEY_KEEP=1 tools/aeneas/survey-modules.sh CRATE      (OUT=DIR)
    python3 research/aeneas/fn_survey.py DIR CRATE > table.tsv

One row per function (and closure body) Aeneas attempted in the crate's own source files:
    module  function  status  reason
status:
    ok       its body translated and its Lean elaborates (no Lean error located in its definition)
    lean     its body translated, but Lean reports an error in its definition
    body     Aeneas could not translate its body (reason: the first error it gave)
    sig      Aeneas could not translate its signature
    charon   the module did not get through Charon (reason: why)
Functions of other crates that the module calls stay opaque (Charon's default for a dependency)
and are listed at the end as `dep` rows, with the crate's functions that reach them.
"""
import os, re, sys, glob, collections

def clean(s):
    return re.sub(r"\x1b\[[0-9;?]*[a-zA-Z]", "", s).replace("\r", "\n")

def short(name, crate=None):
    """forge_bool::classify::{forge_bool::classify::Index<'a>}::classify -> classify::Index::classify;
    a closure's call/call_mut/call_once -> its path ::{closure#k}"""
    # a closure body: {impl core::ops::function::Fn..<..> for PATH::{closure#k}<..>}::call
    m = re.match(r"^.*\{impl core::ops::function::Fn\w*<.*? for (.*?\{closure(?:#\d+)?\})(?:<[^{}]*>)?\}::call\w*$", name)
    if m:
        name = m.group(1)
    # an impl block {path::Type<..>} or {impl Trait for path::Type<..>} -> Type
    for _ in range(3):
        name = re.sub(r"\{impl [^{}]*? for (?:[A-Za-z_0-9]+::)*([A-Za-z_0-9]+)(?:<[^{}]*>)?\}", r"\1", name)
        name = re.sub(r"\{(?:[A-Za-z_0-9]+::)*([A-Za-z_0-9]+)(?:<[^{}]*>)?\}", r"\1", name)
    if crate and name.startswith(crate + "::"):
        name = name[len(crate) + 2:]
    return name

def main():
    out, crate = sys.argv[1], sys.argv[2]
    name = crate.replace("-", "_")
    rows = []
    deps = collections.defaultdict(set)
    tsv = os.path.join(out, "survey.tsv")
    fatal = {}
    for line in open(tsv):
        f = line.rstrip("\n").split("\t")
        if f[0] == crate:
            fatal[f[1]] = f[6]
    for mod, why in sorted(fatal.items()):
        tag = mod.replace("::", ".")
        log = os.path.join(out, crate, tag + ".aeneas.log")
        outdir = os.path.join(out, crate, tag + ".out")
        if not os.path.exists(log):
            rows.append((mod, "*", "charon", why))
            continue
        text = clean(open(log, errors="replace").read())
        failed = {}
        last_err = None
        for ln in text.split("\n"):
            if ln.startswith("[Error]"):
                e = ln[len("[Error]"):].strip()
                if not e.startswith("Ignoring the body"):
                    last_err = e
            m = re.search(r"Could not translate the (body|function signature) of(?: function)? '(.*?)(?: because of previous error\.?)?'?$", ln)
            if m:
                kind = "body" if m.group(1) == "body" else "sig"
                failed.setdefault(m.group(2).strip(), (kind, last_err or ""))
            m = re.search(r"Ignoring the body of '(.*)' because", ln)
            if m:
                failed.setdefault(m.group(1).strip(), ("body", last_err or ""))
        # translated definitions, by their Source comment, and Lean errors by definition
        funs = os.path.join(outdir, "Funs.lean")
        defs = []
        if os.path.exists(funs):
            src = open(funs).read()
            for m in re.finditer(r"/-- \[([^\]]+)\]((?:.|\n)*?)-/\n(?:@\[[^\]]*\]\n)?(?:noncomputable )?(?:partial )?def ([^ \n]+)", src):
                rust, meta, lean = m.group(1), m.group(2), m.group(3)
                sm = re.search(r"Source: '([^']+)'", meta)
                if sm and sm.group(1).startswith("kernel/" + crate + "/"):
                    defs.append((rust, lean))
        for rust, (kind, err) in sorted(failed.items()):
            if rust.startswith(name + "::"):
                rows.append((mod, short(rust, name), kind, err[:120]))
        seen = {r for r, _ in failed.items()}
        for rust, lean in defs:
            if rust in seen:
                continue
            rows.append((mod, short(rust, name), "ok", ""))
        # the opaque functions of other crates this module calls
        tmpl = os.path.join(outdir, "FunsExternal_Template.lean")
        if os.path.exists(tmpl):
            for m in re.finditer(r"/-- \[([^\]]+)\]", open(tmpl).read()):
                rust = m.group(1)
                if not rust.startswith(name + "::"):
                    deps[rust.split("::")[0]].add(short(rust))
    print("module\tfunction\tstatus\treason")
    done = set()
    for r in rows:
        if (r[0], r[1]) in done:
            continue
        done.add((r[0], r[1]))
        print("\t".join(r))
    for c in sorted(deps):
        for d in sorted(deps[c]):
            print("\t".join(["dep:" + c, d, "opaque", ""]))

main()
