// test: execution
// stdlib: true
// expect-exit: 0

// A target arg binds the target's param at its position, whatever the
// param is called: `extend[U] [U]` binds Array's element param as `U`, and
// `extend Pair[B, A]` binds Pair's first param as `B`. Matching by the
// param's own name made `[U]` bind nothing (post-mono verification failed)
// and would have bound `Pair[B, A]` crosswise.

module Test

protocol Tag {
    func tag() -> Int64
}

extend Int64: Tag {
    func tag() -> Int64 { self + 1 }
}

extend[U] [U] where U: Tag {
    func firstTag() -> Int64 { self(0).tag() }
}

struct Pair[A, B] {
    var a: A
    var b: B
}

extend Pair[B, A] {
    func first() -> B { self.a }
}

@main
func main() -> Int64 {
    let nums: [Int64] = [4];
    if nums.firstTag() != 5 { return 1 }
    let p = Pair(a: 3, b: true);
    if p.first() != 3 { return 2 }
    0
}
