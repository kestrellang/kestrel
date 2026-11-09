// test: diagnostics
// stdlib: true

// `Dictionary[K, V] where K: Hashable`: a key type that isn't Hashable is
// rejected where the dictionary type is written, not later inside
// dictionary.ks after monomorphization.

module main

struct Point {
    let x: Int64
}

func literal() {
    var d: [Point: Int64] = [:]; // ERROR: Point !: Hashable
}

func constructed() {
    let d = Dictionary[Point, Int64](); // ERROR: Point !: Hashable
}

func tupleKey() {
    var d: [(Point, Int64): Int64] = [:]; // ERROR: (Point, Int64) !: Hashable
}

@main
func main() {}
