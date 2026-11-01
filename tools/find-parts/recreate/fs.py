"""CADFS FeatureScript as data: a small recursive-descent reader for the one regular form the
dataset's programs take. Nothing is evaluated; a program is text. Run with python3 -I."""
import math
import re

TOK = re.compile(r'\s*(?:(-?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)|("(?:[^"\\]|\\.)*")|([A-Za-z_][\w.]*)|(.))', re.S)
UNITS = {"mm": 1.0, "millimeter": 1.0, "meter": 1000.0, "centimeter": 10.0, "inch": 25.4, "foot": 304.8, "degree": 1.0, "radian": 180.0 / math.pi}


class Bad(Exception):
    pass


class Call(tuple):
    """(name, args)"""


class P:
    def __init__(self, s):
        s = re.sub(r"'([^'\"]*)'", r'"\1"', s)
        self.t = [(m.group(1), m.group(2), m.group(3), m.group(4)) for m in TOK.finditer(s) if m.group(0).strip()]
        self.i = 0

    def peek(self):
        return self.t[self.i] if self.i < len(self.t) else (None, None, None, None)

    def sym(self, c):
        if self.peek()[3] == c:
            self.i += 1
            return True
        return False

    def need(self, c):
        if not self.sym(c):
            raise Bad(f"expected {c} at {self.peek()}")

    def expr(self):
        v = self.term()
        while True:
            if self.sym("+"):
                r = self.term()
                v = id_join(v, r)
            elif self.peek()[3] == "-" or (self.peek()[0] or "").startswith("-"):
                if self.peek()[3] == "-":
                    self.i += 1
                v = arith(v, self.term(), -1)
            else:
                return v

    def term(self):
        v = self.atom()
        while True:
            if self.sym("*"):
                v = times(v, self.atom())
            elif self.sym("/"):
                r = self.atom()
                if not isinstance(v, float) or not isinstance(r, float) or r == 0:
                    raise Bad("division")
                v /= r
            else:
                return v

    def atom(self):
        n, s, w, c = self.peek()
        self.i += 1
        if n is not None:
            return float(n)
        if s is not None:
            return s[1:-1]
        if c == "-":
            v = self.atom()
            return times(v, -1.0)
        if c == "(":
            v = self.expr()
            self.need(")")
            return v
        if c == "[":
            out = []
            while not self.sym("]"):
                out.append(self.expr())
                self.sym(",")
            return out
        if c == "{":
            out = {}
            while not self.sym("}"):
                k = self.atom()
                self.need(":")
                out[k] = self.expr()
                self.sym(",")
            return out
        if w is not None:
            if self.sym("("):
                args = []
                while not self.sym(")"):
                    args.append(self.expr())
                    self.sym(",")
                if w == "getVariable":
                    if len(args) == 2 and args[1] in VARS:
                        return VARS[args[1]]
                    raise Bad("an unknown variable")
                if w in ("v", "vector") and all(isinstance(x, float) for x in args):
                    return args
                return Call((w, args))
            if w in ("true", "false"):
                return w == "true"
            if w in UNITS:
                return UNITS[w]
            return ("name", w)
        raise Bad(f"unexpected {(n, s, w, c)}")


def times(a, b):
    if isinstance(a, float) and isinstance(b, float):
        return a * b
    if isinstance(a, Call) and a[0] in ("v", "vector") and isinstance(b, float):
        return [x * b for x in a[1]]
    if isinstance(a, list) and isinstance(b, float):
        return [x * b for x in a]
    raise Bad("product")


def arith(a, b, s):
    if isinstance(a, float) and isinstance(b, float):
        return a + s * b
    raise Bad("sum")


def id_join(a, b):
    if a == ("name", "id") and isinstance(b, str):
        return ("id", b)
    if isinstance(a, float) and isinstance(b, float):
        return a + b
    raise Bad("join")


VARS = {}


def expr(s):
    p = P(s)
    v = p.expr()
    return v


OPS = ("extrude", "fillet", "chamfer", "revolve", "loft", "mirror", "booleanBodies", "hole", "shell", "sweep", "transform", "circularPattern", "deleteBodies", "cPlane", "cPoint", "assignVariable")


def statements(block):
    """Top-level statements of a block; a scope's braces at a statement's start are dropped."""
    out, cur, depth, q = [], [], 0, False
    i = 0
    while i < len(block):
        c = block[i]
        if q:
            cur.append(c)
            if c == "\\":
                cur.append(block[i + 1])
                i += 1
            elif c == '"':
                q = False
        elif c == '"':
            q = True
            cur.append(c)
        elif depth == 0 and c in "{}" and not "".join(cur).strip():
            cur = []
        elif c == ";" and depth == 0:
            out.append("".join(cur).strip())
            cur = []
        else:
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
            cur.append(c)
        i += 1
    return out


def closed(v, env):
    """A value with the names it uses replaced as they stand now: a scope's `subQ0` is that scope's, and the next scope's reuses the name."""
    if isinstance(v, Call):
        return Call((v[0], [closed(x, env) for x in v[1]]))
    if isinstance(v, tuple) and len(v) == 2 and v[0] == "name":
        return env.get(v[1], v)
    if isinstance(v, list):
        return [closed(x, env) for x in v]
    if isinstance(v, dict):
        return {k: closed(x, env) for k, x in v.items()}
    return v


def program(text):
    """Features in order: {op, id, args, env, ents (sketch)}. env: the block's query variables."""
    i = text.find("precondition")
    feats = []
    VARS.clear()
    for block in re.split(r"\n        \{\n", text[i:])[1:]:
        env, ent, feat = {}, [], None
        for line in statements(block):
            m = re.match(r"(?:var\s+|const\s+)?(\w+)\s*=\s*(.*)$", line, re.S)
            k = re.match(r"(\w+)\s*\(", line)
            if re.match(r"var\s+\w+$", line):
                continue
            if m and m.group(2).startswith("newSketch"):
                c = expr(m.group(2))
                feat = {"op": "sketch", "id": c[1][1][1], "args": c[1][2], "ents": ent, "env": env}
            elif m and m.group(1) == "initialGuess":
                continue
            elif m and not k:
                env[m.group(1)] = closed(expr(m.group(2)), env)
            elif k and k.group(1) in OPS:
                c = expr(line)
                feat = {"op": c[0], "id": c[1][1][1], "args": c[1][2], "env": env}
                if c[0] == "assignVariable":
                    a = c[1][2]
                    for k in ("anyValue", "lengthValue", "numberValue", "angleValue"):
                        if isinstance(a.get(k), float):
                            VARS[a.get("name")] = a[k]
            elif k and k.group(1).startswith("sk") and k.group(1) not in ("skSolve", "skSetInitialGuess"):
                c = expr(line)
                ent.append({"kind": c[0], "id": c[1][1], **c[1][2]})
            elif k and k.group(1) in ("skSolve", "skSetInitialGuess", "setExternalDisambiguation"):
                continue
            else:
                raise Bad("statement " + line[:60])
        if feat:
            feats.append(feat)
    return feats
