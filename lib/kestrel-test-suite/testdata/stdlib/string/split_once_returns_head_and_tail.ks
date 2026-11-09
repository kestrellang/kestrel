// test: execution
// stdlib: true
// expect-stdout: port|80=1\nnone\nx|y\n|rest\n

// `split(once:)` splits at the first separator into (before, after), or
// gives `.None` when the separator doesn't occur. Offsets are relative to
// the slice, so it works on a trimmed slice too.

module Test

func show(text: StringSlice) {
    match text.split(once: "=") {
        .Some((key, value)) => println("\(key)|\(value)"),
        .None => println("none"),
    }
}

@main
func main() {
    show("port=80=1".asSlice());
    show("no-equals".asSlice());
    show("  x=y  ".trimmed());
    show("=rest".asSlice());
}
