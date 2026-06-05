// test: execution
// stdlib: true

module Test

struct Data {
    var value: std.numeric.Int64
}

enum Container {
    case Full(data: Data)
    case Empty
}

func extract(c: Container) -> std.numeric.Int64 {
    match c {
        .Full(data: d) => d.value,
        .Empty => 0
    }
}

@main
func main() -> lang.i64 {
    var container = Container.Full(data: Data(value: 10));
    if extract(container) != 10 { return 1 }

    // Reassign the entire enum with a new struct
    container = Container.Full(data: Data(value: 42));
    if extract(container) != 42 { return 2 }

    0
}
