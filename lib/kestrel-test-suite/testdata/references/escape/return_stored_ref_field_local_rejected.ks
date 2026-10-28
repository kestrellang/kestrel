// test: diagnostics
// stdlib: true

// The extracted-ref return is root-checked like any other: a stored `&T`
// field extracted from a LOCAL-tainted aggregate (the wrapped ref borrows
// a function-local) roots Local and cannot be returned.
module Test

struct Cursor {
    var item: &Int64
}

func bad() -> &Int64 {
    var x = 42;
    let r = &x;
    var c = Cursor(item: r);
    c.item // ERROR(E494)
}
