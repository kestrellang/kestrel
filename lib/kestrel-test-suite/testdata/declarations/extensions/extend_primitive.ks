// test: diagnostics
// stdlib: false
module Test
extend lang.i64 { // ERROR: cannot extend
    func doubled() -> lang.i64 { return lang.i64_mul(self, 2); }
}
