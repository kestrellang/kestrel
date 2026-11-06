// test: execution
// stdlib: false
// include: default_visibility_lib.ks
// expect-exit: 0

// Public members, protocol requirements and enum cases stay reachable from
// another module; only members without a keyword became internal.
module Main

import Lib.(Grid, Shape, Color)

func areaOf(s: Grid) -> lang.i64 { s.area() }

@main
func main() -> lang.i64 {
    let g = Grid(size: 3);
    if lang.i1_not(lang.i64_eq(g(checked: 4), 4)) { return 1 }
    if lang.i1_not(lang.i64_eq(areaOf(g), 9)) { return 2 }
    match g.color() {
        .Red => 3,
        .Blue => 0
    }
}
