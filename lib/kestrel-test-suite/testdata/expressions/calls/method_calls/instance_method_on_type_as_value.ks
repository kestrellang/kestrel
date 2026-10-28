// test: diagnostics
// stdlib: false

// An instance method named through its *type* has no receiver to bind, so it
// cannot become a function value. Name resolution's direct-children walk hands
// back the method entity regardless (its `is_static_method` filter guards only
// the extension fallback), and before this was rejected MIR built an
// `apply_partial` over a two-parameter thunk behind a one-parameter thick type
// — a silent miscompile. Fragility audit G3 §2b.

module Main

struct Box {
    let v: lang.i64
    func doubled(x: lang.i64) -> lang.i64 { 0 }
}

enum Choice {
    case a
    case b
    func pick(x: lang.i64) -> lang.i64 { x }
}

protocol Hopper {
    func hop(x: lang.i64) -> lang.i64
}

func test() -> () {
    let f = Box.doubled;    // ERROR: instance method 'doubled' cannot be used as a value
    let g = Choice.pick;    // ERROR: instance method 'pick' cannot be used as a value
    let h = Hopper.hop;     // ERROR: instance method 'hop' cannot be used as a value
}
