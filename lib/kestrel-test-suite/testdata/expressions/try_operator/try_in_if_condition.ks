// test: execution
// stdlib: true
// expect-exit: 0
//
// #203 (BUG-69): `try` must be allowed in an unparenthesized `if` condition.
// `if try check() { ... }` used to be a parse error ("expected 'let', '-', or
// 9 others, found 'try'") because `try` was missing from the restricted
// condition grammar, even though `if (try check())` parsed fine.

module Test

struct E: Formattable {
    func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.append("e");
    }
}

func check(b: Bool) -> Bool throws E {
    return .Ok(b);
}

@main
func main() -> () throws E {
    var hit = 0;
    if try check(true) {
        hit = hit + 1;
    }
    if try check(false) {
        hit = hit + 10;
    }
    if hit != 1 {
        fatalError("expected hit=1");
    }
    .Ok(())
}
