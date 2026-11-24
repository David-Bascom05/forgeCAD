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

# round 2 (openly published models only)
MAZAK2 = [
    V("mazak-vcn-460", "Mazak VCN-460"),
    V("mazak-vcn-500", "Mazak VCN-500"),
    V("mazak-vcn-600", "Mazak VCN-600"),
    V("mazak-vcn-535c", "Mazak VCN-535C"),
    T("mazak-variaxis-c-600", "Mazak VARIAXIS C-600", "B", [-30, 120]),
    H("mazak-hcn-4000", "Mazak HCN-4000"),
    H("mazak-hcn-6000", "Mazak HCN-6000"),
    L("mazak-quick-turn-250", "Mazak QUICK TURN 250"),
]
DN += [
    V("dn-solutions-dnm-4000", "DN Solutions DNM 4000"),
    H("dn-solutions-nhp-6300", "DN Solutions NHP 6300"),
    H("dn-solutions-nhp-6300-2nd", "DN Solutions NHP 6300 (2nd generation)"),
    L("dn-solutions-lynx-2600", "DN Solutions Lynx 2600"),
    L("dn-solutions-puma-gt3100", "DN Solutions PUMA GT3100"),
    L("dn-solutions-puma-3100ly", "DN Solutions PUMA 3100LY", extra=(Y_NOTE,)),
]
OKUMA += [
    V("okuma-genos-m660-v", "Okuma GENOS M660-V"),
    V("okuma-genos-m460-ve", "Okuma GENOS M460-VE"),
    H("okuma-ma-500hii", "Okuma MA-500HII"),
    H("okuma-ma-600hiii", "Okuma MA-600HIII"),
]

TURN_NOTE = ("C is also a direct-drive turning spindle (stationary tool locked in the milling spindle); "
             "modelled as a rotary joint only, the turning is a format gap (FORMAT-GAPS B5)")
BROTHER2 = [
    V("brother-s300xd2", "Brother SPEEDIO S300Xd2", name="brother-speedio-s300xd2"),
    V("brother-s500xd2", "Brother SPEEDIO S500Xd2", name="brother-speedio-s500xd2"),
    V("brother-s700xd2", "Brother SPEEDIO S700Xd2", name="brother-speedio-s700xd2"),
    V("brother-w1000xd2", "Brother SPEEDIO W1000Xd2", name="brother-speedio-w1000xd2"),
    V("brother-f600x1", "Brother SPEEDIO F600X1", name="brother-speedio-f600x1",
      extra=("Brother publishes this one's layout: the table moves in Y, the column in X and Z; modelled with X on the table, the same tool-to-part motion (static bodies differ)",)),
    V("brother-r450xd1", "Brother SPEEDIO R450Xd1", name="brother-speedio-r450xd1",
      extra=("a 0/180 deg turn table with two 600 x 420 faces swaps the machining and loading faces in 2.7 s (120 kg a face): the swap is not in the format (FORMAT-GAPS B1); the face at the spindle is the table",)),
    V("brother-r650xd1", "Brother SPEEDIO R650Xd1", name="brother-speedio-r650xd1",
      extra=("a 0/180 deg turn table with two faces swaps them in 3.1 s: the swap is not in the format (FORMAT-GAPS B1); the face at the spindle is the table",)),
    T("brother-m200xd1", "Brother SPEEDIO M200Xd1", "A", [-30, 120], name="brother-speedio-m200xd1", pivot=50.0,
      extra=(TURN_NOTE,), table_t=60.0, cradle_t=80.0, cradle_side=40.0, nose_r=80.0),
    T("brother-m300xd1", "Brother SPEEDIO M300Xd1", "A", [-30, 120], name="brother-speedio-m300xd1", pivot=50.0,
      extra=(TURN_NOTE,), table_t=60.0, cradle_t=80.0, cradle_side=40.0, nose_r=80.0),
    T("brother-u500xd1", "Brother SPEEDIO U500Xd1", "A", [-30, 120], name="brother-speedio-u500xd1", pivot=50.0,
      table_t=60.0, cradle_t=80.0, cradle_side=40.0, nose_r=80.0),
    H("brother-h550xd1", "Brother SPEEDIO H550Xd1", name="brother-speedio-h550xd1", nose_r=80.0),
]
HERMLE += [
    T("hermle-c-250", "Hermle C 250", "A", [-115, 115]),
    T("hermle-c-400", "Hermle C 400", "A", [-139, 91]),
    T("hermle-c-650", "Hermle C 650", "A", [-115, 115]),
]
