// test: diagnostics
// stdlib: false

module Main

// Called on the type, with labels no overload of `bind` takes. This is a
// label mismatch, not use of a static member on an instance.

struct Listener {
    static func bind(port: lang.i64) -> Listener { Listener() }
    static func bind(anyInterface port: lang.i64) -> Listener { Listener() }
}

func f() {
    let a = Listener.bind(port: 0); // ERROR: no matching overload for 'bind'
}
