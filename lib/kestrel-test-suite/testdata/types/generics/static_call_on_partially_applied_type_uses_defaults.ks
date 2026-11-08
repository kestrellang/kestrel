// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 1 7 1 65536\n

// A static call on a type named with only some args (`Pair[Bool].make()`)
// leaves the trailing defaulted params to inference: an argument can pin
// one (the arena below), and otherwise it takes the declared default.
module Test

import std.memory.(ArenaAllocator)

struct Pair[T, U = Int64] { var n: U }

extend Pair[T, U] {
    static func one() -> Int64 { 1 }
    static func holding(n: U) -> Pair[T, U] { Pair[T, U](n: n) }
}

extend Array[T, A] {
    static func using(a: A) -> Array[T, A] { Array[T, A](allocator: a) }
}

@main
func main() {
    let p = Pair[Bool].holding(7);
    var arena = ArenaAllocator();
    var xs = Array[Int64].using(arena);
    xs.append(3);
    print("\(Pair[Bool].one()) \(p.n) \(xs.count) \(arena.reservedBytes)\n");
}
