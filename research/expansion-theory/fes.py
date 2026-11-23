from fractions import Fraction as Fr
import random, sys
P = int(sys.argv[1]) if len(sys.argv) > 1 else 4

def rnd(x):
    """round x to P bits, ties to even, unbounded exponent"""
    if x == 0: return Fr(0)
    s = -1 if x < 0 else 1
    a = abs(x)
    e = 0
    while a >= 2**P: a /= 2; e += 1
    while a < 2**(P-1): a *= 2; e -= 1
    n = int(a); fr = a - n
    if fr > Fr(1,2) or (fr == Fr(1,2) and n % 2 == 1): n += 1
    return s * Fr(n) * Fr(2)**e

def two_sum(a, b):
    x = rnd(a + b); return x, a + b - x

def nu(x):  # 2-adic valuation
    x = abs(x); k = 0
    while x.denominator % 2 == 0 or (x.numerator % 2 == 0):
        if x.numerator % 2 == 0: x /= 2; k += 1
        else: x *= 2; k -= 1
    return k
def E(x):  # floor log2
    x = abs(x); k = 0
    while x >= 2: x /= 2; k += 1
    while x < 1: x *= 2; k -= 1
    return k
def pow2(x): return x != 0 and E(x) == nu(x)
def isfloat(x): return x == rnd(x)

def NO(l):
    nz = [x for x in l if x != 0]
    return all(E(a) < nu(b) for a, b in zip(nz, nz[1:]))
def adj(a, b): return E(a) + 1 == nu(b)
def SNO(l):
    nz = [x for x in l if x != 0]
    if not NO(l): return False
    for i in range(len(nz) - 1):
        if adj(nz[i], nz[i+1]) and not (pow2(nz[i]) and pow2(nz[i+1])): return False
    for i in range(len(nz) - 2):
        if adj(nz[i], nz[i+1]) and adj(nz[i+1], nz[i+2]): return False
    return True

def fes(e, f):
    # forge-math sum_into: merge (e first iff |e| < |f| strictly), two_sum chain, drop zero errs
    g = []; i = j = 0
    while i < len(e) or j < len(f):
        if j >= len(f) or (i < len(e) and abs(e[i]) < abs(f[j])): g.append(e[i]); i += 1
        else: g.append(f[j]); j += 1
    if not g: return []
    h = []; q = g[0]
    for x in g[1:]:
        q, err = two_sum(q, x)
        if err != 0: h.append(err)
    if q != 0 or not h: h.append(q)
    return h

def rand_float():
    m = random.randint(1, 2**P - 1); s = random.choice([-1, 1])
    return s * Fr(m) * Fr(2)**random.randint(-12, 12)

def rand_exp():
    while True:
        n = random.randint(0, 4)
        l = sorted([rand_float() for _ in range(n)], key=abs)
        if SNO(l): return l

if __name__ == "__main__":
  random.seed(int(sys.argv[2]) if len(sys.argv) > 2 else 1)
  bad = 0
  for t in range(int(sys.argv[3]) if len(sys.argv) > 3 else 200000):
      e, f = rand_exp(), rand_exp()
      h = fes(e, f)
      assert sum(h) == sum(e) + sum(f)
      if not SNO(h):
          bad += 1
          if bad <= 5: print("NOT SNO", e, f, "->", h, "NO" if NO(h) else "overlap")
  print("bad", bad)
