// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

struct NotFFISafe {
    let value: lang.i64
}

@extern(.C)
func badParam(s: NotFFISafe) -> MyInt // ERROR: FFISafe
