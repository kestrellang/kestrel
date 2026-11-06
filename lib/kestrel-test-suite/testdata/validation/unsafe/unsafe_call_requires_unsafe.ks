// test: diagnostics
// stdlib: false
//
// Using an `unsafe func` needs an unsafe context: the operand of an `unsafe`
// prefix or the body of an `unsafe { … }` block. Naming it as a value counts
// as a use. The rule is lexical, so a closure inside an unsafe region is
// covered and one outside is not.

module Test

unsafe func poke() -> lang.i64 { 1 }

func plain() -> lang.i64 {
    poke() // ERROR: 'poke' is unsafe and requires an `unsafe` expression or block
}

func prefixed() -> lang.i64 {
    unsafe poke()
}

func blocked() -> lang.i64 {
    let n = unsafe {
        let a = poke();
        a
    };
    n
}

func asValue() -> lang.i64 {
    let f = poke; // ERROR: 'poke' is unsafe and requires an `unsafe` expression or block
    let g = unsafe poke;
    g()
}

func closureInsideBlock() -> lang.i64 {
    let f = unsafe { { poke() } };
    f()
}

func closureOutside() -> lang.i64 {
    let f = { poke() }; // ERROR: 'poke' is unsafe and requires an `unsafe` expression or block
    f()
}
