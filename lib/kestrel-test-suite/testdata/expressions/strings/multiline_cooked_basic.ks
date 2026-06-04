// test: execution
// stdlib: true
// expect-exit: 0

module Test

@main
func main() -> lang.i64 {
    // Multi-line cooked: indent strip + escape decoding.
    let s = """
        hello
          world
        """;
    if s != "hello\n  world" { return 1 }

    // Escapes work: `\n` becomes a real newline.
    let e = """
        a\nb
        """;
    if e != "a\nb" { return 2 }

    0
}
