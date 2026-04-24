// test: diagnostics
// stdlib: false

module Test

struct Point { let x: lang.i64; let y: lang.i64 }
struct Wrapper {
    let ptr: lang.ptr[Point]
}
