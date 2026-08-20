// test: execution
// stdlib: true
// expect-exit: 9

// G18 hazard guard for the `is_bool_struct` gate.
//
// `coerce_condition_to_i1` skips the `boolValue()` witness call for exactly one
// type: the `@builtin(.Bool)` entity. A user type that merely *looks* like it —
// same name, or a single `lang.i1` field — must NOT be skipped, or its
// `boolValue()` is silently ignored and the branch reads its payload bits.
//
// This pins the name collision: `Test.Bool` here is a different entity from
// `std.core.Bool`, so it goes through the witness. With the inverting
// `boolValue()` below, a gate that matched on the *name* would answer 7.

module Test

struct Bool: BooleanConditional {
    var v: lang.i64

    func boolValue() -> lang.i1 {
        lang.i64_eq(self.v, 0)
    }
}

// The other half of the hazard: a struct that is structurally identical to
// `std.core.Bool` (one `lang.i1` field) but whose `boolValue()` inverts it. A
// structural "single-field struct wrapping lang.i1" gate would skip this and
// answer 1.
struct MyFlag: BooleanConditional {
    var value: lang.i1

    func boolValue() -> lang.i1 {
        lang.i1_not(self.value)
    }
}

@main
func main() -> lang.i64 {
    let named = Bool(v: 200);
    let flag = MyFlag(value: true);
    let a: lang.i64 = if named { 7 } else { 9 };
    if flag { 1 } else { a }
}
