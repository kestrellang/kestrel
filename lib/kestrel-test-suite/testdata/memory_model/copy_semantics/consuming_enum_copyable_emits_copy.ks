// test: diagnostics
// stdlib: false

module Test

enum Status {
    case Active
    case Inactive
}

func consume(consuming s: Status) {}

func test() {
    let status = Status.Active;
    consume(status)
}
