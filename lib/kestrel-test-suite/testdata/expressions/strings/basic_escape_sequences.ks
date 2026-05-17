// test: diagnostics
// stdlib: false

module Main

func testNewline() -> lang.str { "hello\nworld" }
func testTab() -> lang.str { "hello\tworld" }
func testBackslash() -> lang.str { "hello\\world" }
func testDoubleQuote() -> lang.str { "hello\"world" }
func testNullChar() -> lang.str { "hello\0world" }
