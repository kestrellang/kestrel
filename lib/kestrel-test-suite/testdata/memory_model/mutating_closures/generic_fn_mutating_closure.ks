// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

struct Box { var v: Int64 }

// Generic higher-order fn: the convention survives type-arg substitution.
func transform[T](mutating value: T, by f: (mutating T) -> Int64) -> Int64 {
    f(value)
}

@main
func main() -> lang.i64 {
    var b = Box(v: 7);
    let r = transform(b, by: { (mutating x) in x.v = x.v * 2; x.v });
    if b.v != 14 { return 1 }
    if r != 14 { return 2 }
    0
}
