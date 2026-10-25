// test: execution
// stdlib: false
// include: _http_lib.ks
// backends: cranelift,llvm
// expect-exit: 0

module Test

func name(m: Http.Method) -> lang.i64 {
    match m {
        .Get => 10,
        .Post => 20,
        _ => 30,
    }
}

@main
func main() -> lang.i64 {
    let a = name(Http.Method.Post);
    let b = Http.code(Http.Method.Get);
    if lang.i64_eq(a, 20) { if lang.i64_eq(b, 1) { 0 } else { 2 } } else { 1 }
}
