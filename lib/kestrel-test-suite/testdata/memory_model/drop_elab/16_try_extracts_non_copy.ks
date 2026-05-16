// test: diagnostics
// stdlib: true

// Regression: when bb0's statements only touch non-tracked locals
// (Result/ControlFlow temps were classified as CopyBehavior::Bitwise),
// the dataflow propagation guard `if state != blocks[bi].exit` saw
// bb0's empty exit match the default empty exit and never seeded the
// successors — leaving subsequent `move`s of a tracked local reach a
// default-uninit entry and trip a false E500.

module Test

public struct Thing: not Copyable {
    var n: Int64

    public init(n n: Int64) {
        self.n = n
    }

    deinit {
    }
}

func mk() -> Result[Thing, Int64] {
    .Ok(Thing(n: 1))
}

func consume(t: Thing) -> Int64 {
    t.n
}

public func use() -> Result[Int64, Int64] {
    var thing = try mk();
    let r = consume(thing);
    .Ok(r)
}
