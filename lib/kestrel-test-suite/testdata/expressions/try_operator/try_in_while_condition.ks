// test: execution
// stdlib: true
// expect-exit: 0
//
// #203 (BUG-69): `try` must also be allowed in an unparenthesized `while`
// condition. Companion to try_in_if_condition.ks.

module Test

struct E: Formattable {
    func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.append("e");
    }
}

func below(n: Int64, limit: Int64) -> Bool throws E {
    return .Ok(n < limit);
}

@main
func main() -> () throws E {
    var i = 0;
    while try below(i, 3) {
        i = i + 1;
    }
    if i != 3 {
        fatalError("expected i=3");
    }
    .Ok(())
}
