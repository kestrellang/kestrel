// test: execution
// stdlib: true
// expect-exit: 7

// Known bug: writing an element through a subscript and then returning the
// array as the tail expression fails OSSA verification ("cannot take from
// ValueId(..): active borrow"), as if the setter's borrow of `t` outlived the
// assignment. Returning through `let u = t; u` compiles and runs. Found
// building the SDL package's scancode table (lang/sdl/src/sdl.ks).

module Test

func build() -> Array[Int64] {
    var t = [0, 0, 0];
    t(1) = 7;
    t
}

@main
func main() -> Int64 {
    build()(1)
}
