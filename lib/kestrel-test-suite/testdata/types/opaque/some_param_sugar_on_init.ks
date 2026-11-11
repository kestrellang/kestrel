// test: execution
// stdlib: true
// expect-stdout: 1 2 1 2\n5 5 5\n

module Test

// `some P` on an initializer parameter desugars to a synthetic type
// parameter, the same as on a function. It used to stay a bare `some` type,
// so the body couldn't look up members (E106) and calls failed.
struct Unlabeled {
    var n: Int64
    init(text: some Formattable) { self.n = if text.formatted().isEmpty { 0 } else { 1 } }
}

struct Labeled {
    var n: Int64
    init(text text: some Formattable) { self.n = if text.formatted().isEmpty { 0 } else { 2 } }
}

struct AcceptsSlices {
    var n: Int64
    init(text: some Formattable and not Static) { self.n = if text.formatted().isEmpty { 0 } else { 5 } }
}

@main
func main() {
    let s = "hello";
    print("\(Unlabeled(s).n) \(Labeled(text: s).n) \(Unlabeled("lit").n) \(Labeled(text: "lit").n)\n");
    var line = "";
    for part in "a b".split(by: " ") { line.append("\(AcceptsSlices(part).n) ") }
    print("\(line)\(AcceptsSlices("lit").n)\n");
}
