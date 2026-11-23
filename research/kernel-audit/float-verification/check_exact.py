"""Reads predicates' `dump` example output and checks every sign against exact rationals.

Python floats are IEEE binary64 with round-to-nearest-even, as Rust's f64, so the fast test is
recomputed here to count how many cases went past it to the adaptive and exact paths.
"""
import struct
import sys
from fractions import Fraction as F

EPS = 2.0 ** -53
CCW = (3.0 + 16.0 * EPS) * EPS
CROSS = (2.0 + 8.0 * EPS) * EPS


def d(h):
    return struct.unpack("<d", struct.pack("<Q", int(h, 16)))[0]


def sgn(x):
    return (x > 0) - (x < 0)


cases = slow_o = slow_c = bad = 0
for line in sys.stdin:
    v = [d(h) for h in line.split()]
    ax, ay, bx, by, cx, cy, o, p, q, r, w, x = v
    cases += 1
    left, right = (ax - cx) * (by - cy), (ay - cy) * (bx - cx)
    det = left - right
    bound = CCW * (abs(left) + abs(right))
    if not (det > bound or -det > bound):
        slow_o += 1
    exact = (F(ax) - F(cx)) * (F(by) - F(cy)) - (F(ay) - F(cy)) * (F(bx) - F(cx))
    if sgn(o) != sgn(exact):
        bad += 1
        print("orient2d WRONG", line.strip())
    l2, r2 = p * q, r * w
    if not (abs(l2 - r2) > CROSS * (abs(l2) + abs(r2))):
        slow_c += 1
    if sgn(x) != sgn(F(p) * F(q) - F(r) * F(w)):
        bad += 1
        print("cross_sign WRONG", line.strip())
print(f"{cases} cases: orient2d past the fast test {slow_o}, cross_sign past it {slow_c}, wrong signs {bad}")
sys.exit(1 if bad else 0)
