// test: execution
// stdlib: true
// expect-exit: 0
//
// A struct pattern on an instance of a GENERIC struct binds each field at its
// substituted type: `let Wrap { value: p } = w` with `w: Wrap[Page]` makes
// `p: Page`. Today `p` is typed as the unsubstituted param `T` (E102 "no member
// 'page' on type 'T'"). Found 2026-11-01 building perch-2, whose handlers
// unpack extractors with `let Json { value: new } = body;`.

module Test

struct Wrap[T] { var value: T }
struct Page { var page: Int64 }

func unwrap(w: Wrap[Page]) -> Int64 {
    let Wrap { value: p } = w;
    p.page
}

@main
func main() -> Int64 {
    unwrap(Wrap(value: Page(page: 3))) - 3
}
