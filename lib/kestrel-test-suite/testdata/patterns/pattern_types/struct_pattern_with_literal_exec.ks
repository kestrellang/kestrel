// test: execution
// stdlib: true
// expect-exit: 0
//
// A struct pattern with a literal field tests that field at runtime. The
// decision tree reaches struct fields by position; they used to lower as a
// tuple extract and fail codegen ("TupleExtract on non-tuple").

module Test

import std.numeric.(Int64)

struct Rect { var w: Int64; var h: Int64; }

func classify(r: Rect) -> Int64 {
    match r {
        Rect { w: 0, h: 0 } => 1,
        Rect { w: 0, .. } => 2,
        Rect { w, h: 0 } => w * 10,
        Rect { w, h } => h * 100 + w,
    }
}

@main
func main() -> Int64 {
    if classify(Rect(w: 0, h: 0)) != 1 { return 1 }
    if classify(Rect(w: 0, h: 9)) != 2 { return 2 }
    if classify(Rect(w: 4, h: 0)) != 40 { return 3 }
    if classify(Rect(w: 2, h: 3)) != 302 { return 4 }
    0
}
