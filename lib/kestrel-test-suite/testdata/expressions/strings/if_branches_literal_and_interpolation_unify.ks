// test: execution
// stdlib: true
// expect-exit: 0
//
// An unannotated `if` whose branches are a plain string literal and an
// interpolated string is a String: both literal kinds default to String.
// (Found porting notes-frontend: E109 "expected string literal got string
// interpolation", whichever branch comes first.)

module Test

@main
func main() -> lang.i64 {
    let id: lang.i64 = 7;
    let isNew = false;
    let a = if isNew { "/notes" } else { "/notes/\(id)" };
    if a != "/notes/7" { return 1 }
    let b = if isNew { "/notes/\(id)" } else { "/notes" };
    if b != "/notes" { return 2 }
    0
}
