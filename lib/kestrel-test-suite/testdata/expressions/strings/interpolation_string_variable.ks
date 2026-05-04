// test: execution
// stdlib: true
// expect-exit: 0

module Test

func main() -> lang.i64 {
    let name = "World";
    let result = "Hello, \(name)!";
    if result != "Hello, World!" { return 1 }

    let empty = "";
    let with_empty = "before\(empty)after";
    if with_empty != "beforeafter" { return 2 }

    0
}
