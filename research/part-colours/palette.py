"""The pastel part palette (forge-render src/palette.rs): 12 soft hues at one lightness, picked in
OKLCH and checked with CIEDE2000 against each other and against the app's UI colours."""
import math

def srgb_to_lin(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

def lin_to_srgb(c):
    c = min(max(c, 0.0), 1.0)
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055

def oklch_to_srgb(L, C, h):
    a, b = C * math.cos(math.radians(h)), C * math.sin(math.radians(h))
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l_ ** 3, m_ ** 3, s_ ** 3
    r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
    g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
    bb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    inside = all(-1e-4 <= x <= 1 + 1e-4 for x in (r, g, bb))
    return tuple(lin_to_srgb(x) for x in (r, g, bb)), inside

def lab(rgb):
    r, g, b = (srgb_to_lin(x) for x in rgb)
    X = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    Y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    Z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883
    f = lambda t: t ** (1 / 3) if t > 216 / 24389 else (24389 / 27 * t + 16) / 116
    fx, fy, fz = f(X), f(Y), f(Z)
    return 116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)

def de2000(c1, c2):
    L1, a1, b1 = c1; L2, a2, b2 = c2
    C1, C2 = math.hypot(a1, b1), math.hypot(a2, b2)
    Cm = (C1 + C2) / 2
    G = 0.5 * (1 - math.sqrt(Cm ** 7 / (Cm ** 7 + 25 ** 7)))
    a1p, a2p = (1 + G) * a1, (1 + G) * a2
    C1p, C2p = math.hypot(a1p, b1), math.hypot(a2p, b2)
    h1p = math.degrees(math.atan2(b1, a1p)) % 360
    h2p = math.degrees(math.atan2(b2, a2p)) % 360
    dL, dC = L2 - L1, C2p - C1p
    dh = 0 if C1p * C2p == 0 else (h2p - h1p if abs(h2p - h1p) <= 180 else h2p - h1p - 360 if h2p > h1p else h2p - h1p + 360)
    dH = 2 * math.sqrt(C1p * C2p) * math.sin(math.radians(dh / 2))
    Lm, Cmp = (L1 + L2) / 2, (C1p + C2p) / 2
    hm = h1p + h2p if C1p * C2p == 0 else ((h1p + h2p) / 2 if abs(h1p - h2p) <= 180 else (h1p + h2p + 360) / 2 if h1p + h2p < 360 else (h1p + h2p - 360) / 2)
    T = 1 - 0.17 * math.cos(math.radians(hm - 30)) + 0.24 * math.cos(math.radians(2 * hm)) + 0.32 * math.cos(math.radians(3 * hm + 6)) - 0.20 * math.cos(math.radians(4 * hm - 63))
    dth = 30 * math.exp(-(((hm - 275) / 25) ** 2))
    Rc = 2 * math.sqrt(Cmp ** 7 / (Cmp ** 7 + 25 ** 7))
    Sl = 1 + 0.015 * (Lm - 50) ** 2 / math.sqrt(20 + (Lm - 50) ** 2)
    Sc, Sh = 1 + 0.045 * Cmp, 1 + 0.015 * Cmp * T
    Rt = -math.sin(math.radians(2 * dth)) * Rc
    return math.sqrt((dL / Sl) ** 2 + (dC / Sc) ** 2 + (dH / Sh) ** 2 + Rt * (dC / Sc) * (dH / Sh))

hexrgb = lambda h: tuple(((h >> s) & 255) / 255 for s in (16, 8, 0))
UI = {  # where each is, in forge-render
    "sketch dark": 0xFF2E9A, "sketch light": 0xC4006E, "projected dark": 0xD9C24A, "projected light": 0x9A7A00,
    "focus/centre/origin point": 0xE0A21B, "error/x axis": 0xE0554A, "error light": 0xB83A30,
    "accent": 0x2F7D59, "accent light": 0x1D5C42, "y axis/xz plane": 0x3FAE6E, "z axis/xy plane": 0x5A9BE0,
    "preselect (HOVER)": 0x3380F2, "chosen (CHOSEN)": 0xBE5C03, "picked tint": 0xF28C26, "clash tint": 0xE63333,
    "warning": 0xE0A030, "joint glyph": 0xF08C28, "selection dark": 0x2C4A6B, "selection light": 0xB9D2EC,
    "on_ink": 0x9CC3EA, "part grey": 0xB4B4B4,
}
UI_LAB = {k: lab(hexrgb(v)) for k, v in UI.items()}

L, N = 0.80, 12
# hues the preselect blue and the light selection blues own are left out
GAP = (200.0, 250.0)
best = None
for C10 in range(60, 96, 5):
    C = C10 / 1000
    for start in range(0, 50):
        span = 360 - (GAP[1] - GAP[0])
        hues = [(GAP[1] + start / 50 * span / N + k * span / N) % 360 for k in range(N)]
        cols = [oklch_to_srgb(L, C, h) for h in hues]
        if not all(ok for _, ok in cols):
            continue
        labs = [lab(c) for c, _ in cols]
        inner = min(de2000(labs[i], labs[j]) for i in range(N) for j in range(i + 1, N))
        ui = min(de2000(l, u) for l in labs for u in UI_LAB.values())
        score = min(inner, ui * 0.8)
        if best is None or score > best[0]:
            best = (score, C, hues, cols, labs, inner, ui)

score, C, hues, cols, labs, inner, ui = best
# palette order steps 5 hues on (5 and 12 coprime): neighbours in the list are far apart
order = [(k * 5) % N for k in range(N)]
cols = [cols[k] for k in order]; labs = [labs[k] for k in order]; hues = [hues[k] for k in order]
print(f"OKLCH L={L} C={C}, hues outside {GAP}; min dE00 between any two {inner:.1f}, to any UI colour {ui:.1f}")
for k, ((r, g, b), _) in enumerate(cols):
    near = min(UI_LAB, key=lambda n: de2000(labs[k], UI_LAB[n]))
    nb = de2000(labs[k], labs[(k + 1) % N])
    print(f"    [{r:.3f}, {g:.3f}, {b:.3f}], // #{int(r*255+.5):02X}{int(g*255+.5):02X}{int(b*255+.5):02X} h {hues[k]:.0f}: next {nb:.1f}, nearest UI {near} {de2000(labs[k], UI_LAB[near]):.1f}")
