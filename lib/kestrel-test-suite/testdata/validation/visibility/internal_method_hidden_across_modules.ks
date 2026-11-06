// test: diagnostics
// stdlib: false
// include: default_visibility_lib.ks

// No visibility keyword means internal: `secret` is hidden outside `Lib`.
module Main

import Lib.Grid

func test() -> lang.i64 {
    let g = Grid(size: 3);
    g.secret() // ERROR: is internal and not accessible
}
