// test: diagnostics
// stdlib: true

module Test
import Prelude

enum MyEnum: FFISafe { // ERROR: cannot conform
    case A
    case B
}
