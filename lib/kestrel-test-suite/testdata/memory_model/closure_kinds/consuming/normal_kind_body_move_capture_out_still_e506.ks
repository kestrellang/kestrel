// test: diagnostics
// stdlib: true

// The negative half of consuming_body_moves_capture_out.ks. Per
// docs/design/closures.md, E506 is "kept for normal/`mutating`/`escaping`
// bodies; lifted inside `consuming` bodies" — the very same literal that is
// legal against a `consuming` expected type is rejected against a normal one.
module Test

import std.numeric.Int64

struct Res: not Copyable {
    var id: Int64
    deinit { }
}

// Normal kind: many-call, frame views — moving the capture out would duplicate.
func runNormal(f: () -> Res) -> Int64 { f().id }

func main() -> lang.i64 {
    let a = Res(id: 7);
    let got = runNormal({ () in a }); // ERROR(E506)
    let _ = got;
    0
}
