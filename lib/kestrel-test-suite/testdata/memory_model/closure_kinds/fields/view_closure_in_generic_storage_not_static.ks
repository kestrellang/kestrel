// test: diagnostics
// stdlib: true

// A view-kind closure is not `Static` (references stage 2c,
// docs/design/closure-fields.md), so generic storage that requires `Static`
// (`Array`) rejects it; an `escaping` element type is accepted and the literal
// snapshots its captures.
module Test

func viewElements() -> Int64 {
    var hs: [(Int64) -> Int64] = []; // ERROR(E107)
    hs.count
}

func owningElements() -> Int64 {
    let k: Int64 = 5;
    var hs: [escaping (Int64) -> Int64] = [];
    hs.append({ it + k });
    hs(0)(1)
}
