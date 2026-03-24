// test: diagnostics
// stdlib: false
module Test
enum Status {
    case Active
    case Inactive
}

func test() -> Status {
    Status.Actve // ERROR: undefined name
}
