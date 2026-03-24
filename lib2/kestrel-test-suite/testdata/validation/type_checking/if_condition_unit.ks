// test: diagnostics
// stdlib: true

module Main

func test() {
    let u: () = ();
    if u { // ERROR
        let x: lang.i64 = 1;
    }
}
