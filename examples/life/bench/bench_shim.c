// Shared monotonic clock so the Kestrel and C solvers time the identical thing.
#include <time.h>

long long bench_now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long long)ts.tv_sec * 1000000000LL + (long long)ts.tv_nsec;
}
