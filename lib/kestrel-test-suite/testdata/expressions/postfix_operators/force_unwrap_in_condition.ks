// test: execution
// stdlib: true

// Regression: postfix `!` force-unwrap must parse and work in `if`/`while`
// condition position. The simplified condition grammar previously omitted
// postfix-bang (it wired in call / member / tuple-index / `..` but not `!`),
// so `if opt! > 0 { … }` failed to parse with "expected ..., found '!'".

module Test

@main
func main() -> lang.i64 {
    let opt: std.result.Optional[std.numeric.Int64] = .Some(5);

    // `!` mid-condition
    if opt! < 5 { return 1 }
    if opt! > 5 { return 2 }

    // `!` as the last token before the body block `{`
    let limit: std.result.Optional[std.numeric.Int64] = .Some(3);
    var count = 0;
    while count < limit! {
        count = count + 1;
    }
    if count != 3 { return 3 }

    0
}
