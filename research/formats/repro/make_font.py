#!/usr/bin/env python3
"""A one-glyph TrueType font whose hmtx left side bearing (90) disagrees with its glyph's xMin
(100), made from nothing with fontTools' FontBuilder. FreeType (and fontTools' glyph set) place
the glyph by the side bearing, so its square spans x 90..190 there; ours keeps glyf's 100..200.

  $VENV/bin/python -I make_font.py OUT_DIR
"""
import os, sys
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

pen = TTGlyphPen(None)
pen.moveTo((100, 0)); pen.lineTo((100, 100)); pen.lineTo((200, 100)); pen.lineTo((200, 0)); pen.closePath()
sq = pen.glyph()
empty = TTGlyphPen(None).glyph()
fb = FontBuilder(1000, isTTF=True)
fb.setupGlyphOrder([".notdef", "A"])
fb.setupCharacterMap({ord("A"): "A"})
fb.setupGlyf({".notdef": empty, "A": sq})
fb.setupHorizontalMetrics({".notdef": (500, 0), "A": (300, 90)})
fb.setupHorizontalHeader(ascent=800, descent=-200)
fb.setupNameTable({"familyName": "LsbTest", "styleName": "Regular"})
fb.setupOS2()
fb.setupPost()
# setupGlyf recalculates bounds: keep xMin 100, the hmtx lsb 90
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
fb.save(os.path.join(out, "lsb-not-xmin.ttf"))
