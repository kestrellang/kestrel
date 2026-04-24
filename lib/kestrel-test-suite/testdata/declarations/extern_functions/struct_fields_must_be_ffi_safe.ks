// test: diagnostics
// stdlib: true

module Test
import Prelude

struct NotFFISafe {
    let value: lang.i64
}

struct BadStruct: FFISafe { // ERROR: do not
    let name: NotFFISafe
}
