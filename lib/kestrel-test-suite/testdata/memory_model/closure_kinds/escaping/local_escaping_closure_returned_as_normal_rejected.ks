// test: diagnostics

// A normal closure made from an `escaping` one is a view into its box, so it
// cannot outlive the local that owns the box. Returning it used to be
// accepted, and the caller called a closure whose box was already freed.

module Test

func f() -> (Int64) -> Int64 {
    let k = 3;
    let g: escaping (Int64) -> Int64 = { (x) in x + k };
    g // ERROR(E494)
}
