// test: execution
// stdlib: true
// expect-exit: 5

// Back-compat (#109): a raw `lang.iN` primitive return is still accepted and
// becomes the exit code directly (sign-extended), without going through
// `Exitable`.

module Main

@main
func main() -> lang.i64 { 5 }
