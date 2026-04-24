// test: diagnostics
// stdlib: false

module Test

struct Handler[T] {
    var callback: (T) -> ()
}
