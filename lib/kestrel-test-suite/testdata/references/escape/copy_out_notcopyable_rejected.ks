// test: diagnostics
// stdlib: false
// skip: stage1 — needs stage-1 front-end (S1)

// Copy-out of a NotCopyable pointee: binding a ref-typed call result
// decays (reads the place), and a `not Copyable` value cannot be read
// out — the existing copy guards fire, with ref-aware wording. This is
// the strongest pin that borrow contexts are classified correctly: a
// place context misclassified as value context fails HERE, at compile
// time, instead of as a silent clone.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Box {
    var r: Res
    func peek() -> &Res { self.r }
}

func use(b: Box) {
    let x = b.peek(); // ERROR
}
