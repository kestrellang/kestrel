// test: execution
// stdlib: true
// expect-stdout: found 2\nhas 3\nno 9\ni=3\n10\n20\n30\nfalse\nok\n

// In an `if`/`while`/`match`/`for` head, a `{ … }` is a trailing closure when
// the body's `{` still follows on the same line; otherwise it is the body.
module Test

@main
func main() -> Int64 {
    let xs = [1, 2, 3];
    match xs.find { it > 1 } {
        .Some(v) => println("found \(v)"),
        .None => println("none"),
    }
    if xs.any { it == 3 } { println("has 3") }
    if xs.any { it == 9 } and true { println("bad") } else { println("no 9") }
    var i = 0;
    while xs.any { it > i } { i += 1 }
    println("i=\(i)");
    for x in xs.iter().map { it * 10 } { println("\(x)") }
    // A bare scrutinee's `{` stays the match body, even with `.member` after it.
    let k = 2;
    let s = match k { 1 => "one", _ => "other" }.isEmpty;
    println("\(s)");
    let ok = true;
    if ok { println("ok") } else { println("no") }
    0
}
