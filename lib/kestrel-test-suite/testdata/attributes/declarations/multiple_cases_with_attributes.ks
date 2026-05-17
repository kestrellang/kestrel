// test: diagnostics
// stdlib: false

module Test
enum Status {
    @dummy
    case Active
    @dummy
    case Pending
    case Inactive
}
