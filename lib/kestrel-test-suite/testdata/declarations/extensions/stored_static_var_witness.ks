// test: execution
// stdlib: true
// expect-exit: 0
//
// #147: a stored `static var` witnessing a `static var { get set }` protocol
// requirement must be dispatchable through a type parameter. A stored static
// var has no accessor function, so witness lowering synthesizes a getter
// (clones the global) and a setter (stores into it). Before the fix, the witness
// had no methods bound → post-mono "type 'C' does not implement 'count'".

module Test

protocol Counter {
    static var count: Int64 { get set }
}

struct C: Counter {
    static var count: Int64 = 0;
}

struct D: Counter {
    static var count: Int64 = 100;
}

// read a stored-static witness through a type parameter
func peek[T]() -> Int64 where T: Counter { T.count }

// write a stored-static witness through a type parameter
func bump[T]() where T: Counter { T.count = T.count + 1 }

@main
func main() -> lang.i32 {
    if peek[C]() != 0 { return 1 }     // synthesized getter
    if peek[D]() != 100 { return 2 }   // distinct conformer
    bump[C]();                          // synthesized setter
    bump[C]();
    if peek[C]() != 2 { return 3 }
    if peek[D]() != 100 { return 4 }   // D untouched
    0
}
