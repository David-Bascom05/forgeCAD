# values as python ints scaled by 2^S (all inputs on that grid); exact, fast
import random, sys
P = int(sys.argv[1]); S = 100
def bl(n): return n.bit_length()
def rnd(x):
    if x == 0: return 0
    s = -1 if x < 0 else 1; a = abs(x)
    sh = bl(a) - P
    if sh <= 0: return x
    n = a >> sh; r = a - (n << sh); half = 1 << (sh - 1)
    if r > half or (r == half and n & 1): n += 1
    return s * (n << sh)
def nu(x): x = abs(x); return (x & -x).bit_length() - 1
def E(x): return bl(abs(x)) - 1
def pow2(x): return x != 0 and E(x) == nu(x)
def NO(l):
    nz = [x for x in l if x]; return all(E(a) < nu(b) for a, b in zip(nz, nz[1:]))
def adj(a, b): return E(a) + 1 == nu(b)
def SNO(l):
    if not NO(l): return False
    nz = [x for x in l if x]
    for a, b in zip(nz, nz[1:]):
        if adj(a, b) and not (pow2(a) and pow2(b)): return False
    for a, b, c in zip(nz, nz[1:], nz[2:]):
        if adj(a, b) and adj(b, c): return False
    return True
def two_sum(a, b): x = rnd(a + b); return x, a + b - x
def fes(e, f):
    g = []; i = j = 0
    while i < len(e) or j < len(f):
        if j >= len(f) or (i < len(e) and abs(e[i]) < abs(f[j])): g.append(e[i]); i += 1
        else: g.append(f[j]); j += 1
    if not g: return []
    h = []; q = g[0]
    for x in g[1:]:
        q, err = two_sum(q, x)
        if err: h.append(err)
    if q or not h: h.append(q)
    return h
def two_prod(a, b): x = rnd(a * b >> S); return x, (a * b >> S) - x   # products: exact on grid if no underflow
def scale(e, b):
    if not e: return []
    h = []; q, lo = two_prod(e[0], b)
    if lo: h.append(lo)
    for ei in e[1:]:
        p1, p0 = two_prod(ei, b)
        s, err = two_sum(q, p0)
        if err: h.append(err)
        s2, err2 = two_sum(p1, s)
        if err2: h.append(err2)
        q = s2
    if q or not h: h.append(q)
    return h
def rand_float(lo=110, hi=160):
    m = random.randint(1, 2**P - 1); return random.choice([-1, 1]) * (m << random.randint(lo, hi))
def rand_exp():
    while True:
        l = sorted([rand_float() for _ in range(random.randint(0, 4))], key=abs)
        if SNO(l): return l
if __name__ == "__main__":
    random.seed(int(sys.argv[2])); N = int(sys.argv[3]); mode = sys.argv[4]
    pool = [rand_exp() for _ in range(40)]
    over = wrong = nsno = 0
    for t in range(N):
        a, b = random.choice(pool), random.choice(pool)
        if random.random() < 0.5: b = [-x for x in b]
        if mode == "fes": h = fes(a, b)
        else:
            if random.random() < 0.5: h = fes(a, b)
            else:
                bb = rand_float(100, 130); h = scale(a, bb)
        if not NO(h):
            over += 1
            if over <= 3: print("OVERLAP", a, b, h)
        s = sum(h); top = h[-1] if h else 0
        if (s > 0) != (top > 0) or (s < 0) != (top < 0): wrong += 1
        if not SNO(h): nsno += 1
        if 0 < len(h) <= 10: pool[random.randrange(len(pool))] = h
    print("P", P, mode, "overlap", over, "wrongsign", wrong, "notSNO", nsno)
