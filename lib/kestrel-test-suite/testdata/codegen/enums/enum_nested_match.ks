// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func add_options(a: Option, b: Option) -> std.num.Int64 {
    match a {
        .Some(value: x) => {
            match b {
                .Some(value: y) => x + y,
                .None => x
            }
        },
        .None => {
            match b {
                .Some(value: y) => y,
                .None => 0
            }
        }
    }
}

func main() -> lang.i64 {
    let a = Option.Some(value: 20);
    let b = Option.Some(value: 22);
    if add_options(a, b) != 42 { return 1 }
    0
}
