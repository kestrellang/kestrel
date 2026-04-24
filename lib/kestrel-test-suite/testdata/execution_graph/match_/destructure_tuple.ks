// test: diagnostics
// stdlib: false

module Main

func sum(pair: (lang.i64, lang.i64)) -> lang.i64 {
    let (a, b) = pair;
    lang.i64_add(a, b)
}
