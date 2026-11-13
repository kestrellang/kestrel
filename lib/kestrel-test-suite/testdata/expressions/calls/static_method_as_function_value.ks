// test: execution
// stdlib: true
// expect-exit: 0
//
// A static method named through its type is a plain function value: it can be
// bound with `let` and passed where a function type is expected (`on(Get(), Notes.read)`).

module Test

struct Notes {
    static func read(ctx: Int64, id: Int64) -> Int64 { ctx + id }
}

func call(handler: (Int64, Int64) -> Int64) -> Int64 { handler(40, 2) }

@main
func main() -> Int64 {
    let f = Notes.read;
    let direct = f(1, 2);           // 3
    let passed = call(Notes.read);  // 42
    direct + passed - 45
}
