// test: execution
// stdlib: true
//
// Regression: closure containing a while loop crashed the Cranelift verifier
// because the while's desugared loop typed as Never instead of (), causing
// the thunk to emit `return 0` against a void signature.

module Test

func withCallback(body: () -> ()) {
    body();
}

@main
func main() -> lang.i64 {
    withCallback {
        var y: std.numeric.Int64 = 0;
        while y < 3 {
            y = y + 1;
        }
    };
    0
}
