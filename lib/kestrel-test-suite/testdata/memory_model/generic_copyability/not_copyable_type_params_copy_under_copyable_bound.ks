// test: execution
// stdlib: true

// A `where T: Copyable` on an extension of a not-Copyable type narrows its
// param back to Copyable, so the payload can be copied there.

module Test

enum Box[T]: not Copyable {
    case Full(T)
    case Empty
}

extend Box[T] where T: Copyable {
    func twice() -> (T, T) {
        match self {
            .Full(x) => (x, x),
            .Empty => fatalError("empty")
        }
    }
}

@main
func main() -> lang.i64 {
    let b: Box[Int64] = .Full(21);
    let (x, y) = b.twice();
    if x + y == 42 { 0 } else { 1 }
}
