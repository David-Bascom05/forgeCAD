import random, sys
import fes as F
F.P = 4
import fast
from fast import SNO, E, nu, scale, P
from fractions import Fraction as Fr
def gen():
    while True:
        l = []; k = random.randint(110, 130)
        for _ in range(random.randint(0, 5)):
            if random.random() < 0.4: m = 1 << random.randint(0, P - 1)
            else: m = random.randint(1, 2**P - 1) | 1
            x = random.choice([-1, 1]) * (m << k)
            l.append(x); k = E(x) + 1 + random.choice([0, 0, 1, 2])
        if SNO(l): return l
random.seed(1); n = 0
for t in range(100000):
    a, b = gen(), gen()
    bb = random.choice([-1, 1]) * (random.randint(1, 2**P - 1) << random.randint(100, 110))
    h = scale(a, bb)
    if not SNO(h):
        n += 1
        if n == 1:
            m = min(nu(x) for x in a if x); S = 2**100
            ef = [Fr(x, 2**m) for x in a]; bf = Fr(bb, 2**(nu(bb)))
            def tp(x, y): z = F.rnd(x*y); return z, x*y - z
            hh = []; q, lo = tp(ef[0], bf)
            if lo: hh.append(lo)
            for ei in ef[1:]:
                p1, p0 = tp(ei, bf); s, err = F.two_sum(q, p0)
                if err: hh.append(err)
                s2, err2 = F.two_sum(p1, s)
                if err2: hh.append(err2)
                q = s2
            if q or not hh: hh.append(q)
            print("e =", [str(x) for x in ef], "SNO(e):", F.SNO(ef), "floats:", all(F.rnd(x) == x for x in ef))
            print("b =", str(bf))
            print("h =", [str(x) for x in hh], "SNO(h):", F.SNO(hh), "NO(h):", F.NO(hh), "sum ok:", sum(hh) == bf * sum(ef))
print("violations", n)
