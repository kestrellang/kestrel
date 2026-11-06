// test: execution
// stdlib: false
// expect-exit: 0

// Internal (no keyword) members stay usable anywhere in their own module.
module Main

struct Grid {
    var size: lang.i64
    subscript(index: lang.i64) -> lang.i64 { get { lang.i64_mul(index, self.size) } }
    func secret() -> lang.i64 { 7 }
}

@main
func main() -> lang.i64 {
    let g = Grid(size: 3);
    if lang.i1_not(lang.i64_eq(g(2), 6)) { return 1 }
    if lang.i1_not(lang.i64_eq(g.secret(), 7)) { return 2 }
    0
}
