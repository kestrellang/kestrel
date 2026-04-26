// test: diagnostics

module Test

func test() -> lang.i64 {
    // This is a comment
    42;
    [1, /* comment */ 2, 3];
    /* outer /* inner */ still outer */
    42
}
