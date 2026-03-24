// test: diagnostics
// stdlib: false

module Test

func bad[T](a: T, b: T) -> T {
    return a.add(b) // ERROR: no member 'add'
}
