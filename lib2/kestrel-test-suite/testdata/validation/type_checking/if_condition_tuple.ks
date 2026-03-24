// test: diagnostics
// stdlib: false

module Main

func test() {
    let t: (lang.i1, lang.i1) = (true, false);
    if t { // ERROR
        let x: lang.i64 = 1;
    }
}
