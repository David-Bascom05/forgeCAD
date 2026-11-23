from conv import *

def V(slug, vendor_title, **kw):
    name = kw.pop("name", slug)
    return vmc(slug, name, head(slug, vendor_title), **kw)

def T(slug, vendor_title, letter, rng, **kw):
    name = kw.pop("name", slug)
    return trunnion(slug, name, head(slug, vendor_title), letter, rng, **kw)

def H(slug, vendor_title, **kw):
    name = kw.pop("name", slug)
    return hmc(slug, name, head(slug, vendor_title), **kw)

def L(slug, vendor_title, **kw):
    name = kw.pop("name", slug)
    return lathe(slug, name, head(slug, vendor_title), **kw)

Y_NOTE = "modelled as its 2-axis lathe: the Y axis, C axis and live tools are not modelled (FORMAT-GAPS B12)"

DN = [
    V("dn-solutions-dnm-4500", "DN Solutions DNM 4500"),
    V("dn-solutions-dnm-5700", "DN Solutions DNM 5700"),
    V("dn-solutions-dnm-6700", "DN Solutions DNM 6700"),
    T("dn-solutions-dvf-5000", "DN Solutions DVF 5000", "B", [-30, 110]),
    H("dn-solutions-nhp-4000", "DN Solutions NHP 4000"),
    H("dn-solutions-nhp-5000", "DN Solutions NHP 5000"),
    L("dn-solutions-lynx-2100a", "DN Solutions Lynx 2100A"),
    L("dn-solutions-lynx-2100la", "DN Solutions Lynx 2100LA"),
    L("dn-solutions-lynx-2100lma", "DN Solutions Lynx 2100LMA", extra=(Y_NOTE,)),
    L("dn-solutions-puma-2600", "DN Solutions Puma 2600"),
    L("dn-solutions-puma-2600y-ii", "DN Solutions Puma 2600Y II", extra=(Y_NOTE,)),
]
WIA = [
    V("hyundai-wia-kf5600", "Hyundai WIA KF5600"),
    V("hyundai-wia-f500plus", "Hyundai WIA F500PLUS"),
    V("hyundai-wia-kf5", "Hyundai WIA KF5"),
    H("hyundai-wia-hs4000-ii", "Hyundai WIA HS4000 II"),
    H("hyundai-wia-hs5000-ii", "Hyundai WIA HS5000 II"),
    L("hyundai-wia-se2200", "Hyundai WIA SE2200"),
    L("hyundai-wia-se2600", "Hyundai WIA SE2600"),
]
MAKINO = [
    H("makino-a51nx", "Makino a51nx"),
    H("makino-a61nx", "Makino a61nx"),
    T("makino-d500", "Makino D500", "A", [-120, 30]),
]
MATSUURA = [
    T("matsuura-mx-330", "Matsuura MX-330", "A", [-125, 10], pivot=100.0),
    T("matsuura-mx-520", "Matsuura MX-520", "A", [-125, 10]),
]
KITAMURA = [
    V("kitamura-mycenter-3xv", "Kitamura Mycenter-3XV"),
    H("kitamura-mycenter-hx400ig", "Kitamura Mycenter-HX400iG"),
]
OKUMA = [
    V("okuma-genos-m560-v", "Okuma GENOS M560-V"),
    V("okuma-mb-46vii", "Okuma MB-46VII"),
]
DMG = [
    T("dmg-mori-dmu-50-3rd-generation", "DMG MORI DMU 50 3rd Generation", "B", [-35, 110]),
    H("dmg-mori-nhx-4000", "DMG MORI NHX 4000"),
    H("dmg-mori-nhx-5000", "DMG MORI NHX 5000"),
    L("dmg-mori-nlx-2500-700", "DMG MORI NLX 2500 | 700", extra=(Y_NOTE,)),
    L("dmg-mori-clx-350", "DMG MORI CLX 350", extra=(Y_NOTE,)),
]
HERMLE = [
    T("hermle-c-22-u", "Hermle C 22 U", "A", [-135, 135]),
    T("hermle-c-32-u", "Hermle C 32 U", "A", [-130, 130]),
    T("hermle-c-42-u", "Hermle C 42 U", "A", [-130, 130]),
]
ROMI = [
    V("romi-d-800", "Romi D 800"),
    T("romi-dcm-620-5x", "Romi DCM 620-5X", "B", [-50, 110]),
    L("romi-gl-250", "Romi GL 250"),
]
BRIDGEPORT = [
    V("bridgeport-v480-apc", "Bridgeport V480 APC"),
    V("bridgeport-v710", "Bridgeport V710"),
]
SPINNER = [
    T("spinner-u5-630", "Spinner U5-630", "B", [-120, 180]),
    L("spinner-tc600", "Spinner TC600", extra=(Y_NOTE,)),
]
