// test: diagnostics
// stdlib: false

module Test

@deprecated
struct OldPoint {
    init() {}
}

func body() {
    let p: OldPoint = OldPoint(); // WARN: 'OldPoint' is deprecated
    let f = { (q: OldPoint) in q }; // WARN: 'OldPoint' is deprecated
}
