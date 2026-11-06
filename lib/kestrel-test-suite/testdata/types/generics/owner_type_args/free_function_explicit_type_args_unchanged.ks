// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A free generic function has no owner: `identity[Int64](3)` keeps binding its
// own params from the flat bracket (the owner-arg change must not touch it).

func identity[T](x: T) -> T { x }

@main
func main() -> Int64 {
    if identity[Int64](3) != 3 { return 1 }
    if identity("a") != "a" { return 2 }
    0
}
