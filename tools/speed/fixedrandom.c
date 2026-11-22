/* getrandom(2) with fixed bytes, preloaded into the counted runs of tools/speed/gate.sh: std's
 * HashMaps seed their keys from it, and keys that differ from run to run move the counts (by
 * probe lengths and by what the heap holds when). Not for anything but counting. */
#include <stddef.h>
#include <sys/types.h>

ssize_t getrandom(void *buf, size_t len, unsigned int flags) {
    unsigned char *b = buf;
    (void)flags;
    for (size_t i = 0; i < len; i++) {
        b[i] = (unsigned char)(i * 131 + 7);
    }
    return (ssize_t)len;
}
