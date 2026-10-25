// test: diagnostics
// stdlib: false
// include: _http_lib.ks

module Test

// The '_' arm is reachable: a later version of 'Http' may add cases.
func name(m: Http.Method) -> lang.i64 {
    match m {
        .Get => 1,
        .Post => 2,
        _ => 0,
    }
}

func binding(m: Http.Method) -> lang.i64 {
    match m {
        .Get => 1,
        other => 0,
    }
}
