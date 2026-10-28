// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 (BUG-52) — recursive array-pattern destructuring. The `..rest` binding
// yields an `ArraySlice[T]`, which itself conforms to `ArrayMatchable`, so it
// can be matched again. Also pins the exhaustiveness fix: `[a, ..rest] | []`
// over a *slice* is exhaustive (the slice has infinite length like an array),
// which previously false-tripped E305 (slice classified as a 1-field struct).

module Test

func sumSlice(s: ArraySlice[Int64]) -> Int64 {
    match s {
        [a, ..rest] => a + sumSlice(rest),
        [] => 0
    }
}

func sum(arr: [Int64]) -> Int64 {
    match arr {
        [a, ..rest] => a + sumSlice(rest),
        [] => 0
    }
}

@main
func main() -> lang.i32 {
    if sum([1, 2, 3, 4]) != 10 { return 1 }
    if sum([42]) != 42 { return 2 }
    if sum([]) != 0 { return 3 }
    0
}
