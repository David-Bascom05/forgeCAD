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
        hmin_status="unknown", cites=cites))
    MACHINES[-1].cite("body.column", "estimate",
                      basis="spindle-to-column distance unpublished: column face put 50 mm beyond the table's "
                            "furthest y reach (the least that fits the travel); the real column may be further")

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
        rapid_src=(spec, "max CNC positioning speed 22 in/min (559 mm/min)"), hmin_status="unknown",
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
    MACHINES[-1].cite("body.column", "estimate",
                      basis="column distance unpublished: face 30 mm beyond the table's furthest y reach")

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
