// test: execution
// stdlib: true
// expect-exit: 1

// Was a `diagnostics` test — see `custom_type_in_if_condition.ks` for why that
// could never catch G18. The enum shape matters independently of the struct
// one: `.Some(0)`'s payload bits are zero, so a raw branch on the value takes
// the FALSE arm even though `boolValue()` is true.

module Test
enum Option[T]: BooleanConditional {
    case Some(T)
    case None

    func boolValue() -> lang.i1 {
        match self {
            .Some(_) => true,
            .None => false
        }
    }
}
func test(opt: Option[lang.i64]) -> lang.i64 {
    if opt {
        1
    } else {
        0
    }
}

@main
func main() -> lang.i64 {
    // `some` is a reserved keyword (opaque types), hence the names.
    let present: Option[lang.i64] = .Some(0);
    let absent: Option[lang.i64] = .None;
    lang.i64_add(test(present), test(absent))
}
