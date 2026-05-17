// test: execution
// stdlib: true

module Test

protocol HasValue {
    var value: std.numeric.Int64 { get }
}

struct Box: HasValue {
    var value: std.numeric.Int64 { get { 42 } }
}

func getValue[T](item: T) -> std.numeric.Int64 where T: HasValue {
    item.value
}

func main() -> std.numeric.Int64 {
    let b = Box();
    if getValue[Box](b) != 42 { return 1 }
    0
}
