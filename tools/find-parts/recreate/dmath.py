"""sin, cos and atan2 exactly as forge_math::dmath computes them (fdlibm, IEEE + - * / only), so a translation
is the same on every machine and the same as the Rust importer's. The platform's libm rounds its own way: glibc's
sin is off the nearest double about one time in a thousand, and fdlibm's more often still."""
import math
import struct

PI = math.pi
FRAC_PI_2 = math.pi / 2
TOINT = 1.5 / 2.220446049250313e-16
INVPIO2 = 6.36619772367581382433e-1
PIO2_1 = 1.57079632673412561417
PIO2_1T = 6.07710050650619224932e-11
PIO2_2 = 6.07710050630396597660e-11
PIO2_2T = 2.02226624879595063154e-21
PIO2_3 = 2.02226624871116645580e-21
PIO2_3T = 8.47842766036889956997e-32
S1, S2, S3, S4, S5, S6 = -1.66666666666666324348e-1, 8.33333333332248946124e-3, -1.98412698298579493134e-4, 2.75573137070700676789e-6, -2.50507602534068634195e-8, 1.58969099521155010221e-10
C1, C2, C3, C4, C5, C6 = 4.16666666666666019037e-2, -1.38888888888741095749e-3, 2.48015872894767294178e-5, -2.75573143513906633035e-7, 2.08757232129817482790e-9, -1.13596475577881948265e-11
ATANHI = (4.63647609000806093515e-1, 7.85398163397448278999e-1, 9.82793723247329054082e-1, 1.57079632679489655800)
ATANLO = (2.26987774529616870924e-17, 3.06161699786838301793e-17, 1.39033110312309984516e-17, 6.12323399573676603587e-17)
AT = (3.33333333333329318027e-1, -1.99999999998764832476e-1, 1.42857142725034663711e-1, -1.11111104054623557880e-1, 9.09088713343650656196e-2, -7.69187620504482999495e-2, 6.66107313738753120669e-2, -5.83357013379057348645e-2, 4.97687799461593236017e-2, -3.65315727442169155270e-2, 1.62858201153657823623e-2)
PI_LO = 1.2246467991473531772e-16


def exponent(x):
    return (struct.unpack("<Q", struct.pack("<d", x))[0] >> 52) & 0x7FF


def rem_pio2(x):
    f = x * INVPIO2 + TOINT - TOINT
    n = int(f)
    r = x - f * PIO2_1
    w = f * PIO2_1T
    y0 = r - w
    ex = exponent(x)
    if ex - exponent(y0) > 16:
        t = r
        w = f * PIO2_2
        r = t - w
        w = f * PIO2_2T - ((t - r) - w)
        y0 = r - w
        if ex - exponent(y0) > 49:
            t = r
            w = f * PIO2_3
            r = t - w
            w = f * PIO2_3T - ((t - r) - w)
            y0 = r - w
    return n, y0, (r - y0) - w


def k_sin(x, y):
    z = x * x
    w = z * z
    r = S2 + z * (S3 + z * S4) + z * w * (S5 + z * S6)
    v = z * x
    return x - ((z * (0.5 * y - v * r) - y) - v * S1)


def k_cos(x, y):
    z = x * x
    w = z * z
    r = z * (C1 + z * (C2 + z * C3)) + w * w * (C4 + z * (C5 + z * C6))
    hz = 0.5 * z
    w = 1.0 - hz
    return w + (((1.0 - w) - hz) + (z * r - x * y))


def sin_cos(x):
    if not math.isfinite(x):
        return math.nan, math.nan
    if abs(x) < 1.0 / 134217728.0:
        return x, 1.0
    n, y0, y1 = rem_pio2(x)
    s, c = k_sin(y0, y1), k_cos(y0, y1)
    return ((s, c), (c, -s), (-s, -c), (-c, s))[n & 3]


def sin(x):
    return sin_cos(x)[0]


def cos(x):
    return sin_cos(x)[1]


def atan(x):
    if math.isnan(x):
        return x
    neg = math.copysign(1.0, x) < 0
    ax = abs(x)
    if ax >= 7.3786976294838206e19:
        z = ATANHI[3] + ATANLO[3]
        return -z if neg else z
    if ax < 0.4375:
        if ax < 7.450580596923828e-9:
            return x
        i = -1
    elif ax < 1.1875:
        if ax < 0.6875:
            i = 0
            ax = (2.0 * ax - 1.0) / (2.0 + ax)
        else:
            i = 1
            ax = (ax - 1.0) / (ax + 1.0)
    elif ax < 2.4375:
        i = 2
        ax = (ax - 1.5) / (1.0 + 1.5 * ax)
    else:
        i = 3
        ax = -1.0 / ax
    z = ax * ax
    w = z * z
    s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))))
    s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))))
    if i < 0:
        r = ax - ax * (s1 + s2)
    else:
        r = ATANHI[i] - ((ax * (s1 + s2) - ATANLO[i]) - ax)
    return -r if neg else r


def atan2(y, x):
    if math.isnan(x) or math.isnan(y):
        return math.nan
    if y == 0.0:
        if math.copysign(1.0, x) < 0:
            return -PI if math.copysign(1.0, y) < 0 else PI
        return y
    if x == 0.0:
        return -FRAC_PI_2 if y < 0.0 else FRAC_PI_2
    if math.isinf(x):
        if math.isinf(y):
            q = PI / 4.0 if x > 0.0 else 3.0 * PI / 4.0
        else:
            q = 0.0 if x > 0.0 else PI
        return -q if y < 0.0 else q
    if math.isinf(y):
        return -FRAC_PI_2 if y < 0.0 else FRAC_PI_2
    k = exponent(y) - exponent(x)
    if k > 60:
        z = FRAC_PI_2 + 0.5 * PI_LO
    elif x < 0.0 and k < -60:
        z = 0.0
    else:
        z = atan(abs(y / x))
    if x < 0.0:
        return PI - (z - PI_LO) if y >= 0.0 else (z - PI_LO) - PI
    return -z if y < 0.0 else z
