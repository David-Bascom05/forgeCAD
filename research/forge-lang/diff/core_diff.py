"""Differential test: the Rust evaluator and edit ops (fg) against the Lean core calculus (Core.lean).

One random program is built as a small AST, printed twice: as a .fg document and as a Lean `Doc`
of the core calculus. Lean evaluates the Docs (`run`, fuel 4000) and prints, per case, the model
(feature id, number, handle, sites of its trace) and the log; fg evaluates the .fg and reports
features with their provenance and which params are structural. The two are compared under one
mapping (README.md, "Mapping"). Edit ops are generated per program and applied on both sides: the
Lean side evaluates the edited Doc, fg applies the op (`apply_checked`) and re-runs. A refusal on
the Rust side must meet an error on the Lean side, and the printed text after the op must be the
expected text.

  python3 core_diff.py gen  --seed S --count N --out DIR   cases (.fg) and DIR/cases.lean
  python3 core_diff.py lean --out DIR                       runs Lean: DIR/expected.jsonl
  python3 core_diff.py check --out DIR --fg PATH/fg         runs fg, compares; prints a summary

Deterministic: everything follows from the seed. No third-party modules.
"""
import argparse, json, os, random, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
LEAN = os.path.join(HERE, "..", "lean")

# ---- AST ---------------------------------------------------------------------------------------
# Expr: ('p', name) a param or let by name; ('v', name) a local (fn param, loop variable);
# ('c', k) a loose integer; ('add', a, b); ('mulc', a, k); ('ite', cond, a, b) with
# cond = ('eq', a, b); ('sel', [e..], idx); ('call', f, e).
# Stmt: ('shared', name, lit, kind) kind 'len'|'count' (a site); ('link', name, expr) a shared
# param whose value is an expression (a Lean `let`); ('feat', name, expr) `name = extrude(plan, e)`;
# ('forN', loopname, var, count, [stmt]); ('forK', loopname, var, [(label, expr)], [stmt]);
# ('if', cond, [stmt], [stmt]); ('use', name, proc, expr); ('occ', name, comp, [(param, expr)]).
# Defs: fns {name: (x, expr)}, procs {name: (x, [stmt])}, comps {name: ([(param, k)], [stmt])}.


def fg_expr(e):
    k = e[0]
    if k in ("p", "v"):
        return e[1]
    if k == "c":
        return str(e[1])
    if k == "add":
        return f"({fg_expr(e[1])} + {fg_expr(e[2])})"
    if k == "mulc":
        return f"({fg_expr(e[1])} * {e[2]})"
    if k == "eq":
        return f"{fg_expr(e[1])} == {fg_expr(e[2])}"
    if k == "ite":
        return f"({fg_expr(e[2])} if {fg_expr(e[1])} else {fg_expr(e[3])})"
    if k == "sel":
        return "[" + ", ".join(fg_expr(x) for x in e[1]) + f"][{fg_expr(e[2])}]"
    if k == "call":
        return f"{e[1]}({fg_expr(e[2])})"
    raise ValueError(e)


def fg_stmts(ss, ind, sketch):
    out = []
    p = "  " * ind
    for s in ss:
        k = s[0]
        if k == "shared":
            v = f"count({s[2]})" if s[3] == "count" else str(s[2])
            out.append(f"{p}shared {s[1]} = {v}")
        elif k == "link":
            out.append(f"{p}shared {s[1]} = {fg_expr(s[2])}")
        elif k == "let":
            out.append(f"{p}{s[1]} = {fg_expr(s[2])}")
        elif k == "feat":
            out.append(f"{p}{s[1]} = extrude({sketch}, {fg_expr(s[2])})")
        elif k == "forN":
            out.append(f"{p}for {s[2]} in 0..{fg_expr(s[3])} {{")
            out += fg_stmts(s[4], ind + 1, sketch)
            out.append(p + "}")
        elif k == "forK":
            m = ", ".join(f"{l}: {fg_expr(x)}" for l, x in s[3])
            out.append(f"{p}for {s[2]} in {{{m}}} {{")
            out += fg_stmts(s[4], ind + 1, sketch)
            out.append(p + "}")
        elif k == "if":
            out.append(f"{p}if {fg_expr(s[1])} {{")
            out += fg_stmts(s[2], ind + 1, sketch)
            out.append(p + "} else {")
            out += fg_stmts(s[3], ind + 1, sketch)
            out.append(p + "}")
        elif k == "use":
            out.append(f"{p}{s[1]} = {s[2]}({fg_expr(s[3])})")
        elif k == "occ":
            if s[3]:
                a = ", ".join(f"{n}: {fg_expr(x)}" for n, x in s[3])
                out.append(f"{p}{s[1]} = part({s[2]}({a}))")
            else:
                out.append(f"{p}{s[1]} = part({s[2]})")
        else:
            raise ValueError(s)
    return out


SKETCH = ["sketch plan(on: xy) {", "  outline = rect(corner: (0, 0), size: (10, 10))", "}"]


def fg_doc(doc):
    """The .fg text. Params first, then the sketch, fns and procs, components, the rest."""
    body = doc["body"]
    params = [s for s in body if s[0] in ("shared", "link")]
    rest = [s for s in body if s[0] not in ("shared", "link")]
    out = fg_stmts(params, 0, "plan") + SKETCH
    for f, (x, e) in doc["fns"].items():
        out += [f"fn {f}({x}) {{", f"  return {fg_expr(e)}", "}"]
    for f, (x, ss) in doc["procs"].items():
        out += [f"fn {f}({x}) {{"] + fg_stmts(ss, 1, "plan") + [f"  return {ss[-1][1]}", "}"]
    for c, (ps, ss) in doc["comps"].items():
        hdr = ", ".join(f"{n}: {k}" for n, k in ps)
        out += [f"component {c}({hdr}) {{", "  sketch s(on: xy) {", "    o = circle((0, 0), 5)", "  }"]
        out += fg_stmts(ss, 1, "s") + ["}"]
    out += fg_stmts(rest, 0, "plan")
    return "\n".join(out) + "\n"


def lean_str(s):
    return '"' + s + '"'


def lean_expr(e, sites):
    k = e[0]
    if k in ("p", "v"):
        return f"(.var {lean_str(e[1])})"
    if k == "c":
        return f"(.cst {e[1]})"
    if k == "add":
        return f"(.add {lean_expr(e[1], sites)} {lean_expr(e[2], sites)})"
    if k == "mulc":
        return f"(.mul {lean_expr(e[1], sites)} (.cst {e[2]}))"
    if k == "eq":
        return f"(.eq {lean_expr(e[1], sites)} {lean_expr(e[2], sites)})"
    if k == "ite":
        return f"(.ite {lean_expr(e[1], sites)} {lean_expr(e[2], sites)} {lean_expr(e[3], sites)})"
    if k == "sel":
        return "(.sel [" + ", ".join(lean_expr(x, sites) for x in e[1]) + f"] {lean_expr(e[2], sites)})"
    if k == "call":
        return f"(.call {lean_str(e[1])} {lean_expr(e[2], sites)})"
    raise ValueError(e)


def lean_stmts(ss, sites):
    out = []
    for s in ss:
        k = s[0]
        if k == "shared":
            out.append(f"(.let_ {lean_str(s[1])} (.lit {sites[s[1]]}))")
        elif k in ("link", "let"):
            out.append(f"(.let_ {lean_str(s[1])} {lean_expr(s[2], sites)})")
        elif k == "feat":
            out.append(f"(.feat {lean_str(s[1])} {lean_expr(s[2], sites)})")
        elif k == "forN":
            out.append(f"(.forN {lean_str(s[1])} {lean_str(s[2])} {lean_expr(s[3], sites)} [{', '.join(lean_stmts(s[4], sites))}])")
        elif k == "forK":
            kv = ", ".join(f"({lean_str(l)}, {lean_expr(x, sites)})" for l, x in s[3])
            out.append(f"(.forK {lean_str(s[1])} {lean_str(s[2])} [{kv}] [{', '.join(lean_stmts(s[4], sites))}])")
        elif k == "if":
            out.append(f"(.ifs {lean_expr(s[1], sites)} [{', '.join(lean_stmts(s[2], sites))}] [{', '.join(lean_stmts(s[3], sites))}])")
        elif k == "use":
            out.append(f"(.use {lean_str(s[1])} {lean_str(s[2])} {lean_expr(s[3], sites)})")
        elif k == "occ":
            a = ", ".join(f"({lean_str(n)}, {lean_expr(x, sites)})" for n, x in s[3])
            out.append(f"(.occ {lean_str(s[1])} {lean_str(s[2])} [{a}])")
        else:
            raise ValueError(s)
    return out


def doc_sites(doc):
    """Site numbers: one per literal shared param, in order (1, 2, ...)."""
    sites, table = {}, {}
    for s in doc["body"]:
        if s[0] == "shared":
            sites[s[1]] = len(sites) + 1
            table[sites[s[1]]] = s[2]
    return sites, table


def lean_doc(doc):
    sites, table = doc_sites(doc)
    shared = [s[1] for s in doc["body"] if s[0] in ("shared", "link")]
    fns = ", ".join(f"({lean_str(f)}, ({lean_str(x)}, {lean_expr(e, sites)}))" for f, (x, e) in doc["fns"].items())
    procs = ", ".join(f"({lean_str(f)}, ({lean_str(x)}, [{', '.join(lean_stmts(ss, sites))}]))" for f, (x, ss) in doc["procs"].items())
    vis = "[" + ", ".join(lean_str(n) for n in shared) + "]"
    comps = ", ".join(
        f"({lean_str(c)}, {{ params := [{', '.join(f'({lean_str(n)}, .cst {k})' for n, k in ps)}], body := [{', '.join(lean_stmts(ss, sites))}], vis := {vis} }})"
        for c, (ps, ss) in doc["comps"].items())
    sig = " ".join(f"if s = {k} then {v} else" for k, v in table.items()) + " 0"
    return (f"{{ defs := {{ fns := [{fns}], procs := [{procs}], comps := [{comps}] }}, "
            f"body := [{', '.join(lean_stmts(doc['body'], sites))}], "
            f"σ := fun s => {sig} }}")


LEAN_HEAD = """import ForgeLang.Core
open ForgeLang
set_option linter.unusedVariables false

def segS : Seg → String
  | .nm x => "n:" ++ x
  | .ix i => "i:" ++ toString i
  | .key k => "k:" ++ k

def errS : Err → String
  | .fuel => "fuel" | .unbound x => "unbound " ++ x | .notNum x => "notNum " ++ x
  | .notFeature x => "notFeature " ++ x | .dangling x _ => "dangling " ++ x | .range => "range"
  | .noFn f => "noFn " ++ f | .dupId => "dupId" | .noComp c => "noComp " ++ c
  | .noParam c => "noParam " ++ c | .useCycle f => "useCycle " ++ f | .noFile f => "noFile " ++ f

def js (xs : List String) : String := "[" ++ ",".intercalate xs ++ "]"
def jq (s : String) : String := "\\"" ++ s ++ "\\""

def report (k : String) (d : Doc) : String :=
  match run 4000 d with
  | .error e => "{\\"case\\":" ++ jq k ++ ",\\"ok\\":false,\\"err\\":" ++ jq (errS e) ++ "}"
  | .ok (m, l) =>
    let f := m.map fun (p, t) =>
      js [js (p.map (jq ∘ segS)), toString (t.den d.σ),
        (match t.handle with | some s => toString s | none => "null"), js (t.sites.map toString)]
    "{\\"case\\":" ++ jq k ++ ",\\"ok\\":true,\\"feats\\":" ++ js f ++ ",\\"log\\":" ++ js (l.eraseDups.map toString) ++ "}"
"""

# ---- generator ---------------------------------------------------------------------------------


class Gen:
    def __init__(self, rng, comps=True):
        self.r = rng
        self.n = 0
        self.comps_on = comps

    def fresh(self, p):
        self.n += 1
        return f"{p}{self.n}"

    def len_expr(self, lens, locs, counts, depth, fns, allow_call=True):
        r = self.r
        atoms = [("p", x) for x in lens] + [("v", x) for x in locs]
        if depth <= 0 or r.random() < 0.3:
            return r.choice(atoms) if atoms and r.random() < 0.85 else ("c", r.randint(1, 9))
        c = r.random()
        if c < 0.3:
            return ("add", self.len_expr(lens, locs, counts, depth - 1, fns), self.len_expr(lens, locs, counts, depth - 1, fns))
        if c < 0.45:
            return ("mulc", self.len_expr(lens, locs, counts, depth - 1, fns), r.randint(1, 3))
        if c < 0.65:
            return ("ite", self.cond(lens, locs, counts, fns), self.len_expr(lens, locs, counts, depth - 1, fns),
                    self.len_expr(lens, locs, counts, depth - 1, fns))
        if c < 0.82 and counts:
            n = r.randint(1, 3)
            idx = r.choice(counts)
            return ("sel", [self.len_expr(lens, locs, counts, depth - 1, fns) for _ in range(n)], idx)
        if fns and allow_call:
            return ("call", r.choice(fns), self.len_expr(lens, locs, counts, depth - 1, fns))
        return r.choice(atoms) if atoms else ("c", 2)

    def cond(self, lens, locs, counts, fns):
        r = self.r
        if counts and r.random() < 0.5:
            return ("eq", r.choice(counts), ("c", r.randint(0, 3)))
        return ("eq", self.len_expr(lens, locs, [], 1, fns, False), self.len_expr(lens, locs, [], 1, fns, False))

    def doc(self):
        r = self.r
        body, lens, counts = [], [], []
        for _ in range(r.randint(1, 4)):
            x = self.fresh("w")
            body.append(("shared", x, r.randint(1, 12), "len"))
            lens.append(x)
        for _ in range(r.randint(0, 2)):
            x = self.fresh("n")
            body.append(("shared", x, r.randint(0, 3), "count"))
            counts.append(("p", x))
        if r.random() < 0.4:
            x = self.fresh("q")
            body.append(("link", x, self.len_expr(lens, [], [], 1, [])))
            lens.append(x)
        if counts and r.random() < 0.3:
            x = self.fresh("m")
            base = r.choice(counts)
            body.append(("link", x, ("add", base, ("c", 1))))
            counts.append(("p", x))
        fns, procs, comps = {}, {}, {}
        for _ in range(r.randint(0, 2)):
            f, x = self.fresh("f"), self.fresh("x")
            fns[f] = (x, self.len_expr(lens, [x], [], 2, []))
        fnl = list(fns)
        if r.random() < 0.3:
            pr, x, h = self.fresh("mk"), self.fresh("x"), self.fresh("h")
            procs[pr] = (x, [("feat", h, self.len_expr(lens, [x], counts, 2, fnl))])
        if self.comps_on and r.random() < 0.4:
            c, d, b = self.fresh("cp"), self.fresh("d"), self.fresh("b")
            comps[c] = ([(d, r.randint(1, 9))], [("feat", b, self.len_expr(lens, [d], [], 2, fnl))])
        for _ in range(r.randint(1, 5)):
            k = r.random()
            if k < 0.4:
                body.append(("feat", self.fresh("e"), self.len_expr(lens, [], counts, 3, fnl)))
            elif k < 0.55:
                i = self.fresh("i")
                cnt = r.choice(counts) if counts and r.random() < 0.7 else ("c", r.randint(0, 3))
                body.append(("forN", self.fresh("L"), i, cnt, [("feat", self.fresh("e"), self.len_expr(lens, [], counts + [("v", i)], 2, fnl))]))
            elif k < 0.67:
                v = self.fresh("k")
                kv = [(l, self.len_expr(lens, [], counts, 1, fnl)) for l in r.sample(["lf", "rf", "up", "dn"], r.randint(1, 3))]
                body.append(("forK", self.fresh("L"), v, kv, [("feat", self.fresh("e"), self.len_expr(lens, [v], counts, 2, fnl))]))
            elif k < 0.79:
                body.append(("if", self.cond(lens, [], counts, fnl),
                             [("feat", self.fresh("e"), self.len_expr(lens, [], counts, 2, fnl))],
                             [("feat", self.fresh("e"), self.len_expr(lens, [], counts, 2, fnl))]))
            elif k < 0.9 and procs:
                body.append(("use", self.fresh("u"), r.choice(list(procs)), self.len_expr(lens, [], counts, 2, fnl)))
            elif comps:
                c = r.choice(list(comps))
                ps = comps[c][0]
                args = [] if r.random() < 0.3 else [(ps[0][0], self.len_expr(lens, [], counts, 2, fnl))]
                body.append(("occ", self.fresh("o"), c, args))
            else:
                body.append(("feat", self.fresh("e"), self.len_expr(lens, [], counts, 2, fnl)))
        return {"body": body, "fns": fns, "procs": procs, "comps": comps}

    def ops(self, doc):
        """Edit ops as (fg op json, edited doc or None when the op is not modelled, expected text)."""
        r = self.r
        out = []
        body = doc["body"]
        lits = [s for s in body if s[0] == "shared"]
        for s in r.sample(lits, min(2, len(lits))):
            v = r.randint(0, 3) if s[3] == "count" else r.randint(1, 15)
            nb = [(t[0], t[1], v, t[3]) if t is s else t for t in body]
            out.append(({"op": "SetParam", "param": s[1], "value": {"value": v, "dim": "Count" if s[3] == "count" else "Length"}},
                        dict(doc, body=nb)))
        tops = [k for k, s in enumerate(body) if s[0] in ("feat", "use", "occ")]
        if tops:
            k = r.choice(tops)
            out.append(({"op": "DeleteFeature", "id": body[k][1]}, dict(doc, body=body[:k] + body[k + 1:])))
            j = r.choice([t for t in range(len(body)) if body[t][0] not in ("shared", "link")] + [None])
            if j is not None and j != k and body[j][0] in ("feat", "use", "occ"):
                nb = body[:k] + body[k + 1:]
                at = nb.index(body[j])
                nb = nb[:at] + [body[k]] + nb[at:]
                out.append(({"op": "MoveFeature", "id": body[k][1], "before": body[j][1]}, dict(doc, body=nb)))
            lens = [s[1] for s in body if (s[0] == "shared" and s[3] == "len") or (s[0] == "link" and s[1][0] == "q")]
            counts = [("p", s[1]) for s in body if s[0] == "shared" and s[3] == "count"]
            e = self.len_expr(lens + [t[1] for t in body if t[0] == "feat"][:0], [], counts, 2, list(doc["fns"]))
            if body[k][0] == "feat":
                new = ("feat", body[k][1], e)
                out.append(({"op": "ReplaceFeature", "id": body[k][1], "text": fg_stmts([new], 0, "plan")[0]},
                            dict(doc, body=body[:k] + [new] + body[k + 1:])))
            nf = ("feat", self.fresh("z"), e)
            out.append(({"op": "AddFeature", "before": body[k][1], "text": fg_stmts([nf], 0, "plan")[0]},
                        dict(doc, body=body[:k] + [nf] + body[k:])))
        for c, (ps, ss) in doc["comps"].items():
            v = r.randint(1, 9)
            nc = dict(doc["comps"])
            nc[c] = ([(ps[0][0], v)], ss)
            out.append(({"op": "SetArg", "id": c, "arg": ps[0][0], "text": str(v)}, dict(doc, comps=nc)))
            p2 = self.fresh("g")
            nc = dict(doc["comps"])
            nc[c] = (ps + [(p2, 3)], ss)
            out.append(({"op": "SetArg", "id": c, "arg": p2, "text": "3"}, dict(doc, comps=nc)))
            lens = [s[1] for s in body if (s[0] == "shared" and s[3] == "len") or (s[0] == "link" and s[1][0] == "q")]
            nb_ = ("feat", self.fresh("b"), self.len_expr(lens, [ps[0][0]], [], 2, list(doc["fns"])))
            nc = dict(doc["comps"])
            nc[c] = (ps, ss + [nb_])
            out.append(({"op": "AddFeature", "before": c + ".", "text": fg_stmts([nb_], 0, "s")[0]}, dict(doc, comps=nc)))
            old = ss[0]
            rep = ("feat", old[1], self.len_expr(lens, [ps[0][0]], [], 2, list(doc["fns"])))
            nc = dict(doc["comps"])
            nc[c] = (ps, [rep] + ss[1:])
            out.append(({"op": "ReplaceFeature", "id": f"{c}.{old[1]}", "text": fg_stmts([rep], 0, "s")[0]}, dict(doc, comps=nc)))
            nc = dict(doc["comps"])
            nc[c] = (ps, ss[1:])
            out.append(({"op": "DeleteFeature", "id": f"{c}.{old[1]}"}, dict(doc, comps=nc)))
        return out



# ---- fixed regression cases: every proof counterexample that maps to a .fg ---------------------

def D(body, fns=None, procs=None, comps=None):
    return {"body": body, "fns": fns or {}, "procs": procs or {}, "comps": comps or {}}


def cup(body_expr, ps=(("d", 50),)):
    return ([list(p) for p in ps], [("feat", "b", body_expr)])


def setp(doc, name, v, dim="Length"):
    nb = [(t[0], t[1], v, t[3]) if t[0] == "shared" and t[1] == name else t for t in doc["body"]]
    return ({"op": "SetParam", "param": name, "value": {"value": v, "dim": dim}}, dict(doc, body=nb))


def fixed_cases():
    """(name, doc, ops, what it checks). Lean evaluates each doc; the comparator is the same."""
    out = []
    # Counter/Memo.concrete_cache_wrong: two calls, equal values, different params
    d = D([("shared", "small", 80, "len"), ("shared", "big", 80, "len"),
           ("occ", "c1", "cup", [("d", ("p", "small"))]), ("occ", "c2", "cup", [("d", ("p", "big"))])],
          comps={"cup": cup(("v", "d"))})
    out.append(("concrete_cache_wrong", d, [setp(d, "big", 90)]))
    # the same with equal param sets: E1
    d = D([("shared", "a", 80, "len"),
           ("occ", "c1", "cup", [("d", ("p", "a"))]), ("occ", "c2", "cup", [("d", ("mulc", ("p", "a"), 1))])],
          comps={"cup": cup(("v", "d"))})
    out.append(("concrete_cache_param_set", d, [setp(d, "a", 90)]))
    # Counter/Memo.key_needs_shared: a block reading a shared param
    d = D([("shared", "W", 1, "len"), ("occ", "c1", "cup", [("d", ("c", 1))])],
          comps={"cup": cup(("add", ("v", "d"), ("p", "W")))})
    out.append(("key_needs_shared", d, [setp(d, "W", 2)]))
    # Counter/ControlUntracked: a value chosen by a branch: the condition is structural
    d = D([("shared", "c", 1, "count"), ("shared", "five", 5, "len"), ("shared", "seven", 7, "len"),
           ("feat", "a", ("ite", ("eq", ("p", "c"), ("c", 1)), ("p", "five"), ("p", "seven")))])
    out.append(("control_untracked", d, [setp(d, "c", 0, "Count"), setp(d, "five", 6)]))
    # Counter/ValueEdit fan-out: one param, many instances (handle is the site; SetParam moves all)
    d = D([("shared", "w", 3, "len"), ("shared", "n", 3, "count"),
           ("forN", "L", "i", ("p", "n"), [("feat", "h", ("p", "w"))])])
    out.append(("fan_out", d, [setp(d, "w", 4), setp(d, "n", 2, "Count")]))
    # Loops.count_prefix, and E8: a zero count is still structural
    d = D([("shared", "w", 3, "len"), ("shared", "n", 0, "count"),
           ("forN", "L", "i", ("p", "n"), [("feat", "h", ("p", "w"))])])
    out.append(("zero_count_structural", d, [setp(d, "n", 2, "Count")]))
    # Loops.keyed_insert: labels keep ids
    d = D([("shared", "w", 3, "len"),
           ("forK", "L", "k", [("lf", ("p", "w")), ("rf", ("mulc", ("p", "w"), 2))], [("feat", "h", ("v", "k"))])])
    out.append(("keyed_ids", d, []))
    # E4: an observed link param must log the params under it
    d = D([("shared", "t", 6, "len"), ("shared", "u", 7, "len"), ("link", "r", ("add", ("p", "t"), ("c", 1))),
           ("feat", "g", ("ite", ("eq", ("p", "r"), ("c", 7)), ("p", "t"), ("p", "u")))])
    out.append(("observed_link", d, [setp(d, "t", 7)]))
    # E5: an index is control, not data: the selected value keeps its handle
    d = D([("shared", "t", 6, "len"), ("shared", "u", 7, "len"), ("shared", "k", 1, "count"),
           ("feat", "c", ("sel", [("p", "t"), ("p", "u")], ("p", "k")))])
    out.append(("index_keeps_handle", d, [setp(d, "u", 8)]))
    # Counter/Refactor.make_reads_local: a block reading a document binding that is not shared
    d = D([("shared", "w", 4, "len"), ("let", "L", ("add", ("p", "w"), ("c", 1))),
           ("occ", "o", "N", [])], comps={"N": ([["d", 1]], [("feat", "lid", ("p", "L"))])})
    out.append(("make_reads_local", d, []))
    # Struct/Complete delete_refused: deleting what a later statement reads
    d = D([("shared", "w", 4, "len"), ("let", "a", ("p", "w")), ("feat", "f", ("p", "a"))])
    out.append(("delete_refused", d, [({"op": "DeleteFeature", "id": "a"}, dict(d, body=[d["body"][0], d["body"][2]]))]))
    # Scoped: ops inside a block; procedure ids carry the call name (Counter/Ids.no_call_path_clash)
    d = D([("shared", "w", 4, "len"), ("use", "l1", "boss", ("p", "w")), ("use", "l2", "boss", ("mulc", ("p", "w"), 2))],
          procs={"boss": ("x", [("feat", "hub", ("v", "x"))])})
    out.append(("call_path_ids", d, []))
    return out


def fixed_ops(name, doc, ops):
    return [(o, nd) for o, nd in ops]

# ---- mapping: Lean results to what fg reports --------------------------------------------------


def names_of(doc):
    sites, _ = doc_sites(doc)
    return {v: k for k, v in sites.items()}


def loop_kind(doc):
    """Lean loop names: Rust ids drop them (`for` has no name): L3.[i:2].e4 is e4[2]."""
    return {s[1] for s in doc["body"] if s[0] in ("forN", "forK")}


def rust_id(path, loops):
    segs = [p.split(":", 1) for p in path]
    out, keys = [], []
    for k, v in segs:
        if k == "n" and v in loops:
            continue
        if k in ("i", "k"):
            keys.append(v)
            continue
        out.append(v)
    s = ".".join(out)
    for key in keys:
        s += f"[{key}]"
    return s


def lean_view(doc, res):
    """What fg should say, from the Lean result: {"ok", "feats": {id: (value, src)}, "comps": set,
    "structural": {param: bool}}."""
    if not res["ok"]:
        return {"ok": False, "err": res["err"]}
    nm = names_of(doc)
    loops = loop_kind(doc)
    occs = {s[1]: s[2] for s in doc["body"] if s[0] == "occ"}
    feats, comps, per_occ = {}, set(), {}
    for path, val, h, sites in res["feats"]:
        if h is not None:
            src = ("driving", nm[h])
        elif sites:
            src = ("driven", tuple(sorted({nm[int(x)] for x in sites})))
        else:
            src = ("const",)
        first = path[0].split(":", 1)[1]
        if first in occs:
            inner = rust_id(path[1:], loops)
            comps.add((occs[first] + "." + inner, val, src))
            per_occ.setdefault(first, set()).add((occs[first] + "." + inner, val, src))
        else:
            feats[rust_id(path, loops)] = (val, src)
    log = {nm[int(x)] for x in res["log"]}
    return {"ok": True, "feats": feats, "comps": comps, "per_occ": per_occ,
            "structural": {nm[k]: nm[k] in log for k in nm}}


def rust_src(d):
    f = d["from"]
    if "driving" in f:
        return ("driving", f["driving"])
    if "driven" in f:
        return ("driven", tuple(sorted(f["driven"])))
    return ("const",)


def rust_view(j):
    if j["errors"]:
        return {"ok": False, "err": j["errors"][0]["kind"]}
    feats = {f["id"]: (int(round(f["dims"][0]["value"])), rust_src(f["dims"][0])) for f in j["features"] if f["dims"]}
    comps = set()
    for c in j.get("components", []):
        for f in c["features"]:
            if f["dims"]:
                comps.add((f["id"], int(round(f["dims"][0]["value"])), rust_src(f["dims"][0])))
    st = {p["param"]: p["structural"] for p in j["params"]}
    evals = [({(f["id"], int(round(f["dims"][0]["value"])), rust_src(f["dims"][0])) for f in c["features"] if f["dims"]},
              set(c["occurrences"])) for c in j.get("components", [])]
    used = {p["param"]: p["used_by"] for p in j["params"]}
    return {"ok": True, "feats": feats, "comps": comps, "structural": st, "evals": evals, "used_by": used}


ERRMAP = {"range": {"IndexOut", "Negative", "NegativeCount"}, "unbound": {"UnknownName", "StillRead"},
          "dupId": {"Rebind", "NameTaken"}}


def classify(doc, lv, rv):
    """Differences, each a (category, detail). Categories that are known findings (E4, E5) are named."""
    out = []
    if lv["ok"] != rv["ok"]:
        cat = "ok"
        if lv["ok"] and rv.get("err") == "IndexOut":
            cat = "strict-list"
        out.append((cat, f"lean {lv.get('err', 'ok')} rust {rv.get('err', 'ok')}"))
        return out
    if not lv["ok"]:
        return out
    links = {s[1]: s[2] for s in doc["body"] if s[0] == "link"}
    for k in sorted(set(lv["feats"]) | set(rv["feats"])):
        a, b = lv["feats"].get(k), rv["feats"].get(k)
        if a == b:
            continue
        if a is None or b is None:
            out.append(("ids", f"{k}: lean {a} rust {b}"))
            continue
        if a[0] != b[0]:
            out.append(("value", f"{k}: lean {a[0]} rust {b[0]}"))
            continue
        out.append((src_cat(a[1], b[1], lv, links), f"{k}: lean {a[1]} rust {b[1]}"))
    if lv["comps"] != rv["comps"]:
        la = {(x[0], x[1]) for x in lv["comps"]}
        ra = {(x[0], x[1]) for x in rv["comps"]}
        if la != ra:
            out.append(("comp-value", f"lean {sorted(lv['comps'])} rust {sorted(rv['comps'])}"))
        else:
            cat = "comp-src"
            # E1: equal values and param sets share one evaluation: Rust shows fewer distinct srcs
            if len(rv["comps"]) < len(lv["comps"]):
                cat = "E1"
            else:
                lm = {(x[0], x[1]): x[2] for x in lv["comps"]}
                cats = {src_cat(lm.get((x[0], x[1])), x[2], lv, links) for x in rv["comps"] if lm.get((x[0], x[1])) != x[2]}
                if cats and cats <= {"E5", "E4", "link"}:
                    cat = "/".join(sorted(cats))
            out.append((cat, f"lean {sorted(lv['comps'])} rust {sorted(rv['comps'])}"))
    if lv["comps"] == rv["comps"]:
        # E3: an evaluation lists only occurrences whose fragment it is
        for feats, occs in rv["evals"]:
            want = {o for o, f in lv["per_occ"].items() if f == feats}
            if not occs <= want:
                out.append(("E3", f"evaluation {sorted(feats)}: occurrences rust {sorted(occs)} lean {sorted(want)}"))
        # E2: a param driving a value inside a component is used by a feature
        for _, _, src in lv["comps"]:
            for q in (src[1:] if src[0] == "driving" else src[1] if src[0] == "driven" else ()):
                if q in rv["used_by"] and not rv["used_by"][q] and q not in links:
                    out.append(("E2", f"{q} drives a component's value, used_by []"))
    for p, s in lv["structural"].items():
        rs = rv["structural"].get(p)
        if rs != s:
            # E4: Rust logs the observed link param, not the sites under it
            cat = "E4" if s and not rs and any(rv["structural"].get(q) and p in mentions(e, links) for q, e in links.items()) else \
                "over-structural" if rs and not s else "structural"
            if cat == "structural" and s and p in loop_counts(doc, links):
                cat = "E8"
            if cat == "over-structural" and p in comp_reads(doc, links):
                cat = "E7"
            elif cat == "over-structural" and p in list_reads(doc, links):
                cat = "strict-list"
            out.append((cat, f"{p}: lean {s} rust {rs}"))
    return out


def stmts_walk(ss):
    for s in ss:
        yield s
        if s[0] in ("forN", "forK"):
            yield from stmts_walk(s[4])
        elif s[0] == "if":
            yield from stmts_walk(s[2])
            yield from stmts_walk(s[3])


def loop_counts(doc, links):
    out = set()
    for s in stmts_walk(doc["body"]):
        if s[0] == "forN":
            out |= mentions(s[3], links)
    return out


def exprs_of(doc):
    for s in stmts_walk(doc["body"]):
        for x in s[1:]:
            if isinstance(x, (list, tuple)) and x and isinstance(x[0], str) and x[0] in EXPR_KINDS:
                yield x
        if s[0] == "forK":
            yield from (e for _, e in s[3])
        if s[0] == "occ":
            yield from (e for _, e in s[3])
    for x, e in doc["fns"].values():
        yield e
    for x, ss in doc["procs"].values():
        for t in stmts_walk(ss):
            yield t[2]


EXPR_KINDS = {"p", "v", "c", "add", "mulc", "eq", "ite", "sel", "call"}


def sels(e):
    if not isinstance(e, (list, tuple)) or not e or e[0] not in EXPR_KINDS:
        return
    if e[0] == "sel":
        yield e
        for x in e[1]:
            yield from sels(x)
        yield from sels(e[2])
    elif e[0] in ("add", "eq"):
        yield from sels(e[1]); yield from sels(e[2])
    elif e[0] == "mulc":
        yield from sels(e[1])
    elif e[0] == "ite":
        for x in e[1:]:
            yield from sels(x)
    elif e[0] == "call":
        yield from sels(e[2])


def list_reads(doc, links):
    """Params mentioned in a list's elements: fg evaluates all of them, the model only the chosen."""
    out = set()
    fns = {f: e for f, (x, e) in doc["fns"].items()}
    for e in exprs_of(doc):
        for sl in sels(e):
            for x in sl[1]:
                out |= mentions(x, links)
                for f, fe in fns.items():
                    if f in str(x):
                        out |= mentions(fe, links)
    return out


def comp_reads(doc, links):
    """Params a component body may observe: what it and the functions mention."""
    out = set()
    for ps, ss in doc["comps"].values():
        for s in stmts_walk(ss):
            out |= mentions(s[2], links)
    for x, e in doc["fns"].values():
        out |= mentions(e, links)
    return out


def mentions(e, links, seen=()):
    out = set()
    k = e[0]
    if k == "p":
        out.add(e[1])
        if e[1] in links and e[1] not in seen:
            out |= mentions(links[e[1]], links, seen + (e[1],))
    elif k in ("add", "eq"):
        out |= mentions(e[1], links, seen) | mentions(e[2], links, seen)
    elif k == "mulc":
        out |= mentions(e[1], links, seen)
    elif k == "ite":
        for x in e[1:]:
            out |= mentions(x, links, seen)
    elif k == "sel":
        for x in e[1]:
            out |= mentions(x, links, seen)
        out |= mentions(e[2], links, seen)
    elif k == "call":
        out |= mentions(e[2], links, seen)
    return out


def src_cat(a, b, lv, links):
    """Why two srcs differ, as a known finding where it is one."""
    if a is None or b is None:
        return "src"
    lp = set(a[1:]) if a[0] == "driving" else set(a[1]) if a[0] == "driven" else set()
    rp = set(b[1:]) if b[0] == "driving" else set(b[1]) if b[0] == "driven" else set()
    # a link param read by name: Rust names the link, Lean its sites (mapping, not a finding)
    expand = set()
    for p in rp:
        expand |= ({p} if p not in links else {q for q in mentions(links[p], links) if q not in links})
    log = {p for p, s in lv["structural"].items() if s}
    if expand == lp or (rp & set(links) and lp <= expand):
        return "link"
    if lp <= expand and (expand - lp) <= (log | set(links)):
        return "E5"
    return "src"


# ---- commands ----------------------------------------------------------------------------------


STATIC = [
    # (name, files, main file, error kind wanted ("" = no error), the model's statement)
    ("comp_scope_differs", {"m.fg": "shared w = 4\nfn f(k) {\n  return k + w\n}\ncomponent c(w: 2) {\n  sketch s(on: xy) {\n    o = circle((0, 0), f(1))\n  }\n  e = extrude(s, w)\n}\nx = part(c)\n"},
     "m.fg", "Rebind", "Counter/Refactor.comp_scope_differs; rule: a component param may not reuse a shared param's name"),
    ("use_cycle", {"a.fg": "b = use(\"b.fg\")\nshared x = 1\n", "b.fg": "a = use(\"a.fg\")\nshared y = 2\n",
                   "m.fg": "q = use(\"a.fg\")\nshared z = 1\n"}, "m.fg", "UseFailed", "Modules.use_cycle (useCycle)"),
    ("dead_param", {"m.fg": "shared w = 1\n" + "\n".join(SKETCH) + "\nw = 2\nf = extrude(plan, w)\n"},
     "m.fg", "Rebind", "Counter/Rebind.dead_param: params are not reassigned"),
    ("delete_silent", {"m.fg": "shared one = 1\n" + "\n".join(SKETCH) + "\na = one\na = 2\nf = extrude(plan, a)\n"},
     "m.fg", "Rebind", "Counter/Rebind.delete_silent: single assignment"),
    ("index_shift", {"m.fg": "shared w = 2\n" + "\n".join(SKETCH) + "\nfor x in [w, w] {\n  h = extrude(plan, x)\n}\n"},
     "m.fg", "PositionalId", "Counter/Ids.index_shift: no index ids over edited lists"),
    ("ref_dangles", {"m.fg": "shared w = 2\nshared n = count(2)\n" + "\n".join(SKETCH) + "\nfor i in 0..n {\n  h = extrude(plan, w)\n}\ng = extrude(h[2].end, w)\n"},
     "m.fg", "IndexOut", "Counter/Ids.ref_dangles (dangling)"),
    ("ref_negative", {"m.fg": "shared w = 2\nshared n = count(2)\n" + "\n".join(SKETCH) + "\nfor i in 0..n {\n  h = extrude(plan, w)\n}\ng = extrude(h[-1].end, w)\n"},
     "m.fg", "IndexOut", "Counter/Ids.ref_dangles (range): no relative references"),
    ("diverge", {"m.fg": "shared w = 2\n" + "\n".join(SKETCH) + "\nfn f(n) {\n  return 0 if n == 0 else f(n - 1)\n}\ng = extrude(plan, w + f(-1))\n"},
     "m.fg", "Fuel", "Counter/Diverge.down_budget (fuel)"),
    ("nested_component", {"m.fg": "component a(d: 1) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  e = extrude(s, d)\n}\ncomponent b(d: 1) {\n  x = part(a)\n}\ny = part(b)\n"},
     "m.fg", "InComponent", "Scoped.NoPlace: a block places no component (the scoped-op theorems use it)"),
    ("default_no_capture", {"m.fg": "shared w = 4\n" + "\n".join(SKETCH) + "\nd = w + 100\ncomponent cup(d: 80, h: d * 2) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, h)\n}\nc = part(cup(d: 50))\n"},
     "m.fg", "UnknownName", "ScopedMore.default_no_capture (unbound d); Rust: E10"),
    ("use_member_component", {"lib.fg": "shared W = 7\ncomponent cup(d: 9) {\n  sketch s(on: xy) {\n    o = circle((0, 0), 5)\n  }\n  b = extrude(s, d + W)\n}\n",
                              "m.fg": "t = use(\"lib.fg\")\nshared d = 3\nc1 = part(t.cup(d: d))\n"},
     "m.fg", "", "Modules: a used module's components are linked (design); Rust: E9"),
]

# Cross-run cache keys (Counter/Memo.key_needs_*): one `fg serve` session, the file changed
# between runs; each version's numbers must be the model's for that version.
SERVE = [
    ("key_needs_literals", [D([("occ", "c1", "cup", [("d", ("c", 2))])], comps={"cup": cup(("mulc", ("v", "d"), k))}) for k in (2, 3, 2)]),
    ("key_needs_shared", [D([("shared", "W", w, "len"), ("occ", "c1", "cup", [("d", ("c", 1))])],
                            comps={"cup": cup(("add", ("v", "d"), ("p", "W")))}) for w in (1, 2, 1)]),
    ("key_needs_defs", [D([("occ", "c1", "cup", [("d", ("c", 3))])], fns={"f": ("x", fb)},
                          comps={"cup": cup(("call", "f", ("v", "d")))}) for fb in (("v", "x"), ("add", ("v", "x"), ("v", "x")), ("v", "x"))]),
]


def all_cases(seed, count, fixed=True):
    """Every case: (key, doc, ops). The generated ones from the seed, then the fixed ones."""
    g = Gen(random.Random(seed))
    out = []
    for i in range(count):
        d = g.doc()
        out.append((f"s{seed}c{i}", d, g.ops(d)))
    if fixed:
        for name, d, ops in fixed_cases():
            out.append((f"fixed_{name}", d, ops))
        for name, versions in SERVE:
            for k, d in enumerate(versions):
                out.append((f"serve_{name}_v{k}", d, []))
    return out


def lean_file(cases):
    lean = [LEAN_HEAD]
    keys = []
    for key, d, ops in cases:
        lean.append(f"def d_{key} : Doc := {lean_doc(d)}")
        keys.append(key)
        for k, (o, nd) in enumerate(ops):
            lean.append(f"def d_{key}_{k} : Doc := {lean_doc(nd)}")
            keys.append(f"{key}_{k}")
    parts = [keys[i:i + 40] for i in range(0, len(keys), 40)]
    for n, ks in enumerate(parts):
        lean.append(f"def main{n} : IO Unit := do")
        lean += [f'  IO.println (report "{k}" d_{k})' for k in ks]
    lean.append("def main : IO Unit := do")
    lean += [f"  main{n}" for n in range(len(parts))]
    return "\n".join(lean) + "\n"


def run_lean(text, work):
    path = os.path.join(work, "cases.lean")
    with open(path, "w") as f:
        f.write(text)
    env = dict(os.environ, PATH=os.path.expanduser("~/.elan/bin") + ":" + os.environ.get("PATH", ""))
    out = subprocess.run(["lake", "env", "lean", "--run", path], cwd=LEAN, env=env, capture_output=True, text=True)
    if out.returncode:
        sys.exit(out.stderr[-4000:] + out.stdout[-2000:])
    return "".join(l + "\n" for l in out.stdout.splitlines() if l.startswith("{"))


def sha(text):
    import hashlib
    return hashlib.sha256(text.encode()).hexdigest()


def cmd_fixture(a):
    """Runs Lean on the cases of (seed, count) and writes the fixture the test reads."""
    cases = all_cases(a.seed, a.count)
    text = lean_file(cases)
    os.makedirs(a.dir, exist_ok=True)
    out = run_lean(text, tempfile.mkdtemp(prefix="core-diff-lean-"))
    with open(os.path.join(a.dir, "expected.jsonl"), "w") as f:
        f.write(out)
    with open(os.path.join(a.dir, "meta.json"), "w") as f:
        json.dump({"seed": a.seed, "count": a.count, "lean_sha256": sha(text)}, f, indent=1)
        f.write("\n")
    print(f"{len(cases)} programs, {len(out.splitlines())} Lean results -> {a.dir}")


def fg_json(fg, path, build):
    out = subprocess.run([fg, "run" if build else "check", path, "--json"], capture_output=True, text=True)
    return json.loads(out.stdout)


def norm(text):
    return [l.strip() for l in text.splitlines() if l.strip()]


def compare(cases, lean, fg, tmp, note):
    for key, d, ops in cases:
        if key.startswith("serve_"):
            continue
        fgtext = fg_doc(d)
        build = bool(d["comps"])
        path = os.path.join(tmp, key + ".fg")
        with open(path, "w") as f:
            f.write(fgtext)
        rv = rust_view(fg_json(fg, path, build))
        for cat, det in classify(d, lean_view(d, lean[key]), rv):
            note(cat, key, det, fgtext)
        note("programs", key, "", "")
        if not lean[key]["ok"]:
            # the document already fails: an op is refused only on a new kind of error
            continue
        for k, (o, nd) in enumerate(ops):
            ek = f"{key}_{k}"
            p2 = os.path.join(tmp, ek + ".fg")
            with open(p2, "w") as f:
                f.write(fgtext)
            r = subprocess.run([fg, "edit", p2, json.dumps(o)], capture_output=True, text=True)
            try:
                res = json.loads(r.stdout.splitlines()[0])
            except (json.JSONDecodeError, IndexError):
                res = {"ok": False, "error": r.stderr.strip()}
            le, opn = lean[ek], o["op"]
            note("ops", ek, "", "")
            if not res.get("ok"):
                if le["ok"]:
                    err = res.get("error")
                    strict = isinstance(err, dict) and err.get("kind") == "IndexOut"
                    note(f"op-refused:{opn}" + (":strict-list" if strict else ""), ek, f"{o} rust {err} lean ok", fgtext)
                continue
            if not le["ok"]:
                note(f"op-applied:{opn}", ek, f"{o} lean {le['err']}", fgtext)
                continue
            new = open(p2).read()
            want = fg_doc(nd)
            if norm(new) != norm(want):
                note(f"op-text:{opn}", ek, f"{o}\n--- rust\n{new}--- want\n{want}", fgtext)
            for cat, det in classify(nd, lean_view(nd, le), rust_view(fg_json(fg, p2, build))):
                note(f"after-{opn}:{cat}", ek, det, new)


def compare_static(fg, tmp, note):
    for name, files, main, want, why in STATIC:
        d = os.path.join(tmp, "static_" + name)
        os.makedirs(d, exist_ok=True)
        for f, text in files.items():
            with open(os.path.join(d, f), "w") as fh:
                fh.write(text)
        j = fg_json(fg, os.path.join(d, main), True)
        kinds = [e["kind"] for e in j["errors"]]
        ok = (want in kinds) if want else not kinds
        note("static", name, "", "")
        if not ok:
            note(f"static:{name}", name, f"want {want or 'no error'}, rust {kinds} ({why})", files[main])


def compare_serve(lean, fg, tmp, note):
    for name, versions in SERVE:
        path = os.path.join(tmp, f"serve_{name}.fg")
        p = subprocess.Popen([fg, "serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        for k, d in enumerate(versions):
            with open(path, "w") as f:
                f.write(fg_doc(d))
            p.stdin.write(json.dumps({"file": path, "cmd": "run"}) + "\n")
            p.stdin.flush()
            j = json.loads(p.stdout.readline())
            lv, rv = lean_view(d, lean[f"serve_{name}_v{k}"]), rust_view(j)
            vol = (j.get("physical") or {}).get("volume")
            note("serve", f"{name}_v{k}", "", "")
            for cat, det in classify(d, lv, rv):
                note(f"serve:{cat}", f"{name}_v{k}", det, fg_doc(d))
            # the geometry follows the numbers: a disc of r 5, as deep as the model's value
            depth = next(iter(lv["comps"]))[1] if lv["ok"] and lv["comps"] else None
            if depth is not None and (vol is None or abs(vol - 3.141592653589793 * 25 * depth) > 1e-3 * vol):
                note("serve:geometry", f"{name}_v{k}", f"volume {vol} for depth {depth}", fg_doc(d))
        p.stdin.close()
        p.wait()


def cmd_test(a):
    """The test: the fixture's Lean results against fg, on cases regenerated from the seed."""
    meta = json.load(open(os.path.join(a.dir, "meta.json")))
    cases = all_cases(meta["seed"], meta["count"])
    if sha(lean_file(cases)) != meta["lean_sha256"]:
        sys.exit("the cases no longer match the fixture: rerun `core_diff.py fixture` (needs Lean)")
    lean = {}
    for line in open(os.path.join(a.dir, "expected.jsonl")):
        j = json.loads(line)
        lean[j["case"]] = j
    run_all(cases, lean, a)


def cmd_long(a):
    """Long run: Lean and fg on fresh seeds (needs Lean)."""
    for seed in range(a.seed, a.seed + a.seeds):
        cases = all_cases(seed, a.count, fixed=seed == a.seed)
        out = run_lean(lean_file(cases), tempfile.mkdtemp(prefix="core-diff-lean-"))
        lean = {json.loads(l)["case"]: json.loads(l) for l in out.splitlines()}
        print(f"seed {seed}")
        run_all(cases, lean, a, exit_=False)


def run_all(cases, lean, a, exit_=True):
    tmp = tempfile.mkdtemp(prefix="core-diff-")
    tally, examples = {}, {}

    def note(cat, key, detail, text):
        tally[cat] = tally.get(cat, 0) + 1
        examples.setdefault(cat, (key, detail, text))

    compare(cases, lean, a.fg, tmp, note)
    if any(k.startswith("fixed_") for k, _, _ in cases):
        compare_static(a.fg, tmp, note)
        compare_serve(lean, a.fg, tmp, note)
    known = set(a.known.split(","))
    counts = {"programs", "ops", "static", "serve"}
    bad = sorted(k for k in tally if k not in counts and k.split(":")[-1] not in known)
    w = max(len(k) for k in tally)
    for k in sorted(tally):
        flag = "  NEW" if k in bad else ""
        print(f"{k:{w}}  {tally[k]}{flag}")
    if a.examples or bad:
        print("\n# examples")
        for k, (key, det, text) in sorted(examples.items()):
            if k in counts or (not a.examples and k not in bad):
                continue
            print(f"\n## {k}: {key}\n{det}\n{text}")
    if exit_:
        sys.exit(1 if bad else 0)


# Differences that are filed findings (impl-proof-findings.md), or the mapping itself (`link`:
# fg names a shared param whose value is an expression where the model names its sites;
# `strict-list`: fg evaluates every element of a list, the model's `sel` only the chosen one).
KNOWN = "link,strict-list,E1,E2,E3,E4,E5,E7,E8,comp_scope_differs_ok,default_no_capture,use_member_component"


def main():
    p = argparse.ArgumentParser()
    p.add_argument("cmd", choices=["fixture", "test", "long"])
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--seeds", type=int, default=5)
    p.add_argument("--count", type=int, default=60)
    p.add_argument("--dir", default=os.path.join(HERE, "fixture"))
    p.add_argument("--fg", default="fg")
    p.add_argument("--examples", action="store_true")
    p.add_argument("--known", default=KNOWN)
    a = p.parse_args()
    {"fixture": cmd_fixture, "test": cmd_test, "long": cmd_long}[a.cmd](a)


if __name__ == "__main__":
    main()
