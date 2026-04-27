// Game of Life benchmark — C reference port mirroring examples/life/life.ks.
// Same algorithm, same LCG seed, same toroidal double-modulo wrap, same
// double-buffered B3/S23 step. Built so the comparison isolates the
// compiler / runtime, not the algorithm.
//
// Build: cc -O2 -o life_bench life_bench.c
// Run:   ./life_bench WIDTH HEIGHT ITERS

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

static int64_t mod_pos(int64_t a, int64_t m) {
    int64_t r = a % m;
    if (r < 0) r += m;
    return r;
}

typedef struct {
    int64_t width, height;
    uint8_t *cells;
    uint8_t *next;
} Grid;

static int64_t idx(const Grid *g, int64_t x, int64_t y) {
    return mod_pos(y, g->height) * g->width + mod_pos(x, g->width);
}

static int cell_at(const Grid *g, int64_t x, int64_t y) {
    return g->cells[idx(g, x, y)];
}

static int64_t neighbor_count(const Grid *g, int64_t x, int64_t y) {
    int64_t n = 0;
    for (int64_t dy = -1; dy <= 1; dy++) {
        for (int64_t dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) continue;
            if (cell_at(g, x + dx, y + dy)) n++;
        }
    }
    return n;
}

static void step(Grid *g) {
    for (int64_t y = 0; y < g->height; y++) {
        for (int64_t x = 0; x < g->width; x++) {
            int alive = cell_at(g, x, y);
            int64_t n = neighbor_count(g, x, y);
            int next_alive = alive ? (n == 2 || n == 3) : (n == 3);
            g->next[idx(g, x, y)] = next_alive;
        }
    }
    uint8_t *tmp = g->cells;
    g->cells = g->next;
    g->next = tmp;
}

// LCG64 matching std.num.Lcg64 in lang/std/num/random.ks. Same multiplier
// and increment so the C run sees the same starting board as the Kestrel run.
static uint64_t lcg_state;
static uint64_t lcg_next(void) {
    lcg_state = lcg_state * 6364136223846793005ULL + 1442695040888963407ULL;
    return lcg_state;
}

static void randomize(Grid *g, uint64_t seed) {
    lcg_state = seed;
    int64_t total = g->width * g->height;
    for (int64_t i = 0; i < total; i++) {
        g->cells[i] = (lcg_next() % 10) < 3 ? 1 : 0;
    }
}

static int64_t monotonic_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: %s WIDTH HEIGHT ITERS\n", argv[0]);
        return 1;
    }
    Grid g;
    g.width = atoll(argv[1]);
    g.height = atoll(argv[2]);
    int64_t iters = atoll(argv[3]);
    int64_t total = g.width * g.height;
    g.cells = calloc(total, 1);
    g.next = calloc(total, 1);

    randomize(&g, 12648430ULL);

    int64_t t0 = monotonic_ms();
    for (int64_t i = 0; i < iters; i++) step(&g);
    int64_t elapsed = monotonic_ms() - t0;
    int64_t denom = elapsed > 0 ? elapsed : 1;
    int64_t gps = iters * 1000 / denom;

    printf("%lldx%lld  gens=%lld  elapsed_ms=%lld  gens_per_sec=%lld\n",
           (long long)g.width, (long long)g.height,
           (long long)iters, (long long)elapsed, (long long)gps);

    free(g.cells);
    free(g.next);
    return 0;
}
