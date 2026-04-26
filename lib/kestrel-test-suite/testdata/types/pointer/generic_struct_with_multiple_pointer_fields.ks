// test: diagnostics
// stdlib: false

module Test

struct Pair[A, B] {
    let first: lang.ptr[A]
    let second: lang.ptr[B]
}
