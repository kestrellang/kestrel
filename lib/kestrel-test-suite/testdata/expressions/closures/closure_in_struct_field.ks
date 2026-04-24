// test: diagnostics
// stdlib: false

module Main

struct Callback {
    let action: () -> lang.i64
}

func test() -> lang.i64 {
    let cb = Callback(action: { 42 });
    (cb.action)()
}
