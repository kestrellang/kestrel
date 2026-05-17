// test: diagnostics
// stdlib: false

module Test
func null_unicode() -> lang.i32 { '\u{0}' }
func letter_a() -> lang.i32 { '\u{41}' }
func emoji() -> lang.i32 { '\u{1F600}' }
func max_unicode() -> lang.i32 { '\u{10FFFF}' }
