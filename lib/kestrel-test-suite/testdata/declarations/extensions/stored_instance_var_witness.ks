// test: execution
// stdlib: true
// expect-exit: 0
//
// Stored INSTANCE var witnessing a `var { get set }` protocol property,
// dispatched through a type parameter. A stored var has no accessor function, so
// witness lowering synthesizes an instance getter (clone `self.field`) and
// setter (`self.field = value`). Before the fix the witness bound no methods →
// post-mono "type 'C' does not implement 'x'". (Generalizes the static-var
// case to instance fields.)

module Test

protocol HasX {
    var x: Int64 { get set }
}

struct C: HasX {
    var x: Int64;
}

// read a stored instance witness through a type parameter
func peek[T](t: T) -> Int64 where T: HasX { t.x }

// read+write a stored instance witness through a type parameter
func bump[T](t: T) -> Int64 where T: HasX {
    var u = t;
    u.x = u.x + 1;
    u.x
}

@main
func main() -> lang.i32 {
    if peek(C(x: 7)) != 7 { return 1 }  // synthesized instance getter
    if bump(C(x: 7)) != 8 { return 2 }  // synthesized instance setter
    0
}
