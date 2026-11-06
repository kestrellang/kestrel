// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Type sugar works as an extension target: `extend [T]` is `extend Array[T]`,
// `extend T?` is `extend Optional[T]`, and conditional conformances through
// sugar apply (they used to extend nothing).

module Test

protocol P { func f() -> Int64 }

extend [T]: P where T: P {
    func f() -> Int64 {
        var s = 0;
        for x in self { s = s + x.f(); }
        s
    }
}

extend Int64: P { func f() -> Int64 { self } }

extend [T] {
    func twiceCount() -> Int64 { self.count * 2 }
}

extend T? {
    func isPresent() -> Bool { if let .Some(_) = self { true } else { false } }
}

func run[T](x: T) -> Int64 where T: P { x.f() }

@main
func main() -> lang.i64 {
    if run([[1, 2], [3]]) != 6 { return 1 }
    if [1, 2, 3].twiceCount() != 6 { return 2 }
    let o: Optional[Int64] = .Some(1);
    if o.isPresent() == false { return 3 }
    0
}
