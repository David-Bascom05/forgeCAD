/* Preloaded into the counted runs of tools/speed/gate.sh, so that a count is a function of the
 * binary and its input alone. Not for anything but counting.
 *
 * getrandom(2) with fixed bytes: std's HashMaps seed their keys from it, and keys that differ from
 * run to run move the counts (by probe lengths and by what the heap holds when).
 *
 * clock_gettime(2) a clock that advances one microsecond per reading: the kernel reads the clock
 * for timings and for when to start helper threads (forge-model's `ahead`, forge-math's
 * `map_adaptive`), and a `Duration`'s sums and products take a few instructions more or fewer
 * by its value, so a run preempted under load counted up to 24 fewer on regen1 tour seq
 * (9,429,120 to 9,429,144 on one binary). No result depends on the clock; with this, no count does. */
#include <stddef.h>
#include <stdatomic.h>
#include <sys/types.h>
#include <time.h>

ssize_t getrandom(void *buf, size_t len, unsigned int flags) {
    unsigned char *b = buf;
    (void)flags;
    for (size_t i = 0; i < len; i++) {
        b[i] = (unsigned char)(i * 131 + 7);
    }
    return (ssize_t)len;
}

static atomic_ullong readings;

int clock_gettime(clockid_t clk, struct timespec *tp) {
    /* microseconds since a fixed start; the wall clock starts at 2026-01-01T00:00:00Z */
    unsigned long long us = atomic_fetch_add(&readings, 1ULL);
    unsigned long long base = (clk == CLOCK_REALTIME || clk == CLOCK_REALTIME_COARSE) ? 1767225600ULL : 1000ULL;
    tp->tv_sec = (time_t)(base + us / 1000000ULL);
    tp->tv_nsec = (long)((us % 1000000ULL) * 1000ULL);
    return 0;
}
