/* getrandom(2) from a seed (LAT_SEED, an integer), preloaded into the latency gate's runs
 * (latency.sh): std's HashMaps seed their keys from it, and a process's random keys move its
 * time by several percent (probe lengths, what the heap holds when). A round runs base and head
 * with the same seed, and each round another, so a pair differs by the code alone and the rounds
 * still average over many seeds. Not for anything but measuring. */
#include <stddef.h>
#include <stdlib.h>
#include <sys/types.h>

ssize_t getrandom(void *buf, size_t len, unsigned int flags) {
    static unsigned long long s;
    static int init;
    unsigned char *b = buf;
    (void)flags;
    if (!init) {
        const char *e = getenv("LAT_SEED");
        s = 0x9e3779b97f4a7c15ULL ^ (e ? strtoull(e, 0, 10) : 0);
        init = 1;
    }
    for (size_t i = 0; i < len; i++) {
        /* splitmix64, one byte per step */
        unsigned long long z = (s += 0x9e3779b97f4a7c15ULL);
        z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL;
        z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL;
        b[i] = (unsigned char)(z ^ (z >> 31));
    }
    return (ssize_t)len;
}
