# Benchmarks Game in Kestrel

Kestrel versions of the ten [Computer Language Benchmarks Game](https://benchmarksgame-team.pages.debian.net/benchmarksgame/index.html) programs, tuned for single-threaded speed, and the harness that compares them with C, Rust, Go, Swift, C#, TypeScript, JavaScript (Node) and Python.

They need the compiler and std from PR #258 (`FoldHasher`, `unsafe` unchecked subscripts). The results below were measured on PR #258 at `4d0eae3f` merged with nightly at `7221a32d` (local merge commit `2014aa37`).

## Results

Seconds, best of 3 runs (Python: 1 run, from the first round), every program pinned to one core with `taskset -c 0`. Official input sizes. `#n` is the Benchmarks Game program number that was fastest for that language on one core.

| Benchmark | Kestrel | C | Rust | Go | Swift | C# | TypeScript | Node | Python |
|---|---|---|---|---|---|---|---|---|---|
| n-body 50M | 2.14 | 1.39 #9 | 1.45 #9 | 2.32 #3 | 2.23 #3 | 1.84 #9 | 3.11 #6 | 2.98 #6 | 162.2 #1 |
| spectral-norm 5500 | 0.80 | 0.64 #2 | 0.68 #4 | 1.30 #2 | 1.28 #8 | 1.34 #2 | 1.85 #7 | 1.69 #1 | 150.2 #7 |
| fannkuch-redux 12 | 12.01 | 8.24 #6 | 13.47 #6 | 22.44 #3 | 22.44 #3 | 7.36 #5 | 33.68 #5 | 28.09 #5 | 492.3 #6 |
| mandelbrot 16000 | 2.97 | 2.65 #6 | 2.19 #4 | 7.68 #1 | 2.93 #7 | 4.24 #7 | 8.42 #3 | 8.44 #3 | 318.6 #7 |
| fasta 25M | 1.17 | 0.74 #3 | 0.92 #7 | 3.08 #2 | 2.61 #2 | 0.78 #9 | 13.08 #1 | 4.18 #5 | 19.43 #3 |
| k-nucleotide 25M | 5.31 | 5.74 #1 | 3.44 #3 | 9.26 #6 | 23.30 #2 | 9.20 #6 | 34.21 #3 | 34.12 #3 | 92.81 #3 |
| reverse-complement 25M | 0.71 | 0.21 #9 | 0.20 #1 | 0.42 #1 | 0.53 #1 | 0.46 #7 | 1.08 #7 | 1.00 #7 | 0.93 #3 |
| binary-trees 21 | 2.96 | 2.25 #3 | 2.56 #2 | 16.68 #6 | 2.87 #9 | 11.02 #2 | — | 16.21 #7 | 52.68 #2 |
| pidigits 10000 | 0.54 | 0.53 #2 | 0.54 #1 | 0.67 #4 | 0.56 #2 | 0.57 #5 | 0.74 #4 | 0.74 #4 | 0.76 #3 |
| regex-redux 5M | 1.56 | 1.41 #5 | 0.87 #6 | 16.86 #4 | 18.19 #2 | 1.63 #2 | 3.61 #4 | 3.65 #4 | 1.88 #2 |

The binary-trees row is the std-arena version (see below), measured at `b00a92b2`; the other rows are from `2014aa37`.

Geometric mean of Kestrel's time over the other language's (below 1 means Kestrel is faster): C 1.42, Rust 1.46, C# 0.91, Swift 0.59, Go 0.49, Node 0.41, TypeScript 0.38 (9 benchmarks, no binary-trees), Python 0.07. With the `malloc` binary-trees (11.69 s) they were C 1.63, Rust 1.68, C# 1.04, Swift 0.68, Go 0.56, Node 0.47, Python 0.08.

Variants in `variants/`:

- `binarytrees_malloc.ks` does one `malloc` and one `free` per node, like C #1: 11.1 s, against 11.0 s for that C program.
- `binarytrees_enum.ks` uses an `indirect enum`: 11.7 s with this branch's compiler and std (16.0 s before). Each node is 32 bytes (two 16-byte `Tree` values), against 16 bytes for the raw-pointer version.
- `regexredux_pure.ks` uses the pure-Kestrel `lang/regex` package instead of PCRE2: 102 s.

The fast C, Rust and Swift binary-trees entries use APR pools or arena crates; the Kestrel program uses std's `ArenaAllocator` the same way.

## std and compiler changes in this branch

Profiling these programs with `perf` led to three changes, measured at -O2 on one core, best of 3, against the same programs built by the unchanged compiler (`2014aa37`, PR #258 merged with nightly):

| Program | Before | After |
|---|---|---|
| k-nucleotide 25M | 8.09 s | 5.31 s |
| binary-trees 21, `indirect enum` variant | 16.02 s | 11.74 s |
| the other nine | | within ±2% (noise) |

- **`CowBox.ensureUnique` is inlined.** Every `Array`/`Dictionary` write called it out of line, because the inlined fork made it too big. The fork is now a separate `@inline(.Never)` function.
- **`Dictionary` is SwissTable-shaped.** One block of entry slots plus a control byte per slot (empty, deleted, or 7 bits of the hash), probed home slot first and then 8 control bytes at a time with bit tricks. Entries lost the enum tag and cached hash, so `[UInt64: Int64]` takes 17 bytes per slot instead of 32. `dict(key, default:)` and `insert` probe once instead of two or three times.
- **Indirect enum nodes are smaller and drop without copying.** `OwnedBox` marks a given-up payload with a null pointer instead of a separate flag (48 → 32 bytes per binary-trees node), and the compiler no longer memcpys a value out of memory before destroying it in place, which recursive drop glue did at every level.

## How the Kestrel programs get their speed

- **n-body**: bodies in an `Array` of structs, the step loop takes a `mutating Slice` so writes skip the copy-on-write check, and the body count is a constant so LLVM unrolls. The C, Rust and C# winners use AVX intrinsics and an approximate `rsqrt`.
- **spectral-norm**: denominators in floating point and 8 independent partial sums, so LLVM's SLP vectorizer emits packed divides (1.52 s → 0.81 s). The C and Rust winners use explicit AVX.
- **fannkuch-redux**: the permutation lives in one `UInt64`, 4 bits per element, so a flip is a byte swap, a nibble swap, a shift and a splice (24 s → 12.9 s). C# #5 and C #6 do the same with SSE shuffles.
- **mandelbrot**: 8 pixels per byte as 8 scalar lanes, with the inner iterations unrolled so the SLP vectorizer packs them (6.2 s → 3.1 s).
- **fasta**: a lookup table indexed by the generator state replaces the cumulative-probability search; output goes through one buffer.
- **k-nucleotide**: k-mers packed 2 bits per base into a `UInt64` key, counted in `Dictionary[UInt64, Int64, FoldHasher]`. It's held back by `Dictionary` itself (see below).
- **reverse-complement**: read straight into one buffer, squeeze newlines out in place, write the complement in 1 MB pieces.
- **binary-trees**: 16-byte nodes of two raw `Pointer`s, bump-allocated from std's `ArenaAllocator` (1 MB first chunk). Each tree is freed at once with `reset()`, which keeps the chunk for the next tree (11.1 s → 2.96 s).
- **pidigits**: GMP through `@extern(.C)`, the same library the C, Rust and Go programs use.
- **regex-redux**: PCRE2 with its JIT through `@extern(.C)`, like the C program.

## Where Kestrel loses, and why

- **No SIMD types or intrinsics.** n-body, spectral-norm and mandelbrot are close to the scalar or auto-vectorized C, but the winners hand-write AVX.
- **`Dictionary` cache misses.** After this branch's Dictionary rework, k-nucleotide is memory-bound on the entry load of the 140k-key tables (Rust #3, the same algorithm on hashbrown: 3.44 s).
- **binary-trees allocation.** With std's arena it is 1.3x behind C #3 (APR pools, 2.25 s) and 1.2x behind Rust #2 (bumpalo, 2.56 s).
- **reverse-complement I/O.** About half the time is kernel time: reading, and page faults from growing and zero-filling the input buffer.

## Running

`harness/harness.py` downloads nothing. It expects the Benchmarks Game source zip unpacked at `/tmp/claude-0/bg/dl/x`, generated fasta inputs in `/tmp/claude-0/bg/inputs`, and the toolchains at the paths at the top of the file. Steps: `build` (all programs, all languages), `kbuild` (the Kestrel programs), `select` (run every program at a reduced size, check its output, keep each language's fastest), `full` (official sizes, best of 3).

Build commands used:

- Kestrel: `kestrel build -O2` (LLVM, host CPU), `-l gmp` / `-l pcre2-8` for pidigits / regex-redux.
- C: `gcc 13.3 -O3 -march=native -fopenmp` with `-lgmp -lpcre -lpcre2-8 -lapr-1`.
- Rust 1.92: one Cargo project (`rust-Cargo.toml`), `opt-level=3`, `codegen-units=1`, LTO, `-C target-cpu=native`.
- Go 1.24.7 with `GOAMD64=v3`. Swift 6.0.3 `-Ounchecked -wmo`. .NET 9 (`bg.csproj`). TypeScript 6.0 (`tsc --target es2022`) and JavaScript on Node 22.23. CPython 3.13.

A program was dropped if its output didn't match (mandelbrot allows a few differing bytes, from floating-point edge pixels). Several multi-threaded programs are among the winners; pinned to one core they run single-threaded in effect, at a small cost for the unused threads.
