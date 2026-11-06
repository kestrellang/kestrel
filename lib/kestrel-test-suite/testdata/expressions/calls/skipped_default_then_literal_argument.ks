// test: execution
// stdlib: true
// expect-stdout: 2 9 1 0\n

// Skipping a defaulted parameter must not shift later arguments onto it
// while type-checking. An array literal, an implicit member or an integer
// literal after a skipped default is checked against its own parameter, for
// free functions and for initializers alike. These used to be checked
// against the skipped parameter by position ("Int64 !:
// _ExpressibleByArrayLiteral").

module Test

enum Mode {
    case Fast
    case Slow
}

func count(first first: Int64, mode mode: Mode = .Fast, items items: [Int64] = [1]) -> Int64 {
    items.count
}

func pick(first first: Int64, mode mode: Mode = .Fast, value value: Int64? = .None) -> Int64 {
    value ?? 0
}

struct Options {
    var mode: Mode
    var items: [Int64]

    init(mode mode: Mode = .Fast, items items: [Int64] = [1, 2]) {
        self.mode = mode;
        self.items = items;
    }
}

@main
func main() -> Int64 {
    let skippedFunction = count(first: 0, items: [4, 5]);
    let skippedOptional = pick(first: 0, value: .Some(9));
    let skippedInit = Options(items: [7]);
    let empty = Options(items: []);
    println("\(skippedFunction) \(skippedOptional) \(skippedInit.items.count) \(empty.items.count)");
    0
}
