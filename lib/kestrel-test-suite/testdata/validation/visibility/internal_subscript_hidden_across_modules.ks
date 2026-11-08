// test: diagnostics
// stdlib: false
// include: default_visibility_lib.ks

// A subscript with no visibility keyword is internal to its module, like
// every other member. It used to be callable from anywhere.
module Main

import Lib.Grid

func test() -> lang.i64 {
    let g = Grid(size: 3);
    g(2) // ERROR: is internal and not accessible
}
