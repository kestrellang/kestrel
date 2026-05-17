// test: diagnostics
// stdlib: false

module Test
func newline() -> lang.str { "hello\nworld" }
func tab() -> lang.str { "hello\tworld" }
func quote() -> lang.str { "say \"hello\"" }
func backslash() -> lang.str { "path\\to\\file" }
