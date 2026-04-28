// test: execution
// stdlib: true

module Test

struct Data {
    let value: std.numeric.Int64
}

enum Container {
    case Full(data: Data, extra: std.numeric.Int64)
    case Empty
}

func extract_sum(c: Container) -> std.numeric.Int64 {
    match c {
        .Full(data: d, extra: e) => d.value + e,
        .Empty => 0
    }
}

func main() -> lang.i64 {
    let container = Container.Full(data: Data(value: 30), extra: 12);
    if extract_sum(container) != 42 { return 1 }
    0
}
