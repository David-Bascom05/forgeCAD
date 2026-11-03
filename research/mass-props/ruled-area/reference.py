#!/usr/bin/env python3
"""Reference areas for area.rs's four bodies, by 40-digit quadrature (mpmath) of |S_u x S_v| over each
wall's parameter square, plus the caps in closed form. Independent of the kernel."""
import mpmath as mp

mp.mp.dps = 40
pi = mp.pi


def quad2(f):
    return mp.quad(f, [0, 0.5, 1], [0, 1])


# plug: disc radius 3 about (2, 1) in (s, h), unroll 10.8, radii 10..12
A, U, LO, HI = 3, mp.mpf(54) / 5, 10, 12
wall = mp.quad(lambda u, r: mp.sqrt(r**2 * (A * mp.sin(u) / U) ** 2 + (A * mp.cos(u)) ** 2),
               [0, pi / 2, pi, 3 * pi / 2, 2 * pi], [LO, HI])
plug = wall + pi * A**2 * (LO + HI) / U

# hyperboloid: circles radius 30 at z 0 and 24 at z 40, the second turned 0.5; rulings join equal angles
a, b, h, turn = 30, 24, 40, mp.mpf("0.5")


def hyp(t, v):
    p0 = mp.matrix([a * mp.cos(t), a * mp.sin(t), 0])
    p1 = mp.matrix([b * mp.cos(t + turn), b * mp.sin(t + turn), h])
    d0 = mp.matrix([-a * mp.sin(t), a * mp.cos(t), 0])
    d1 = mp.matrix([-b * mp.sin(t + turn), b * mp.cos(t + turn), 0])
    su = d0 + v * (d1 - d0)
    sv = p1 - p0
    return mp.norm(mp.matrix([su[1] * sv[2] - su[2] * sv[1], su[2] * sv[0] - su[0] * sv[2], su[0] * sv[1] - su[1] * sv[0]]))


hyperboloid = mp.quad(hyp, [0, pi / 2, pi, 3 * pi / 2, 2 * pi], [0, 1]) + pi * (a * a + b * b)

# twisted square: 20 square at z 0 to a 12 square at z 15 turned 0.6; side k joins corner k..k+1 of each
c, s = mp.cos(mp.mpf("0.6")), mp.sin(mp.mpf("0.6"))


def corners(w, z, rot):
    out = []
    for x, y in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
        x, y = x * w / 2, y * w / 2
        out.append(mp.matrix([x * rot[0] - y * rot[1], x * rot[1] + y * rot[0], z]))
    return out


lo, hi = corners(20, 0, (1, 0)), corners(12, 15, (c, s))


def cross(p, q):
    return mp.matrix([p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2], p[0] * q[1] - p[1] * q[0]])


sq = 400 + 144
for k in range(4):
    a0, a1, b0, b1 = lo[k], lo[(k + 1) % 4], hi[k], hi[(k + 1) % 4]
    sq += quad2(lambda t, v: mp.norm(cross((a1 - a0) + v * ((b1 - b0) - (a1 - a0)), (b0 + t * (b1 - b0)) - (a0 + t * (a1 - a0)))))
twisted = sq

# glyph ring wrapped at radius (and unroll) 10, radii 9..11: per segment the wall integral of 2,
# caps (rho / unroll) times the flat area
P = lambda x, y: (mp.mpf(x), mp.mpf(y))
quads = [(P(3, 0), P(6, 0), P(6, 3.5)), (P(6, 3.5), P(6, 7), P(3, 7)), (P(3, 7), P(0, 7), P(0, 3.5)), (P(0, 3.5), P(0, 0), P(3, 0))]
sqc = [P(2, 2), P(2, 5), P(4, 5), P(4, 2)]
lines = [(sqc[k], sqc[(k + 1) % 4]) for k in range(4)]
UN, R0, R1 = 10, 9, 11


def wall_of(d):  # d(t) -> (s'(t), h'(t))
    return quad2(lambda t, r: mp.sqrt((R0 + r * (R1 - R0)) ** 2 * (d(t)[0] / UN) ** 2 + d(t)[1] ** 2) * (R1 - R0))


walls = 0
for p0, c1, p2 in quads:
    walls += wall_of(lambda t, p0=p0, c1=c1, p2=p2: (2 * (1 - t) * (c1[0] - p0[0]) + 2 * t * (p2[0] - c1[0]), 2 * (1 - t) * (c1[1] - p0[1]) + 2 * t * (p2[1] - c1[1])))
for p0, p1 in lines:
    walls += wall_of(lambda t, p0=p0, p1=p1: (p1[0] - p0[0], p1[1] - p0[1]))
flat = 0
for p0, c1, p2 in quads:
    flat += (p0[0] * p2[1] - p2[0] * p0[1]) / 2 + mp.mpf(2) / 3 * ((c1[0] - p0[0]) * (p2[1] - p0[1]) - (c1[1] - p0[1]) * (p2[0] - p0[0])) / 2
flat -= 6
ring = walls + flat * (R0 + R1) / UN

for name, v in [("plug", plug), ("hyperboloid", hyperboloid), ("twisted-square", twisted), ("glyph-ring", ring)]:
    print(name, mp.nstr(v, 25))
