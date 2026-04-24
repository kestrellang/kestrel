// test: diagnostics
// stdlib: false

module Test

struct Wrapper[T] {
    let ptr: lang.ptr[T]
}
