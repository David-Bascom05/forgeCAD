import random, sys
import fast
from fast import *
def W2(l):
    if not NO(l): return False
    nz = [x for x in l if x]
    return all(pow2(a) for a, b in zip(nz, nz[1:]) if adj(a, b))
def W1(l):
    if not W2(l): return False
    nz = [x for x in l if x]
    return not any(adj(a, b) and adj(b, c) for a, b, c in zip(nz, nz[1:], nz[2:]))
cls = {"W1": W1, "W2": W2, "SNO": SNO}[sys.argv[4]]
def gen():
    # build bottom-up, biased toward adjacency and powers of two
    while True:
        l = []; k = random.randint(110, 130)
        for _ in range(random.randint(0, 5)):
            r = random.random()
            if r < 0.4: m = 1 << random.randint(0, P - 1)       # power of two
            else: m = random.randint(1, 2**P - 1) | 1
            x = random.choice([-1, 1]) * (m << k)
            l.append(x); k = E(x) + 1 + random.choice([0, 0, 1, 2])   # next low bit: adjacent often
        if cls(l): return l
random.seed(int(sys.argv[2])); N = int(sys.argv[3]); fails = 0
for t in range(N):
    a, b = gen(), gen()
    if sys.argv[5] == "fes": h = fes(a, b)
    else: h = scale(a, random.choice([-1, 1]) * (random.randint(1, 2**P - 1) << random.randint(100, 110)))
    s = sum(h); top = h[-1] if h else 0
    ok = cls(h) and ((s > 0) == (top > 0)) and ((s < 0) == (top < 0))
    if not ok:
        fails += 1
        if fails <= 3: print("FAIL", a, b if sys.argv[5] == "fes" else "", h, "NO", NO(h), "W2", W2(h))
print(sys.argv[4], sys.argv[5], "P", P, "fails", fails, "of", N)
