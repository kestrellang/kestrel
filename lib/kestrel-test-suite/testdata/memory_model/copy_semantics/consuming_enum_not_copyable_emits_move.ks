// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

enum State: not Copyable {
    case Open
    case Closed
}

func consume(consuming s: State) {}

func test() {
    let state = State.Open;
    consume(state)
}
