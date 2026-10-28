// test: diagnostics
// stdlib: true

// A normal-kind closure value holds *views* of its frame, so it can never fill
// an `escaping` parameter: row 1, column 4 of the passing table in
// docs/design/closures.md ("frame-bound"). The lazy builders store their
// callback, so every one of them rejects a frame-view closure value.
module Test

func mapRejectsFrameView() {
    var factor: Int64 = 2;
    let scale: (Int64) -> Int64 = { (x) in x * factor };
    let mapped = [1, 2, 3].iter().map(as: scale); // ERROR(E624)
}

func filterRejectsFrameView() {
    var threshold: Int64 = 2;
    let big: (Int64) -> Bool = { (x) in x > threshold };
    let filtered = [1, 2, 3].iter().filter(where: big); // ERROR(E624)
}

func splitWhereRejectsFrameView() {
    var sepChar: Char = ' ';
    let isSep: (Char) -> Bool = { (c) in c == sepChar };
    let text: String = "one two";
    let words = text.split(where: isSep); // ERROR(E624)
}
