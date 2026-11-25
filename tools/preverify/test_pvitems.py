#!/usr/bin/env python3
"""Tests of pre-verify's judgement: a step that did not build, or whose failure no test accounts
for, stays RED, and only failures made wholly of tests are ever judged again (time budget, main).

    python3 tools/preverify/test_pvitems.py
"""
import os, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pvitems import items, stays_red  # noqa: E402

# fix/asm-shape-first @3a73019c's render log, as it failed: a test target that did not compile
COMPILE_ERROR = """\
   Compiling forge-render v0.1.0 (/w/interface/forge-render)
error[E0063]: missing field `pinned` in initializer of `AgentContext`
  --> tests/agent_context.rs:41:15
error: could not compile `forge-render` (test "agent_context") due to 1 previous error
warning: build failed, waiting for other jobs to finish...
"""
# a compile error in one target while another target's tests ran and failed
COMPILE_AND_TESTS = COMPILE_ERROR + """\
     Running tests/cam_ui.rs (target/release/deps/cam_ui-0123456789abcdef)
test a_tab_opens ... FAILED
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
"""
# a native-tests.py log: one test failed, one over the time budget
NATIVE_FAIL = """\
test chip_load_and_equal_work ... FAILED
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s (guards)
error: over the 2 s budget: contour fuzz_random_profiles took 177.67 s in the run, 118.97 s alone
"""
# only over the budget: the one kind the time-budget rule may relax
NATIVE_SLOW_ONLY = """\
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s (guards)
error: over the 2 s budget: contour fuzz_random_profiles took 177.67 s in the run, 118.97 s alone
"""
# a failure with no test line at all (a panic in a build script, a killed binary)
NO_TEST_LINES = """\
error: failed to run custom build command for `forge-render v0.1.0`
process didn't exit successfully: `build-script-build` (signal: 9, SIGKILL)
"""


class StaysRed(unittest.TestCase):
    def test_compile_error_stays_red(self):
        for step in ("render", "native", "wasm"):
            self.assertEqual(stays_red(step, COMPILE_ERROR), "a build or compile error", step)

    def test_compile_error_with_test_failures_stays_red(self):
        self.assertEqual(stays_red("render", COMPILE_AND_TESTS), "a build or compile error")

    def test_failure_no_test_accounts_for_stays_red(self):
        self.assertEqual(stays_red("native", NO_TEST_LINES), "a failure no test accounts for")
        self.assertEqual(stays_red("native", ""), "a failure no test accounts for")

    def test_test_failures_may_be_judged(self):
        self.assertIsNone(stays_red("native", NATIVE_FAIL))
        kinds = {x[0] for x in items("native", NATIVE_FAIL, {})}
        self.assertEqual(kinds, {"fail", "slow"})

    def test_budget_only_may_be_judged(self):
        self.assertIsNone(stays_red("native", NATIVE_SLOW_ONLY))
        self.assertEqual({x[0] for x in items("native", NATIVE_SLOW_ONLY, {})}, {"slow"})


class RunPyUsesIt(unittest.TestCase):
    """run.py's relative() returns before any re-judging when stays_red says so: the record keeps
    ok False. Exercised on a copy of relative() fed a step whose log is COMPILE_ERROR."""

    def test_relative_keeps_a_compile_error_red(self):
        import re, tempfile
        here = os.path.dirname(os.path.abspath(__file__))
        src = open(os.path.join(here, "run.py")).read()
        body = src[src.index("def relative("):src.index("\ndef judge(")]
        log = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
        log.write(COMPILE_ERROR)
        log.close()
        record = {"steps": {"render": {"ok": False, "seconds": 1, "log": log.name}}}
        ns = {"record": record, "items": items, "stays_red": stays_red, "re": re,
              "ALONE": {}, "corpus": lambda x: False, "CORPUS_BUDGET": 15, "SLOW_RED": 4,
              "target_pkgs": lambda t: set(), "touched": lambda: set()}
        exec(body, ns)
        ns["relative"]("render", lambda p: [], lambda it: set())
        os.unlink(log.name)
        self.assertIs(record["steps"]["render"]["ok"], False)
        self.assertEqual(record["steps"]["render"]["why_red"], "a build or compile error")


if __name__ == "__main__":
    unittest.main(verbosity=2)
