// test: execution
// stdlib: true
// expect-exit: 0
//
// A String-valued `if` expression used directly as an array literal element.
// Found in the website's html-builder guide (`anchor([href(if isNew { "/" }
// else { "/note/" + id })])`): OSSA verify ICE "use of consumed value". An
// Int64 element, or hoisting the `if` into a `let`, both compile.

module Test

@main
func main() -> lang.i64 {
    let isNew = false;
    let id = "7";
    let a = [if isNew { "/" } else { id }];
    if a(0) != "7" { return 1 }
    let b = [if isNew { "/" } else { "/note/" + id }, "x"];
    if b(0) != "/note/7" { return 2 }
    0
}
