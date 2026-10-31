#!/usr/bin/env python3
"""python3 -m unittest tools/find-parts/test_gens.py"""
import math
import os
import sys
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gens


class Exprs(unittest.TestCase):
    def test_reads_what_part_files_write(self):
        env = {"w": 40.0, "pi": math.pi}
        for text, want in (("w*3 + 1", 121.0), ("(w + 2)/2", 21.0), ("2^3", 8.0), ("-w", -40.0), ("sqrt(16)", 4.0),
                           ("min(w, 3)", 3.0), ("sin(90)", 1.0), ("atan2(1, 1)", 45.0), ("pi", math.pi), ("1e3", 1000.0)):
            self.assertAlmostEqual(gens.expr_value(text, env), want, msg=text)

    def test_refuses_anything_else(self):
        for text in ("__import__('os')", "().__class__", "(1).real", "open('x')", "[1][0]", "1 if 1 else 2", "lambda: 1",
                     "'a'", "True", "undefined", "w()", "sqrt(x=1)", "1 << 9", "1 ^^ 2", "f'{1}'", "(y := 1)", "abs.__call__(1)"):
            with self.assertRaises(ValueError, msg=text):
                gens.expr_value(text, {"w": 1.0})

    def test_cannot_hang_or_blow_up(self):
        t = time.monotonic()
        for text in ("9**9**9", "9^9^9", "2^1e9", "1e308*1e308", "1/0", "(-8)^0.5", "sqrt(-1)", "(" * 400 + "1" + ")" * 400,
                     "1+" * 400 + "1", "9" * 501):
            with self.assertRaises(ValueError, msg=text[:30]):
                gens.expr_value(text, {})
        self.assertLess(time.monotonic() - t, 1.0)

    def test_every_bench_parameter_still_reads(self):
        for name, src in gens.bench():
            lines = [l.split("#")[0].strip() for l in src.splitlines()]
            want = sum(1 for l in lines if gens.PARAM.match(l))
            self.assertEqual(len(gens.params(src)), want, name)


if __name__ == "__main__":
    unittest.main()
