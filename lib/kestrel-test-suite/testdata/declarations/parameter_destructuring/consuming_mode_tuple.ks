// test: diagnostics
// stdlib: false

module Main

struct Resource {
    var value: lang.i64
}

func consume(consuming (a, b): (Resource, Resource)) -> lang.i64 {
    // a and b are owned and mutable
    a.value = 100;
    lang.i64_add(a.value, b.value)
}

func test() -> lang.i64 {
    let r1 = Resource(value: 1);
    let r2 = Resource(value: 2);
    consume((r1, r2))
}
