// test: execution
// stdlib: true

module Test

enum Color {
    case Red
    case Green
    case Blue
}

func is_blue(c: Color) -> std.core.Bool {
    match c {
        .Blue => true,
        _ => false
    }
}

@main
func main() -> lang.i64 {
    if is_blue(Color.Blue) == false { return 1 }
    if is_blue(Color.Red) { return 2 }
    0
}
