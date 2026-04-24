// test: diagnostics
// stdlib: false

module Main

func mutate(mutating (a, b): (lang.i64, lang.i64)) {
    a = 10;  // OK: mutating mode makes bindings mutable
    b = 20;
}

func test() -> lang.i64 {
    var tuple = (1, 2);
    mutate(tuple);  // mutating is not a label at call site
    tuple.0
}
