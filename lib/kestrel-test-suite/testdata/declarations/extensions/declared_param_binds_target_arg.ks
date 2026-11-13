// test: execution
// stdlib: true
// expect-exit: 0

// A declared extension param written as the target's own arg is that
// target param, exactly as the undeclared spelling: `extend[T] Holder[T]`
// means `extend Holder[T]`. It used to become a second, unbound param, so
// monomorphization failed ("type arg arity mismatch for Holder.p: expected
// 2, got 1").

module Test

protocol P {
    func p() -> Int64
}

struct Holder[T] {
    var value: T
}

extend[T] Holder[T]: P {
    func p() -> Int64 { 7 }
}

extend[T] Holder[T] {
    func unwrap() -> T { self.value }
}

func use[X](x: X) -> Int64 where X: P {
    x.p()
}

@main
func main() -> Int64 {
    if use(Holder(value: 5)) != 7 { return 1 }
    if Holder(value: true).p() != 7 { return 2 }
    if Holder(value: 9).unwrap() != 9 { return 3 }
    0
}
