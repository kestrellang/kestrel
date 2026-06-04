// test: execution
// stdlib: true

module Test

// A unique, owning resource — never bit-copyable.
struct Res: not Copyable {
    var fd: lang.i64
}

@main
func main() -> lang.i64 {
    // `Layout.of` only reads `sizeof`/`alignof` of the type — it never
    // materializes or copies a `Res` value, so its Copyable-ness is
    // irrelevant. This must compile and run.
    let layout = std.memory.Layout.of[Res]();
    if layout.size != 8 { return 1 }
    if layout.alignment != 8 { return 2 }
    0
}
