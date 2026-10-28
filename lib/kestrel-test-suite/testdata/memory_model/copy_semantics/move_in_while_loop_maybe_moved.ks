// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) {}

func test(cond: lang.i1) {
    var h = Handle(fd: 42);
    while cond {
        consume(h) // ERROR: may have been moved
    }
    // #163: the in-loop back-edge re-use above is the earlier of the two re-uses
    // and is now flagged; the post-loop use shares the one-error-per-variable
    // slot, so it stays silent.
    consume(h)
}
