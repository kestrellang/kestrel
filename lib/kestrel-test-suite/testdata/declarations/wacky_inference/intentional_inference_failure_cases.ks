// test: diagnostics
// stdlib: false

module Test

protocol P { type A; func read() -> A }

struct S[T] { var val: T }

extend S[T] where T: P, T.A = lang.i64 {
    func fail_it() -> lang.str {
        return self.val.read(); // ERROR: type mismatch
    }
}
