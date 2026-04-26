// test: diagnostics
// stdlib: false

module Main

func getTuple() -> (lang.i64, lang.str) {
    return (42, "hello");
}

func test() {
    let x: lang.i64 = getTuple().0;
    let y: lang.str = getTuple().1;
}
