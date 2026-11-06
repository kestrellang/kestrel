// test: diagnostics
// stdlib: true

module Test

struct NotFFISafe {
    let value: lang.i64
}

struct BadStruct: FFISafe { // ERROR: do not
    let name: NotFFISafe
}
