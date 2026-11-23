/* A measurement shim (tools/speed/gate.sh, phases.sh): getrandom(2) as a fixed stream, so std's
 * HashMap seeds are the same on every run and a part's count is exact (without it the bench
 * parts' counts move by up to ~0.4% from run to run). LD_PRELOAD only; nothing ships with it.
 * Build: cc -O2 -shared -fPIC -o fixrand.so fixrand.c */
#include <stddef.h>
#include <sys/types.h>

static unsigned long long state = 0x9e3779b97f4a7c15ULL;

ssize_t getrandom(void *buf, size_t len, unsigned int flags) {
  (void)flags;
  unsigned char *p = buf;
  for (size_t i = 0; i < len; i++) {
    state = state * 6364136223846793005ULL + 1442695040888963407ULL;
    p[i] = (unsigned char)(state >> 56);
  }
  return (ssize_t)len;
}
