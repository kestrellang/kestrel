// test: diagnostics
// stdlib: false
module Test
struct Immutable {
    let x: lang.i64
    let y: lang.i64
}

struct Mixed {
    let id: lang.i64
    var value: lang.i64
}

func makeImmutable() -> Immutable {
    Immutable(x: 1, y: 2)
}

func makeMixed() -> Mixed {
    Mixed(id: 1, value: 2)
}
