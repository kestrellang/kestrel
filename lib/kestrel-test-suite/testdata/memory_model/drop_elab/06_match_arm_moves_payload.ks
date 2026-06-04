// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a `match` on an *owned* (`consuming`) scrutinee binds an enum
// payload and moves it into a consuming call. The arm must move the payload
// OUT of the scrutinee (`destructure_enum` + `move_value`), not copy it — the
// payload is non-Copyable. The `.empty` arm has no binding, so the dataflow
// joins a moved-out path with a non-moved one.
//
// The borrowed-scrutinee counterpart (which is rejected with E503) lives in
// `copy_semantics/move_out_of_borrowed_match_payload.ks`.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

enum Opt: not Copyable {
    case value(h: Handle)
    case empty
}

func consume(consuming h: Handle) {}

func example(consuming o: Opt) {
    match o {
        .value(h: payload) => consume(payload),
        .empty => {}
    }
}
