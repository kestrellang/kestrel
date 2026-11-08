// test: execution
// stdlib: true
// expect-exit: 0

// A hole binds to the nearest call; an argument placeholder binds to the
// nearest argument. A call can mix the two forms.

module Test

struct Group {
    let members: Array[Member]
}

struct Member {
    let active: Bool
}

func twice(f: (Int64) -> Int64, x: Int64) -> Int64 { f(f(x)) }
func inc(x: Int64) -> Int64 { x + 1 }
func pick(a: Int64, by f: (Int64) -> Int64) -> Int64 { f(a) }

@main
func main() -> lang.i64 {
    // `twice(inc(_), 3)`: the hole binds to `inc`, giving `{ p in inc(p) }`
    if twice(inc(_), 3) != 5 { return 1 }
    // nested argument placeholders each bind to their own argument
    let groups = [
        Group(members: [Member(active: true), Member(active: false)]),
        Group(members: [Member(active: true)]),
    ];
    let counts = groups.map(_.members.filter(where: _.active).count);
    if counts(0) != 1 or counts(1) != 1 { return 2 }
    // mixed: the second argument becomes a closure first, then the hole
    // makes the whole call a closure
    let f = pick(_, by: _ * 3);
    if f(4) != 12 { return 3 }
    0
}
