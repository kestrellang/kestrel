// test: diagnostics
// stdlib: false

module Test

enum Recursive { // ERROR: recursive enum requires `indirect`
    case Base
    case Next(value: Recursive)
}
