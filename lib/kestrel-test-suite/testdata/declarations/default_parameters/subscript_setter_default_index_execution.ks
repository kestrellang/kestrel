// test: execution
// stdlib: true
// expect-exit: 0

// Regression (#151/#149): a subscript SETTER with a defaulted index, called with
// the index omitted (`c() = v`), built its call-arg list manually and never
// materialized the default — emitting 2 args against the 3-param setter, which
// failed codegen verification. The function was then skipped (downgraded to a
// warning) and replaced with a trap stub, so a clean-looking build SIGILL'd at
// runtime. The getter/read path already worked (it routes through standard call
// lowering that expands defaults). The setter now fills omitted index defaults
// via `expand_default_args` before the trailing `newValue`.

module Test

import std.numeric.Int64

struct C {
    var v: Int64
    subscript(i: Int64 = 0) -> Int64 {
        get { self.v + i }
        set { self.v = newValue - i; }
    }
}

@main
func main() -> lang.i64 {
    var c = C(v: 1);

    // defaulted index omitted on the SETTER (the #151/#149 crash): i defaults to
    // 0, so set computes v = 10 - 0 = 10.
    c() = 10;
    if c.v != 10 { return 1 }

    // explicit index on the setter (worked before the fix): v = 20 - 5 = 15.
    c(5) = 20;
    if c.v != 15 { return 2 }

    // defaulted index omitted on the GETTER (read path, worked before): v + 0.
    if c() != 15 { return 3 }

    // explicit index on the getter: v + 7.
    if c(7) != 22 { return 4 }

    0
}
