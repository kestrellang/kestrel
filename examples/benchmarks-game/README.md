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
| k-nucleotide 25M | 7.64 | 5.74 #1 | 3.44 #3 | 9.26 #6 | 23.30 #2 | 9.20 #6 | 34.21 #3 | 34.12 #3 | 92.81 #3 |
| reverse-complement 25M | 0.71 | 0.21 #9 | 0.20 #1 | 0.42 #1 | 0.53 #1 | 0.46 #7 | 1.08 #7 | 1.00 #7 | 0.93 #3 |
| binary-trees 21 | 11.69 | 2.25 #3 | 2.56 #2 | 16.68 #6 | 2.87 #9 | 11.02 #2 | — | 16.21 #7 | 52.68 #2 |
| pidigits 10000 | 0.54 | 0.53 #2 | 0.54 #1 | 0.67 #4 | 0.56 #2 | 0.57 #5 | 0.74 #4 | 0.74 #4 | 0.76 #3 |
| regex-redux 5M | 1.56 | 1.41 #5 | 0.87 #6 | 16.86 #4 | 18.19 #2 | 1.63 #2 | 3.61 #4 | 3.65 #4 | 1.88 #2 |

Geometric mean of Kestrel's time over the other language's (below 1 means Kestrel is faster): C 1.63, Rust 1.68, C# 1.04, Swift 0.68, Go 0.56, Node 0.47, TypeScript 0.38 (9 benchmarks), Python 0.08.

Variants in `variants/`:

- `binarytrees_enum.ks` uses an `indirect enum`: 16.4 s. Each node is 48 bytes there (a 24-byte `Tree` value per child, with `OwnedBox`'s liveness flag), against 16 bytes for the raw-pointer version.
- `regexredux_pure.ks` uses the pure-Kestrel `lang/regex` package instead of PCRE2: 102 s.

For reference, the plain-`malloc` C binary-trees (#1) takes 11.0 s on the same core; the fast C, Rust and Swift entries use APR pools or arenas, which Kestrel's std doesn't have.

## How the Kestrel programs get their speed

- **n-body**: bodies in an `Array` of structs, the step loop takes a `mutating Slice` so writes skip the copy-on-write check, and the body count is a constant so LLVM unrolls. The C, Rust and C# winners use AVX intrinsics and an approximate `rsqrt`.
- **spectral-norm**: denominators in floating point and 8 independent partial sums, so LLVM's SLP vectorizer emits packed divides (1.52 s → 0.81 s). The C and Rust winners use explicit AVX.
- **fannkuch-redux**: the permutation lives in one `UInt64`, 4 bits per element, so a flip is a byte swap, a nibble swap, a shift and a splice (24 s → 12.9 s). C# #5 and C #6 do the same with SSE shuffles.
- **mandelbrot**: 8 pixels per byte as 8 scalar lanes, with the inner iterations unrolled so the SLP vectorizer packs them (6.2 s → 3.1 s).
- **fasta**: a lookup table indexed by the generator state replaces the cumulative-probability search; output goes through one buffer.
- **k-nucleotide**: k-mers packed 2 bits per base into a `UInt64` key, counted in `Dictionary[UInt64, Int64, FoldHasher]`. It's held back by `Dictionary` itself (see below).
- **reverse-complement**: read straight into one buffer, squeeze newlines out in place, write the complement in 1 MB pieces.
- **binary-trees**: 16-byte nodes of two raw `Pointer`s, one `malloc` and one `free` per node, like C #1.
- **pidigits**: GMP through `@extern(.C)`, the same library the C, Rust and Go programs use.
- **regex-redux**: PCRE2 with its JIT through `@extern(.C)`, like the C program.

## Where Kestrel loses, and why

- **No SIMD types or intrinsics.** n-body, spectral-norm and mandelbrot are close to the scalar or auto-vectorized C, but the winners hand-write AVX.
- **`Dictionary` per-operation cost.** Even with only 4 keys (everything in L1), a lookup-and-increment costs about 4.5 ns. Rust #3 runs the same algorithm on hashbrown in 3.85 s. Each lookup calls an out-of-line `CowBox.ensureUnique`, and buckets are 32 bytes (`Bucket` enum with a cached hash), so the 140k-key 18-mer table is 8 MB and misses cache. Switching the hasher (custom multiply, `FoldHasher`, folded `multiplyFull`) made no difference.
- **No arena or pool allocator in std.** binary-trees is at C-with-`malloc` speed, 5x behind the pooled programs.
- **reverse-complement I/O.** About half the time is kernel time: reading, and page faults from growing and zero-filling the input buffer.

## Running

`harness/harness.py` downloads nothing. It expects the Benchmarks Game source zip unpacked at `/tmp/claude-0/bg/dl/x`, generated fasta inputs in `/tmp/claude-0/bg/inputs`, and the toolchains at the paths at the top of the file. Steps: `build` (all programs, all languages), `kbuild` (the Kestrel programs), `select` (run every program at a reduced size, check its output, keep each language's fastest), `full` (official sizes, best of 3).

Build commands used:

- Kestrel: `kestrel build -O2` (LLVM, host CPU), `-l gmp` / `-l pcre2-8` for pidigits / regex-redux.
- C: `gcc 13.3 -O3 -march=native -fopenmp` with `-lgmp -lpcre -lpcre2-8 -lapr-1`.
- Rust 1.92: one Cargo project (`rust-Cargo.toml`), `opt-level=3`, `codegen-units=1`, LTO, `-C target-cpu=native`.
- Go 1.24.7 with `GOAMD64=v3`. Swift 6.0.3 `-Ounchecked -wmo`. .NET 9 (`bg.csproj`). TypeScript 6.0 (`tsc --target es2022`) and JavaScript on Node 22.23. CPython 3.13.

A program was dropped if its output didn't match (mandelbrot allows a few differing bytes, from floating-point edge pixels). Several multi-threaded programs are among the winners; pinned to one core they run single-threaded in effect, at a small cost for the unused threads.
