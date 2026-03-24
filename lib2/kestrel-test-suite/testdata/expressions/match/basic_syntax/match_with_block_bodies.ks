// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> lang.i64 {
    match c {
        .Red => {
            let x = 1;
            x
        },
        .Green => {
            let y = 2;
            y
        },
        .Blue => 3
    }
}
