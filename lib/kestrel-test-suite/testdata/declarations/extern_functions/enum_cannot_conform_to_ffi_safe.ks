// test: diagnostics
// stdlib: true

module Test

enum MyEnum: FFISafe { // ERROR: cannot conform
    case A
    case B
}
