// test: diagnostics
// stdlib: false
module Test
enum Status {
    case Pending
    case Active
    case Complete
}

func test() {
    var status: Status = .Pending;
    status = .Active;
    status = .Complete;
}
