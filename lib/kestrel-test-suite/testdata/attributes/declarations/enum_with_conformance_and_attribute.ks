// test: diagnostics
// stdlib: false

module Test
protocol Printable {}

@dummy
enum Status: Printable {
    case Active
    case Inactive
}
