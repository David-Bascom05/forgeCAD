# fix-shell round 2 (0ab77a8): the user-facing lines over be2947e, function by function

The exact callgrind gate (GATE_COUNTER=callgrind, exact mode) of 0ab77a8 against a baseline saved
on be2947e: every synthetic line below base (-1.6k to -8.2k), most kernel seq/nary lines below,
volute's app, edit and cli -1.86M, -3.7M, -1.9M. Thirteen parts' app/edit/cli lines over, by +184
to +6,297, all of about the same size. Accepted by the master under the layout-only rule.

## Where the excess goes

Each line's difference split by where the instructions run (two lines, both processes counted
exactly; `tools/speed/gate.sh`'s environment):

| line | glibc reading /proc/self/maps | ld.so | other libc | our code | total |
|---|---|---|---|---|---|
| bracket app | +4,422 | -351 | +255 | +4 | +4,330 |
| heatsink edit (the most) | +4,422 | -351 | +2,051 | +178 | +6,300 |

- **/proc/self/maps.** Rust's std reads the main thread's stack bounds through glibc's
  pthread_getattr_np, which parses /proc/self/maps line by line (sscanf) up to the stack's line.
  0ab77a8's benchparts has one more mapping before it: its .data starts at page offset 0xc50 (it
  was 0x3f0 at be2947e). .data and .bss together are 5,131 bytes, so past 0xbf5 the .bss runs onto
  a third page, and the kernel maps it as a separate anonymous mapping. It is 91 bytes over here.
  That costs one line natively (38 lines to 39) and two under valgrind (stack at line 53, then 54).
  regen1 does not spill (0xa00), which is why the synthetic lines are clean. The offset is the sum
  of every section laid out before .data, modulo the page: any change of code size moves it.
- **ld.so.** -351: 27 fewer relative relocations (19,056 to 19,029).
- **other libc (heatsink).** memcmp +1,945, now called from inside forge_bool::nary::nary_body
  where a quicksort comparison it was called from before was inlined into it: the same
  comparisons, attributed to another frame.
- **our code.** Net +4 (bracket) and +178 (heatsink). Inlining moved work between frames, for
  example in heatsink edit:

| change | function |
|---|---|
| -2,752 | `Copied<Iter<f64>> as Iterator>::try_fold` (a filter over a chain's parameters) |
| +1,760 | `line_implicit<line_cone>::{closure#8}`: the same work, now not inlined there |
| +624 | `Blend::spring`: counted in boolean::apply, where rolls_off was inlined |
| +512 | `Vec<f64>::from_iter` (filter over copied f64) |

The kernel's own lines for the same thirteen parts (each regen call alone, diagnostic) are
below base except heatsink seq +99, bracket seq +6, insert seq/nary1 +117/+117 and clamp
seq/nary1 +31/+49 (not profiled; the size of the moves between frames above).

## Tried and reverted

`#[inline]` on Curve::eval and Curve::bbox (the dispatch) and on Line::eval and Line::bbox. Each
subset helped some parts and hurt others, by moving codegen elsewhere:

| inlined | bracket app | hub app | coplanar seq | volute app |
|---|---|---|---|---|
| all four | -23.6k | +10 | +105k | +0.98M |
| bbox only | -4.0k | -3.7k | -113k | +2.15M |
| eval only | -13.3k | +15.3k | +213k | -3.03M |
| Line's only | +3.4k | +4.3k | -26k | -1.86M |

No subset is below base on every line. The page boundary itself was not moved: any merge re-rolls
where it falls.
