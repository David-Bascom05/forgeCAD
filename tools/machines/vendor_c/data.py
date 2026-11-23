# MACHINE-LIB C data: every number restated from the cited page (read 2026-11-23).
NOTE = ("real model, published numbers cited per value; bodies are conservative boxes, not the maker's "
        "CAD; verify against your machine before trusting a collision verdict")
RPM_MIN_UNK = ("minimum rpm unpublished; held at {v} rpm so slower programs are flagged (conservative), "
               "the real floor may be lower")
FEED_UNK = ("max cutting feed unpublished; held at the rapid so t_min stays a lower bound; the feed clamp "
            "check is not meaningful on this machine")
CHG_T2T = ("chip-to-chip time unpublished; the published tool-to-tool {v} s held (shorter, so t_min stays a "
           "lower bound)")

# ---------------------------------------------------------------- SYIL (Siemens 828D; no siemens dialect)
for nm, url, tx, ty, tz, hmin, hmax, D, table, rpm, kw, iface, tmax, chg, kg, feed_kind, nose_r, head, tt in [
    ("syil-x5-pro", "https://syil.com/x5", 300, 260, 300, 150, 450, 330, (560, 260), 20000, "4.8 kW (rate/max as one value)",
     "BT30", (200, 63), (16, 1.0, "wheel, 16 tools, tool to tool 1 s"), 3, None, 70, (280, 420), 60),
    ("syil-x7-pro", "https://syil.com/x7", 400, 300, 380, 150, 530, 358, (700, 300), 12000, "7.5 / 11 kW (rate/max)",
     "BT30", (200, 63), (16, 1.5, "arm, 16 tools, tool to tool 1.5 s"), 3, None, 70, (300, 450), 60),
    ("syil-x9-pro", "https://syil.com/x9", 760, 500, 500, 100, 600, 530, (860, 500), 10000, "11 / 17 kW",
     "BT40", (240, 125), (24, 2.0, "arm, 24 tools, tool to tool 2.0 s"), 7, None, 95, (380, 520), 80),
]:
    MACHINES.append(vmc3(
        "syil", nm, "siemens", url, NOTE + "; control SINUMERIK 828D: forge-post has no siemens dialect (finding)",
        tx=tx, ty=ty, tz=tz, hmin=hmin, hmax=hmax, D=D, table=table, rapid=30000, feed=10000,
        rpm=[500, rpm], iface=iface, tmax=tmax, changer=(chg[0], chg[1], kg), coolants=("flood", "air"),
        nose_r=nose_r, head=head, tt=tt,
        cites={
            "joint.X.max_feed": pub(url, "cutting feed 10,000 mm/min"),
            "joint.Y.max_feed": pub(url, "cutting feed 10,000 mm/min"),
            "joint.Z.max_feed": pub(url, "cutting feed 10,000 mm/min"),
            "spindle.S1.rpm": [pub(url, f"max {rpm} rpm"), unk(RPM_MIN_UNK.format(v=500))],
            "spindle.S1.max": pub(url, f"max tool length {tmax[0]} mm, diameter {tmax[1]} mm"),
            "spindle.S1.interface": pub(url, iface),
            "changer.pockets": pub(url, chg[2]),
            "changer.change_s": est(CHG_T2T.format(v=chg[1]), source=url, value=chg[2]),
            "changer.max_tool_kg": pub(url, f"max tool weight {kg} kg"),
            "machine.control": pub(url, "Siemens SINUMERIK 828D standard"),
            "machine.power": pub(url, f"spindle motor {kw}; no rpm points published, so no power curve"),
        }))

# ---------------------------------------------------------------- GRIZZLY (spec sheets on cdn0.grizzly.com)
GZ = "https://cdn0.grizzly.com/specsheets/{}_ds.pdf"
for nm, model, tx, ty, tz, table, rapid, feed, rpm, iface, chg, ctrl, ctrl_v, cool, hp in [
    ("grizzly-g0875", "G0875", 356, 152, 229, (619, 178), 6096, None, [90, 3000], "BT30", None, "siemens",
     "Siemens Sinumerik (model not stated)", ("flood",), "1.5 HP DC"),
    ("grizzly-g0876", "G0876", 310, 197, 298, (686, 210), 10160, None, [50, 10000], "BT30", (8, 6.0), "siemens",
     "Siemens Sinumerik (model not stated)", ("flood",), "3 HP"),
    ("grizzly-g0877", "G0877", 397, 229, 397, (800, 260), 10160, None, [50, 10000], "BT30", (10, None), "siemens",
     "Siemens Sinumerik (model not stated)", ("flood",), "3 HP"),
    ("grizzly-g0951", "G0951", 279, 117.5, 270, (549, 159), 2000, 500, [100, 5000], "R8", None, "mach3",
     "Mach3 on a customer-supplied Windows PC", (), "1-1/3 HP DC"),
]:
    url = GZ.format(model)
    f = feed or rapid
    fc = pub(url, f"max feed {feed} mm/min (19-11/16 in/min)") if feed else unk(FEED_UNK)
    ys = ty / 2 + table[1] / 2
    cites = {
        "joint.X.max_feed": fc, "joint.Y.max_feed": fc, "joint.Z.max_feed": fc,
        "spindle.S1.rpm": pub(url, f"{rpm[0]}-{rpm[1]} rpm"),
        "spindle.S1.max": unk("max tool length/diameter unpublished; 150 x 63 mm ours (taper class); "
                              "longer tools are not flagged"),
        "spindle.S1.interface": pub(url, iface),
        "machine.control": pub(url, ctrl_v),
        "machine.power": pub(url, f"spindle motor {hp}; no curve published"),
    }
    if chg:
        cites["changer.pockets"] = pub(url, f"{chg[0]}-position ATC")
        cites["changer.change_s"] = (pub(url, f"tool change about {chg[1]} s") if chg[1] else
                                     unk("change time unpublished; 0.1 s held (the format needs > 0) so t_min stays a lower bound"))
        cites["changer.max_tool_kg"] = unk("max tool weight unpublished; 3 kg held (what SYIL publishes for "
                                           "its BT30 changers); heavier tools are flagged")
    MACHINES.append(vmc3(
        "grizzly", nm, ctrl, url,
        NOTE + f"; {ctrl_v}" + ("; forge-post has no siemens dialect (finding)" if ctrl == "siemens" else
                                "; forge-post has no mach3 dialect (finding)") +
        "; grizzly.com product pages refuse automated clients (403): numbers from Grizzly's own spec-sheet PDF",
        tx=tx, ty=ty, tz=tz, hmin=5, hmax=tz + 5, D=ys + 50, table=table, rapid=rapid, feed=f, rpm=rpm,
        iface=iface, tmax=(150, 63), changer=(chg[0], chg[1] or 0.1, 3) if chg else None, coolants=cool,
        nose_r=60 if iface == "R8" else 70, head=(260, 380), tt=50,
        rapid_src=(url, f"rapid {rapid} mm/min ({round(rapid / 25.4, 2)} in/min published)"),
        hmin_status="unknown", D_pub=False, cites=cites))

# ---------------------------------------------------------------- SHERLINE (LinuxCNC systems)
SH = "https://www.sherline.com/product/{}/"
for nm, page, tx, ty, tz, table in [
    ("sherline-8540", "8540", 220, 127, 159, (330, 70)),
    ("sherline-8580", "8580", 347, 279, 235, (457, 70)),
]:
    url = SH.format(page)
    spec = "https://www.sherline.com/cnc-specifications/"
    MACHINES.append(vmc3(
        "sherline", nm, "linuxcnc", url,
        NOTE + "; Sherline stopped taking orders 2026-10-19 (notice on sherline.com); manual tool change "
               "(no [[machine.changer]]: M6 is refused until the manual-change format lands, FORMAT-GAPS 8); "
               "X travel conflicts: 8.65 in (product page) vs 8.68 in (cnc-specifications page)",
        tx=tx, ty=ty, tz=tz, hmin=5, hmax=tz + 5, D=ty / 2 + table[1] / 2 + 30, table=table, rapid=559, feed=559,
        rpm=[70, 2800], iface="MT1 (3/4-16 nose)", tmax=(80, 20), changer=None, coolants=(),
        nose_r=35, head=(110, 200), tt=20,
        rapid_src=(spec, "max CNC positioning speed 22 in/min (559 mm/min)"), hmin_status="unknown", D_pub=False,
        cites={
            "joint.X.max_feed": pub(spec, "22 in/min max; cutting usually 6 in/min or slower"),
            "joint.Y.max_feed": pub(spec, "22 in/min max"),
            "joint.Z.max_feed": pub(spec, "22 in/min max"),
            "spindle.S1.rpm": pub(url, "70-2800 rpm"),
            "spindle.S1.max": unk("tool limits unpublished; 80 mm x 20 mm ours"),
            "spindle.S1.interface": pub(url, "#1 Morse taper, 3/4-16 TPI nose"),
            "machine.control": pub(url, "PC running LinuxCNC"),
            "machine.power": pub("https://www.sherline.com/product/hill-house-dc-motor-2013/",
                                 "90 VDC motor, 60 W output"),
        }))

# ---------------------------------------------------------------- DATRON (DATRON next, SimPL)
DN = "https://datron.com/cnc-machines/datron-neo/"
DF = "https://www.datron.de/fileadmin/dokumente/broschueren/cnc/english/DATRON_neo_family_EN.pdf"
DK = "https://www.datron.de/fileadmin/dokumente/broschueren/cnc/german/DATRON_Maschinenkatalog_DE.pdf"
D24 = "https://www.datron.de/fileadmin/dokumente/broschueren/cnc/german/DATRON_neo_neu_Fl_240325_DE_V1.0.pdf"
DM = "https://datron.com/cnc-machines/m8-cube/"
DM12 = "https://datron.com/wp-content/uploads/2022/11/DATRON_M8Cube.pdf"
for nm, src, tx, ty, tz, passage, rapid, rmax, kw, pockets, bed, extra in [
    ("datron-neo", DF, 520, 420, 220, 175, 28000, 40000, "2.0 kW HF", 24, (500, 400),
     dict(conflict=(D24, "2024 flyer: the then-neo 18 m/min (neo+ 28 m/min); the 2026 family sheet gives 28 m/min for "
                         "neo and neo Pro, held here"))),
    ("datron-neo-entry", DF, 520, 420, 220, 175, 12000, 27000, "1.0 kW HF", 12, (500, 400), {}),
    ("datron-m8cube", DK, 1020, 720, 245, 200, 22000, 60000, "1.8-4.0 kW HF (options)", 12, (1000, 700),
     dict(y=(DM12, "Y 720 mm with the tool changer (830 mm without)"))),
]:
    pg = DM if "m8" in nm else DN
    rc = [pub(src, f"feed and positioning feed up to {rapid / 1000:g} m/min")]
    if "conflict" in extra:
        rc.append(dict(status="conflict", source=extra["conflict"][0], note=extra["conflict"][1]))
    cites = {f"joint.{a}.{f}": rc for a in "XYZ" for f in ("rapid", "max_feed")}
    cites.update({
        "spindle.S1.rpm": [pub(src, f"up to {rmax} rpm, {kw}"), unk(RPM_MIN_UNK.format(v=3000))],
        "spindle.S1.max": unk("tool limits unpublished; 100 x 20 mm ours (the 2012 M8Cube brochure names "
                              "tools 0.1-20 mm)"),
        "spindle.S1.interface": pub(src, "direct shank" if "m8" not in nm else "direct shank or HSK-E 25"),
        "changer.pockets": pub(src, f"{pockets}-station tool magazine"),
        "changer.change_s": unk("change time unpublished; 0.1 s held (the format needs > 0) so t_min stays a lower bound"),
        "changer.max_tool_kg": unk("max tool weight unpublished; 1 kg held (small direct-shank HF tools); "
                                   "heavier tools are flagged"),
        "machine.control": pub(pg, "DATRON next; runs SimPL, with a G-code wrapper translating G/M codes "
                                   "(datron.com post-processor guide); forge-post has no datron dialect"),
    })
    if "y" in extra:
        cites["joint.Y.range"] = pub(*extra["y"])
    MACHINES.append(gantry(
        "datron", nm, "datron-next", src,
        NOTE + "; DATRON next control (SimPL + G-code wrapper): no forge-post dialect (finding)",
        tx=tx, ty=ty, tz=tz, passage=passage, rapid=rapid, feed=rapid, rpm=[3000, rmax],
        iface="direct shank" if "m8" not in nm else "HSK-E25", tmax=(100, 20), changer=(pockets, 0.1, 1),
        cites=cites, coolants=("mist", "air"), spindle_r=45, bed_xy=bed))

# ================================================================ batch 2: gantry routers
# Only machines whose maker states the moving-gantry layout. Manual tool change (no changer) unless
# an ATC is part of the model: M6 is refused until FORMAT-GAPS row 8 lands.
RPM_BAND_UNK = ("spindle speed unpublished; a narrow band ({lo}-{hi} rpm) held so programs outside it "
                "are flagged: check your spindle")
PASSAGE_UNK = ("gantry clearance unpublished; the beam's underside put at the Z travel ({v} mm) above the "
               "bed: errs low (more Hits)")


def router(vendor, nm, ctrl, src, note, tx, ty, tz, *, rapid, feed, rpm, iface, passage, cites,
           changer=None, tmax=(80, 13)):
    rc = cites.pop("rapid")
    c = {f"joint.{a}.rapid": rc for a in "XYZ"}
    fc = cites.pop("feed")
    c.update({f"joint.{a}.max_feed": fc for a in "XYZ"})
    c.update(cites)
    c.setdefault("spindle.S1.max", unk(f"tool limits unpublished; {tmax[0]} x {tmax[1]} mm ours"))
    if passage is None:
        c["body.gantry"] = unk(PASSAGE_UNK.format(v=tz))
    MACHINES.append(gantry(vendor, nm, ctrl, src, note, tx=tx, ty=ty, tz=tz,
                           passage=passage if passage is not None else tz, rapid=rapid, feed=feed, rpm=rpm,
                           iface=iface, tmax=tmax, changer=changer, cites=c, coolants=("air",),
                           spindle_r=40))
    if passage is None:  # the builder's published passage cite does not apply
        MACHINES[-1].cites = [x for x in MACHINES[-1].cites
                              if not (x["path"] == "body.gantry" and x["status"] == "published")]


# ---- Carbide 3D Shapeoko (Carbide Motion; a GRBL-style subset per guides.carbide3d.com)
SO4 = "https://shop.carbide3d.com/products/shapeoko4"
SO5 = "https://carbide3d.com/shapeoko/shapeoko5pro-specs/"
CR = "https://shop.carbide3d.com/products/carbide-compact-router"
CMG = "https://guides.carbide3d.com/faq/supported-gcodes/"
CNOTE = (NOTE + "; Carbide Motion control (supported codes: guides.carbide3d.com/faq/supported-gcodes): no "
         "carbide-motion dialect yet (the grbl dialect is the nearest); manual tool change (no changer)")
ROUTER = dict(rpm=[12000, 30000], iface="1/4 in collet (65 mm trim router)")
ROUTER_C = {"spindle.S1.rpm": pub(CR, "Carbide Compact Router 12,000-30,000 rpm"),
            "spindle.S1.interface": pub(CR, "1/4 in collet; 65 mm router mount")}
for nm, x_in, y_in in (("shapeoko-4", 17.5, 17.5), ("shapeoko-4-xl", 33, 17.5), ("shapeoko-4-xxl", 33, 33)):
    x, y, z = round(x_in * 25.4, 1), round(y_in * 25.4, 1), 101.6
    router("carbide3d", nm, "carbide-motion", SO4, CNOTE, x, y, z, rapid=2540, feed=2540, passage=None,
           cites=dict(
               rapid=unk("speeds unpublished by Carbide 3D; 2,540 mm/min (100 in/min) held low"),
               feed=unk("speeds unpublished by Carbide 3D; held at the rapid"),
               **{"joint.X.range": pub(SO4, f"cutting area {x_in} x {y_in} x 4 in"),
                  "machine.control": pub(SO4, "Carbide Motion on V3 electronics; accepts standard G-code"),
                  "machine.dialect": pub(CMG, "G0-G4, G17-19, G20/21, G28, G90/91; G40 G43 G49 G54-59 "
                                              "accepted but ignored; M0-M3 M5 M6 M30")}),
           **ROUTER)
    [MACHINES[-1].cite(k, **v) for k, v in ROUTER_C.items()]
for nm, x, y in (("shapeoko-5-pro-2x2", 623, 623), ("shapeoko-5-pro-4x2", 1237, 623),
                 ("shapeoko-5-pro-4x4", 1237, 1237)):
    router("carbide3d", nm, "carbide-motion", SO5, CNOTE, x, y, 155, rapid=5080, feed=5080, passage=101.6,
           cites=dict(
               rapid=dict(status="estimate", source="https://community.carbide3d.com/t/max-feed-speed-on-pro-5/91075",
                          value="user report (not Carbide 3D): XY 200 in/min as shipped, 275 in/min cap",
                          basis="unpublished by the maker; the user-reported 200 in/min (5,080 mm/min) held "
                                "(the lower), Z assumed no faster: secondary source"),
               feed=est("unpublished; held at the rapid"),
               **{"machine.control": pub(SO4, "Carbide Motion")}),
           **ROUTER)
    [MACHINES[-1].cite(k, **v) for k, v in ROUTER_C.items()]
    MACHINES[-1].cite("joint.Z.range", **pub(SO5, "Z travel 155 mm (140 mm with Sweepy Pro)"))

# ---- Avid CNC PRO (moving gantry, rack and pinion XY; EX controller with Centroid CNC12)
AV = {"pro4824": ("https://www.avidcnc.com/pro4824-4-x-2-cnc-router-machine-p-1334.html", 1260, 609),
      "pro4848": ("https://www.avidcnc.com/pro4848-4-x-4-cnc-router-machine-p-1335.html", 1260, 1260),
      "pro6060": ("https://www.avidcnc.com/pro6060-5-x-5-cnc-router-machine-p-1336.html", 1559, 1559)}
AVS = "https://www.avidcnc.com/4-hp-plug-and-play-spindle-vfd-system-p-856.html"
for k, (url, x, y) in AV.items():
    router("avid", f"avid-{k}", "centroid-cnc12", url,
           NOTE + "; NEMA 23 drive option (the slowest; N34 and servo options are faster); 4 HP spindle; "
                  "8 in Z; EX controller with Centroid CNC12 (Mach4 before rev 22.2): no dialect yet; manual "
                  "tool change (the Smart ATC is an option, not modelled)",
           x, y, 203, rapid=12700, feed=6350, passage=None, rpm=[6000, 24000], iface="ER20",
           cites=dict(rapid=pub(url, "rapid 500 in/min with N23 motors (800 N34, 1200 servo)"),
                      feed=pub(url, "cut speed 250 in/min with N23 motors (400 N34, 600 servo)"),
                      **{"joint.X.range": pub(url, f"work area {x} x {y} mm, Z 8 in (203 mm)"),
                         "spindle.S1.rpm": pub(AVS, "4 HP spindle 6,000-24,000 rpm"),
                         "spindle.S1.interface": pub(AVS, "ER20, 1/2 in shank max"),
                         "machine.control": pub("https://www.avidcnc.com/support/faq",
                                                "EX controller, Centroid CNC12 from 8/2024; Mach4 rev 22.2 and earlier")}))

# ---- ShopBot (dual Y drive motors stated on the Desktop pages; OpenSBP control: SB3 runs G-code)
SB = "https://shopbottools.com/products/{}/"
SBC = ("ShopBot SB3 control (OpenSBP; G-code run or converted by the FP/FC commands, some codes "
       "unsupported): no shopbot dialect yet")
for nm, page, x, y, z, rapid, feed, iface, chg in (
        ("shopbot-desktop", "desktop", 640, 510, 130, 9000, 6000, "ER20", None),
        ("shopbot-desktop-max", "desktop-max", 1042, 640, 130, 9000, 6000, "ER20", None),
        ("shopbot-desktop-max-atc", "desktop-max-atc", 1042, 640, 130, 9000, 6000, "ISO20 (ER20)", 7),
        ("shopbot-prs5-alpha-96-48", "shopbot-prs5-alpha", 2616, 1245, 200, 45000, 18000, "ER25", None),
        ("shopbot-prs5-standard-96-48", "shopbot-prs5-standard", 2616, 1245, 200, 18000, 12000, "ER20", None)):
    url = SB.format(page)
    cites = dict(rapid=pub(url, f"max rapid {rapid / 60:g} mm/s"), feed=pub(url, f"max cut speed {feed / 60:g} mm/s"),
                 **{"joint.X.range": pub(url, f"XYZ movement {x} x {y} x {z} mm"),
                    "spindle.S1.rpm": unk(RPM_BAND_UNK.format(lo=10000, hi=18000)),
                    "spindle.S1.interface": pub(url, iface),
                    "machine.control": pub(url, "ShopBot control software (OpenSBP)")})
    if chg:
        cites["changer.pockets"] = pub(url, "7-position tool bar ATC")
        cites["changer.change_s"] = unk("change time unpublished; 0.1 s held (the format needs > 0) so t_min "
                                        "stays a lower bound")
        cites["changer.max_tool_kg"] = unk("max tool weight unpublished; 0.5 kg held (ISO20 holders); heavier "
                                           "tools are flagged")
    router("shopbot", nm, "shopbot-opensbp", url,
           NOTE + "; " + SBC + ("" if chg else "; manual tool change (no changer)"),
           x, y, z, rapid=rapid, feed=feed, passage=None, rpm=[10000, 18000], iface=iface, cites=cites,
           changer=(chg, 0.1, 0.5) if chg else None)

# ================================================================ batch 3
# ---- Onefinity Elite Gen 2 (moving gantry: manual REV 02.12.24 pp.27-42 shows Y rails on the table,
# the X rail riding them, the Z slider on the X block; that manual is Gen 1, the layout is the same family)
OF = "https://www.onefinitycnc.com/compare-product"
OFM = "https://www.onefinitycnc.com/_files/ugd/1cee9f_11486493866543b388bd72ccac5c5b95.pdf"
OFP = "https://www.onefinitycnc.com/post/introducingtheallnewgen2eliteseriesbyonefinitycnc"
for nm, x_in, y_in in (("onefinity-elite-woodworker", 33, 33), ("onefinity-elite-journeyman", 49, 33),
                       ("onefinity-elite-foreman", 49, 49.175)):
    x, y = round(x_in * 25.4, 1), round(y_in * 25.4, 1)
    rap = {"X": 15240, "Y": 15240, "Z": 5715}
    rc = pub(OF, "rapid XY up to 600 in/min (15,240 mm/min), Z 225 in/min (5,715 mm/min)")
    fc = unk("cutting feed unpublished; held at the rapid")
    cites = {f"joint.{a}.rapid": [rc] for a in "XYZ"}
    for a in "XY":
        cites[f"joint.{a}.rapid"].append(dict(status="conflict", source=OFP,
                                              note="the Gen 2 launch post says pre-set rapids of 300 in/min; the "
                                                   "compare page's 600 in/min is held"))
    cites.update({f"joint.{a}.max_feed": fc for a in "XYZ"})
    cites.update({
        "joint.X.range": pub(OF, f"cutting area {x_in} x {y_in} in"),
        "machine.layout": pub(OFM, "Y rails fixed to the table, the X rail (gantry) rides them, the Z slider on "
                                   "the X block (Gen 1 assembly steps, pp.27-42)"),
        "spindle.S1.rpm": unk(RPM_BAND_UNK.format(lo=10000, hi=24000) + " (no spindle included; 65/80 mm mount)"),
        "spindle.S1.interface": pub("https://www.onefinitycnc.com/product-page/gen2eliteseries",
                                    "spindle not included; 65 or 80 mm mount"),
        "spindle.S1.max": unk("tool limits unpublished; 80 x 13 mm ours"),
        "machine.control": pub("https://www.onefinitycnc.com/product-page/gen2eliteseries",
                               "Redline CNC controller with RealTime CNC RTS-2"),
    })
    MACHINES.append(gantry(
        "onefinity", nm, "onefinity-redline", OF,
        NOTE + "; Gen 2 Elite; Redline controller (RTS-2): no dialect yet; manual tool change (no changer)",
        tx=x, ty=y, tz=163, passage=177.8, rapid=rap, feed=rap, rpm=[10000, 24000], iface="65/80 mm mount",
        tmax=(80, 13), changer=None, cites=cites, coolants=("air",), spindle_r=40))
    MACHINES[-1].cite("joint.Z.range", **pub(OF, "Z travel 6.4 in (163 mm), cutting area Z 7 in"))
    MACHINES[-1].cites = [c for c in MACHINES[-1].cites if not (c["path"] == "body.gantry")]
    MACHINES[-1].cite("body.gantry", **pub(OFP, "7 in of gantry clearance"),
                      basis="beam underside at the published 7 in (177.8 mm); beam and uprights ours")

# ---- fixed bridge, moving Y table
N3G = "https://my.carbide3d.com/pdf/Nomad3_Getting_Started_Guide_12_22_2020_v1_smaller.pdf"
N3 = "https://shop.carbide3d.com/products/nomad-3"
MACHINES.append(bridge(
    "carbide3d", "nomad-3", "carbide-motion", N3,
    NOTE + "; Carbide Motion: no carbide-motion dialect yet; manual tool change (no changer); enclosed",
    tx=203.2, ty=203.2, tz=76.2, nose=81.2, table=(203.2, 203.2), rapid=2540, feed=2540, rpm=[9000, 24000],
    iface="ER11", tmax=(60, 7), changer=None,
    nose_cite=dict(status="estimate", basis="true value unpublished; nose put 5 mm above the table at Z's "
                   "bottom: the least value consistent with the self-collision test, not data"),
    cites={**{f"joint.{a}.{f}": unk("speeds unpublished by Carbide 3D; 2,540 mm/min (100 in/min) held low")
              for a in "XYZ" for f in ("rapid", "max_feed")},
           "machine.layout": pub(N3G, "the Y axis is the table; X and Z move the spindle (p.31)"),
           "spindle.S1.rpm": pub(N3, "130 W brushless, 9,000-24,000 rpm"),
           "spindle.S1.interface": pub(N3, "ER-11, cutters up to 7 mm"),
           "spindle.S1.max": pub(N3, "cutters up to 7 mm (length ours: 60 mm)"),
           "machine.control": pub(N3, "Carbide Motion")}))
MACHINES[-1].cite("body.table", "estimate", basis="table size unpublished; the 8 x 8 in cutting area used")

R50 = "https://www.rolanddg.com/products/milling-machines/mdx-50-benchtop-cnc-mill/specifications"
R50M = "https://downloadcenter.rolanddg.com/contents/manuals/MDX-50_USE_EN_R6.pdf"
MACHINES.append(bridge(
    "roland", "roland-mdx-50", "roland-nc", R50,
    NOTE + "; RML-1 or NC code over USB: no Roland dialect yet; resins only (metal not supported per Roland)",
    tx=400, ty=305, tz=135, nose=140, table=(400, 305), rapid={"X": 3600, "Y": 3600, "Z": 3000},
    feed={"X": 3600, "Y": 3600, "Z": 3000}, rpm=[4500, 15000], iface="collet (6.35 / 3.175 mm shanks)",
    tmax=(60, 6.35), changer=(5, 0.1, 0.2), spindle_r=40,
    nose_cite=dict(status="estimate", source=R50M, value="figure p.18: 130 mm spindle-head bottom to table; Z position not stated",
                   basis="with 135 mm Z travel the 130 mm figure cannot be at Z's top; nose put 5 mm above the "
                         "table at Z's bottom, the least value consistent with the self-collision test, not data"),
    cites={**{f"joint.{a}.{f}": pub(R50, "feed XY 7-3,600 mm/min, Z 7-3,000 mm/min")
              for a in "XYZ" for f in ("max_feed",)},
           **{f"joint.{a}.rapid": est("rapid unpublished; held at the published max feed (a lower bound "
                                      "for speed, so t_min may be long)", source=R50, value="max feed only")
              for a in "XYZ"},
           "machine.layout": pub(R50M, "the table moves in Y, the spindle head in X (pp.33-34); Z on the head "
                                       "(p.18 figure)"),
           "spindle.S1.rpm": pub(R50, "4,500-15,000 rpm"),
           "spindle.S1.interface": pub(R50, "collet chuck, 6.35 or 3.175 mm shanks"),
           "spindle.S1.max": unk("tool length unpublished; 60 mm ours, shank 6.35 mm published"),
           "changer.pockets": pub(R50, "ATC: 5 tools plus a detection pin"),
           "changer.change_s": unk("change time unpublished; 0.1 s held (the format needs > 0) so t_min stays a "
                                   "lower bound"),
           "changer.max_tool_kg": unk("max tool weight unpublished; 0.2 kg held (small collet tools)"),
           "machine.control": pub(R50, "RML-1 and NC code, USB")},
    unmodelled="enclosure, door, ATC magazine and frame"))

SRM = "https://www.rolanddg.com/products/milling-machines/srm-20-compact-milling-machine/specifications"
SRMM = "https://downloadcenter.rolanddg.com/contents/manuals/SRM-20_USE_EN_R4.pdf"
MACHINES.append(bridge(
    "roland", "roland-srm-20", "roland-nc", SRM,
    NOTE + "; RML-1 or NC code: no Roland dialect yet; manual tool change (no changer)",
    tx=203.2, ty=152.4, tz=60.5, nose=130.75, table=(232.2, 156.6), rapid=1800, feed=1800, rpm=[3000, 7000],
    iface="collet (6 mm included)", tmax=(50, 6), changer=None, spindle_r=30,
    nose_cite=dict(status="published", source=SRM, value="collet tip to table 130.75 mm max",
                   basis="the collet tip taken as the gauge point, at Z's top"),
    cites={**{f"joint.{a}.max_feed": pub(SRM, "operating speed 6-1,800 mm/min") for a in "XYZ"},
           **{f"joint.{a}.rapid": est("rapid unpublished; held at the published max feed", source=SRM,
                                      value="max 1,800 mm/min") for a in "XYZ"},
           "machine.layout": pub(SRMM, "the table moves in Y (p.15); the spindle head in X and Z (p.14)"),
           "spindle.S1.rpm": pub(SRM, "3,000-7,000 rpm (DC motor type 380)"),
           "spindle.S1.interface": pub(SRM, "collet; 6 mm collet included"),
           "spindle.S1.max": unk("tool limits unpublished; 50 x 6 mm ours"),
           "machine.control": pub(SRM, "RML-1 and NC code")}))
MACHINES[-1].cite("body.table", "published", SRM, value="table 232.2 x 156.6 mm")

# ================================================================ batch 4: lathes
G884 = GZ.format("G0884")
MACHINES.append(lathe(
    "grizzly", "grizzly-g0884", "siemens", G884,
    NOTE + "; Siemens Sinumerik (model not stated): no siemens lathe dialect yet; grizzly.com product pages "
           "refuse automated clients (403): numbers from Grizzly's own spec-sheet PDF",
    xt=158.75, zt=596.9, swing=330.2, centres=685.8, rpm=[30, 4000], iface="A2-4",
    rapid={"X": 5969, "Z": 8001}, feed={"X": 5969, "Z": 8001}, chuck_r=90, chuck_l=80, stations=6, index_s=0.1,
    cites={"joint.X.range": pub(G884, "cross slide travel 6-1/4 in (158.75 mm)"),
           "joint.Z.range": pub(G884, "carriage travel 23-1/2 in (596.9 mm)"),
           "joint.X.rapid": pub(G884, "rapid X 235 in/min"), "joint.Z.rapid": pub(G884, "rapid Z 315 in/min"),
           "joint.X.max_feed": unk(FEED_UNK), "joint.Z.max_feed": unk(FEED_UNK),
           "spindle.main.rpm": pub(G884, "30-4,000 rpm, 4 HP"),
           "spindle.main.max": pub(G884, "swing 13 in over bed, 6 in over cross slide; 27 in between centres"),
           "spindle.main.interface": pub(G884, "A2-4 nose, 1.5 in bore"),
           "turret.T1.stations": pub(G884, "6-position turret"),
           "turret.T1.index_s": unk("index time unpublished; 0.1 s held so t_min stays a lower bound"),
           "machine.control": pub(G884, "Siemens Sinumerik"),
           "machine.tailstock": pub(G884, "MT3 tailstock (not modelled)")}))

SL = "https://www.sherline.com/product/8440/"
SL4 = "https://www.sherline.com/product/4400-cnc4410-cnc-lathe/"
MACHINES.append(lathe(
    "sherline", "sherline-8440", "linuxcnc", SL,
    NOTE + "; Sherline stopped taking orders 2026-10-19; a single tool post (no turret), modelled as a "
           "1-station turret; LinuxCNC; Sherline says the standard CNC lathe cannot cut threads",
    xt=110, zt=300, swing=90, centres=430, rpm=[70, 2800], iface="3/4-16 nose, #1 Morse",
    rapid=559, feed=559, chuck_r=50, chuck_l=50, stations=1, index_s=0.1, coolants=(), turret_r=40, hs=80,
    cites={"joint.X.range": pub(SL4, "crosslide travel 4.25 in (110 mm)"),
           "joint.Z.range": unk("carriage travel unpublished (17 in / 430 mm between centres); 300 mm held "
                                "(smaller: overtravel errs early)"),
           **{f"joint.{a}.rapid": pub("https://www.sherline.com/cnc-specifications/", "22 in/min max positioning")
              for a in "XZ"},
           **{f"joint.{a}.max_feed": pub("https://www.sherline.com/cnc-specifications/", "22 in/min max")
              for a in "XZ"},
           "spindle.main.rpm": pub(SL4, "70-2,800 rpm"),
           "spindle.main.max": pub(SL4, "swing 3.5 in over bed; 17 in between centres"),
           "spindle.main.interface": pub(SL4, "3/4-16 TPI nose, #1 Morse taper"),
           "turret.T1.stations": est("a single tool post, no turret: one station"),
           "turret.T1.index_s": unk("no indexing; 0.1 s held (the format needs > 0)"),
           "machine.control": pub(SL, "PC with LinuxCNC")}))

# ================================================================ slice 2: job-shop builders
CHG_UNK = "change time unpublished; 0.1 s held (the format needs > 0) so t_min stays a lower bound"
NOSE_UNK = dict()  # vmc3 with hmin_status="unknown" writes its own basis

# ---- Takumi (takumiusa.com; Fanuc 0i-MF): web pages (VCX versions) held over the 2024 catalog
TK = "https://takumiusa.com/cnc-machines/{}/"
TKC = "https://takumiusa.com/wp-content/uploads/2021/08/Takumi-Technical-Catalog.pdf"
for nm, path, tx, ty, tz, hmin, hmax, table, rpm, iface, rap, feed, pockets, tool, kg, extra in [
    ("takumi-vc0852", "linear-guide-machining-center/vc0852", 861, 521, 610, 114, 724, (1001, 521), 15000, "CAT40",
     {"X": 36000, "Y": 36000, "Z": 24000}, 12000, 40, (299.7, 74.9), 7,
     dict(conflict=("catalog p.12: 12,000 rpm, 15 kW, 24 tools, Fanuc 0i Plus; the web page's 15,000 rpm / 40 "
                    "tools (newer) held"))),
    ("takumi-vc1052", "linear-guide-machining-center/vc1052", 1059, 521, 610, 114, 724, (1161, 521), 15000, "CAT40",
     {"X": 36000, "Y": 36000, "Z": 24000}, 12000, 40, (299.7, 74.9), 7, {}),
    ("takumi-vc1200", "linear-guide-machining-center/vc1200", 1270, 660, 610, 150, 760, (1501, 660), 15000, "CAT40",
     {"X": 36000, "Y": 36000, "Z": 24000}, 12000, 40, (299.7, 74.9), 7,
     dict(conflict=("catalog p.12: 30 tools; the web page's 40 (newer) held"))),
    ("takumi-vc1500", "linear-guide-machining-center/vc1500", 1524, 762, 610, 120, 729, (1679, 762), 15000, "CAT40",
     {"X": 36000, "Y": 36000, "Z": 24000}, 12000, 40, (299.7, 74.9), 7, {}),
    ("takumi-v10", "box-ways-machining-center/v10", 1003, 660, 610, 150, 762, (1059, 650), 12000, "CAT40",
     {"X": 24000, "Y": 24000, "Z": 20000}, 8000, 32, (300, 79), 7, {}),
    ("takumi-v11", "box-ways-machining-center/v11", 1100, 660, 610, 150, 762, (1059, 650), 6000, "CAT50",
     {"X": 24000, "Y": 24000, "Z": 20000}, 8000, 32, (300, 79), 7,
     dict(table=("catalog p.16: table 1150 x 650 mm; the web page's 1059 x 650 held"))),
    ("takumi-v15", "box-ways-machining-center/v15", 1524, 762, 719, 150, 871, (1600, 762), 6000, "CAT50",
     {"X": 18000, "Y": 18000, "Z": 15800}, 5000, 24, (300, 109), 15, {}),
]:
    url = TK.format(path)
    cites = {f"joint.{a}.max_feed": pub(url, f"cutting feed {feed} mm/min") for a in "XYZ"}
    cites.update({
        "spindle.S1.rpm": [pub(url, f"max {rpm} rpm"), unk(RPM_MIN_UNK.format(v=100))],
        "spindle.S1.max": pub(url, f"max tool {tool[1]} mm diameter (adjacent pockets full), {tool[0]} mm long"),
        "spindle.S1.interface": pub(url, iface),
        "changer.pockets": pub(url, f"{pockets} tools"),
        "changer.change_s": unk(CHG_UNK),
        "changer.max_tool_kg": pub(url, f"max tool weight {kg} kg"),
        "machine.control": pub(url, "Fanuc 0i-MF"),
    })
    if "conflict" in extra:
        cites["machine.catalog"] = dict(status="conflict", source=TKC, note=extra["conflict"])
    if "table" in extra:
        cites["machine.table"] = dict(status="conflict", source=TKC, note=extra["table"])
    big = iface == "CAT50"
    MACHINES.append(vmc3(
        "takumi", nm, "fanuc", url, NOTE + "; Fanuc 0i-MF",
        tx=tx, ty=ty, tz=tz, hmin=hmin, hmax=hmax, D=ty / 2 + table[1] / 2 + 50, table=table, rapid=rap, feed=feed,
        rpm=[100, rpm], iface=iface, tmax=tool, changer=(pockets, 0.1, kg), coolants=("flood", "air"),
        nose_r=100 if big else 75, head=(440, 560) if big else (340, 480), tt=80 if big else 60,
        rapid_src=False, D_pub=False, cites=cites))
    for a in "XYZ":
        MACHINES[-1].cite(f"joint.{a}.rapid", **pub(url, f"rapid {per(rap, a)} mm/min"))

# ---- Hwacheon (Korean site is script-built; numbers from Hwacheon Machinery Europe's pages)
HW = "https://www.hwacheon-europe.com/en/{}"
for nm, path, tx, ty, tz, table, rpm, pockets, rap, feed, kw in [
    ("hwacheon-vesta-660", "universal-milling-machines/vesta-660", 660, 430, 400, (720, 420), 10000, 20,
     {"X": 36000, "Y": 36000, "Z": 30000}, 24000, "15 / 11 kW, 95.5 / 52.5 Nm (which is continuous not stated)"),
    ("hwacheon-vesta-1000", "universal-milling-machines/vesta-1000", 1000, 550, 500, (1100, 502), 12000, 30,
     {"X": 36000, "Y": 36000, "Z": 30000}, 24000, "18.5 kW, 118 Nm"),
    ("hwacheon-vesta-1100", "universal-milling-machines/vesta-1100", 1100, 600, 600, (1300, 600), 12000, 30,
     {"X": 36000, "Y": 36000, "Z": 36000}, None, "18.5 kW"),
]:
    url = HW.format(path)
    f = feed or {"X": rap["X"], "Y": rap["Y"], "Z": rap["Z"]}
    fc = pub(url, f"cutting feed up to {feed} mm/min") if feed else unk(FEED_UNK)
    cites = {f"joint.{a}.max_feed": fc for a in "XYZ"}
    cites.update({
        "spindle.S1.rpm": [pub(url, f"max {rpm} rpm"), unk(RPM_MIN_UNK.format(v=100))],
        "spindle.S1.max": unk("tool limits unpublished; 250 x 80 mm ours (BT40 class)"),
        "spindle.S1.interface": pub(url, "BBT-40"),
        "changer.pockets": pub(url, f"{pockets} tools"),
        "changer.change_s": unk(CHG_UNK),
        "changer.max_tool_kg": unk("max tool weight unpublished; 7 kg held (what Takumi publishes for CAT40); "
                                   "heavier tools are flagged"),
        "machine.control": pub(url, "Siemens 828D, Fanuc 0i Plus or Heidenhain iTNC 620 (option); fanuc held"),
        "machine.power": pub(url, kw + "; no curve published"),
    })
    MACHINES.append(vmc3(
        "hwacheon", nm, "fanuc", url,
        NOTE + "; hwacheon.com builds its pages by script and hwacheonusa.com is parked: numbers from the maker's "
               "European subsidiary's pages; control by option (Siemens / Fanuc / Heidenhain), fanuc held",
        tx=tx, ty=ty, tz=tz, hmin=5, hmax=tz + 5, D=ty / 2 + table[1] / 2 + 50, table=table, rapid=rap, feed=f,
        rpm=[100, rpm], iface="BBT40", tmax=(250, 80), changer=(pockets, 0.1, 7), coolants=("flood", "air"),
        nose_r=80, head=(360, 480), tt=60, rapid_src=False, hmin_status="unknown", D_pub=False, cites=cites))
    for a in "XYZ":
        MACHINES[-1].cite(f"joint.{a}.rapid", **pub(url, f"rapid {rap[a] / 1000:g} m/min"))

HWL = "https://www.hwacheon-europe.com/en/{}"
for nm, path, xt, zt, swing, turnl, chuck, rpm, rap, st, tr, hs, cl in [
    ("hwacheon-hi-tech-200a", "heavy-duty-cutting-lathes/hi-tech-200", 195, 430, 500, 401, 200, 5000, 30000, 12, 150, 250, 90),
    ("hwacheon-cutex-160a", "universal-lathes/cutex-160", 170, 335, 550, 300, 155, 6000, 36000, 12, 140, 220, 80),
    ("hwacheon-hi-tech-550a", "heavy-duty-cutting-lathes/hi-tech-550", 345, 1150, 800, 1100, 380, 2000,
     {"X": 20000, "Z": 24000}, 10, 220, 350, 140),
]:
    url = HWL.format(path)
    col = ("the page lists values per variant column without saying which goes with which; the first column "
           "read as type A")
    MACHINES.append(lathe(
        "hwacheon", nm, "fanuc", url,
        NOTE + "; numbers from Hwacheon Machinery Europe's page; control Siemens 828D or Fanuc 0i Plus (option), "
               "fanuc held (CAM-TURN owns lathe dialects); " + col,
        xt=xt, zt=zt, swing=swing, centres=turnl, rpm=[50, rpm], iface=f"{chuck} mm chuck (nose unpublished)",
        rapid=rap, feed=rap, chuck_r=chuck / 2, chuck_l=cl, stations=st, index_s=0.1, turret_r=tr, hs=hs,
        chuck_pub=(url, f"chuck {chuck} mm (first variant column)"),
        cites={"joint.X.range": est(col, source=url, value=f"X travel {xt} mm"),
               "joint.Z.range": est(col, source=url, value=f"Z travel {zt} mm"),
               **{f"joint.{a}.rapid": est(col, source=url, value=f"rapid {per(rap, a) / 1000:g} m/min") for a in "XZ"},
               **{f"joint.{a}.max_feed": unk(FEED_UNK) for a in "XZ"},
               "spindle.main.rpm": [est(col, source=url, value=f"max {rpm} rpm"), unk(RPM_MIN_UNK.format(v=50))],
               "spindle.main.max": est(col, source=url, value=f"swing over bed {swing} mm; max turning length {turnl} mm"),
               "turret.T1.stations": est(col, source=url, value=f"{st}-station turret"),
               "turret.T1.index_s": unk("index time unpublished; 0.1 s held so t_min stays a lower bound"),
               "machine.control": pub(url, "Siemens 828D or Fanuc 0i Plus")}))

# ---- Quaser (quaser.com /shop pages; the PDF downloads need a login: not used)
QS = "https://www.quaser.com/shop/{}"
for nm, path, tx, ty, tz, table, tool, pockets, typo in [
    ("quaser-mv1", "hx404-37", 661, 572, 560, (940, 550), (300, 76.2), 30, False),
    ("quaser-mv3", "mv134-11", 1020, 610, 610, (1200, 600), (280, 76.2), 30, True),
    ("quaser-mv4-40", "mv134-13", 1270, 700, 610, (1400, 700), (280, 76.2), 30, False),
    ("quaser-mv5-40", "mv184-14", 1524, 700, 610, (1700, 700), (280, 76.2), 48, False),
    ("quaser-mv6-40", "mv184-15", 2040, 800, 661, (2250, 800), (300, 76.2), 48, False),
]:
    url = QS.format(path)
    slow = unk("rapids unpublished on the open pages; 10,000 mm/min held low")
    cites = {f"joint.{a}.{f}": slow if f == "rapid" else unk(FEED_UNK) for a in "XYZ" for f in ("rapid", "max_feed")}
    cites.update({
        "spindle.S1.rpm": [pub(url, "spindle options 12,000 to 24,000 rpm" + (" (printed \"112K ~ 24K\")" if typo else "")),
                           est("the lowest option's 12,000 rpm held as the max (flags more)"),
                           unk(RPM_MIN_UNK.format(v=100))],
        "spindle.S1.max": pub(url, f"max tool {tool[1]} mm diameter (adjacent full), {tool[0]} mm long"),
        "spindle.S1.interface": pub(url, "taper 40"),
        "changer.pockets": pub(url, f"{pockets} tools"),
        "changer.change_s": unk(CHG_UNK),
        "changer.max_tool_kg": unk("max tool weight unpublished; 7 kg held (40 taper class)"),
        "machine.control": unk("control not stated on the open pages; recorded as \"unknown\""),
    })
    MACHINES.append(vmc3(
        "quaser", nm, "unknown", url,
        NOTE + "; Quaser's open pages give travels, table, taper, rpm and ATC only (spec PDFs need a login)",
        tx=tx, ty=ty, tz=tz, hmin=5, hmax=tz + 5, D=ty / 2 + table[1] / 2 + 50, table=table, rapid=10000,
        feed=10000, rpm=[100, 12000], iface="40 taper", tmax=tool, changer=(pockets, 0.1, 7),
        coolants=("flood",), nose_r=80, head=(360, 480), tt=60, rapid_src=False, hmin_status="unknown",
        D_pub=False, cites=cites))

# ---- Smart Machine Tool (smartmachinetool.com; brochures behind a form: not used)
SM = "https://www.smartmachinetool.com/{}"
for nm, path, tx, ty, tz, table, kw, pockets in [
    ("smart-sv1", "products/smart-sv1-linear-way-cat40-12k/", 599, 450, 500, (749, 450), "15/20 hp", None),
    ("smart-sv2", "vertical-machining-center/sv-2", 701, 450, 500, (800, 450), "20/25 hp", 30),
    ("smart-sv3", "vertical-machining-center/sv-3cat40_12k", 1050, 520, 520, (1150, 500), "20/25 hp", 30),
    ("smart-sv4", "vertical-machining-center/sv-4", 1300, 650, 600, (1450, 600), "20/25 hp", 30),
    ("smart-sv6", "vertical-machining-center/sv-6", 1600, 800, 800, (1750, 800), "20/25 hp", 30),
]:
    url = SM.format(path)
    slow = unk("rapids unpublished; 10,000 mm/min held low")
    cites = {f"joint.{a}.{f}": slow if f == "rapid" else unk(FEED_UNK) for a in "XYZ" for f in ("rapid", "max_feed")}
    cites.update({
        "spindle.S1.rpm": [pub(url, "12,000 rpm"), unk(RPM_MIN_UNK.format(v=100))],
        "spindle.S1.max": unk("tool limits unpublished; 250 x 80 mm ours (CAT40 class)"),
        "spindle.S1.interface": pub(url, "CAT40"),
        "machine.control": unk("control not stated; recorded as \"unknown\""),
        "machine.power": pub(url, kw + " (inch figures printed without units, read as inches)"),
    })
    if pockets:
        cites.update({"changer.pockets": pub(url, f"ATC {pockets}"), "changer.change_s": unk(CHG_UNK),
                      "changer.max_tool_kg": unk("max tool weight unpublished; 7 kg held (CAT40 class)")})
    MACHINES.append(vmc3(
        "smart", nm, "unknown", url,
        NOTE + "; the open page gives travels, table, taper, rpm, power and ATC only" +
        ("" if pockets else "; ATC not stated: no changer recorded (M6 refused)"),
        tx=tx, ty=ty, tz=tz, hmin=5, hmax=tz + 5, D=ty / 2 + table[1] / 2 + 50, table=table, rapid=10000,
        feed=10000, rpm=[100, 12000], iface="CAT40", tmax=(250, 80), changer=(pockets, 0.1, 7) if pockets else None,
        coolants=("flood",), nose_r=80, head=(360, 480), tt=60, rapid_src=False, hmin_status="unknown",
        D_pub=False, cites=cites))
