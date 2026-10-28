// test: execution
// stdlib: false
// expect-exit: 42

// Intrinsic `lang.*` types are extendable. An inherent method added to
// `lang.i64` resolves and runs against the primitive scalar `self`.
module Main

extend lang.i64 {
    func doubled() -> lang.i64 { return lang.i64_mul(self, 2); }
}

@main
func main() -> lang.i64 {
    let x: lang.i64 = 21;
    x.doubled()
}
