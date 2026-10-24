// test: execution
// stdlib: false
// backends: cranelift,llvm
//
// `@layout(.C)` on a payload-less enum makes it a C `int`: four bytes whose
// values are the case indices.

module Main

@layout(.C)
enum Mode {
    case Read
    case Write
    case Append
}

func code(m: Mode) -> lang.i64 {
    match m {
        .Read => 10,
        .Write => 20,
        .Append => 30,
    }
}

@main
func main() -> lang.i64 {
    if lang.i64_ne(lang.sizeof[Mode](), 4) { return 1; }
    if lang.i64_ne(lang.alignof[Mode](), 4) { return 2; }
    if lang.i64_ne(code(.Append), 30) { return 3; }
    if lang.i64_ne(code(.Read), 10) { return 4; }
    0
}
