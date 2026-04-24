// test: execution
// stdlib: true

module Test

enum Color {
    case Red
    case Green
    case Blue
}

func color_value(c: Color) -> std.num.Int64 {
    match c {
        .Red => 1,
        .Green => 2,
        .Blue => 42
    }
}

func main() -> lang.i64 {
    if color_value(Color.Blue) != 42 { return 1 }
    0
}
