// test: execution
// stdlib: true
// expect-exit: 0

module Test

// An unapplied static method named through an explicit owner is a function
// value carrying the owner's args: `let f = Box[Int64].make; f(3)`.

struct Box[T] {
    var value: T
    static func make(v: T) -> Box[T] { Box(value: v) }
}

@main
func main() -> Int64 {
    let f = Box[Int64].make;
    if f(3).value != 3 { return 1 }
    0
}
