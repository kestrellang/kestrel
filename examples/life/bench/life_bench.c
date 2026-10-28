// C port of examples/life/src/grid.ks — identical algorithm, LCG, and B3/S23
// rules, so the checksum (live-cell count) must match the Kestrel build.
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

extern long long bench_now_ns(void);

typedef struct {
    long width, height;
    unsigned char *cells;
    unsigned char *next;
} Grid;

// std Lcg64: state = state*a + c (mod 2^64); nextUInt64 returns the NEW state.
static uint64_t lcg_state;
static inline uint64_t lcg_next(void) {
    lcg_state = lcg_state * 6364136223846793005ULL + 1442695040888963407ULL;
    return lcg_state;
}
// std nextInt(below: 10): (nextUInt64() % 10)
static inline long lcg_below(long bound) {
    return (long)(lcg_next() % (uint64_t)bound);
}

static inline unsigned char cell_at(const Grid *g, long x, long y) {
    long w = g->width, h = g->height;
    long xx = x; if (xx < 0) xx += w; else if (xx >= w) xx -= w;
    long yy = y; if (yy < 0) yy += h; else if (yy >= h) yy -= h;
    return g->cells[yy * w + xx];
}

static inline long neighbor_count(const Grid *g, long x, long y) {
    long count = 0;
    for (long dy = -1; dy <= 1; dy++) {
        for (long dx = -1; dx <= 1; dx++) {
            if (!(dx == 0 && dy == 0)) {
                if (cell_at(g, x + dx, y + dy)) count++;
            }
        }
    }
    return count;
}

static void step(Grid *g) {
    for (long y = 0; y < g->height; y++) {
        for (long x = 0; x < g->width; x++) {
            unsigned char alive = cell_at(g, x, y);
            long n = neighbor_count(g, x, y);
            unsigned char next_alive = alive ? (n == 2 || n == 3) : (n == 3);
            g->next[y * g->width + x] = next_alive;
        }
    }
    unsigned char *tmp = g->cells;
    g->cells = g->next;
    g->next = tmp;
}

static void randomize(Grid *g, uint64_t seed) {
    lcg_state = seed;
    long total = g->width * g->height;
    for (long i = 0; i < total; i++) {
        g->cells[i] = lcg_below(10) < 3;
    }
}

int main(void) {
    long W = 512, H = 512, ITERS = 2000;
    long n = W * H;

    Grid g;
    g.width = W; g.height = H;
    g.cells = calloc(n, 1);
    g.next  = calloc(n, 1);
    randomize(&g, 12648430ULL);

    long long t0 = bench_now_ns();
    for (long it = 0; it < ITERS; it++) step(&g);
    long long t1 = bench_now_ns();

    long live = 0;
    for (long i = 0; i < n; i++) if (g.cells[i]) live++;

    long long ms = (t1 - t0) / 1000000LL;
    long gps = ms > 0 ? (ITERS * 1000 / ms) : 0;
    printf("c       W=%ld H=%ld iters=%ld ms=%lld gens_per_sec=%ld live=%ld\n",
           W, H, ITERS, ms, gps, live);
    return 0;
}
