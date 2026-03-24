// test: diagnostics
// stdlib: false

module Test

struct Config[A, B = lang.i64, C = lang.str] { }
type SimpleConfig = Config[lang.i1];
type CustomConfig = Config[lang.i1, lang.f64];
