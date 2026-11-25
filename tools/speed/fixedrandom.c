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
 * (9,429,120 to 9,429,144 on one binary). No result depends on the clock; with this, no count does.
 *
 * memcmp, bcmp, memcpy, memmove, memset, strlen whose instruction count is a function of the
 * length (and, for a compare, where the first difference is) alone: glibc's AVX2 versions cost
 * more or less by where the bytes sit, so a static string that moved with the binary's size moved
 * every count that compares it (forge-lang's name compares: up to +0.0055% on an edit line with
 * no change to their code). These still count the work in proportion to the bytes; only the
 * alignment luck is gone. Eight bytes per step through unaligned loads, then the tail a byte at a
 * time. Built with -fno-builtin -ffreestanding (gate.sh), so the compiler cannot turn these loops
 * back into calls to themselves. */
#include <stddef.h>
#include <stdatomic.h>
#include <sys/types.h>
#include <time.h>
#include <stdint.h>

typedef uint64_t u64u __attribute__((aligned(1), may_alias));

static inline uint64_t ld8(const unsigned char *p) { return *(const u64u *)p; }
static inline void st8(unsigned char *p, uint64_t v) { *(u64u *)p = v; }

static int cmp(const void *a, const void *b, size_t n) {
    const unsigned char *x = a, *y = b;
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        uint64_t u = ld8(x + i), v = ld8(y + i);
        if (u != v) {
            /* the first differing byte decides: compare the words as big-endian */
            u = __builtin_bswap64(u);
            v = __builtin_bswap64(v);
            return u < v ? -1 : 1;
        }
    }
    for (; i < n; i++) {
        if (x[i] != y[i]) {
            return x[i] < y[i] ? -1 : 1;
        }
    }
    return 0;
}

int memcmp(const void *a, const void *b, size_t n) { return cmp(a, b, n); }

int bcmp(const void *a, const void *b, size_t n) { return cmp(a, b, n) != 0; }

void *memcpy(void *restrict d, const void *restrict s, size_t n) {
    unsigned char *to = d;
    const unsigned char *from = s;
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        st8(to + i, ld8(from + i));
    }
    for (; i < n; i++) {
        to[i] = from[i];
    }
    return d;
}

void *memmove(void *d, const void *s, size_t n) {
    unsigned char *to = d;
    const unsigned char *from = s;
    if (to <= from || to >= from + n) {
        /* forward: each step reads before it writes, and what it writes is behind what is left */
        size_t i = 0;
        for (; i + 8 <= n; i += 8) {
            st8(to + i, ld8(from + i));
        }
        for (; i < n; i++) {
            to[i] = from[i];
        }
    } else {
        /* backward (the destination overlaps after the source) */
        size_t i = n;
        for (; i >= 8; i -= 8) {
            st8(to + i - 8, ld8(from + i - 8));
        }
        while (i > 0) {
            i--;
            to[i] = from[i];
        }
    }
    return d;
}

void *memset(void *d, int c, size_t n) {
    unsigned char *to = d;
    uint64_t w = (unsigned char)c * 0x0101010101010101ULL;
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        st8(to + i, w);
    }
    for (; i < n; i++) {
        to[i] = (unsigned char)c;
    }
    return d;
}

size_t strlen(const char *s) {
    size_t n = 0;
    while (s[n]) {
        n++;
    }
    return n;
}

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
