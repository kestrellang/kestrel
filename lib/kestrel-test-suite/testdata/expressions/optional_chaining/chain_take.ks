// test: execution
// stdlib: true
// expect-exit: 0

// A chain on a temporary calls `chainTake`: the payload is owned, so a
// consuming method or a mutation runs on it directly.

module Test

struct Token: not Copyable {
    var id: Int64
    consuming func spend() -> Int64 { self.id }
    mutating func bump() -> Int64 { self.id = self.id + 1; self.id }
}

func load(id: Int64) -> Token? {
    if id < 0 { return .None }
    .Some(Token(id: id))
}

@main
func main() -> lang.i64 {
    if load(4)?.spend() != .Some(4) { return 1 }
    if load(-1)?.spend() != .None { return 2 }
    if load(4)?.bump() != .Some(5) { return 3 }
    0
}
