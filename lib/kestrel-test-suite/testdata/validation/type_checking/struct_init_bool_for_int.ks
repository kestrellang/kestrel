// test: diagnostics
// stdlib: false

module Main

struct Config {
    var count: lang.i64
    var enabled: lang.i1
}

func test() {
    let c: Config = Config(count: true, enabled: 42); // ERROR
}
