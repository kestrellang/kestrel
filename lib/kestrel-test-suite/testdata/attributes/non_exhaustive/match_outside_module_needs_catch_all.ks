// test: diagnostics
// stdlib: false
// include: _http_lib.ks

module Test

func name(m: Http.Method) -> lang.i64 {
    match m { // ERROR: non-exhaustive match: missing _
        .Get => 1,
        .Post => 2,
    }
}
