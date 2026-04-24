// test: diagnostics
// stdlib: false

module Test
struct Config {
    @dummy(default: 42)
    var timeout: lang.i64
}
